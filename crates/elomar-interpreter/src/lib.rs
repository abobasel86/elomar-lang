//! Interpreter for Elomar-Lang (spec §7): a tree walker, and the bytecode
//! engine in [`vm`] that runs functions it can compile, faster.
//!
//! The interpreter assumes the program passed `elomar_types::check`; it
//! trusts static types and only checks what can fail at run time (division
//! by zero, overflow, bounds, recursion depth).

mod ai;
mod builtins;
#[cfg_attr(not(feature = "native"), path = "stubs/db.rs")]
mod db;
mod debug;
#[cfg_attr(not(feature = "native"), path = "stubs/http.rs")]
mod http;
mod mcp;
mod parallel;
mod process;
mod py;
mod server;
mod session;
mod stdextra;
mod stdlib;
mod value;
mod vm;

use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;
use std::sync::Arc;
use std::sync::atomic::AtomicBool;

use elomar_ai_core::{AiRuntime, Schema};
pub use elomar_policy::FsPolicy;

use elomar_ast::*;
use elomar_diagnostics::{Diagnostic, Text};

pub use ai::{InferCall, RunCall, RunTool};
pub use debug::{Debugger, Inspect, Resume, StackFrame};
pub use parallel::{parallel, set_parallel};
pub use session::Session;
pub use value::{Function, FutureState, MapKey, Thunk, Value, format_duration, format_float};
pub use vm::{Engine, engine, set_engine};

/// Services the running program can use. Implemented by the CLI with real
/// stdin/stdout and by tests with in-memory buffers.
pub trait Host {
    fn print(&mut self, text: &str);
    /// Shows `prompt` and reads one line, without its line terminator.
    /// `None` at end of input.
    fn input(&mut self, prompt: &str) -> Option<String>;
    /// The program's arguments (`elomar run main.elo -- a b`), for `os.args()`.
    fn args(&self) -> Vec<String> {
        Vec::new()
    }
}

/// A [`Host`] that records output and replays canned input; for tests.
#[derive(Debug, Default)]
pub struct BufferHost {
    pub output: String,
    pub inputs: Vec<String>,
    pub args: Vec<String>,
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

    fn args(&self) -> Vec<String> {
        self.args.clone()
    }
}

#[derive(Debug, Clone)]
pub struct RuntimeError {
    pub code: &'static str,
    pub message: Text,
    pub span: Span,
    /// Call sites from innermost to outermost: (function name, call span).
    pub trace: Vec<(String, Span)>,
    /// Set by `os.exit(code)`: not an error, the program asked to stop.
    pub exit: Option<i32>,
}

impl RuntimeError {
    pub fn new(code: &'static str, span: Span, message: Text) -> Self {
        RuntimeError {
            code,
            message,
            span,
            trace: Vec::new(),
            exit: None,
        }
    }

