//! Running `await all([...])`, `par_map`, `par_filter` and `par_fold` on
//! several processor cores.
//!
//! The values of the interpreter are shared through `Rc`, so they cannot
//! move between threads. Instead, a call to an `async fn` that is isolated
//! (see [`Interpreter::isolated`]), or a part of the list `par_map` goes
//! through with an isolated function, runs in a separate interpreter on its
//! own thread, with copies of its arguments and of the file-level constants;
//! its result is copied back.
//!
//! The results are the ones running the calls one after another would give:
//! - only calls that cannot be told apart from running alone are run in
//!   parallel: they print nothing, read no input, touch no file, database,
//!   process, Python, MCP or AI model, and use no random numbers (which can
//!   be seeded); they may compute and make HTTP requests (as `ai.infer` calls
//!   in `all` already do together);
//! - only neighbours in the list run together, so the order of everything
//!   else is kept;
//! - if a call fails, the first one in the list wins, as it would have
//!   stopped the others: the calls after it are stopped, and their futures
//!   are left as they were, not started (`par_map` likewise fails with the
//!   error of the first item that fails);
//! - the call depth limit counts the calls already running, and `timeout`s
//!   around `all` apply inside the threads.

use std::cell::RefCell;
use std::rc::Rc;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};

use elomar_ast::*;

use crate::value::EnumValue;
use crate::{
    BufferHost, Exec, FastMap, Flow, Function, FutureState, Interpreter, MODULES, MapKey,
    RuntimeError, Thunk, Value,
};

static SEQUENTIAL: AtomicBool = AtomicBool::new(false);

/// Lets `await all([...])` use several cores (the default), or not.
/// `elomar` turns it off when `ELOMAR_PARALLEL=0`.
pub fn set_parallel(on: bool) {
    SEQUENTIAL.store(!on, Ordering::Relaxed);
}

/// Whether `await all([...])` may use several cores.
pub fn parallel() -> bool {
    !SEQUENTIAL.load(Ordering::Relaxed)
}

/// Standard modules an isolated call may use.
const ISOLATED_MODULES: &[&str] = &[
    "std.math",
    "std.json",
    "std.regex",
    "std.encoding",
    "std.crypto",
    "std.csv",
    "std.time",
    "std.http",
];

/// The parallel state of one interpreter.
pub(crate) struct Par<'p> {
    /// Off in the threads themselves, under the debugger and in the browser.
    pub(crate) enabled: bool,
    /// What each function does by itself: whether its own body is isolated,
    /// and the functions it calls or uses as values.
    facts: FastMap<&'p str, (bool, Vec<&'p str>)>,
    isolated: FastMap<&'p str, bool>,
    /// Closure expressions by address, to rebuild closures made in a thread.
    lambdas: Option<FastMap<usize, &'p Expr>>,
}

impl Par<'_> {
    pub(crate) fn new() -> Self {
        Par {
            enabled: parallel() && cfg!(not(target_family = "wasm")),
            facts: FastMap::default(),
            isolated: FastMap::default(),
            lambdas: None,
        }
    }
}

// ----- which calls are isolated ------------------------------------------

/// Calls `f` on every expression in `b`, closures' bodies included.
fn each_expr_in_block<'p, F: FnMut(&'p Expr) + ?Sized>(b: &'p Block, f: &mut F) {
    for s in &b.stmts {
        each_expr_in_stmt(s, f);
    }
}

fn each_expr_in_stmt<'p, F: FnMut(&'p Expr) + ?Sized>(s: &'p Stmt, f: &mut F) {
    match &s.kind {
        StmtKind::Let { value, .. } | StmtKind::LetTuple { value, .. } => each_expr(value, f),
        StmtKind::Assign { target, value, .. } => {
            each_expr(target, f);
            each_expr(value, f);
        }
        StmtKind::If {
            cond,
            then,
            otherwise,
        } => {
            each_expr(cond, f);
            each_expr_in_block(then, f);
            match otherwise {
                Some(Else::Block(b)) => each_expr_in_block(b, f),
                Some(Else::If(s)) => each_expr_in_stmt(s, f),
                None => {}
            }
        }
        StmtKind::While { cond, body } => {
            each_expr(cond, f);
            each_expr_in_block(body, f);
        }
        StmtKind::For { iter, body, .. } => {
            each_expr(iter, f);
            each_expr_in_block(body, f);
        }
        StmtKind::Return(value) => {
            if let Some(e) = value {
                each_expr(e, f);
            }
        }
        StmtKind::Block(b) => each_expr_in_block(b, f),
        StmtKind::Expr(e) => each_expr(e, f),
        StmtKind::Break | StmtKind::Continue => {}
    }
}

