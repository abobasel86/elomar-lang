//! Tree-walking interpreter for Elomar-Lang (spec §7, phase A).
//!
//! The interpreter assumes the program passed `elomar_types::check`; it
//! trusts static types and only checks what can fail at run time (division
//! by zero, overflow, bounds, recursion depth).

mod ai;
mod builtins;
mod stdlib;
mod value;

use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

use elomar_ai_core::AiRuntime;
pub use elomar_policy::FsPolicy;

use elomar_ast::*;
use elomar_diagnostics::{Diagnostic, Text};

pub use ai::InferCall;
pub use value::{FutureState, MapKey, Thunk, Value, format_duration, format_float};

/// Services the running program can use. Implemented by the CLI with real
/// stdin/stdout and by tests with in-memory buffers.
pub trait Host {
    fn print(&mut self, text: &str);
    /// Shows `prompt` and reads one line, without its line terminator.
    /// `None` at end of input.
    fn input(&mut self, prompt: &str) -> Option<String>;
}

/// A [`Host`] that records output and replays canned input; for tests.
#[derive(Debug, Default)]
pub struct BufferHost {
    pub output: String,
    pub inputs: Vec<String>,
}

impl Host for BufferHost {
    fn print(&mut self, text: &str) {
        self.output.push_str(text);
    }

    fn input(&mut self, prompt: &str) -> Option<String> {
        self.output.push_str(prompt);
        if self.inputs.is_empty() {
            None
        } else {
            Some(self.inputs.remove(0))
        }
    }
}

#[derive(Debug, Clone)]
pub struct RuntimeError {
    pub code: &'static str,
    pub message: Text,
    pub span: Span,
    /// Call sites from innermost to outermost: (function name, call span).
    pub trace: Vec<(String, Span)>,
}

impl RuntimeError {
    pub fn new(code: &'static str, span: Span, message: Text) -> Self {
        RuntimeError {
            code,
            message,
            span,
            trace: Vec::new(),
        }
    }

    /// As a diagnostic, with call sites as secondary labels.
    pub fn to_diagnostic(&self) -> Diagnostic {
        let mut d = Diagnostic::error(self.code, self.span, self.message.clone());
        for (name, span) in self.trace.iter().take(8) {
            d = d.with_note(
                *span,
                Text::new(
                    format!("in a call to `{name}`"),
                    format!("أثناء استدعاء `{name}`"),
                ),
            );
        }
        d
    }
}

/// Maximum depth of nested function calls before `R0005`.
pub const MAX_CALL_DEPTH: usize = 3_000;

/// Stack size for the thread that runs a program. A tree-walking
/// interpreter uses several Rust frames per call, so [`MAX_CALL_DEPTH`]
/// needs far more than the default 1–8 MiB.
pub const STACK_SIZE: usize = 256 * 1024 * 1024;

/// Runs `f` on a thread with a [`STACK_SIZE`] stack and waits for it.
pub fn with_large_stack<T: Send>(f: impl FnOnce() -> T + Send) -> T {
    std::thread::scope(|s| {
        std::thread::Builder::new()
            .stack_size(STACK_SIZE)
            .spawn_scoped(s, f)
            .expect("spawn interpreter thread")
            .join()
            .unwrap_or_else(|panic| std::panic::resume_unwind(panic))
    })
}

/// Runs a checked program: evaluates file-level constants in order, then
/// calls `main`.
pub fn run(program: &Program, host: &mut dyn Host) -> Result<(), RuntimeError> {
    run_with_ai(program, host, &mut AiRuntime::default())
}

/// Like [`run`], with the models and policy `ai.infer` calls go through.
/// No file may be read or written.
pub fn run_with_ai(
    program: &Program,
    host: &mut dyn Host,
    ai: &mut AiRuntime,
) -> Result<(), RuntimeError> {
    run_with(program, host, ai, FsPolicy::deny_all("."))
}

/// Runs with explicit AI models and file permissions for `std.fs`.
pub fn run_with(
    program: &Program,
    host: &mut dyn Host,
    ai: &mut AiRuntime,
    fs: FsPolicy,
) -> Result<(), RuntimeError> {
    run_function(program, "main", host, ai, fs).map(|_| ())
}