    /// The program called `os.exit(code)`.
    pub fn exit(code: i32) -> Self {
        RuntimeError {
            exit: Some(code),
            ..RuntimeError::new(
                "R0000",
                Span::new(0, 0),
                Text::new(
                    format!("the program exited with code {code}"),
                    format!("أنهى البرنامج التنفيذ بالرمز {code}"),
                ),
            )
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
#[cfg(not(target_family = "wasm"))]
pub const MAX_CALL_DEPTH: usize = 3_000;

/// In the browser the stack is set by the browser and is much smaller
/// (about 300-450 calls in Chrome), so stop early with a clear `R0005`.
#[cfg(target_family = "wasm")]
pub const MAX_CALL_DEPTH: usize = 200;

/// Stack size for the thread that runs a program. A tree-walking
/// interpreter uses several Rust frames per call, so [`MAX_CALL_DEPTH`]
/// needs far more than the default 1–8 MiB.
pub const STACK_SIZE: usize = 256 * 1024 * 1024;

/// Runs `f` on a thread with a [`STACK_SIZE`] stack and waits for it.
#[cfg(not(target_family = "wasm"))]
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

/// WebAssembly has no threads; the stack size is set when linking instead.
#[cfg(target_family = "wasm")]
pub fn with_large_stack<T: Send>(f: impl FnOnce() -> T + Send) -> T {
    f()
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
    run_function_with(program, name, host, ai, fs, None, None)
}

/// Like [`run_with`], stopping the program with error `R0013` once it has
/// run for `limit_ms` milliseconds (checked at every statement and loop
/// step, like `timeout`).
pub fn run_limited(
    program: &Program,
    host: &mut dyn Host,
    ai: &mut AiRuntime,
    fs: FsPolicy,
    limit_ms: i64,
) -> Result<(), RuntimeError> {
    run_function_with(program, "main", host, ai, fs, None, Some(limit_ms)).map(|_| ())
}

/// Like [`run_with`], reporting each statement and runtime error to
/// `debugger`.
pub fn run_debug(
    program: &Program,
    host: &mut dyn Host,
    ai: &mut AiRuntime,
    fs: FsPolicy,
    debugger: Box<dyn Debugger>,
) -> Result<(), RuntimeError> {
    run_function_with(program, "main", host, ai, fs, Some(debugger), None).map(|_| ())
}

fn run_function_with(
    program: &Program,
    name: &str,
    host: &mut dyn Host,
    ai: &mut AiRuntime,
    fs: FsPolicy,
    debugger: Option<Box<dyn Debugger>>,
    limit_ms: Option<i64>,
) -> Result<Value, RuntimeError> {
    let mut interp = Interpreter::new(program, host, ai, fs);
    // The debugger follows statements in the tree, so it turns the bytecode
    // engine off.
    interp.vm.enabled &= debugger.is_none();
    interp.debugger = debugger;
    if let Some(ms) = limit_ms {
        interp
            .deadlines
            .push(stdextra::now_ms().saturating_add(ms.max(0)));
    }
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

/// Integer operations that cannot fail; `None` for the others (overflow,
/// division), which [`builtins::binary`] handles with their errors.
#[inline]
fn int_binary(op: BinOp, a: i64, b: i64) -> Option<Value> {
    Some(match op {
        BinOp::Add => Value::Int(a.checked_add(b)?),
        BinOp::Sub => Value::Int(a.checked_sub(b)?),
        BinOp::Mul => Value::Int(a.checked_mul(b)?),
        BinOp::Rem if b != 0 => Value::Int(a.checked_rem(b)?),
        BinOp::IntDiv if b != 0 => Value::Int(a.checked_div(b)?),
        BinOp::Eq => Value::Bool(a == b),
        BinOp::Ne => Value::Bool(a != b),
        BinOp::Lt => Value::Bool(a < b),
        BinOp::Le => Value::Bool(a <= b),
        BinOp::Gt => Value::Bool(a > b),
        BinOp::Ge => Value::Bool(a >= b),
        _ => return None,
    })
}

/// Variables of one block: names point into the program.
type Scope<'p> = Vec<(&'p str, Value)>;

/// A closure's copies of the variables visible where it was made.
type Captured = Rc<Vec<(Rc<str>, Value)>>;

/// The variables of one call.
pub(crate) struct Frame<'p> {
    /// A closure's copies of the variables visible where it was made;
    /// shared by every call of that closure.
    captured: Option<Captured>,
    /// Every variable of the call, inner blocks' last, so a later one
    /// shadows an earlier one with the same name.
    vars: Scope<'p>,
    /// Where each open block's variables start in `vars`, innermost last.
    /// Leaving a block cuts `vars` back to its mark: no allocation per block.
    marks: Vec<usize>,
}

impl<'p> Frame<'p> {
    fn new(captured: Option<Captured>, vars: Scope<'p>) -> Self {
        Frame {
            captured,
            vars,
            marks: Vec::new(),
        }
    }
}

/// A fast hash for the interpreter's maps of names (FxHash): the keys are
/// short and come from the program, so a strong hash only costs time.
#[derive(Default, Clone, Copy)]
pub(crate) struct FxHasher(u64);

impl std::hash::Hasher for FxHasher {
    #[inline]
    fn write(&mut self, bytes: &[u8]) {
        const K: u64 = 0x51_7c_c1_b7_27_22_0a_95;
        let (words, rest) = bytes.as_chunks::<8>();
        for word in words {
            self.0 = (self.0.rotate_left(5) ^ u64::from_le_bytes(*word)).wrapping_mul(K);
        }
        for &b in rest {
            self.0 = (self.0.rotate_left(5) ^ b as u64).wrapping_mul(K);
        }
    }
    #[inline]
    fn write_usize(&mut self, n: usize) {
        self.0 = (self.0.rotate_left(5) ^ n as u64).wrapping_mul(0x51_7c_c1_b7_27_22_0a_95);
    }
    #[inline]
    fn finish(&self) -> u64 {
        self.0
    }
}

pub(crate) type FastMap<K, V> = HashMap<K, V, std::hash::BuildHasherDefault<FxHasher>>;

/// Non-local control flow, propagated with `?`.
enum Flow {
    Break,
    Continue,
    Return(Value),
    Error(Box<RuntimeError>),
    /// The time of `deadlines[i]` is up: unwinds to that `timeout`.
    Timeout(usize),
}

impl Flow {
    fn into_error(self) -> RuntimeError {
        match self {
            Flow::Error(e) => *e,
            // Only the whole program's time limit reaches the top.
            Flow::Timeout(_) => RuntimeError::new(
                "R0013",
                Span::default(),
                Text::new(
                    "the program ran longer than its time limit and was stopped",
                    "استغرق البرنامج أكثر من الوقت المسموح له فأُوقف",
                ),
            ),
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
enum Step<'a> {
    Field(&'a str),
    /// A list index or a map key.
    Index(Value, Span),
}

/// The modules whose functions [`Interpreter::call_module`] runs.
const MODULES: &[&str] = &[
    "std.ai",
    "std.agent",
    "std.mcp",
    "std.server",
    "std.process",
    "std.time",
    "std.math",
    "std.random",
    "std.os",
    "std.regex",
    "std.encoding",
    "std.crypto",
    "std.csv",
    "std.fs",
    "std.db",
    "std.py",
    "std.http",
    "std.json",
];

/// Follows `steps` from `slot` to the value they name, copying shared
/// lists, maps and records on the way (copy-on-write). With
/// `insert_missing_key`, a missing key at the last step is added.
fn walk_place<'v>(
    mut slot: &'v mut Value,
    steps: &[Step<'_>],
    insert_missing_key: bool,
) -> Result<&'v mut Value, RuntimeError> {
    let last = steps.len().saturating_sub(1);
    for (i, step) in steps.iter().enumerate() {
        slot = match (step, slot) {
            (Step::Field(name), Value::Record(fields)) => {
                let fields = Rc::make_mut(fields);
                &mut fields
                    .iter_mut()
                    .find(|(n, _)| &**n == *name)
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
                        None => return Err(builtins::missing_key(key, *span)),
                    }
                }
            }
            _ => unreachable!("checked place"),
        };
    }
    Ok(slot)
}

struct Interpreter<'p, 'h> {
    fns: FastMap<&'p str, &'p FnDecl>,
    globals: FastMap<String, Value>,
    /// One per call.
    frames: Vec<Frame<'p>>,
    host: &'h mut dyn Host,
    ai: &'h mut AiRuntime,
    fs: FsPolicy,
    /// Enums whose variants are all plain names, for AI answer schemas.
    unit_enums: HashMap<String, Vec<String>>,
    /// Closure expressions that have been evaluated, by address.
    lambdas: FastMap<usize, &'p Expr>,
    /// MCP servers started so far; stopped when the run ends.
    mcp: mcp::McpHub,
    /// Set when a debugger runs the program.
    debugger: Option<Box<dyn Debugger>>,
    /// With a debugger: each call's name and the statement it is running,
    /// outermost first. `calls[i]` runs in `frames[i + 1]`.
    calls: Vec<StackFrame>,
    /// The runtime error last shown to the debugger, by address, so it is
    /// shown once while it unwinds.
    reported_error: usize,
    /// When each running `timeout` ends (milliseconds since 1970),
    /// outermost first.
    deadlines: Vec<i64>,
    /// Frames of finished calls, emptied, so a call reuses their memory
    /// instead of allocating.
    spare: Vec<Frame<'p>>,
    /// One shared copy of each record field name written in the program,
    /// by the address of its text, so making a record allocates no names.
    field_names: FastMap<usize, Rc<str>>,
    /// The bytecode engine: compiled functions and their registers.
    vm: vm::Vm<'p>,
    program: &'p Program,
    /// Running `await all([...])` on several cores.
    par: parallel::Par<'p>,
    /// In a thread of `await all([...])`: set when the call must stop.
    stop: Option<Arc<AtomicBool>>,
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
        let mcp = mcp::McpHub::new(ai.mcp().clone(), fs.base().to_path_buf());
        Interpreter {
            fns,
            mcp,
            globals: FastMap::default(),
            frames: vec![Frame::new(None, Vec::new())],
            host,
            ai,
            fs,
            lambdas: FastMap::default(),
            debugger: None,
            calls: Vec::new(),
            reported_error: 0,
            deadlines: Vec::new(),
            spare: Vec::new(),
            field_names: FastMap::default(),
            vm: vm::Vm::new(program),
            program,
            par: parallel::Par::new(),
            stop: None,
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

    fn frame(&mut self) -> &mut Frame<'p> {
        self.frames.last_mut().expect("frame")
    }

    fn define(&mut self, name: &'p str, value: Value) {
        self.frame().vars.push((name, value));
    }

    fn lookup(&self, name: &str, span: Span) -> Exec<Value> {
        if let Some(v) = self.find_variable(name) {
            return Ok(v);
        }
        if name == "None" {
            return Ok(Value::none());
        }
        if self.fns.contains_key(name) {
            return Ok(Value::Function(Rc::new(Function::Named(name.to_string()))));
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

    /// A local variable or constant called `name`, if there is one.
    fn find_variable(&self, name: &str) -> Option<Value> {
        let frame = self.frames.last().expect("frame");
        if let Some((_, v)) = frame.vars.iter().rev().find(|(n, _)| *n == name) {
            return Some(v.clone());
        }
        if let Some(captured) = &frame.captured
            && let Some((_, v)) = captured.iter().rev().find(|(n, _)| &**n == name)
        {
            return Some(v.clone());
        }
        self.globals.get(name).cloned()
    }

    fn lookup_mut(&mut self, name: &str) -> Option<&mut Value> {
        let frame = self.frames.last_mut().expect("frame");
        if let Some((_, v)) = frame.vars.iter_mut().rev().find(|(n, _)| *n == name) {
            return Some(v);
        }
        let captured = Rc::make_mut(frame.captured.as_mut()?);
        captured
            .iter_mut()
            .rev()
            .find(|(n, _)| &**n == name)
            .map(|(_, v)| v)
    }

    #[inline]
    fn in_scope<T>(&mut self, f: impl FnOnce(&mut Self) -> Exec<T>) -> Exec<T> {
        let frame = self.frame();
        let mark = frame.vars.len();
        frame.marks.push(mark);
        let result = f(self);
        let frame = self.frame();
        frame.marks.pop();
        frame.vars.truncate(mark);
        result
    }

    // ----- statements ------------------------------------------------------

    fn exec_block(&mut self, block: &'p Block) -> Exec<()> {
        self.in_scope(|this| {
            for stmt in &block.stmts {
                this.exec(stmt)?;
            }
            Ok(())
        })
    }

    fn exec(&mut self, stmt: &'p Stmt) -> Exec<()> {
        if self.watching() {
            self.check_deadlines()?;
        }
        if self.debugger.is_none() {
            return self.exec_stmt(stmt);
        }
        if let Some(call) = self.calls.last_mut() {
            call.span = stmt.span;
        }
        let mut debugger = self.debugger.take().expect("debugger");
        let resume = debugger.statement(&*self, stmt.span);
        self.debugger = Some(debugger);
        if resume == Resume::Stop {
            return Err(RuntimeError::exit(0).into());
        }
        let result = self.exec_stmt(stmt);
        if let Err(Flow::Error(err)) = &result {
            let address = &**err as *const RuntimeError as usize;
            if address != self.reported_error && err.exit.is_none() {
                self.reported_error = address;
                let mut debugger = self.debugger.take().expect("debugger");
                let resume = debugger.error(&*self, stmt.span, err);
                self.debugger = Some(debugger);
                if resume == Resume::Stop {
                    return Err(RuntimeError::exit(0).into());
                }
            }
        }
        result
    }

    /// Common statements are handled here, so this function stays small;
    /// [`Interpreter::exec_other`] handles the rest.
    fn exec_stmt(&mut self, stmt: &'p Stmt) -> Exec<()> {
        match &stmt.kind {
            StmtKind::Expr(e) => {
                self.eval(e)?;
                Ok(())
            }
            StmtKind::Let { name, value, .. } => {
                let v = self.eval(value)?;
                self.define(&name.name, v);
                Ok(())
            }
            // `x = v`, `x += v` on a plain variable.
            StmtKind::Assign { target, op, value } if matches!(target.kind, ExprKind::Name(_)) => {
                let ExprKind::Name(n) = &target.kind else {
                    unreachable!()
                };
                let rhs = self.eval(value)?;
                // `x += 1` on an `Int`: one search for `x`, changed in place.
                if matches!(op, AssignOp::Add | AssignOp::Sub | AssignOp::Mul)
                    && let Value::Int(b) = rhs
                    && let Some(slot) = self.lookup_mut(n)
                    && let Value::Int(a) = *slot
                {
                    let bin = match op {
                        AssignOp::Add => BinOp::Add,
                        AssignOp::Sub => BinOp::Sub,
                        _ => BinOp::Mul,
                    };
                    if let Some(v) = int_binary(bin, a, b) {
                        *slot = v;
                        return Ok(());
                    }
                }
                let new = match op {
                    AssignOp::Set => rhs,
                    _ => {
                        let current = self.lookup(n, target.span)?;
                        let bin = match op {
                            AssignOp::Add => BinOp::Add,
                            AssignOp::Sub => BinOp::Sub,
                            AssignOp::Mul => BinOp::Mul,
                            _ => BinOp::Div,
                        };
                        match (&current, &rhs) {
                            (Value::Int(a), Value::Int(b)) if bin != BinOp::Div => {
                                match int_binary(bin, *a, *b) {
                                    Some(v) => v,
                                    None => self.binary(bin, current, rhs, stmt.span)?,
                                }
                            }
                            _ => self.binary(bin, current, rhs, stmt.span)?,
                        }
                    }
                };
                let Some(slot) = self.lookup_mut(n) else {
                    unreachable!("checker only allows local `var` roots");
                };
                *slot = new;
                Ok(())
            }
            StmtKind::If {
                cond,
                then,
                otherwise,
            } => {
                if self.eval_bool(cond)? {
                    self.exec_block(then)
                } else {
                    match otherwise {
                        Some(Else::Block(b)) => self.exec_block(b),
                        Some(Else::If(s)) => self.exec(s),
                        None => Ok(()),
                    }
                }
            }
            StmtKind::While { cond, body } => {
                while self.eval_bool(cond)? {
                    if self.watching() {
                        // Also stops loops whose body is empty.
                        self.check_deadlines()?;
                    }
                    match self.exec_block(body) {
                        Ok(()) | Err(Flow::Continue) => {}
                        Err(Flow::Break) => break,
                        Err(other) => return Err(other),
                    }
                }
                Ok(())
            }
            _ => self.exec_other(stmt),
        }
    }

    #[inline(never)]
    fn exec_other(&mut self, stmt: &'p Stmt) -> Exec<()> {
        match &stmt.kind {
            StmtKind::Let { name, value, .. } => {
                let v = self.eval(value)?;
                self.define(&name.name, v);
            }
            StmtKind::LetTuple { names, value, .. } => {
                let v = self.eval(value)?;
                for (name, x) in names.iter().zip(unpack(&v)) {
                    if name.name != "_" {
                        self.define(&name.name, x);
                    }
                }
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
            StmtKind::For { vars, iter, body } => {
                let items = self.eval(iter)?;
                let run = |this: &mut Self, item: Value| -> Exec<bool> {
                    if this.watching() {
                        this.check_deadlines()?;
                    }
                    let r = this.in_scope(|this| {
                        if vars.len() == 1 {
                            this.define(&vars[0].name, item);
                        } else {
                            for (var, v) in vars.iter().zip(unpack(&item)) {
                                if var.name != "_" {
                                    this.define(&var.name, v);
                                }
                            }
                        }
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
                    Value::Set(items) => {
                        for k in items.iter() {
                            if !run(self, k.to_value())? {
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

    fn eval_bool(&mut self, e: &'p Expr) -> Exec<bool> {
        Ok(matches!(self.eval(e)?, Value::Bool(true)))
    }

    /// Resolves `target` to a mutable slot and applies `f` to it, copying
    /// shared lists/records on the way (copy-on-write).
    fn with_place<T>(
        &mut self,
        target: &'p Expr,
        insert_missing_key: bool,
        f: impl FnOnce(&mut Value) -> Result<T, RuntimeError>,
    ) -> Exec<T> {
        // `x = ...` and `x += ...`: no path to follow.
        if let ExprKind::Name(n) = &target.kind {
            let Some(slot) = self.lookup_mut(n) else {
                unreachable!("checker only allows local `var` roots");
            };
            return f(slot).map_err(Flow::from);
        }
        let mut steps = Vec::new();
        let mut cur = target;
        let root = loop {
            match &cur.kind {
                ExprKind::Name(n) => break n.as_str(),
                ExprKind::Field { target, name } => {
                    steps.push(Step::Field(&name.name));
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
        let Some(slot) = self.lookup_mut(root) else {
            unreachable!("checker only allows local `var` roots");
        };
        let slot = walk_place(slot, &steps, insert_missing_key)?;
        Ok(f(slot)?)
    }

    // ----- expressions -----------------------------------------------------

    /// The most common expressions are handled here, so this function stays
    /// small; [`Interpreter::eval_other`] handles the rest.
    fn eval(&mut self, e: &'p Expr) -> Exec<Value> {
        match &e.kind {
            ExprKind::Int(n) => Ok(Value::Int(*n)),
            ExprKind::Bool(b) => Ok(Value::Bool(*b)),
            ExprKind::Name(n) => self.lookup(n, e.span),
            ExprKind::Binary { op, lhs, rhs } if !matches!(op, BinOp::And | BinOp::Or) => {
                let l = self.operand(lhs)?;
                let r = self.operand(rhs)?;
                if let (Value::Int(a), Value::Int(b)) = (&l, &r)
                    && let Some(v) = int_binary(*op, *a, *b)
                {
                    return Ok(v);
                }
                self.binary(*op, l, r, e.span)
            }
            _ => self.eval_other(e),
        }
    }

    /// An operand of an arithmetic or comparison: a number or a variable
    /// is read at once, anything else is evaluated.
    #[inline(always)]
    fn operand(&mut self, e: &'p Expr) -> Exec<Value> {
        match &e.kind {
            ExprKind::Int(n) => Ok(Value::Int(*n)),
            ExprKind::Name(n) => match self.local(n) {
                Some(v) => Ok(v),
                None => self.lookup(n, e.span),
            },
            _ => self.eval(e),
        }
    }

    /// A variable of the current call (not a captured one or a constant).
    #[inline(always)]
    fn local(&self, name: &str) -> Option<Value> {
        let frame = self.frames.last()?;
        frame
            .vars
            .iter()
            .rev()
            .find(|(n, _)| *n == name)
            .map(|(_, v)| v.clone())
    }

    #[inline(never)]
    fn eval_other(&mut self, e: &'p Expr) -> Exec<Value> {
        Ok(match &e.kind {
            ExprKind::Int(n) => Value::Int(*n),
            ExprKind::Float(x) => Value::Float(*x),
            ExprKind::Str(s) => Value::str(s.as_str()),
            ExprKind::Tuple(items) => {
                let mut out = Vec::with_capacity(items.len());
                for item in items {
                    out.push(self.eval(item)?);
                }
                Value::Tuple(Rc::new(out))
            }
            ExprKind::Interp(parts) => {
                let mut out = String::new();
                for part in parts {
                    match part {
                        InterpPart::Lit(text) => out.push_str(text),
                        InterpPart::Expr(inner) => {
                            use std::fmt::Write as _;
                            let v = self.eval(inner)?;
                            let _ = write!(out, "{v}");
                        }
                    }
                }
                Value::str(out)
            }
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
                    let name = self.field_name(&field.name.name);
                    out.push((name, self.eval(&field.value)?));
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
                if let ExprKind::Name(module) = &target.kind
                    && module == "std.math"
                {
                    return Ok(stdextra::math_constant(&name.name));
                }
                let v = self.eval(target)?;
                match v {
                    Value::Record(fields) => fields
                        .iter()
                        .find(|(n, _)| **n == *name.name)
                        .map(|(_, v)| v.clone())
                        .expect("checked field"),
                    Value::List(items) => Value::Int(items.len() as i64),
                    Value::Map(entries) => Value::Int(entries.len() as i64),
                    Value::Set(items) => Value::Int(items.len() as i64),
                    Value::Str(s) => Value::Int(builtins::str_length(&s) as i64),
                    Value::Tuple(items) => {
                        let i: usize = name.name.parse().expect("checked tuple index");
                        items[i].clone()
                    }
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
                Value::enum_value(enum_name.as_str(), variant.as_str(), fields)
            }
            // `?`: unwrap, or return the `Err`/`None` from this function.
            ExprKind::Try(inner) => match self.eval(inner)? {
                Value::Result(Ok(v)) | Value::Option(Some(v)) => (*v).clone(),
                err @ (Value::Result(Err(_)) | Value::Option(None)) => {
                    return Err(Flow::Return(err));
                }
                _ => unreachable!("checker only allows `?` on Result and Option"),
            },
            ExprKind::Lambda { .. } => {
                let key = e as *const Expr as usize;
                self.lambdas.insert(key, e);
                // Copies of everything visible here; inner scopes come last
                // so they shadow outer ones on lookup.
                let frame = self.frames.last().expect("frame");
                let mut captured: Vec<(Rc<str>, Value)> = frame
                    .captured
                    .as_deref()
                    .map(|c| c.to_vec())
                    .unwrap_or_default();
                for (name, value) in &frame.vars {
                    captured.push((Rc::from(*name), value.clone()));
                }
                Value::Function(Rc::new(Function::Closure {
                    key,
                    captured: Rc::new(captured),
                }))
            }
            ExprKind::Match { scrutinee, arms } => {
                let v = self.eval(scrutinee)?;
                for arm in arms {
                    let mut bindings = Vec::new();
                    if !match_pattern(&arm.pattern, &v, &mut bindings) {
                        continue;
                    }
                    return self.in_scope(|this| {
                        for (name, value) in bindings {
                            this.define(name, value);
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
        callee: &'p Expr,
        type_args: &'p [TypeExpr],
        args: &'p [Arg],
        span: Span,
    ) -> Exec<Value> {
        match &callee.kind {
            ExprKind::Name(name) => {
                if let Some(Value::Function(func)) = self.find_variable(name) {
                    let values = self.eval_args(args)?;
                    return self.call_value(&func, values, callee.span);
                }
                if let Some(f) = self.fns.get(name.as_str()).copied() {
                    // Arguments in order: no reordering needed.
                    if !f.is_async && args.iter().all(|a| a.name.is_none()) {
                        // Straight into the new call's variables.
                        let mut frame = self.take_frame();
                        for (param, arg) in f.params.iter().zip(args) {
                            match self.eval(&arg.value) {
                                Ok(v) => frame.vars.push((param.name.name.as_str(), v)),
                                Err(e) => {
                                    self.recycle(frame);
                                    return Err(e);
                                }
                            }
                        }
                        return self.enter_fn(f, frame, callee.span);
                    }
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
                if name == "all" {
                    let items = match values.into_iter().next() {
                        Some(Value::List(items)) => items,
                        _ => Rc::new(Vec::new()),
                    };
                    return Ok(Value::future(Thunk::All(items)));
                }
                if name == "timeout" {
                    let mut values = values.into_iter();
                    let future = values.next().unwrap_or(Value::Unit);
                    let limit = match values.next() {
                        Some(Value::Duration(ms)) => ms,
                        _ => 0,
                    };
                    return Ok(Value::future(Thunk::Timeout(future, limit)));
                }
                builtins::call_function(&mut *self.host, name, values, span).map_err(Flow::from)
            }
            ExprKind::Field { target, name } => {
                let values = self.eval_args(args)?;
                if let ExprKind::Name(module) = &target.kind
                    && MODULES.contains(&module.as_str())
                {
                    return self.call_module(module, &name.name, type_args, values, span);
                }
                let method = name.name.as_str();
                if builtins::is_mutating(method) {
                    return self.with_place(target, false, |slot| {
                        builtins::call_mutating(slot, method, values, span)
                    });
                }
                let recv = self.eval(target)?;
                self.call_method_value(recv, method, values, span)
            }
            _ => match self.eval(callee)? {
                Value::Function(func) => {
                    let values = self.eval_args(args)?;
                    self.call_value(&func, values, callee.span)
                }
                _ => unreachable!("checked callee"),
            },
        }
    }

    /// A function of a standard module (one of [`MODULES`]).
    fn call_module(
        &mut self,
        module: &str,
        name: &str,
        type_args: &'p [TypeExpr],
        values: Vec<Value>,
        span: Span,
    ) -> Exec<Value> {
        match module {
            "std.ai" if name == "run" => Ok(self.run_future(type_args, values, false)),
            "std.agent" => Ok(self.run_future(type_args, values, true)),
            "std.mcp" => Ok(mcp::call(&mut self.mcp, name, &values)),
            "std.server" if name == "serve" => self.serve(values, span),
            "std.process" => Ok(process::run(self.ai.permissions(), self.fs.base(), &values)),
            "std.server" => Ok(server::helper(name, &values)),
            "std.time" if name == "sleep" && !self.deadlines.is_empty() => {
                // Sleep no longer than the nearest `timeout` allows.
                let ms = match values.first() {
                    Some(Value::Duration(ms)) => *ms,
                    _ => 0,
                };
                let nearest = self.deadlines.iter().min().copied().unwrap_or(i64::MAX);
                let left = (nearest - stdextra::now_ms()).max(0);
                stdextra::call(
                    module,
                    "sleep",
                    &[Value::Duration(ms.min(left))],
                    &mut *self.host,
                    span,
                )
                .map_err(Flow::from)?;
                self.check_deadlines()?;
                Ok(Value::Unit)
            }
            "std.math" | "std.random" | "std.time" | "std.os" | "std.regex" | "std.encoding"
            | "std.crypto" | "std.csv" => {
                stdextra::call(module, name, &values, &mut *self.host, span).map_err(Flow::from)
            }
            "std.ai" if name == "embed" => {
                let mut values = values.into_iter();
                let mut text = || match values.next() {
                    Some(Value::Str(s)) => s.to_string(),
                    _ => String::new(),
                };
                let (model, text) = (text(), text());
                let timeout_ms = match values.next() {
                    Some(Value::Duration(ms)) => Some(ms),
                    _ => None,
                };
                Ok(Value::future(Thunk::Embed {
                    model,
                    text,
                    timeout_ms,
                }))
            }
            "std.ai" if name == "similarity" => Ok(Value::Float(ai::similarity(&values))),
            "std.ai" if name == "stream" => {
                let mut values = values.into_iter();
                let mut text = || match values.next() {
                    Some(Value::Str(s)) => s.to_string(),
                    _ => String::new(),
                };
                let (model, instructions) = (text(), text());
                let input = values.next().unwrap_or(Value::Unit);
                let Some(Value::Function(on_text)) = values.next() else {
                    unreachable!("checked function argument");
                };
                let timeout_ms = match values.next() {
                    Some(Value::Duration(ms)) => Some(ms),
                    _ => None,
                };
                Ok(Value::future(Thunk::Stream(Box::new(ai::StreamCall {
                    infer: ai::InferCall {
                        schema: elomar_ai_core::Schema::String,
                        model,
                        instructions,
                        input,
                        timeout_ms,
                    },
                    on_text,
                }))))
            }
            "std.ai" => Ok(infer_future(type_args, values, &self.unit_enums)),
            "std.fs" => Ok(stdlib::fs_call(&self.fs, name, &values)),
            "std.db" => Ok(db::call(
                &self.fs,
                name,
                type_args,
                &values,
                &self.unit_enums,
            )),
            "std.py" => {
                let (value, printed) = py::call(
                    self.ai.permissions(),
                    self.fs.base(),
                    type_args,
                    &values,
                    &self.unit_enums,
                );
                if !printed.is_empty() {
                    self.host.print(&printed);
                }
                Ok(value)
            }
            "std.http" => Ok(http::call(self.ai.permissions(), name, &values)),
            "std.json" => Ok(stdlib::json_call(
                name,
                type_args,
                &values,
                &self.unit_enums,
            )),
            _ => unreachable!("not a module: {module}"),
        }
    }

    /// `recv.method(values)` for a method that does not change `recv`.
    fn call_method_value(
        &mut self,
        recv: Value,
        method: &str,
        values: Vec<Value>,
        span: Span,
    ) -> Exec<Value> {
        if let Value::List(items) = &recv
            && HIGHER_ORDER.contains(&method)
        {
            return self.list_higher_order(items, method, values, span);
        }
        if matches!(
            method,
            "filter" | "any" | "all" | "count" | "map_values" | "map"
        ) && matches!(recv, Value::Map(_) | Value::Set(_))
        {
            return self.map_set_higher_order(&recv, method, values, span);
        }
        if matches!(recv, Value::Option(_) | Value::Result(_))
            && let Some(v) = self.option_result_method(&recv, method, &values, span)?
        {
            return Ok(v);
        }
        builtins::call_method(recv, method, values, span).map_err(Flow::from)
    }

    /// `filter`, `any`, `all` and `count` on a map (`f(key, value)`) or a
    /// set, `map_values` on a map and `map` on a set: in the order of the
    /// keys, as `entries()` and `to_list()` give them.
    fn map_set_higher_order(
        &mut self,
        recv: &Value,
        method: &str,
        args: Vec<Value>,
        span: Span,
    ) -> Exec<Value> {
        let Some(Value::Function(f)) = args.into_iter().next() else {
            unreachable!("checked function argument");
        };
        match recv {
            Value::Map(entries) => {
                if method == "map_values" {
                    let mut out = std::collections::BTreeMap::new();
                    for (k, v) in entries.iter() {
                        out.insert(k.clone(), self.call_value(&f, vec![v.clone()], span)?);
                    }
                    return Ok(Value::Map(Rc::new(out)));
                }
                let mut kept = std::collections::BTreeMap::new();
                let mut n = 0;
                for (k, v) in entries.iter() {
                    let yes = matches!(
                        self.call_value(&f, vec![k.to_value(), v.clone()], span)?,
                        Value::Bool(true)
                    );
                    match method {
                        "any" if yes => return Ok(Value::Bool(true)),
                        "all" if !yes => return Ok(Value::Bool(false)),
                        _ if yes => {
                            n += 1;
                            if method == "filter" {
                                kept.insert(k.clone(), v.clone());
                            }
                        }
                        _ => {}
                    }
                }
                Ok(match method {
                    "filter" => Value::Map(Rc::new(kept)),
                    "count" => Value::Int(n),
                    m => Value::Bool(m == "all"),
                })
            }
            Value::Set(items) => {
                let mut out = std::collections::BTreeSet::new();
                let mut n = 0;
                for k in items.iter() {
                    let r = self.call_value(&f, vec![k.to_value()], span)?;
                    if method == "map" {
                        out.insert(MapKey::from_value(&r));
                        continue;
                    }
                    let yes = matches!(r, Value::Bool(true));
                    match method {
                        "any" if yes => return Ok(Value::Bool(true)),
                        "all" if !yes => return Ok(Value::Bool(false)),
                        _ if yes => {
                            n += 1;
                            if method == "filter" {
                                out.insert(k.clone());
                            }
                        }
                        _ => {}
                    }
                }
                Ok(match method {
                    "filter" | "map" => Value::Set(Rc::new(out)),
                    "count" => Value::Int(n),
                    m => Value::Bool(m == "all"),
                })
            }
            _ => unreachable!("checked map or set"),
        }
    }

    fn eval_args(&mut self, args: &'p [Arg]) -> Exec<Vec<Value>> {
        let mut values = Vec::with_capacity(args.len());
        for arg in args {
            values.push(self.eval(&arg.value)?);
        }
        Ok(values)
    }

    /// Calls a function value with positional arguments.
    fn call_value(&mut self, func: &Function, args: Vec<Value>, span: Span) -> Exec<Value> {
        match func {
            Function::Named(name) => {
                let f = self.fns[name.as_str()];
                if f.is_async {
                    return Ok(Value::future(Thunk::Call {
                        name: name.clone(),
                        args,
                        span,
                    }));
                }
                self.call_fn(f, args, span)
            }
            Function::Closure { key, captured } => {
                let ExprKind::Lambda { params, body } = &self.lambdas[key].kind else {
                    unreachable!("lambda table holds closures");
                };
                if self.vm.enabled
                    && let Some(code) = self.vm.lambdas.get(key).cloned()
                {
                    return self.call_closure_compiled(&code, params.len(), captured, args, span);
                }
                self.check_depth(span)?;
                let params = params
                    .iter()
                    .map(|p| p.name.name.as_str())
                    .zip(args)
                    .collect();
                self.frames.push(Frame::new(Some(captured.clone()), params));
                if self.debugger.is_some() {
                    self.calls.push(StackFrame {
                        name: "<closure>".into(),
                        span,
                    });
                }
                let result = match &**body {
                    ArmBody::Expr(e) => self.eval(e),
                    ArmBody::Block(b) => self.exec_block(b).map(|()| Value::Unit),
                };
                self.frames.pop();
                if self.debugger.is_some() {
                    self.calls.pop();
                }
                match result {
                    Ok(v) | Err(Flow::Return(v)) => Ok(v),
                    Err(Flow::Error(mut err)) => {
                        if err.trace.len() < 64 {
                            err.trace.push(("<closure>".into(), span));
                        }
                        Err(Flow::Error(err))
                    }
                    Err(other) => Err(other),
                }
            }
        }
    }

    /// `map`, `and_then` and the other `Option`/`Result` methods that are
    /// not in `builtins`. `None` for methods handled there.
    fn option_result_method(
        &mut self,
        recv: &Value,
        method: &str,
        args: &[Value],
        span: Span,
    ) -> Exec<Option<Value>> {
        let func = |i: usize| match args.get(i) {
            Some(Value::Function(f)) => Some(f.clone()),
            _ => None,
        };
        let call = |this: &mut Self, i: usize, x: Vec<Value>| -> Exec<Value> {
            let f = func(i).expect("checked function argument");
            this.call_value(&f, x, span)
        };
        Ok(Some(match (recv, method) {
            (Value::Option(o), "map") => match o {
                Some(v) => Value::some(call(self, 0, vec![(**v).clone()])?),
                None => Value::none(),
            },
            (Value::Option(o), "and_then") => match o {
                Some(v) => call(self, 0, vec![(**v).clone()])?,
                None => Value::none(),
            },
            (Value::Option(o), "filter") => match o {
                Some(v) if matches!(call(self, 0, vec![(**v).clone()])?, Value::Bool(true)) => {
                    recv.clone()
                }
                _ => Value::none(),
            },
            (Value::Option(o), "is_some_and") => match o {
                Some(v) => call(self, 0, vec![(**v).clone()])?,
                None => Value::Bool(false),
            },
            (Value::Option(o), "unwrap_or_else") => match o {
                Some(v) => (**v).clone(),
                None => call(self, 0, Vec::new())?,
            },
            (Value::Option(o), "or") => match o {
                Some(_) => recv.clone(),
                None => args[0].clone(),
            },
            (Value::Option(o), "ok_or") => match o {
                Some(v) => Value::Result(Ok(v.clone())),
                None => Value::Result(Err(Rc::new(args[0].clone()))),
            },
            (Value::Result(r), "map") => match r {
                Ok(v) => Value::Result(Ok(Rc::new(call(self, 0, vec![(**v).clone()])?))),
                Err(_) => recv.clone(),
            },
            (Value::Result(r), "map_err") => match r {
                Ok(_) => recv.clone(),
                Err(e) => Value::Result(Err(Rc::new(call(self, 0, vec![(**e).clone()])?))),
            },
            (Value::Result(r), "and_then") => match r {
                Ok(v) => call(self, 0, vec![(**v).clone()])?,
                Err(_) => recv.clone(),
            },
            (Value::Result(r), "unwrap_or_else") => match r {
                Ok(v) => (**v).clone(),
                Err(e) => call(self, 0, vec![(**e).clone()])?,
            },
            (Value::Result(r), "ok") => match r {
                Ok(v) => Value::Option(Some(v.clone())),
                Err(_) => Value::none(),
            },
            (Value::Result(r), "err") => match r {
                Ok(_) => Value::none(),
                Err(e) => Value::Option(Some(e.clone())),
            },
            _ => return Ok(None),
        }))
    }

    /// `f` from the first of `items` over the others: `f(f(a, b), c)`.
    pub(crate) fn fold_part(&mut self, f: &Function, items: Vec<Value>, span: Span) -> Exec<Value> {
        let mut items = items.into_iter();
        let mut acc = items.next().expect("a part is not empty");
        for x in items {
            acc = self.call_value(f, vec![acc, x], span)?;
        }
        Ok(acc)
    }

    /// `map`, `filter`, `fold` and the other list methods that take a function.
    fn list_higher_order(
        &mut self,
        items: &Rc<Vec<Value>>,
        method: &str,
        args: Vec<Value>,
        span: Span,
    ) -> Exec<Value> {
        let mut args = args.into_iter();
        let (init, f) = if method == "fold" || method == "par_fold" {
            let init = args.next().expect("checked arity");
            (Some(init), args.next())
        } else {
            (None, args.next())
        };
        let Some(Value::Function(f)) = f else {
            unreachable!("checked function argument");
        };
        let test = |this: &mut Self, x: &Value| -> Exec<bool> {
            Ok(matches!(
                this.call_value(&f, vec![x.clone()], span)?,
                Value::Bool(true)
            ))
        };
        // `par_flat_map`, `par_group_by`, `par_sum_by`, `par_min_by` and
        // `par_max_by`: `f` on every item first, on the cores, then the
        // method itself over those results.
        let (method, done) = match method.strip_prefix("par_") {
            Some(
                m @ ("flat_map" | "group_by" | "sum_by" | "sum_by.float" | "min_by" | "max_by"),
            ) => {
                let done = match self.par_map(items, &f, span) {
                    Ok(Some(Value::List(results))) => Some(results),
                    Ok(_) => None,
                    // `sum_by` may overflow before the item that failed:
                    // going over the items again in order gives the error
                    // `sum_by` gives. The calls were isolated, so running
                    // them again cannot be seen, and those before the
                    // failing item ended.
                    Err(Flow::Error(_)) if m.starts_with("sum_by") => None,
                    Err(e) => return Err(e),
                };
                (m, done)
            }
            _ => (method, None),
        };
        let key = |this: &mut Self, i: usize, x: &Value| -> Exec<Value> {
            match &done {
                Some(results) => Ok(results[i].clone()),
                None => this.call_value(&f, vec![x.clone()], span),
            }
        };
        Ok(match method {
            "map" | "par_map" => {
                if method == "par_map"
                    && let Some(out) = self.par_map(items, &f, span)?
                {
                    return Ok(out);
                }
                let mut out = Vec::with_capacity(items.len());
                for x in items.iter() {
                    out.push(self.call_value(&f, vec![x.clone()], span)?);
                }
                Value::list(out)
            }
            "filter" | "par_filter" => {
                if method == "par_filter"
                    && let Some(Value::List(keep)) = self.par_map(items, &f, span)?
                {
                    let kept = items.iter().zip(keep.iter());
                    return Ok(Value::list(
                        kept.filter(|(_, k)| matches!(k, Value::Bool(true)))
                            .map(|(x, _)| x.clone())
                            .collect(),
                    ));
                }
                let mut out = Vec::new();
                for x in items.iter() {
                    if test(self, x)? {
                        out.push(x.clone());
                    }
                }
                Value::list(out)
            }
            "par_any" | "par_all" | "par_find" => {
                let want = method != "par_all";
                let hit = match self.par_find(items, &f, want, span)? {
                    Some(hit) => hit,
                    None => {
                        let mut hit = None;
                        for (i, x) in items.iter().enumerate() {
                            if test(self, x)? == want {
                                hit = Some(i);
                                break;
                            }
                        }
                        hit
                    }
                };
                match method {
                    "par_find" => hit.map_or_else(Value::none, |i| Value::some(items[i].clone())),
                    "par_any" => Value::Bool(hit.is_some()),
                    _ => Value::Bool(hit.is_none()),
                }
            }
            "any" | "all" => {
                let want = method == "any";
                for x in items.iter() {
                    if test(self, x)? == want {
                        return Ok(Value::Bool(want));
                    }
                }
                Value::Bool(!want)
            }
            "find" => {
                for x in items.iter() {
                    if test(self, x)? {
                        return Ok(Value::some(x.clone()));
                    }
                }
                Value::none()
            }
            "count" | "par_count" => {
                if method == "par_count"
                    && let Some(Value::List(results)) = self.par_map(items, &f, span)?
                {
                    let n = results
                        .iter()
                        .filter(|r| matches!(r, Value::Bool(true)))
                        .count();
                    return Ok(Value::Int(n as i64));
                }
                let mut n = 0;
                for x in items.iter() {
                    if test(self, x)? {
                        n += 1;
                    }
                }
                Value::Int(n)
            }
            "fold" => {
                let mut acc = init.expect("fold has an initial value");
                for x in items.iter() {
                    acc = self.call_value(&f, vec![acc, x.clone()], span)?;
                }
                acc
            }
            "par_fold" => {
                let init = init.expect("par_fold has an initial value");
                let partials = match self.par_fold(&init, items, &f, span)? {
                    Some(partials) => partials,
                    None => {
                        let mut partials = Vec::new();
                        for part in parallel::fold_parts(&init, items) {
                            partials.push(self.fold_part(&f, part, span)?);
                        }
                        partials
                    }
                };
                self.fold_part(&f, partials, span)?
            }
            "flat_map" => {
                let mut out = Vec::new();
                for (i, x) in items.iter().enumerate() {
                    if let Value::List(part) = key(self, i, x)? {
                        out.extend(part.iter().cloned());
                    }
                }
                Value::list(out)
            }
            "group_by" => {
                let mut groups: std::collections::BTreeMap<MapKey, Vec<Value>> =
                    std::collections::BTreeMap::new();
                for (i, x) in items.iter().enumerate() {
                    let key = key(self, i, x)?;
                    groups
                        .entry(MapKey::from_value(&key))
                        .or_default()
                        .push(x.clone());
                }
                Value::Map(Rc::new(
                    groups
                        .into_iter()
                        .map(|(k, v)| (k, Value::list(v)))
                        .collect(),
                ))
            }
            "sum_by" => {
                let mut total: i64 = 0;
                for (i, x) in items.iter().enumerate() {
                    let Value::Int(n) = key(self, i, x)? else {
                        unreachable!("checked Int function");
                    };
                    total = total
                        .checked_add(n)
                        .ok_or_else(|| builtins::overflow(span))?;
                }
                Value::Int(total)
            }
            "sum_by.float" => {
                let mut total = 0.0;
                for (i, x) in items.iter().enumerate() {
                    let v = key(self, i, x)?;
                    total += v.as_f64().expect("checked Float function");
                }
                Value::Float(total)
            }
            "min_by" | "max_by" => {
                let mut best: Option<(Value, Value)> = None;
                for (i, x) in items.iter().enumerate() {
                    let key = key(self, i, x)?;
                    let better = match &best {
                        None => true,
                        // The first of equal keys wins.
                        Some((k, _)) if method == "min_by" => key.compare(k).is_lt(),
                        Some((k, _)) => key.compare(k).is_gt(),
                    };
                    if better {
                        best = Some((key, x.clone()));
                    }
                }
                best.map_or(Value::none(), |(_, x)| Value::some(x))
            }
            "sort_by" | "par_sort_by" => {
                let mut keyed = Vec::with_capacity(items.len());
                if method == "par_sort_by"
                    && let Some(Value::List(keys)) = self.par_map(items, &f, span)?
                {
                    keyed.extend(keys.iter().cloned().zip(items.iter().cloned()));
                } else {
                    for x in items.iter() {
                        keyed.push((self.call_value(&f, vec![x.clone()], span)?, x.clone()));
                    }
                }
                // Stable: equal keys keep their order.
                keyed.sort_by(|(a, _), (b, _)| a.compare(b));
                Value::list(keyed.into_iter().map(|(_, x)| x).collect())
            }
            _ => unreachable!("not a higher-order method: {method}"),
        })
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
            FutureState::Cancelled => {
                *state.borrow_mut() = FutureState::Cancelled;
                return Err(RuntimeError::new(
                    "R0012",
                    span,
                    Text::new(
                        "this future was stopped by `timeout`, so it has no result",
                        "أُوقفت هذه العملية بسبب `timeout`، فليس لها ناتج",
                    ),
                )
                .into());
            }
        };
        let result = match thunk {
            Thunk::Call { name, args, span } => {
                let f = self.fns[name.as_str()];
                self.call_fn(f, args, span)?
            }
            Thunk::Infer(call) => ai::run_infer(self.ai, &call),
            Thunk::Embed {
                model,
                text,
                timeout_ms,
            } => ai::run_embed(self.ai, model, text, timeout_ms),
            Thunk::Stream(call) => self.run_stream(&call, span)?,
            Thunk::Run(call) => self.run_agent(&call, span)?,
            Thunk::All(items) => self.await_all(&items, span)?,
            Thunk::Timeout(future, limit) => self.await_timeout(&future, limit, span)?,
        };
        *state.borrow_mut() = FutureState::Done(result.clone());
        Ok(result)
    }

    /// `ai.stream`: the provider streams on its own thread and sends each
    /// piece here, where `on_text` runs as the pieces arrive. If `on_text`
    /// fails, the stream is stopped and its error is the program's.
    fn run_stream(&mut self, call: &ai::StreamCall, span: Span) -> Exec<Value> {
        let request = call.infer.request(self.ai);
        let mut provider = match self.ai.take_checked(&request) {
            Ok(p) => p,
            Err(e) => return Ok(Value::Result(Err(Rc::new(ai::ai_error_value(&e))))),
        };
        let mut failed: Option<Flow> = None;
        #[cfg(not(target_family = "wasm"))]
        let answer = {
            let (tx, rx) = std::sync::mpsc::channel::<String>();
            let provider = &mut provider;
            let req = &request;
            std::thread::scope(|scope| {
                let handle = scope.spawn(move || {
                    provider.stream(req, &mut |piece| tx.send(piece.to_string()).is_ok())
                });
                for piece in rx.iter() {
                    if let Err(e) = self.call_value(&call.on_text, vec![Value::str(piece)], span) {
                        failed = Some(e);
                        break;
                    }
                }
                // Further pieces fail to send, which stops the stream.
                drop(rx);
                handle.join().unwrap_or_else(|_| {
                    Err(elomar_ai_core::AiError::new(
                        elomar_ai_core::AiErrorKind::ProviderUnavailable,
                        "the stream stopped unexpectedly",
                    ))
                })
            })
        };
        // WebAssembly (the browser playground) has no threads: the pieces
        // are collected first, then given to `on_text` in order.
        #[cfg(target_family = "wasm")]
        let answer = {
            let mut pieces = Vec::new();
            let answer = provider.stream(&request, &mut |piece| {
                pieces.push(piece.to_string());
                true
            });
            for piece in pieces {
                if let Err(e) = self.call_value(&call.on_text, vec![Value::str(piece)], span) {
                    failed = Some(e);
                    break;
                }
            }
            answer
        };
        self.ai.put_back(request.model.clone(), provider);
        if let Some(e) = failed {
            return Err(e);
        }
        Ok(match answer {
            Ok(text) => Value::Result(Ok(Rc::new(Value::str(text)))),
            Err(e) => Value::Result(Err(Rc::new(ai::ai_error_value(&e)))),
        })
    }

    /// Whether statements and loop steps must check the time limits (or
    /// whether a parallel call was stopped).
    #[inline]
    fn watching(&self) -> bool {
        !self.deadlines.is_empty() || self.stop.is_some()
    }

    /// Unwinds to the outermost `timeout` whose time is up, if any.
    #[inline(never)]
    fn check_deadlines(&self) -> Exec<()> {
        if let Some(stop) = &self.stop
            && stop.load(std::sync::atomic::Ordering::Relaxed)
        {
            return Err(Flow::Timeout(parallel::STOPPED));
        }
        if self.deadlines.is_empty() {
            return Ok(());
        }
        let now = stdextra::now_ms();
        match self.deadlines.iter().position(|d| now >= *d) {
            Some(i) => Err(Flow::Timeout(i)),
            None => Ok(()),
        }
    }

    /// `await timeout(future, limit)`: `Some(result)`, or `None` if the
    /// future was still running when the time was up; it is then stopped
    /// at its next statement. Waiting for the network or an AI model is not
    /// interrupted, but the program stops as soon as that call returns.
    fn await_timeout(&mut self, future: &Value, limit: i64, span: Span) -> Exec<Value> {
        let Value::Future(state) = future else {
            return Ok(Value::some(future.clone()));
        };
        let level = self.deadlines.len();
        self.deadlines
            .push(stdextra::now_ms().saturating_add(limit.max(0)));
        let frames = self.frames.len();
        let calls = self.calls.len();
        let result = self.force(state, span);
        self.deadlines.truncate(level);
        match result {
            Ok(v) => Ok(Value::some(v)),
            Err(Flow::Timeout(i)) => {
                // Unwinding skipped the code that pops call frames.
                self.frames.truncate(frames);
                self.calls.truncate(calls);
                *state.borrow_mut() = FutureState::Cancelled;
                if i == level {
                    Ok(Value::none())
                } else {
                    Err(Flow::Timeout(i))
                }
            }
            Err(other) => Err(other),
        }
    }

    /// `await all([...])`: the results in order. `ai.infer` futures that
    /// have not started run together (see `AiRuntime::infer_all`); the
    /// others run one after another.
    fn await_all(&mut self, items: &[Value], span: Span) -> Exec<Value> {
        let mut batch: Vec<(usize, ai::InferCall)> = Vec::new();
        for (i, item) in items.iter().enumerate() {
            if let Value::Future(state) = item {
                let pending_infer =
                    matches!(&*state.borrow(), FutureState::Pending(Thunk::Infer(_)));
                if pending_infer {
                    let previous =
                        std::mem::replace(&mut *state.borrow_mut(), FutureState::Running);
                    if let FutureState::Pending(Thunk::Infer(call)) = previous {
                        batch.push((i, call));
                    }
                }
            }
        }
        let mut results: Vec<Option<Value>> = vec![None; items.len()];
        if !batch.is_empty() {
            let requests: Vec<_> = batch.iter().map(|(_, c)| c.request(self.ai)).collect();
            let answers = self.ai.infer_all(&requests);
            for ((i, call), answer) in batch.iter().zip(answers) {
                let value = ai::infer_result(call, answer);
                if let Value::Future(state) = &items[*i] {
                    *state.borrow_mut() = FutureState::Done(value.clone());
                }
                results[*i] = Some(value);
            }
        }
        let mut out = Vec::with_capacity(items.len());
        let mut i = 0;
        while i < items.len() {
            // Neighbouring calls that can run in parallel do (see `parallel`).
            if results[i].is_none() {
                let ran = self.run_parallel(items, i, &mut results)?;
                if ran > 0 {
                    out.extend(
                        results[i..i + ran]
                            .iter_mut()
                            .map(|r| r.take().expect("ran")),
                    );
                    i += ran;
                    continue;
                }
            }
            out.push(match results[i].take() {
                Some(v) => v,
                None => match &items[i] {
                    Value::Future(state) => self.force(state, span)?,
                    other => other.clone(),
                },
            });
            i += 1;
        }
        Ok(Value::list(out))
    }

    /// The future for `ai.run<T>(model, instructions, input, tools,
    /// max_steps, timeout)` or `agent.run<T>(model, instructions, input,
    /// tools, mcp, require_approval, approve, on_step, max_steps,
    /// timeout)`; the checker put the arguments in this order and appended
    /// each tool's parameter and result types to `type_args`.
    fn run_future(&self, type_args: &[TypeExpr], args: Vec<Value>, agent: bool) -> Value {
        let mut args = args.into_iter();
        let mut text = || match args.next() {
            Some(Value::Str(s)) => s.to_string(),
            _ => String::new(),
        };
        let model = text();
        let instructions = text();
        let input = args.next().unwrap_or(Value::Unit);
        let tool_values = match args.next() {
            Some(Value::List(items)) => items,
            _ => Rc::new(Vec::new()),
        };
        let strings = |v: &Value| match v {
            Value::Str(s) => s.to_string(),
            _ => String::new(),
        };
        let function = |v: Option<Value>| match v {
            Some(Value::Function(f)) => Some(f),
            _ => None,
        };
        let (servers, require_approval, approve, on_step) = if agent {
            let servers = match args.next() {
                Some(Value::List(items)) => items.iter().map(strings).collect(),
                _ => Vec::new(),
            };
            let require = match args.next() {
                Some(Value::List(items)) => Some(items.iter().map(strings).collect()),
                _ => None,
            };
            (
                servers,
                require,
                function(args.next()),
                function(args.next()),
            )
        } else {
            (Vec::new(), None, None, None)
        };
        let max_steps = match args.next() {
            Some(Value::Int(n)) => n,
            _ => 5,
        };
        let timeout_ms = match args.next() {
            Some(Value::Duration(ms)) => Some(ms),
            _ => None,
        };
        let mut tools = Vec::new();
        for (i, v) in tool_values.iter().enumerate() {
            let Value::Function(func) = v else { continue };
            let Function::Named(fn_name) = &**func else {
                continue;
            };
            let f = self.fns[fn_name.as_str()];
            let param_schema = ai::schema_of(&type_args[1 + 2 * i], &self.unit_enums);
            let Schema::Record(fields) = param_schema else {
                continue;
            };
            let params = f
                .params
                .iter()
                .filter_map(|p| {
                    fields
                        .iter()
                        .find(|(n, _)| *n == p.name.name)
                        .map(|(n, s)| (n.clone(), s.clone()))
                })
                .collect();
            let ret_expr = &type_args[2 + 2 * i];
            let ret = match &ret_expr.kind {
                TypeExprKind::Named { name, .. } if name.name == "Unit" => None,
                _ => Some(ai::schema_of(ret_expr, &self.unit_enums)),
            };
            let name = fn_name
                .rsplit_once('.')
                .map_or(fn_name.as_str(), |(_, n)| n);
            tools.push(RunTool {
                name: name.to_string(),
                fn_name: fn_name.clone(),
                params,
                ret,
            });
        }
        Value::future(Thunk::Run(Box::new(RunCall {
            infer: InferCall {
                schema: ai::schema_of(&type_args[0], &self.unit_enums),
                model,
                instructions,
                input,
                timeout_ms,
            },
            tools,
            max_steps,
            servers,
            require_approval,
            approve,
            on_step,
        })))
    }

    /// The `ai.run` loop: ask the model, run the tools it asks for, send
    /// back their results, until it answers or `max_steps` is reached.
    fn run_agent(&mut self, call: &RunCall, span: Span) -> Exec<Value> {
        use elomar_ai_core::{AiError, AiErrorKind, Message, Step, ToolSpec};
        let request = call.infer.request(self.ai);
        let mut specs: Vec<ToolSpec> = call
            .tools
            .iter()
            .map(|t| ToolSpec {
                name: t.name.clone(),
                params: Schema::Record(t.params.clone()),
                description: String::new(),
                raw_schema: None,
            })
            .collect();
        // The model sees an MCP tool as `server__tool`; the program as
        // `server.tool`.
        let mut remote: HashMap<String, (String, String)> = HashMap::new();
        for server in &call.servers {
            let tools = match self.mcp.tools(server) {
                Ok(tools) => tools,
                Err((kind, message)) => {
                    let kind = if kind == "NotConfigured" {
                        AiErrorKind::PolicyDenied
                    } else {
                        AiErrorKind::ProviderUnavailable
                    };
                    return Ok(Value::Result(Err(Rc::new(ai::ai_error_value(
                        &AiError::new(kind, message),
                    )))));
                }
            };
            for t in tools {
                let name: String = format!("{server}__{}", t.name)
                    .chars()
                    .map(|c| {
                        if c.is_ascii_alphanumeric() || c == '_' || c == '-' {
                            c
                        } else {
                            '_'
                        }
                    })
                    .take(64)
                    .collect();
                if remote.contains_key(&name) || specs.iter().any(|s| s.name == name) {
                    continue;
                }
                specs.push(ToolSpec {
                    name: name.clone(),
                    params: Schema::Record(Vec::new()),
                    description: t.description,
                    raw_schema: Some(t.input_schema),
                });
                remote.insert(name, (server.clone(), t.name));
            }
        }
        let mut history = Vec::new();
        for _ in 0..call.max_steps.max(1) {
            match self.ai.step(&request, &specs, &history) {
                Err(e) => return Ok(Value::Result(Err(Rc::new(ai::ai_error_value(&e))))),
                Ok(Step::Final(json)) => {
                    return Ok(Value::Result(Ok(Rc::new(ai::from_json(
                        &call.infer.schema,
                        &json,
                    )))));
                }
                Ok(Step::ToolCalls(calls)) => {
                    history.push(Message::ToolCalls(calls.clone()));
                    for c in calls {
                        let shown = match remote.get(&c.name) {
                            Some((server, tool)) => format!("{server}.{tool}"),
                            None => c.name.clone(),
                        };
                        let arguments = c.arguments.to_string();
                        let needs_approval = call.approve.is_some()
                            && call
                                .require_approval
                                .as_ref()
                                .is_none_or(|list| list.contains(&shown));
                        let approved = match (&call.approve, needs_approval) {
                            (Some(f), true) => {
                                let request = Value::Record(Rc::new(vec![
                                    (Rc::from("tool"), Value::str(shown.as_str())),
                                    (Rc::from("arguments"), Value::str(arguments.as_str())),
                                ]));
                                matches!(
                                    self.call_value(f, vec![request], span)?,
                                    Value::Bool(true)
                                )
                            }
                            _ => true,
                        };
                        let content = if !approved {
                            serde_json::json!({ "error": "the user did not approve this call" })
                                .to_string()
                        } else if let Some((server, tool)) = remote.get(&c.name) {
                            let arguments = match &c.arguments {
                                serde_json::Value::Object(_) => c.arguments.clone(),
                                _ => serde_json::json!({}),
                            };
                            match self.mcp.call(server, tool, arguments) {
                                Ok(text) => text,
                                Err((_, message)) => {
                                    serde_json::json!({ "error": message }).to_string()
                                }
                            }
                        } else {
                            self.run_tool(call, &c, span)?
                        };
                        if let Some(f) = &call.on_step {
                            let step = Value::Record(Rc::new(vec![
                                (Rc::from("tool"), Value::str(shown.as_str())),
                                (Rc::from("arguments"), Value::str(arguments.as_str())),
                                (Rc::from("result"), Value::str(content.as_str())),
                                (Rc::from("approved"), Value::Bool(approved)),
                            ]));
                            self.call_value(f, vec![step], span)?;
                        }
                        history.push(Message::ToolResult { id: c.id, content });
                    }
                }
            }
        }
        Ok(Value::Result(Err(Rc::new(ai::ai_error_value(
            &elomar_ai_core::AiError::new(
                elomar_ai_core::AiErrorKind::BudgetExceeded,
                format!(
                    "the model did not finish within {} step(s) (`max_steps`)",
                    call.max_steps.max(1)
                ),
            ),
        )))))
    }

    /// Runs one tool call and returns its result as JSON text. Unknown
    /// tools and invalid arguments are reported back to the model.
    fn run_tool(
        &mut self,
        call: &RunCall,
        c: &elomar_ai_core::ToolCall,
        span: Span,
    ) -> Exec<String> {
        let Some(tool) = call.tools.iter().find(|t| t.name == c.name) else {
            return Ok(
                serde_json::json!({ "error": format!("unknown tool `{}`", c.name) }).to_string(),
            );
        };
        let schema = Schema::Record(tool.params.clone());
        if let Err(problem) = schema.validate(&c.arguments) {
            return Ok(
                serde_json::json!({ "error": format!("invalid arguments: {problem}") }).to_string(),
            );
        }
        let null = serde_json::Value::Null;
        let args = tool
            .params
            .iter()
            .map(|(n, s)| ai::from_json(s, c.arguments.get(n).unwrap_or(&null)))
            .collect();
        let f = self.fns[tool.fn_name.as_str()];
        let result = self.call_fn(f, args, span)?;
        Ok(match tool.ret {
            None => "null".into(),
            Some(_) => ai::to_json(&result).to_string(),
        })
    }

    fn check_depth(&self, call_span: Span) -> Exec<()> {
        if self.frames.len() + self.vm.depth > MAX_CALL_DEPTH {
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
        Ok(())
    }

    fn call_fn(&mut self, f: &'p FnDecl, args: Vec<Value>, call_span: Span) -> Exec<Value> {
        let mut frame = self.take_frame();
        frame
            .vars
            .extend(f.params.iter().map(|p| p.name.name.as_str()).zip(args));
        self.enter_fn(f, frame, call_span)
    }

    /// The shared copy of a field name from the program.
    #[inline]
    fn field_name(&mut self, name: &'p str) -> Rc<str> {
        self.field_names
            .entry(name.as_ptr() as usize)
            .or_insert_with(|| Rc::from(name))
            .clone()
    }

    /// An empty frame, reusing a finished call's memory when there is one.
    #[inline]
    fn take_frame(&mut self) -> Frame<'p> {
        self.spare
            .pop()
            .unwrap_or_else(|| Frame::new(None, Vec::with_capacity(8)))
    }

    /// Keeps `frame`'s memory for a later call.
    #[inline]
    fn recycle(&mut self, mut frame: Frame<'p>) {
        frame.vars.clear();
        frame.marks.clear();
        frame.captured = None;
        if self.spare.len() < 64 {
            self.spare.push(frame);
        }
    }

    /// Runs `f` with its parameters already in `frame`.
    fn enter_fn(&mut self, f: &'p FnDecl, mut frame: Frame<'p>, call_span: Span) -> Exec<Value> {
        if self.vm.enabled
            && let Some(code) = self.compiled(f)
        {
            let args = frame.vars.drain(..).map(|(_, v)| v);
            self.vm.stack.extend(args);
            self.recycle(frame);
            return self.call_compiled(f, &code, call_span);
        }
        if let Err(e) = self.check_depth(call_span) {
            self.recycle(frame);
            return Err(e);
        }
        self.frames.push(frame);
        if self.debugger.is_some() {
            self.calls.push(StackFrame {
                name: f.name.name.clone(),
                span: call_span,
            });
        }
        let result = self.exec_block(&f.body);
        if let Some(frame) = self.frames.pop() {
            self.recycle(frame);
        }
        if self.debugger.is_some() {
            self.calls.pop();
        }
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

/// List methods that take a function argument.
const HIGHER_ORDER: &[&str] = &[
    "map",
    "par_map",
    "filter",
    "par_filter",
    "any",
    "all",
    "find",
    "count",
    "fold",
    "par_fold",
    "sort_by",
    "par_sort_by",
    "par_any",
    "par_all",
    "par_find",
    "par_count",
    "flat_map",
    "group_by",
    "sum_by",
    "sum_by.float",
    "min_by",
    "max_by",
    "par_flat_map",
    "par_group_by",
    "par_sum_by",
    "par_sum_by.float",
    "par_min_by",
    "par_max_by",
];

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

/// The values for `let (a, b) = v` or `for (k, v) in ...`: a tuple's
/// elements, or a map entry's key and value.
fn unpack(v: &Value) -> Vec<Value> {
    match v {
        Value::Tuple(items) => items.to_vec(),
        Value::Record(fields) => fields.iter().map(|(_, x)| x.clone()).collect(),
        _ => Vec::new(),
    }
}

fn match_pattern<'a>(p: &'a Pattern, v: &Value, bindings: &mut Vec<(&'a str, Value)>) -> bool {
    match (&p.kind, v) {
        (PatternKind::Wildcard, _) => true,
        (PatternKind::Name(n), Value::Option(None)) if n == "None" => true,
        (PatternKind::Name(n), _) if n == "None" => false,
        (PatternKind::Name(n), _) => {
            bindings.push((n.as_str(), v.clone()));
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
        (PatternKind::Variant { variant, args, .. }, Value::Enum(e)) => {
            *e.variant == *variant.name
                && args
                    .iter()
                    .zip(e.fields.iter())
                    .all(|(p, f)| match_pattern(p, f, bindings))
        }
        (PatternKind::Variant { .. }, _) => false,
        (PatternKind::Int(n), Value::Int(m)) => n == m,
        (PatternKind::Float(x), _) => v.as_f64() == Some(*x),
        (PatternKind::Int(n), Value::Float(x)) => *n as f64 == *x,
        (PatternKind::Str(s), Value::Str(t)) => **s == **t,
        (PatternKind::Bool(a), Value::Bool(b)) => a == b,
        (PatternKind::Or(alts), _) => alts.iter().any(|a| match_pattern(a, v, bindings)),
        (PatternKind::Tuple(ps), Value::Tuple(items)) => {
            ps.len() == items.len()
                && ps
                    .iter()
                    .zip(items.iter())
                    .all(|(p, x)| match_pattern(p, x, bindings))
        }
        (
            PatternKind::Range {
                start,
                end,
                inclusive,
            },
            Value::Int(n),
        ) => n >= start && if *inclusive { n <= end } else { n < end },
        _ => false,
    }
}

#[cfg(test)]
mod tests;

impl Inspect for Interpreter<'_, '_> {
    fn depth(&self) -> usize {
        self.calls.len()
    }

    fn stack(&self) -> Vec<StackFrame> {
        self.calls.iter().rev().cloned().collect()
    }

    fn locals(&self, frame: usize) -> Vec<(String, Value)> {
        let Some(index) = self.calls.len().checked_sub(frame + 1) else {
            return Vec::new();
        };
        let Some(frame) = self.frames.get(index + 1) else {
            return Vec::new();
        };
        // Inner scopes and later definitions shadow earlier ones.
        let mut out: Vec<(String, Value)> = Vec::new();
        let captured = frame
            .captured
            .iter()
            .flat_map(|c| c.iter().map(|(n, v)| (&**n, v)));
        let locals = frame.vars.iter().map(|(n, v)| (*n, v));
        for (name, value) in captured.chain(locals) {
            match out.iter_mut().find(|(n, _)| n == name) {
                Some(slot) => slot.1 = value.clone(),
                None => out.push((name.to_string(), value.clone())),
            }
        }
        out
    }

    fn globals(&self) -> Vec<(String, Value)> {
        let mut out: Vec<(String, Value)> = self
            .globals
            .iter()
            .map(|(n, v)| (n.clone(), v.clone()))
            .collect();
        out.sort_by(|a, b| a.0.cmp(&b.0));
        out
    }
}