fn each_expr<'p, F: FnMut(&'p Expr) + ?Sized>(e: &'p Expr, f: &mut F) {
    f(e);
    match &e.kind {
        ExprKind::Interp(parts) => {
            for p in parts {
                if let InterpPart::Expr(x) = p {
                    each_expr(x, f);
                }
            }
        }
        ExprKind::Tuple(items) | ExprKind::List(items) => {
            for x in items {
                each_expr(x, f);
            }
        }
        ExprKind::Variant { args, .. } => {
            for x in args {
                each_expr(x, f);
            }
        }
        ExprKind::Record(fields) => {
            for x in fields {
                each_expr(&x.value, f);
            }
        }
        ExprKind::Unary { expr, .. }
        | ExprKind::Await(expr)
        | ExprKind::Try(expr)
        | ExprKind::Field { target: expr, .. } => each_expr(expr, f),
        ExprKind::Binary { lhs, rhs, .. } => {
            each_expr(lhs, f);
            each_expr(rhs, f);
        }
        ExprKind::Range { start, end, .. } => {
            each_expr(start, f);
            each_expr(end, f);
        }
        ExprKind::Index { target, index } => {
            each_expr(target, f);
            each_expr(index, f);
        }
        ExprKind::Call { callee, args, .. } => {
            each_expr(callee, f);
            for a in args {
                each_expr(&a.value, f);
            }
        }
        ExprKind::Map(entries) => {
            for (k, v) in entries {
                each_expr(k, f);
                each_expr(v, f);
            }
        }
        ExprKind::Match { scrutinee, arms } => {
            each_expr(scrutinee, f);
            for arm in arms {
                match &arm.body {
                    ArmBody::Expr(x) => each_expr(x, f),
                    ArmBody::Block(b) => each_expr_in_block(b, f),
                }
            }
        }
        ExprKind::Lambda { body, .. } => match &**body {
            ArmBody::Expr(x) => each_expr(x, f),
            ArmBody::Block(b) => each_expr_in_block(b, f),
        },
        ExprKind::Int(_)
        | ExprKind::Float(_)
        | ExprKind::Str(_)
        | ExprKind::Bool(_)
        | ExprKind::Duration(_)
        | ExprKind::Name(_) => {}
    }
}

impl<'p> Interpreter<'p, '_> {
    /// Whether `f`'s body is isolated by itself, and the functions it uses.
    fn facts(&mut self, f: &'p FnDecl) -> (bool, Vec<&'p str>) {
        self.scan(|visit| each_expr_in_block(&f.body, visit))
    }

    /// Whether the expressions `walk` visits are isolated by themselves, and
    /// the functions they use.
    fn scan(&self, walk: impl FnOnce(&mut dyn FnMut(&'p Expr))) -> (bool, Vec<&'p str>) {
        let fns = &self.fns;
        let mut ok = true;
        let mut uses = Vec::new();
        walk(&mut |e| match &e.kind {
            ExprKind::Name(n) => {
                if let Some((name, _)) = fns.get_key_value(n.as_str()) {
                    uses.push(*name);
                }
            }
            ExprKind::Call { callee, .. } => match &callee.kind {
                ExprKind::Name(n) if n == "print" || n == "input" => ok = false,
                ExprKind::Field { target, .. } => {
                    if let ExprKind::Name(m) = &target.kind
                        && MODULES.contains(&m.as_str())
                        && !ISOLATED_MODULES.contains(&m.as_str())
                    {
                        ok = false;
                    }
                }
                _ => {}
            },
            _ => {}
        });
        (ok, uses)
    }

    /// Whether the closure expression `e` is isolated: its body, and every
    /// function it may call.
    fn lambda_isolated(&mut self, e: &'p Expr) -> bool {
        let ExprKind::Lambda { body, .. } = &e.kind else {
            return false;
        };
        let (own, uses) = self.scan(|visit| match &**body {
            ArmBody::Expr(x) => each_expr(x, visit),
            ArmBody::Block(b) => each_expr_in_block(b, visit),
        });
        own && uses.iter().all(|n| self.isolated(n))
    }

    /// Whether calling `name` can be told apart from running it alone: see
    /// the module documentation. Every function it may call is checked.
    pub(crate) fn isolated(&mut self, name: &str) -> bool {
        let Some((&root, _)) = self.fns.get_key_value(name) else {
            return false;
        };
        if let Some(&known) = self.par.isolated.get(root) {
            return known;
        }
        let mut seen: FastMap<&'p str, ()> = FastMap::default();
        let mut stack = vec![root];
        let mut ok = true;
        while let Some(n) = stack.pop() {
            if seen.insert(n, ()).is_some() {
                continue;
            }
            if !self.par.facts.contains_key(n) {
                let f = self.fns[n];
                let facts = self.facts(f);
                self.par.facts.insert(n, facts);
            }
            let (own, uses) = &self.par.facts[n];
            if !own {
                ok = false;
                break;
            }
            stack.extend(uses.iter().copied());
        }
        self.par.isolated.insert(root, ok);
        ok
    }
}

// ----- values that cross threads -----------------------------------------

/// A copy of a [`Value`] that can move to another thread.
pub(crate) enum Portable {
    Int(i64),
    Float(f64),
    Bool(bool),
    Str(String),
    Unit,
    List(Vec<Portable>),
    Record(Vec<(String, Portable)>),
    Tuple(Vec<Portable>),
    Range {
        start: i64,
        end: i64,
        inclusive: bool,
    },
    Duration(i64),
    Enum(String, String, Vec<Portable>),
    Map(Vec<(Key, Portable)>),
    Set(Vec<Key>),
    Option(Option<Box<Portable>>),
    Result(Result<Box<Portable>, Box<Portable>>),
    Named(String),
    Closure {
        key: usize,
        captured: Vec<(String, Portable)>,
    },
    /// The `i`th future of the copy: copies of one future stay one future.
    Future(usize),
}

pub(crate) enum Key {
    Int(i64),
    Str(String),
}

pub(crate) enum PortableFuture {
    Call {
        name: String,
        args: Vec<Portable>,
        span: Span,
    },
    All(Vec<Portable>),
    Timeout(Box<Portable>, i64),
    Done(Portable),
    Cancelled,
}

/// Copies values out; `futures` collects the futures met, each once.
struct Out<'a> {
    /// Whether closures and futures may be copied (results), or not
    /// (arguments and constants, whose code was not checked).
    everything: bool,
    named_ok: &'a dyn Fn(&str) -> bool,
    seen: FastMap<usize, usize>,
    futures: Vec<Option<PortableFuture>>,
}