/// Evaluates file-level constants, then calls the parameterless function
/// `name` (awaiting it if it is `async`) and returns its value. Used for
/// `main` and for each `test_*` function of `elomar test`.
pub fn run_function(
    program: &Program,
    name: &str,
    host: &mut dyn Host,
    ai: &mut AiRuntime,
    fs: FsPolicy,
) -> Result<Value, RuntimeError> {
    let mut interp = Interpreter::new(program, host, ai, fs);
    for item in &program.items {
        if let Item::Const(c) = item {
            let v = interp.eval(&c.value).map_err(Flow::into_error)?;
            interp.globals.insert(c.name.name.clone(), v);
        }
    }
    let Some(f) = interp.fns.get(name).copied() else {
        return Err(RuntimeError::new(
            "E1005",
            Span::default(),
            Text::new(
                format!("no `{name}` function found"),
                format!("لا توجد دالة `{name}`"),
            ),
        ));
    };
    interp
        .call_fn(f, Vec::new(), f.name.span)
        .and_then(|v| match v {
            // An `async fn` returns a future; the runtime awaits it.
            Value::Future(state) => interp.force(&state, f.name.span),
            v => Ok(v),
        })
        .map_err(|flow| {
            let mut err = flow.into_error();
            // The runtime's own call has no call site in the source.
            if err.trace.last().is_some_and(|(n, _)| n == name) {
                err.trace.pop();
            }
            err
        })
}

/// Non-local control flow, propagated with `?`.
enum Flow {
    Break,
    Continue,
    Return(Value),
    Error(Box<RuntimeError>),
}

impl Flow {
    fn into_error(self) -> RuntimeError {
        match self {
            Flow::Error(e) => *e,
            // The checker rejects break/continue outside loops, and `main`
            // and constants absorb returns.
            _ => RuntimeError::new(
                "R0009",
                Span::default(),
                Text::new(
                    "internal error: stray control flow",
                    "خطأ داخلي: تدفق تحكم غير متوقع",
                ),
            ),
        }
    }
}

impl From<RuntimeError> for Flow {
    fn from(e: RuntimeError) -> Self {
        Flow::Error(Box::new(e))
    }
}

type Exec<T> = Result<T, Flow>;

/// One step in an assignable place: `.field` or `[index]`.
enum Step {
    Field(String),
    /// A list index or a map key.
    Index(Value, Span),
}

struct Interpreter<'p, 'h> {
    fns: HashMap<&'p str, &'p FnDecl>,
    globals: HashMap<String, Value>,
    /// Per call frame, a stack of block scopes.
    frames: Vec<Vec<Vec<(String, Value)>>>,
    host: &'h mut dyn Host,
    ai: &'h mut AiRuntime,
    fs: FsPolicy,
    /// Enums whose variants are all plain names, for AI answer schemas.
    unit_enums: HashMap<String, Vec<String>>,
}

impl<'p, 'h> Interpreter<'p, 'h> {
    fn new(
        program: &'p Program,
        host: &'h mut dyn Host,
        ai: &'h mut AiRuntime,
        fs: FsPolicy,
    ) -> Self {
        let fns = program
            .items
            .iter()
            .filter_map(|i| match i {
                Item::Fn(f) => Some((f.name.name.as_str(), f)),
                _ => None,
            })
            .collect();
        Interpreter {
            fns,
            globals: HashMap::new(),
            frames: vec![vec![Vec::new()]],
            host,
            ai,
            fs,
            unit_enums: program
                .items
                .iter()
                .filter_map(|i| match i {
                    Item::Enum(e) if e.variants.iter().all(|v| v.fields.is_empty()) => Some((
                        e.name.name.clone(),
                        e.variants.iter().map(|v| v.name.name.clone()).collect(),
                    )),
                    _ => None,
                })
                .collect(),
        }
    }

    // ----- environment -----------------------------------------------------

    fn scopes(&mut self) -> &mut Vec<Vec<(String, Value)>> {
        self.frames.last_mut().expect("frame")
    }

    fn define(&mut self, name: &str, value: Value) {
        self.scopes()
            .last_mut()
            .expect("scope")
            .push((name.to_string(), value));
    }

    fn lookup(&self, name: &str, span: Span) -> Exec<Value> {
        let frame = self.frames.last().expect("frame");
        for scope in frame.iter().rev() {
            if let Some((_, v)) = scope.iter().rev().find(|(n, _)| n == name) {
                return Ok(v.clone());
            }
        }
        if let Some(v) = self.globals.get(name) {
            return Ok(v.clone());
        }
        if name == "None" {
            return Ok(Value::none());
        }
        Err(RuntimeError::new(
            "R0008",
            span,
            Text::new(
                format!("`{name}` was used before it was initialized"),
                format!("استُخدم `{name}` قبل تهيئته"),
            ),
        )
        .into())
    }

    fn lookup_mut(&mut self, name: &str) -> Option<&mut Value> {
        let frame = self.frames.last_mut().expect("frame");
        for scope in frame.iter_mut().rev() {
            if let Some((_, v)) = scope.iter_mut().rev().find(|(n, _)| n == name) {
                return Some(v);
            }
        }
        None
    }

    fn in_scope<T>(&mut self, f: impl FnOnce(&mut Self) -> Exec<T>) -> Exec<T> {
        self.scopes().push(Vec::new());
        let result = f(self);
        self.scopes().pop();
        result
    }

    // ----- statements ------------------------------------------------------

    fn exec_block(&mut self, block: &Block) -> Exec<()> {
        self.in_scope(|this| {
            for stmt in &block.stmts {
                this.exec(stmt)?;
            }
            Ok(())
        })
    }

    fn exec(&mut self, stmt: &Stmt) -> Exec<()> {
        match &stmt.kind {
            StmtKind::Let { name, value, .. } => {
                let v = self.eval(value)?;
                self.define(&name.name, v);
            }
            StmtKind::Assign { target, op, value } => {
                let rhs = self.eval(value)?;
                let new = match op {
                    AssignOp::Set => rhs,
                    _ => {
                        let current = self.eval(target)?;
                        let bin = match op {
                            AssignOp::Add => BinOp::Add,
                            AssignOp::Sub => BinOp::Sub,
                            AssignOp::Mul => BinOp::Mul,
                            _ => BinOp::Div,
                        };
                        self.binary(bin, current, rhs, stmt.span)?
                    }
                };
                // `m[k] = v` adds the key; `m[k] += 1` needs it to exist.
                let insert = *op == AssignOp::Set;
                self.with_place(target, insert, |slot| {
                    *slot = new;
                    Ok(())
                })?;
            }
            StmtKind::If {
                cond,
                then,
                otherwise,
            } => {
                if self.eval_bool(cond)? {
                    self.exec_block(then)?;
                } else {
                    match otherwise {
                        Some(Else::Block(b)) => self.exec_block(b)?,
                        Some(Else::If(s)) => self.exec(s)?,
                        None => {}
                    }
                }
            }
            StmtKind::While { cond, body } => {
                while self.eval_bool(cond)? {
                    match self.exec_block(body) {
                        Ok(()) | Err(Flow::Continue) => {}
                        Err(Flow::Break) => break,
                        Err(other) => return Err(other),
                    }
                }
            }
            StmtKind::For { var, iter, body } => {
                let items = self.eval(iter)?;
                let run = |this: &mut Self, item: Value| -> Exec<bool> {
                    let r = this.in_scope(|this| {
                        this.define(&var.name, item);
                        this.exec_block(body)
                    });
                    match r {
                        Ok(()) | Err(Flow::Continue) => Ok(true),
                        Err(Flow::Break) => Ok(false),
                        Err(other) => Err(other),
                    }
                };
                match items {
                    Value::List(list) => {
                        for item in list.iter() {
                            if !run(self, item.clone())? {
                                break;
                            }
                        }
                    }
                    Value::Range {
                        start,
                        end,
                        inclusive,
                    } => {
                        let mut i = start;
                        while i < end || (inclusive && i == end) {
                            if !run(self, Value::Int(i))? {
                                break;
                            }
                            match i.checked_add(1) {
                                Some(next) => i = next,
                                None => break,
                            }
                        }
                    }
                    Value::Str(s) => {
                        for g in builtins::graphemes(&s) {
                            if !run(self, Value::str(g))? {
                                break;
                            }
                        }
                    }
                    Value::Map(entries) => {
                        for (k, v) in entries.iter() {
                            let entry = Value::Record(Rc::new(vec![
                                ("key".into(), k.to_value()),
                                ("value".into(), v.clone()),
                            ]));
                            if !run(self, entry)? {
                                break;
                            }
                        }
                    }
                    _ => {}
                }
            }
            StmtKind::Return(value) => {
                let v = match value {
                    Some(e) => self.eval(e)?,
                    None => Value::Unit,
                };
                return Err(Flow::Return(v));
            }
            StmtKind::Break => return Err(Flow::Break),
            StmtKind::Continue => return Err(Flow::Continue),
            StmtKind::Block(b) => self.exec_block(b)?,
            StmtKind::Expr(e) => {
                self.eval(e)?;
            }
        }
        Ok(())
    }