impl Out<'_> {
    fn copy(&mut self, v: &Value) -> Option<Portable> {
        Some(match v {
            Value::Int(n) => Portable::Int(*n),
            Value::Float(x) => Portable::Float(*x),
            Value::Bool(b) => Portable::Bool(*b),
            Value::Str(s) => Portable::Str(s.to_string()),
            Value::Unit => Portable::Unit,
            Value::List(items) => Portable::List(self.all(items)?),
            Value::Tuple(items) => Portable::Tuple(self.all(items)?),
            Value::Record(fields) => Portable::Record(
                fields
                    .iter()
                    .map(|(n, x)| Some((n.to_string(), self.copy(x)?)))
                    .collect::<Option<_>>()?,
            ),
            Value::Range {
                start,
                end,
                inclusive,
            } => Portable::Range {
                start: *start,
                end: *end,
                inclusive: *inclusive,
            },
            Value::Duration(ms) => Portable::Duration(*ms),
            Value::Enum(e) => Portable::Enum(
                e.enum_name.to_string(),
                e.variant.to_string(),
                self.all(&e.fields)?,
            ),
            Value::Map(entries) => Portable::Map(
                entries
                    .iter()
                    .map(|(k, x)| Some((key(k), self.copy(x)?)))
                    .collect::<Option<_>>()?,
            ),
            Value::Set(items) => Portable::Set(items.iter().map(key).collect()),
            Value::Option(o) => Portable::Option(match o {
                Some(x) => Some(Box::new(self.copy(x)?)),
                None => None,
            }),
            Value::Result(r) => Portable::Result(match r {
                Ok(x) => Ok(Box::new(self.copy(x)?)),
                Err(x) => Err(Box::new(self.copy(x)?)),
            }),
            Value::Function(f) => match &**f {
                Function::Named(name) if (self.named_ok)(name) => Portable::Named(name.clone()),
                Function::Closure { key, captured } if self.everything => Portable::Closure {
                    key: *key,
                    captured: captured
                        .iter()
                        .map(|(n, x)| Some((n.to_string(), self.copy(x)?)))
                        .collect::<Option<_>>()?,
                },
                _ => return None,
            },
            Value::Future(state) if self.everything => {
                let address = Rc::as_ptr(state) as usize;
                if let Some(&i) = self.seen.get(&address) {
                    return Some(Portable::Future(i));
                }
                let i = self.futures.len();
                self.seen.insert(address, i);
                self.futures.push(None);
                let copy = match &*state.borrow() {
                    FutureState::Pending(Thunk::Call { name, args, span }) => {
                        PortableFuture::Call {
                            name: name.clone(),
                            args: self.all(args)?,
                            span: *span,
                        }
                    }
                    FutureState::Pending(Thunk::All(items)) => {
                        PortableFuture::All(self.all(items)?)
                    }
                    FutureState::Pending(Thunk::Timeout(f, ms)) => {
                        PortableFuture::Timeout(Box::new(self.copy(f)?), *ms)
                    }
                    FutureState::Done(x) => PortableFuture::Done(self.copy(x)?),
                    FutureState::Cancelled => PortableFuture::Cancelled,
                    _ => return None,
                };
                self.futures[i] = Some(copy);
                Portable::Future(i)
            }
            Value::Future(_) | Value::Db(_) => return None,
        })
    }

    fn all(&mut self, items: &[Value]) -> Option<Vec<Portable>> {
        items.iter().map(|x| self.copy(x)).collect()
    }
}

fn key(k: &MapKey) -> Key {
    match k {
        MapKey::Int(n) => Key::Int(*n),
        MapKey::Str(s) => Key::Str(s.to_string()),
    }
}