    fn eval_bool(&mut self, e: &Expr) -> Exec<bool> {
        Ok(matches!(self.eval(e)?, Value::Bool(true)))
    }

    /// Resolves `target` to a mutable slot and applies `f` to it, copying
    /// shared lists/records on the way (copy-on-write).
    fn with_place<T>(
        &mut self,
        target: &Expr,
        insert_missing_key: bool,
        f: impl FnOnce(&mut Value) -> Result<T, RuntimeError>,
    ) -> Exec<T> {
        let mut steps = Vec::new();
        let mut cur = target;
        let root = loop {
            match &cur.kind {
                ExprKind::Name(n) => break n.as_str(),
                ExprKind::Field { target, name } => {
                    steps.push(Step::Field(name.name.clone()));
                    cur = target;
                }
                ExprKind::Index { target, index } => {
                    let key = self.eval(index)?;
                    steps.push(Step::Index(key, index.span));
                    cur = target;
                }
                _ => unreachable!("checker only allows places here"),
            }
        };
        steps.reverse();
        let Some(mut slot) = self.lookup_mut(root) else {
            unreachable!("checker only allows local `var` roots");
        };
        let last = steps.len().saturating_sub(1);
        for (i, step) in steps.iter().enumerate() {
            slot = match (step, slot) {
                (Step::Field(name), Value::Record(fields)) => {
                    let fields = Rc::make_mut(fields);
                    &mut fields
                        .iter_mut()
                        .find(|(n, _)| &**n == name)
                        .expect("checked field")
                        .1
                }
                (Step::Index(Value::Int(n), span), Value::List(items)) => {
                    let len = items.len();
                    let idx = builtins::check_index(*n, len, *span)?;
                    &mut Rc::make_mut(items)[idx]
                }
                (Step::Index(key, span), Value::Map(entries)) => {
                    let entries = Rc::make_mut(entries);
                    let k = MapKey::from_value(key);
                    if insert_missing_key && i == last {
                        entries.entry(k).or_insert(Value::Unit)
                    } else {
                        match entries.get_mut(&k) {
                            Some(v) => v,
                            None => return Err(builtins::missing_key(key, *span).into()),
                        }
                    }
                }
                _ => unreachable!("checked place"),
            };
        }
        Ok(f(slot)?)
    }

    // ----- expressions -----------------------------------------------------