/// Copies values back in; copies of one future become one future again.
struct In<'a, 'p> {
    lambdas: &'a FastMap<usize, &'p Expr>,
    registered: &'a mut FastMap<usize, &'p Expr>,
    futures: Vec<Option<PortableFuture>>,
    made: Vec<Option<Rc<RefCell<FutureState>>>>,
}

impl In<'_, '_> {
    fn value(&mut self, p: Portable) -> Value {
        match p {
            Portable::Int(n) => Value::Int(n),
            Portable::Float(x) => Value::Float(x),
            Portable::Bool(b) => Value::Bool(b),
            Portable::Str(s) => Value::str(s),
            Portable::Unit => Value::Unit,
            Portable::List(items) => Value::list(self.all(items)),
            Portable::Tuple(items) => Value::Tuple(Rc::new(self.all(items))),
            Portable::Record(fields) => Value::Record(Rc::new(
                fields
                    .into_iter()
                    .map(|(n, x)| (Rc::from(n), self.value(x)))
                    .collect(),
            )),
            Portable::Range {
                start,
                end,
                inclusive,
            } => Value::Range {
                start,
                end,
                inclusive,
            },
            Portable::Duration(ms) => Value::Duration(ms),
            Portable::Enum(e, v, fields) => Value::Enum(Rc::new(EnumValue {
                enum_name: e.into(),
                variant: v.into(),
                fields: self.all(fields),
            })),
            Portable::Map(entries) => Value::Map(Rc::new(
                entries
                    .into_iter()
                    .map(|(k, x)| (map_key(k), self.value(x)))
                    .collect(),
            )),
            Portable::Set(items) => Value::Set(Rc::new(items.into_iter().map(map_key).collect())),
            Portable::Option(o) => Value::Option(o.map(|x| Rc::new(self.value(*x)))),
            Portable::Result(r) => Value::Result(match r {
                Ok(x) => Ok(Rc::new(self.value(*x))),
                Err(x) => Err(Rc::new(self.value(*x))),
            }),
            Portable::Named(name) => Value::Function(Rc::new(Function::Named(name))),
            Portable::Closure { key, captured } => {
                if let Some(e) = self.lambdas.get(&key) {
                    self.registered.insert(key, e);
                }
                Value::Function(Rc::new(Function::Closure {
                    key,
                    captured: Rc::new(
                        captured
                            .into_iter()
                            .map(|(n, x)| (Rc::from(n), self.value(x)))
                            .collect(),
                    ),
                }))
            }
            Portable::Future(i) => Value::Future(self.future(i)),
        }
    }

    fn future(&mut self, i: usize) -> Rc<RefCell<FutureState>> {
        if let Some(Some(made)) = self.made.get(i) {
            return made.clone();
        }
        // Made before its parts, so a part that refers back to it finds it.
        let state = Rc::new(RefCell::new(FutureState::Cancelled));
        if self.made.len() <= i {
            self.made.resize(i + 1, None);
        }
        self.made[i] = Some(state.clone());
        let copy = self.futures[i].take();
        let inner = match copy {
            Some(PortableFuture::Call { name, args, span }) => FutureState::Pending(Thunk::Call {
                name,
                args: self.all(args),
                span,
            }),
            Some(PortableFuture::All(items)) => {
                FutureState::Pending(Thunk::All(Rc::new(self.all(items))))
            }
            Some(PortableFuture::Timeout(f, ms)) => {
                FutureState::Pending(Thunk::Timeout(self.value(*f), ms))
            }
            Some(PortableFuture::Done(x)) => FutureState::Done(self.value(x)),
            Some(PortableFuture::Cancelled) | None => FutureState::Cancelled,
        };
        *state.borrow_mut() = inner;
        state
    }

    fn all(&mut self, items: Vec<Portable>) -> Vec<Value> {
        items.into_iter().map(|x| self.value(x)).collect()
    }
}

fn map_key(k: Key) -> MapKey {
    match k {
        Key::Int(n) => MapKey::Int(n),
        Key::Str(s) => MapKey::Str(s.into()),
    }
}

// ----- running -----------------------------------------------------------

/// How one call ended, in its thread.
enum Outcome {
    Done(Portable, Vec<Option<PortableFuture>>),
    Error(Box<RuntimeError>),
    /// The time of the `i`th `timeout` around `all` was up.
    Timeout(usize),
    /// Stopped because a call before it failed; or never started.
    Stopped,
}

/// What one thread runs at a time, and the call site.
struct Task {
    job: Job,
    span: Span,
}

enum Job {
    /// A call to the `async fn` `name`.
    Call { name: String, args: Vec<Portable> },
    /// `f` on each item of a part of a list, for `par_map` and `par_filter`.
    Map { f: Portable, items: Vec<Portable> },
    /// `f` from the first item of a part over the others, for `par_fold`.
    Fold { f: Portable, items: Vec<Portable> },
}

/// The value used to stop a thread's call: never reaches the program.
pub(crate) const STOPPED: usize = usize::MAX;

impl<'p> Interpreter<'p, '_> {
    /// The calls in `items` that may run in parallel: pending calls to
    /// isolated `async fn`s whose arguments can be copied. `None` for the
    /// others.
    fn task_for(&mut self, item: &Value) -> Option<Task> {
        let Value::Future(state) = item else {
            return None;
        };
        let (name, args, span) = match &*state.borrow() {
            FutureState::Pending(Thunk::Call { name, args, span }) => {
                (name.clone(), args.clone(), *span)
            }
            _ => return None,
        };
        if !self.isolated(&name) {
            return None;
        }
        let args = self.copy_out(&args)?;
        Some(Task {
            job: Job::Call { name, args },
            span,
        })
    }

    /// Copies of `values` for a thread; `None` if one holds something that
    /// cannot be copied or whose code was not checked.
    fn copy_out(&mut self, values: &[Value]) -> Option<Vec<Portable>> {
        // Named functions are copied when they are isolated themselves.
        let mut named: Vec<String> = Vec::new();
        collect_named(values, &mut named);
        if !named.iter().all(|n| self.isolated(n)) {
            return None;
        }
        let ok = |_: &str| true;
        let mut out = Out {
            everything: false,
            named_ok: &ok,
            seen: FastMap::default(),
            futures: Vec::new(),
        };
        out.all(values)
    }

    /// Copies of the file-level constants for the threads; `None` if one
    /// cannot be copied.
    fn copy_globals(&mut self) -> Option<Vec<(String, Portable)>> {
        let constants: Vec<(String, Value)> = self
            .globals
            .iter()
            .map(|(n, v)| (n.clone(), v.clone()))
            .collect();
        let mut globals = Vec::with_capacity(constants.len());
        for (name, v) in &constants {
            let copy = self.copy_out(std::slice::from_ref(v))?;
            globals.push((name.clone(), copy.into_iter().next().expect("one value")));
        }
        Some(globals)
    }

    /// `items.par_map(f)` on several cores: `None` when it should run like
    /// `map` instead (parallelism is off, the list is short, `f` is not
    /// isolated, or something cannot be copied). The result and the error
    /// are the ones `map` gives. `par_filter` keeps the items whose result is
    /// `true`.
    pub(crate) fn par_map(
        &mut self,
        items: &[Value],
        f: &Function,
        span: Span,
    ) -> Exec<Option<Value>> {
        if items.len() < 2 {
            return Ok(None);
        }
        // Several parts per thread, so a thread that finishes early takes
        // another.
        let size = items.len().div_ceil((threads() * 4).min(items.len()));
        let parts = items.chunks(size).map(<[Value]>::to_vec).collect();
        let Some(results) = self.run_parts(parts, f, false, span)? else {
            return Ok(None);
        };
        let mut out = Vec::with_capacity(items.len());
        for part in results {
            let Value::List(values) = part else {
                unreachable!("a part's result is a list")
            };
            out.extend(values.iter().cloned());
        }
        Ok(Some(Value::list(out)))
    }

    /// The results of the parts of `items.par_fold(init, f)` (see
    /// [`fold_parts`]), each folded on its own core; `None` when they should
    /// be folded here instead.
    pub(crate) fn par_fold(
        &mut self,
        init: &Value,
        items: &[Value],
        f: &Function,
        span: Span,
    ) -> Exec<Option<Vec<Value>>> {
        if items.len() < 2 {
            return Ok(None);
        }
        self.run_parts(fold_parts(init, items), f, true, span)
    }

    /// Runs `f` over each part on the threads (`fold`: from its first item
    /// over the others; otherwise on each item, giving a list). The results
    /// are in the order of the parts, and the error is the first part's.
    fn run_parts(
        &mut self,
        parts: Vec<Vec<Value>>,
        f: &Function,
        fold: bool,
        span: Span,
    ) -> Exec<Option<Vec<Value>>> {
        if !self.par.enabled || self.debugger.is_some() {
            return Ok(None);
        }
        let Some(f) = self.copy_function(f) else {
            return Ok(None);
        };
        let mut tasks = Vec::with_capacity(parts.len());
        for part in &parts {
            let Some(items) = self.copy_out(part) else {
                return Ok(None);
            };
            let f = copy_portable(&f);
            tasks.push(Task {
                job: match fold {
                    true => Job::Fold { f, items },
                    false => Job::Map { f, items },
                },
                span,
            });
        }
        let Some(globals) = self.copy_globals() else {
            return Ok(None);
        };
        let outcomes = self.run_threads(tasks, globals);
        let lambdas = self.lambda_index();
        let mut out = Vec::with_capacity(parts.len());
        for outcome in outcomes {
            match outcome {
                Outcome::Done(value, futures) => {
                    let mut back = In {
                        lambdas: &lambdas,
                        registered: &mut self.lambdas,
                        made: Vec::new(),
                        futures,
                    };
                    out.push(back.value(value));
                }
                Outcome::Error(err) => return Err(Flow::Error(err)),
                Outcome::Timeout(level) => return Err(Flow::Timeout(level)),
                Outcome::Stopped => unreachable!("only parts after a failure are stopped"),
            }
        }
        Ok(Some(out))
    }

    /// A copy of `f` for the threads; `None` if it is an `async fn`, is not
    /// isolated, or holds something that cannot be copied.
    fn copy_function(&mut self, f: &Function) -> Option<Portable> {
        Some(match f {
            Function::Named(name) => {
                let async_fn = self.fns.get(name.as_str()).is_none_or(|d| d.is_async);
                if async_fn || !self.isolated(name) {
                    return None;
                }
                Portable::Named(name.clone())
            }
            Function::Closure { key, captured } => {
                let &e = self.lambdas.get(key)?;
                if !self.lambda_isolated(e) {
                    return None;
                }
                // A closure holds every variable visible where it was made;
                // only the ones its body (or a closure in it) names are
                // copied to the threads, the others become `()`. The names
                // stay, in order: compiled code finds a variable by its
                // place, and closures made in a thread come back here.
                let read = names_in(e);
                let values: Vec<Value> = captured
                    .iter()
                    .map(|(n, v)| match read.contains_key(&**n) {
                        true => v.clone(),
                        false => Value::Unit,
                    })
                    .collect();
                let values = self.copy_out(&values)?;
                Portable::Closure {
                    key: *key,
                    captured: captured
                        .iter()
                        .map(|(n, _)| n.to_string())
                        .zip(values)
                        .collect(),
                }
            }
        })
    }

    /// `await all([...])` for `items[from..]`: runs the neighbouring calls
    /// that may run in parallel from `from`, if there are at least two.
    /// Returns how many items it handled (0 if none), with their results put
    /// in `results`, or the error of the first that failed.
    pub(crate) fn run_parallel(
        &mut self,
        items: &[Value],
        from: usize,
        results: &mut [Option<Value>],
    ) -> Exec<usize> {
        if !self.par.enabled || self.debugger.is_some() {
            return Ok(0);
        }
        let mut tasks = Vec::new();
        let mut seen: Vec<usize> = Vec::new();
        for (i, item) in items.iter().enumerate().skip(from) {
            if results[i].is_some() {
                break;
            }
            // The same future twice runs once: stop the group there.
            let Value::Future(state) = item else { break };
            let address = Rc::as_ptr(state) as usize;
            if seen.contains(&address) {
                break;
            }
            let Some(task) = self.task_for(item) else {
                break;
            };
            seen.push(address);
            tasks.push(task);
        }
        if tasks.len() < 2 {
            return Ok(0);
        }
        let Some(globals) = self.copy_globals() else {
            return Ok(0);
        };
        let count = tasks.len();
        let outcomes = self.run_threads(tasks, globals);
        let lambdas = self.lambda_index();
        for (k, outcome) in outcomes.into_iter().enumerate() {
            let i = from + k;
            let Value::Future(state) = &items[i] else {
                unreachable!("tasks are futures")
            };
            match outcome {
                Outcome::Done(value, futures) => {
                    let mut back = In {
                        lambdas: &lambdas,
                        registered: &mut self.lambdas,
                        made: Vec::new(),
                        futures,
                    };
                    let value = back.value(value);
                    *state.borrow_mut() = FutureState::Done(value.clone());
                    results[i] = Some(value);
                }
                // As if run alone: the failed future was left running, and
                // the ones after it were not started.
                Outcome::Error(err) => {
                    *state.borrow_mut() = FutureState::Running;
                    return Err(Flow::Error(err));
                }
                Outcome::Timeout(level) => {
                    *state.borrow_mut() = FutureState::Running;
                    return Err(Flow::Timeout(level));
                }
                Outcome::Stopped => unreachable!("only calls after a failure are stopped"),
            }
        }
        Ok(count)
    }

    /// Closure expressions in the program, by address.
    fn lambda_index(&mut self) -> FastMap<usize, &'p Expr> {
        if self.par.lambdas.is_none() {
            let mut index = FastMap::default();
            let mut add = |e: &'p Expr| {
                if matches!(e.kind, ExprKind::Lambda { .. }) {
                    index.insert(e as *const Expr as usize, e);
                }
            };
            for item in &self.program.items {
                match item {
                    Item::Fn(f) => each_expr_in_block(&f.body, &mut add),
                    Item::Const(c) => each_expr(&c.value, &mut add),
                    Item::Impl(i) => {
                        for m in &i.methods {
                            each_expr_in_block(&m.body, &mut add);
                        }
                    }
                    Item::Trait(t) => {
                        for m in &t.methods {
                            if let Some(b) = &m.body {
                                each_expr_in_block(b, &mut add);
                            }
                        }
                    }
                    _ => {}
                }
            }
            self.par.lambdas = Some(index);
        }
        self.par.lambdas.clone().unwrap_or_default()
    }

    /// Runs `tasks` on threads, each in its own interpreter, and returns how
    /// each ended, in order. When one fails, the ones after it are stopped.
    fn run_threads(&mut self, tasks: Vec<Task>, globals: Vec<(String, Portable)>) -> Vec<Outcome> {
        let program = self.program;
        let lambdas = self.lambda_index();
        let lambdas = &lambdas;
        let count = tasks.len();
        let stop: Vec<Arc<AtomicBool>> = (0..count).map(|_| Arc::default()).collect();
        let next = AtomicUsize::new(0);
        let outcomes: Mutex<Vec<Option<Outcome>>> = Mutex::new((0..count).map(|_| None).collect());
        let tasks: Vec<Mutex<Option<Task>>> =
            tasks.into_iter().map(|t| Mutex::new(Some(t))).collect();
        let deadlines = self.deadlines.clone();
        let depth = self.frames.len() - 1 + self.vm.depth;
        let permissions = self.ai.permissions().clone();
        let fs = self.fs.clone();
        let threads = threads().min(count);
        let globals = &globals;
        std::thread::scope(|scope| {
            for _ in 0..threads {
                let (stop, next, outcomes, tasks) = (&stop, &next, &outcomes, &tasks);
                let (deadlines, permissions, fs) = (&deadlines, &permissions, &fs);
                std::thread::Builder::new()
                    .stack_size(crate::STACK_SIZE)
                    .spawn_scoped(scope, move || {
                        let mut host = BufferHost::default();
                        let mut ai = elomar_ai_core::AiRuntime::default()
                            .with_permissions(permissions.clone());
                        let mut interp = Interpreter::new(program, &mut host, &mut ai, fs.clone());
                        interp.par.enabled = false;
                        let mut back = In {
                            lambdas: &FastMap::default(),
                            registered: &mut FastMap::default(),
                            futures: Vec::new(),
                            made: Vec::new(),
                        };
                        for (name, v) in globals {
                            let v = back.value(copy_portable(v));
                            interp.globals.insert(name.clone(), v);
                        }
                        loop {
                            let i = next.fetch_add(1, Ordering::Relaxed);
                            if i >= count {
                                break;
                            }
                            let task = tasks[i].lock().expect("task").take().expect("once");
                            let outcome = if stop[i].load(Ordering::Relaxed) {
                                Outcome::Stopped
                            } else {
                                interp.run_task(task, lambdas, &stop[i], deadlines, depth)
                            };
                            let failed = !matches!(outcome, Outcome::Done(..) | Outcome::Stopped);
                            if failed {
                                for s in &stop[i + 1..] {
                                    s.store(true, Ordering::Relaxed);
                                }
                            }
                            outcomes.lock().expect("outcomes")[i] = Some(outcome);
                        }
                    })
                    .expect("start a thread");
            }
        });
        outcomes
            .into_inner()
            .expect("outcomes")
            .into_iter()
            .map(|o| o.unwrap_or(Outcome::Stopped))
            .collect()
    }

    /// Runs one call in this (thread's) interpreter.
    fn run_task(
        &mut self,
        task: Task,
        lambdas: &FastMap<usize, &'p Expr>,
        stop: &Arc<AtomicBool>,
        deadlines: &[i64],
        depth: usize,
    ) -> Outcome {
        self.frames.truncate(1);
        self.calls.clear();
        self.vm.stack.clear();
        self.vm.depth = depth;
        self.deadlines = deadlines.to_vec();
        self.stop = Some(stop.clone());
        let mut back = In {
            lambdas,
            registered: &mut self.lambdas,
            futures: Vec::new(),
            made: Vec::new(),
        };
        let result = match task.job {
            Job::Call { name, args } => {
                let args = back.all(args);
                let f = self.fns[name.as_str()];
                self.call_fn(f, args, task.span)
            }
            Job::Fold { f, items } => {
                let Value::Function(f) = back.value(f) else {
                    unreachable!("par_fold copies a function")
                };
                let items = back.all(items);
                self.prepare_closure(&f);
                self.fold_part(&f, items, task.span)
            }
            Job::Map { f, items } => {
                let f = back.value(f);
                let items = back.all(items);
                let Value::Function(f) = f else {
                    unreachable!("par_map copies a function")
                };
                self.prepare_closure(&f);
                let mut out = Vec::with_capacity(items.len());
                let mut failed = None;
                for x in items {
                    match self.call_value(&f, vec![x], task.span) {
                        Ok(v) => out.push(v),
                        Err(e) => {
                            failed = Some(e);
                            break;
                        }
                    }
                }
                match failed {
                    Some(e) => Err(e),
                    None => Ok(Value::list(out)),
                }
            }
        };
        match result {
            Ok(v) => {
                let named = |_: &str| true;
                let mut out = Out {
                    everything: true,
                    named_ok: &named,
                    seen: FastMap::default(),
                    futures: Vec::new(),
                };
                match out.copy(&v) {
                    Some(p) => Outcome::Done(p, out.futures),
                    None => Outcome::Error(Box::new(RuntimeError::new(
                        "R0009",
                        task.span,
                        elomar_diagnostics::Text::new(
                            "internal error: the result of a parallel call could not be copied",
                            "خطأ داخلي: تعذّر نسخ ناتج استدعاء متوازٍ",
                        ),
                    ))),
                }
            }
            Err(Flow::Error(e)) => Outcome::Error(e),
            Err(Flow::Timeout(STOPPED)) => Outcome::Stopped,
            Err(Flow::Timeout(level)) => Outcome::Timeout(level),
            Err(other) => Outcome::Error(Box::new(other.into_error())),
        }
    }
}