    fn eval(&mut self, e: &Expr) -> Exec<Value> {
        Ok(match &e.kind {
            ExprKind::Int(n) => Value::Int(*n),
            ExprKind::Float(x) => Value::Float(*x),
            ExprKind::Str(s) => Value::str(s.as_str()),
            ExprKind::Bool(b) => Value::Bool(*b),
            ExprKind::Name(n) => self.lookup(n, e.span)?,
            ExprKind::List(items) => {
                let mut out = Vec::with_capacity(items.len());
                for item in items {
                    out.push(self.eval(item)?);
                }
                Value::list(out)
            }
            ExprKind::Record(fields) => {
                let mut out = Vec::with_capacity(fields.len());
                for field in fields {
                    out.push((Rc::from(field.name.name.as_str()), self.eval(&field.value)?));
                }
                Value::Record(Rc::new(out))
            }
            ExprKind::Unary { op, expr } => {
                let v = self.eval(expr)?;
                match (op, v) {
                    (UnaryOp::Not, Value::Bool(b)) => Value::Bool(!b),
                    (UnaryOp::Neg, Value::Int(n)) => {
                        Value::Int(n.checked_neg().ok_or_else(|| builtins::overflow(e.span))?)
                    }
                    (UnaryOp::Neg, Value::Float(x)) => Value::Float(-x),
                    (_, v) => v,
                }
            }
            ExprKind::Binary { op, lhs, rhs } => match op {
                BinOp::And => Value::Bool(self.eval_bool(lhs)? && self.eval_bool(rhs)?),
                BinOp::Or => Value::Bool(self.eval_bool(lhs)? || self.eval_bool(rhs)?),
                _ => {
                    let l = self.eval(lhs)?;
                    let r = self.eval(rhs)?;
                    self.binary(*op, l, r, e.span)?
                }
            },
            ExprKind::Range {
                start,
                end,
                inclusive,
            } => {
                let (Value::Int(s), Value::Int(en)) = (self.eval(start)?, self.eval(end)?) else {
                    unreachable!("checked range bounds");
                };
                Value::Range {
                    start: s,
                    end: en,
                    inclusive: *inclusive,
                }
            }
            ExprKind::Call {
                callee,
                type_args,
                args,
            } => self.eval_call(callee, type_args, args, e.span)?,
            ExprKind::Duration(ms) => Value::Duration(*ms),
            ExprKind::Await(inner) => match self.eval(inner)? {
                Value::Future(state) => self.force(&state, e.span)?,
                _ => unreachable!("checker only awaits futures"),
            },
            ExprKind::Field { target, name } => {
                let v = self.eval(target)?;
                match v {
                    Value::Record(fields) => fields
                        .iter()
                        .find(|(n, _)| **n == *name.name)
                        .map(|(_, v)| v.clone())
                        .expect("checked field"),
                    Value::List(items) => Value::Int(items.len() as i64),
                    Value::Map(entries) => Value::Int(entries.len() as i64),
                    Value::Str(s) => Value::Int(builtins::graphemes(&s).count() as i64),
                    _ => unreachable!("checked field access"),
                }
            }
            ExprKind::Index { target, index } => match (self.eval(target)?, self.eval(index)?) {
                (Value::List(items), Value::Int(i)) => {
                    let idx = builtins::check_index(i, items.len(), index.span)?;
                    items[idx].clone()
                }
                (Value::Map(entries), key) => match entries.get(&MapKey::from_value(&key)) {
                    Some(v) => v.clone(),
                    None => return Err(builtins::missing_key(&key, index.span).into()),
                },
                _ => unreachable!("checked indexing"),
            },
            ExprKind::Map(entries) => {
                let mut out = std::collections::BTreeMap::new();
                for (k, v) in entries {
                    let key = MapKey::from_value(&self.eval(k)?);
                    out.insert(key, self.eval(v)?);
                }
                Value::Map(Rc::new(out))
            }
            ExprKind::Variant {
                enum_name,
                variant,
                args,
            } => {
                let mut fields = Vec::with_capacity(args.len());
                for a in args {
                    fields.push(self.eval(a)?);
                }
                Value::Enum {
                    enum_name: enum_name.as_str().into(),
                    variant: variant.as_str().into(),
                    fields: Rc::new(fields),
                }
            }
            // `?`: unwrap, or return the `Err`/`None` from this function.
            ExprKind::Try(inner) => match self.eval(inner)? {
                Value::Result(Ok(v)) | Value::Option(Some(v)) => (*v).clone(),
                err @ (Value::Result(Err(_)) | Value::Option(None)) => {
                    return Err(Flow::Return(err));
                }
                _ => unreachable!("checker only allows `?` on Result and Option"),
            },
            ExprKind::Match { scrutinee, arms } => {
                let v = self.eval(scrutinee)?;
                for arm in arms {
                    let mut bindings = Vec::new();
                    if !match_pattern(&arm.pattern, &v, &mut bindings) {
                        continue;
                    }
                    return self.in_scope(|this| {
                        for (name, value) in bindings {
                            this.define(&name, value);
                        }
                        match &arm.body {
                            ArmBody::Expr(e) => this.eval(e),
                            ArmBody::Block(b) => this.exec_block(b).map(|_| Value::Unit),
                        }
                    });
                }
                return Err(RuntimeError::new(
                    "R0009",
                    e.span,
                    Text::new(
                        "internal error: no `match` arm matched",
                        "خطأ داخلي: لم يطابق أي فرع في `match`",
                    ),
                )
                .into());
            }
        })
    }

    fn binary(&mut self, op: BinOp, l: Value, r: Value, span: Span) -> Exec<Value> {
        builtins::binary(op, l, r, span).map_err(Flow::from)
    }

    fn eval_call(
        &mut self,
        callee: &Expr,
        type_args: &[TypeExpr],
        args: &[Arg],
        span: Span,
    ) -> Exec<Value> {
        match &callee.kind {
            ExprKind::Name(name) => {
                if let Some(f) = self.fns.get(name.as_str()).copied() {
                    let mut values: Vec<Option<Value>> = vec![None; f.params.len()];
                    for (i, arg) in args.iter().enumerate() {
                        let slot = match &arg.name {
                            None => i,
                            Some(id) => f
                                .params
                                .iter()
                                .position(|p| p.name.name == id.name)
                                .expect("checked named argument"),
                        };
                        values[slot] = Some(self.eval(&arg.value)?);
                    }
                    let values: Vec<Value> = values
                        .into_iter()
                        .map(|v| v.expect("checked arity"))
                        .collect();
                    if f.is_async {
                        // Lazy: the body runs when the future is awaited.
                        return Ok(Value::future(Thunk::Call {
                            name: name.clone(),
                            args: values,
                            span: callee.span,
                        }));
                    }
                    return self.call_fn(f, values, callee.span);
                }
                let mut values = Vec::with_capacity(args.len());
                for arg in args {
                    values.push(self.eval(&arg.value)?);
                }
                builtins::call_function(&mut *self.host, name, values, span).map_err(Flow::from)
            }
            ExprKind::Field { target, name } => {
                let mut values = Vec::with_capacity(args.len());
                for arg in args {
                    values.push(self.eval(&arg.value)?);
                }
                if let ExprKind::Name(module) = &target.kind {
                    match module.as_str() {
                        "std.ai" => return Ok(infer_future(type_args, values, &self.unit_enums)),
                        "std.fs" => return Ok(stdlib::fs_call(&self.fs, &name.name, &values)),
                        "std.json" => {
                            return Ok(stdlib::json_call(
                                &name.name,
                                type_args,
                                &values,
                                &self.unit_enums,
                            ));
                        }
                        _ => {}
                    }
                }
                let method = name.name.as_str();
                if builtins::is_mutating(method) {
                    return self.with_place(target, false, |slot| {
                        builtins::call_mutating(slot, method, values, span)
                    });
                }
                let recv = self.eval(target)?;
                builtins::call_method(recv, method, values, span).map_err(Flow::from)
            }
            _ => unreachable!("checked callee"),
        }
    }

    /// Runs a future the first time it is awaited; later awaits of the same
    /// future return the stored result.
    fn force(&mut self, state: &Rc<RefCell<FutureState>>, span: Span) -> Exec<Value> {
        let previous = std::mem::replace(&mut *state.borrow_mut(), FutureState::Running);
        let thunk = match previous {
            FutureState::Done(v) => {
                *state.borrow_mut() = FutureState::Done(v.clone());
                return Ok(v);
            }
            FutureState::Running => {
                return Err(RuntimeError::new(
                    "R0010",
                    span,
                    Text::new(
                        "this future is awaited while it is already running",
                        "يُنتظر ناتج هذه العملية بينما هي قيد التنفيذ أصلًا",
                    ),
                )
                .into());
            }
            FutureState::Pending(thunk) => thunk,
        };
        let result = match thunk {
            Thunk::Call { name, args, span } => {
                let f = self.fns[name.as_str()];
                self.call_fn(f, args, span)?
            }
            Thunk::Infer(call) => ai::run_infer(self.ai, &call),
        };
        *state.borrow_mut() = FutureState::Done(result.clone());
        Ok(result)
    }