/// The names the closure expression `e` reads, in its body and in the
/// closures inside it.
fn names_in(e: &Expr) -> FastMap<&str, ()> {
    let mut names = FastMap::default();
    each_expr(e, &mut |x| {
        if let ExprKind::Name(n) = &x.kind {
            names.insert(n.as_str(), ());
        }
    });
    names
}

/// How many threads to use.
/// The parts `par_fold` folds on their own, then together: at most
/// [`FOLD_PARTS`] neighbouring runs of `items`, the first after `init`. They
/// depend on the length alone, so the result is the same on every machine,
/// with or without threads; it is `fold`'s when `f` is associative.
pub(crate) fn fold_parts(init: &Value, items: &[Value]) -> Vec<Vec<Value>> {
    let mut parts = vec![vec![init.clone()]];
    if items.is_empty() {
        return parts;
    }
    let size = items.len().div_ceil(items.len().min(FOLD_PARTS));
    for (i, chunk) in items.chunks(size).enumerate() {
        if i == 0 {
            parts[0].extend(chunk.iter().cloned());
        } else {
            parts.push(chunk.to_vec());
        }
    }
    parts
}

/// How many parts `par_fold` splits a list into, at most.
const FOLD_PARTS: usize = 64;

fn threads() -> usize {
    std::thread::available_parallelism()
        .map_or(4, |n| n.get())
        .clamp(2, 16)
}