    fn call_fn(&mut self, f: &'p FnDecl, args: Vec<Value>, call_span: Span) -> Exec<Value> {
        if self.frames.len() > MAX_CALL_DEPTH {
            return Err(RuntimeError::new(
                "R0005",
                call_span,
                Text::new(
                    format!("too many nested calls (more than {MAX_CALL_DEPTH}); is the recursion missing a base case?"),
                    format!("استدعاءات متداخلة كثيرة جدًا (أكثر من {MAX_CALL_DEPTH})؛ هل ينقص الاستدعاءَ الذاتي شرطُ توقف؟"),
                ),
            )
            .into());
        }
        let params = f
            .params
            .iter()
            .map(|p| p.name.name.clone())
            .zip(args)
            .collect();
        self.frames.push(vec![params]);
        let result = self.exec_block(&f.body);
        self.frames.pop();
        match result {
            Ok(()) => Ok(Value::Unit),
            Err(Flow::Return(v)) => Ok(v),
            Err(Flow::Error(mut err)) => {
                if err.trace.len() < 64 {
                    err.trace.push((f.name.name.clone(), call_span));
                }
                Err(Flow::Error(err))
            }
            Err(other) => Err(other),
        }
    }
}

/// The future for `ai.infer<T>(model, instructions, input[, timeout])`;
/// the checker put the arguments in this order.
fn infer_future(
    type_args: &[TypeExpr],
    args: Vec<Value>,
    unit_enums: &HashMap<String, Vec<String>>,
) -> Value {
    let mut args = args.into_iter();
    let mut text = || match args.next() {
        Some(Value::Str(s)) => s.to_string(),
        _ => String::new(),
    };
    let model = text();
    let instructions = text();
    let input = args.next().unwrap_or(Value::Unit);
    let timeout_ms = match args.next() {
        Some(Value::Duration(ms)) => Some(ms),
        _ => None,
    };
    Value::future(Thunk::Infer(InferCall {
        schema: ai::schema_of(&type_args[0], unit_enums),
        model,
        instructions,
        input,
        timeout_ms,
    }))
}

fn match_pattern(p: &Pattern, v: &Value, bindings: &mut Vec<(String, Value)>) -> bool {
    match (&p.kind, v) {
        (PatternKind::Wildcard, _) => true,
        (PatternKind::Name(n), Value::Option(None)) if n == "None" => true,
        (PatternKind::Name(n), _) if n == "None" => false,
        (PatternKind::Name(n), _) => {
            bindings.push((n.clone(), v.clone()));
            true
        }
        (PatternKind::Ctor { name, args }, _) => {
            let inner = match (name.name.as_str(), v) {
                ("Some", Value::Option(Some(x))) => x,
                ("Ok", Value::Result(Ok(x))) => x,
                ("Err", Value::Result(Err(x))) => x,
                _ => return false,
            };
            args.first()
                .is_some_and(|a| match_pattern(a, inner, bindings))
        }
        // The checker made sure the pattern's enum is the value's enum (the
        // pattern may name it as written, e.g. `Level` inside its module).
        (
            PatternKind::Variant { variant, args, .. },
            Value::Enum {
                variant: v, fields, ..
            },
        ) => {
            **v == *variant.name
                && args
                    .iter()
                    .zip(fields.iter())
                    .all(|(p, f)| match_pattern(p, f, bindings))
        }
        (PatternKind::Variant { .. }, _) => false,
        (PatternKind::Int(n), Value::Int(m)) => n == m,
        (PatternKind::Float(x), _) => v.as_f64() == Some(*x),
        (PatternKind::Int(n), Value::Float(x)) => *n as f64 == *x,
        (PatternKind::Str(s), Value::Str(t)) => **s == **t,
        (PatternKind::Bool(a), Value::Bool(b)) => a == b,
        _ => false,
    }
}

#[cfg(test)]
mod tests;