/// The names of the functions held by `values`.
fn collect_named(values: &[Value], out: &mut Vec<String>) {
    for v in values {
        match v {
            Value::Function(f) => {
                if let Function::Named(n) = &**f {
                    out.push(n.clone());
                }
            }
            Value::List(items) | Value::Tuple(items) => collect_named(items, out),
            Value::Record(fields) => {
                for (_, x) in fields.iter() {
                    collect_named(std::slice::from_ref(x), out);
                }
            }
            Value::Enum(e) => collect_named(&e.fields, out),
            Value::Map(entries) => {
                for x in entries.values() {
                    collect_named(std::slice::from_ref(x), out);
                }
            }
            Value::Option(Some(x)) | Value::Result(Ok(x) | Err(x)) => {
                collect_named(std::slice::from_ref(&**x), out);
            }
            _ => {}
        }
    }
}

/// A copy of a portable value (constants go to every thread).
fn copy_portable(p: &Portable) -> Portable {
    match p {
        Portable::Int(n) => Portable::Int(*n),
        Portable::Float(x) => Portable::Float(*x),
        Portable::Bool(b) => Portable::Bool(*b),
        Portable::Str(s) => Portable::Str(s.clone()),
        Portable::Unit => Portable::Unit,
        Portable::List(items) => Portable::List(items.iter().map(copy_portable).collect()),
        Portable::Tuple(items) => Portable::Tuple(items.iter().map(copy_portable).collect()),
        Portable::Record(fields) => Portable::Record(
            fields
                .iter()
                .map(|(n, x)| (n.clone(), copy_portable(x)))
                .collect(),
        ),
        Portable::Range {
            start,
            end,
            inclusive,
        } => Portable::Range {
            start: *start,
            end: *end,
            inclusive: *inclusive,
        },
        Portable::Duration(ms) => Portable::Duration(*ms),
        Portable::Enum(e, v, fields) => Portable::Enum(
            e.clone(),
            v.clone(),
            fields.iter().map(copy_portable).collect(),
        ),
        Portable::Map(entries) => Portable::Map(
            entries
                .iter()
                .map(|(k, x)| (copy_key(k), copy_portable(x)))
                .collect(),
        ),
        Portable::Set(items) => Portable::Set(items.iter().map(copy_key).collect()),
        Portable::Option(o) => Portable::Option(o.as_ref().map(|x| Box::new(copy_portable(x)))),
        Portable::Result(r) => Portable::Result(match r {
            Ok(x) => Ok(Box::new(copy_portable(x))),
            Err(x) => Err(Box::new(copy_portable(x))),
        }),
        Portable::Named(n) => Portable::Named(n.clone()),
        Portable::Closure { key, captured } => Portable::Closure {
            key: *key,
            captured: captured
                .iter()
                .map(|(n, x)| (n.clone(), copy_portable(x)))
                .collect(),
        },
        Portable::Future(i) => Portable::Future(*i),
    }
}

fn copy_key(k: &Key) -> Key {
    match k {
        Key::Int(n) => Key::Int(*n),
        Key::Str(s) => Key::Str(s.clone()),
    }
}
