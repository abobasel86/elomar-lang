//! The bytecode engine.
//!
//! The first time a function is called, its body is compiled into a list of
//! instructions over numbered registers: every local variable gets its own
//! register, so reading `x` is an array access instead of a search by name,
//! and loops are jumps instead of nested calls in Rust. The result is the
//! same as the tree-walking interpreter's, only faster.
//!
//! Each function is compiled on its own. A function that uses something the
//! compiler does not handle (see [`Compiler`]) keeps running in the tree
//! walker, and the two call each other freely: closures, futures and the
//! standard library are shared. The debugger turns the engine off, as it
//! follows statements in the tree. With the environment variable
//! `ELOMAR_VM_DEBUG` set, each function left to the tree walker is named on
//! standard error.
//!
//! What the engine must keep exactly as the tree walker does it:
//! - the order in which expressions are evaluated, and so which error comes
//!   first;
//! - the span of every error, and the call trace;
//! - what a closure captures: every variable visible where it is made, in
//!   the order they were defined;
//! - the call depth limit (`R0005`) and `timeout`, checked on every call and
//!   loop step.

use std::rc::Rc;
use std::sync::atomic::{AtomicBool, Ordering};

use elomar_ast::*;
use elomar_diagnostics::Text;

use crate::{
    Captured, Exec, FastMap, Flow, Function, Interpreter, MODULES, MapKey, RuntimeError, Step,
    Thunk, Value, builtins, int_binary, match_pattern, stdextra, unpack, walk_place,
};

/// Which engine runs programs: the bytecode engine (the default) or only the
/// tree walker.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Engine {
    Bytecode,
    Tree,
}

static TREE_ONLY: AtomicBool = AtomicBool::new(false);

/// Sets the engine for programs started from now on, in this process.
/// `elomar` sets it from the `ELOMAR_ENGINE` environment variable (`tree`),
/// so the two engines can be compared on any program.
pub fn set_engine(engine: Engine) {
    TREE_ONLY.store(engine == Engine::Tree, Ordering::Relaxed);
}

/// The engine that programs started now use.
pub fn engine() -> Engine {
    if TREE_ONLY.load(Ordering::Relaxed) {
        Engine::Tree
    } else {
        Engine::Bytecode
    }
}

type R = u32;

/// The state of the bytecode engine inside one [`Interpreter`].
pub(crate) struct Vm<'p> {
    pub(crate) enabled: bool,
    /// Calls running in the engine, for the call depth limit.
    pub(crate) depth: usize,
    /// The registers of every running compiled call, innermost last.
    pub(crate) stack: Vec<Value>,
    fns: Vec<&'p FnDecl>,
    fn_index: FastMap<&'p str, u32>,
    codes: Vec<Compiled<'p>>,
    /// Compiled closure bodies, by the address of their `|...|` expression.
    pub(crate) lambdas: FastMap<usize, Rc<Code<'p>>>,
    /// Names of file-level constants: a call through one of them is left
    /// to the tree walker.
    const_names: FastMap<&'p str, ()>,
}

enum Compiled<'p> {
    NotYet,
    Unsupported,
    Ready(Rc<Code<'p>>),
}

impl<'p> Vm<'p> {
    pub(crate) fn new(program: &'p Program) -> Self {
        let mut fns = Vec::new();
        let mut fn_index = FastMap::default();
        let mut const_names = FastMap::default();
        for item in &program.items {
            match item {
                Item::Fn(f) => {
                    // A later function with the same name wins, as in the
                    // tree walker's table.
                    fn_index.insert(f.name.name.as_str(), fns.len() as u32);
                    fns.push(f);
                }
                Item::Const(c) => {
                    const_names.insert(c.name.name.as_str(), ());
                }
                _ => {}
            }
        }
        Vm {
            enabled: engine() == Engine::Bytecode,
            depth: 0,
            stack: Vec::new(),
            codes: fns.iter().map(|_| Compiled::NotYet).collect(),
            lambdas: FastMap::default(),
            fns,
            fn_index,
            const_names,
        }
    }
}

/// A compiled function.
pub(crate) struct Code<'p> {
    ops: Vec<Op<'p>>,
    /// The span of each instruction, for its errors.
    spans: Vec<Span>,
    consts: Vec<Value>,
    nregs: usize,
    /// Field names of each record built.
    records: Vec<Rc<[Rc<str>]>>,
    /// Registers that receive each `let (a, b)` or `for (k, v)`; `None` for `_`.
    unpacks: Vec<Vec<Option<R>>>,
    /// Registers that receive each `match` arm's bindings, in pattern order.
    binds: Vec<Vec<R>>,
    /// Each closure's captured variables (after the running closure's own,
    /// inside a closure).
    captures: Vec<Vec<(Rc<str>, R)>>,
    interps: Vec<Vec<Part<'p>>>,
    places: Vec<Place<'p>>,
    modules: Vec<ModuleCall<'p>>,
    /// Closures made in this code that were compiled too, by key.
    nested: Vec<(usize, Code<'p>)>,
}

enum Part<'p> {
    Lit(&'p str),
    Expr(R),
}

/// A place that is assigned or changed: a local variable, then fields and
/// indexes whose keys are in registers.
struct Place<'p> {
    root: R,
    steps: Vec<PlaceStep<'p>>,
}

enum PlaceStep<'p> {
    Field(&'p str),
    Index(R, Span),
}

struct ModuleCall<'p> {
    module: &'p str,
    name: &'p str,
    type_args: &'p [TypeExpr],
}

/// One instruction. `dst` is the register written; arguments of a call are
/// in `argc` registers from `args`.
#[derive(Clone, Copy)]
enum Op<'p> {
    Int {
        dst: R,
        n: i64,
    },
    Const {
        dst: R,
        k: u32,
    },
    Move {
        dst: R,
        src: R,
    },
    /// A name that is not a local variable: a constant, `None`, a function.
    Name {
        dst: R,
        name: &'p str,
    },
    /// Inside a closure: the `i`th captured variable.
    Captured {
        dst: R,
        i: u32,
    },
    Bin {
        op: BinOp,
        dst: R,
        a: R,
        b: R,
    },
    BinInt {
        op: BinOp,
        dst: R,
        a: R,
        n: i64,
    },
    Not {
        dst: R,
        src: R,
    },
    Neg {
        dst: R,
        src: R,
    },
    Jump {
        to: u32,
    },
    /// Jumps unless `cond` is `true`.
    JumpIfNot {
        cond: R,
        to: u32,
    },
    JumpIf {
        cond: R,
        to: u32,
    },
    /// Compares, and jumps unless the comparison is true.
    CmpJump {
        op: BinOp,
        a: R,
        b: R,
        to: u32,
    },
    CmpIntJump {
        op: BinOp,
        a: R,
        n: i64,
        to: u32,
    },
    /// A loop step: checks the time limits.
    Tick,
    Call {
        f: u32,
        args: R,
        argc: u32,
        dst: R,
    },
    CallAsync {
        name: &'p str,
        args: R,
        argc: u32,
        dst: R,
    },
    /// Calls the function value in `func`; `name` is the variable's name.
    CallValue {
        func: R,
        name: &'p str,
        args: R,
        argc: u32,
        dst: R,
    },
    CallBuiltin {
        name: &'p str,
        args: R,
        argc: u32,
        dst: R,
    },
    CallModule {
        m: u32,
        args: R,
        argc: u32,
        dst: R,
    },
    CallMethod {
        method: &'p str,
        recv: R,
        args: R,
        argc: u32,
        dst: R,
    },
    CallMutating {
        method: &'p str,
        place: u32,
        args: R,
        argc: u32,
        dst: R,
    },
    Assign {
        place: u32,
        src: R,
        insert: bool,
    },
    List {
        dst: R,
        start: R,
        n: u32,
    },
    Tuple {
        dst: R,
        start: R,
        n: u32,
    },
    Record {
        dst: R,
        start: R,
        names: u32,
    },
    Map {
        dst: R,
        start: R,
        n: u32,
    },
    Interp {
        dst: R,
        parts: u32,
    },
    Field {
        dst: R,
        src: R,
        name: &'p str,
    },
    Index {
        dst: R,
        target: R,
        index: R,
    },
    Range {
        dst: R,
        start: R,
        end: R,
        inclusive: bool,
    },
    Variant {
        dst: R,
        enum_name: &'p str,
        variant: &'p str,
        start: R,
        n: u32,
    },
    Try {
        dst: R,
        src: R,
    },
    Await {
        dst: R,
        src: R,
    },
    Closure {
        dst: R,
        lambda: &'p Expr,
        captures: u32,
    },
    Unpack {
        src: R,
        targets: u32,
    },
    IterStart {
        src: R,
    },
    /// The next item into `dst`, or the end of the loop: jumps to `to`.
    IterNext {
        dst: R,
        to: u32,
    },
    /// Leaves a `for` loop early (`break`).
    IterEnd,
    /// Tests an arm's pattern; jumps to `to` (the next arm) if it fails.
    MatchArm {
        pattern: &'p Pattern,
        src: R,
        binds: u32,
        to: u32,
    },
    NoArm,
    /// Empties registers whose variables went out of scope, so the values
    /// they held can be freed or changed in place.
    Clear {
        start: R,
        n: u32,
    },
    Return {
        src: R,
    },
    ReturnUnit,
}

/// The items of a `for` loop.
enum Iter {
    List(Rc<Vec<Value>>, usize),
    Range {
        next: Option<i64>,
        end: i64,
        inclusive: bool,
    },
    Items(std::vec::IntoIter<Value>),
}

impl Iter {
    fn new(v: Value) -> Iter {
        match v {
            Value::List(items) => Iter::List(items, 0),
            Value::Range {
                start,
                end,
                inclusive,
            } => Iter::Range {
                next: Some(start),
                end,
                inclusive,
            },
            Value::Str(s) => Iter::Items(
                builtins::graphemes(&s)
                    .map(Value::str)
                    .collect::<Vec<_>>()
                    .into_iter(),
            ),
            Value::Set(items) => Iter::Items(
                items
                    .iter()
                    .map(|k| k.to_value())
                    .collect::<Vec<_>>()
                    .into_iter(),
            ),
            Value::Map(entries) => Iter::Items(
                entries
                    .iter()
                    .map(|(k, v)| {
                        Value::Record(Rc::new(vec![
                            ("key".into(), k.to_value()),
                            ("value".into(), v.clone()),
                        ]))
                    })
                    .collect::<Vec<_>>()
                    .into_iter(),
            ),
            _ => Iter::Items(Vec::new().into_iter()),
        }
    }

    fn next(&mut self) -> Option<Value> {
        match self {
            Iter::List(items, i) => {
                let v = items.get(*i)?.clone();
                *i += 1;
                Some(v)
            }
            Iter::Range {
                next,
                end,
                inclusive,
            } => {
                let i = (*next)?;
                if !(i < *end || (*inclusive && i == *end)) {
                    return None;
                }
                *next = i.checked_add(1);
                Some(Value::Int(i))
            }
            Iter::Items(items) => items.next(),
        }
    }
}

// ----- compiler ----------------------------------------------------------

/// The construct the compiler does not handle; the function stays in the
/// tree walker.
struct Unsupported;

type C<T> = Result<T, Unsupported>;

struct Loop {
    /// Where `continue` jumps.
    next: u32,
    /// `break` jumps to patch, at the loop's end.
    breaks: Vec<usize>,
}

struct Compiler<'p, 'v> {
    vm: &'v Vm<'p>,
    code: Code<'p>,
    /// Variables in scope with their registers, in the order defined.
    scope: Vec<(&'p str, R)>,
    /// The first free register.
    next: R,
    loops: Vec<Loop>,
    /// Inside a closure: the names of its captured variables, in order.
    captured: Option<Vec<Rc<str>>>,
}

fn new_code<'p>() -> Code<'p> {
    Code {
        ops: Vec::new(),
        spans: Vec::new(),
        consts: Vec::new(),
        nregs: 0,
        records: Vec::new(),
        unpacks: Vec::new(),
        binds: Vec::new(),
        captures: Vec::new(),
        interps: Vec::new(),
        places: Vec::new(),
        modules: Vec::new(),
        nested: Vec::new(),
    }
}

fn compiler<'p, 'v>(vm: &'v Vm<'p>, captured: Option<Vec<Rc<str>>>) -> Compiler<'p, 'v> {
    Compiler {
        vm,
        code: new_code(),
        scope: Vec::new(),
        next: 0,
        loops: Vec::new(),
        captured,
    }
}

fn compile<'p>(vm: &Vm<'p>, f: &'p FnDecl) -> Option<Code<'p>> {
    let mut c = compiler(vm, None);
    for p in &f.params {
        let r = c.alloc();
        c.scope.push((&p.name.name, r));
    }
    c.block(&f.body).ok()?;
    c.emit(Op::ReturnUnit, f.body.span);
    c.code.nregs = c.code.nregs.max(f.params.len());
    Some(c.code)
}

/// Compiles a closure's body; `captured` names the variables it captures,
/// in order (the closure reads the last one of each name).
fn compile_lambda<'p>(
    vm: &Vm<'p>,
    params: &'p [LambdaParam],
    body: &'p ArmBody,
    captured: Vec<Rc<str>>,
) -> Option<Code<'p>> {
    let mut c = compiler(vm, Some(captured));
    for p in params {
        let r = c.alloc();
        c.scope.push((&p.name.name, r));
    }
    match body {
        ArmBody::Expr(e) => {
            let r = c.alloc();
            c.expr_into(e, r).ok()?;
            c.emit(Op::Return { src: r }, e.span);
        }
        ArmBody::Block(b) => {
            c.block(b).ok()?;
            c.emit(Op::ReturnUnit, b.span);
        }
    }
    c.code.nregs = c.code.nregs.max(params.len());
    Some(c.code)
}

/// Whether running `e` may run a statement (in a `match` arm's block) that
/// assigns a variable. Then a variable read before `e` is copied first, as
/// the tree walker reads its value before running `e`.
fn may_assign(e: &Expr) -> bool {
    match &e.kind {
        ExprKind::Match { scrutinee, arms } => {
            may_assign(scrutinee)
                || arms.iter().any(|a| match &a.body {
                    ArmBody::Block(_) => true,
                    ArmBody::Expr(e) => may_assign(e),
                })
        }
        ExprKind::Interp(parts) => parts.iter().any(|p| match p {
            InterpPart::Expr(e) => may_assign(e),
            InterpPart::Lit(_) => false,
        }),
        ExprKind::Tuple(items) | ExprKind::List(items) => items.iter().any(may_assign),
        ExprKind::Variant { args, .. } => args.iter().any(may_assign),
        ExprKind::Record(fields) => fields.iter().any(|f| may_assign(&f.value)),
        ExprKind::Unary { expr, .. }
        | ExprKind::Await(expr)
        | ExprKind::Try(expr)
        | ExprKind::Field { target: expr, .. } => may_assign(expr),
        ExprKind::Binary { lhs, rhs, .. } => may_assign(lhs) || may_assign(rhs),
        ExprKind::Range { start, end, .. } => may_assign(start) || may_assign(end),
        ExprKind::Index { target, index } => may_assign(target) || may_assign(index),
        ExprKind::Call { callee, args, .. } => {
            may_assign(callee) || args.iter().any(|a| may_assign(&a.value))
        }
        ExprKind::Map(entries) => entries.iter().any(|(k, v)| may_assign(k) || may_assign(v)),
        // A closure's body runs on its own copies of the variables.
        ExprKind::Lambda { .. }
        | ExprKind::Int(_)
        | ExprKind::Float(_)
        | ExprKind::Str(_)
        | ExprKind::Bool(_)
        | ExprKind::Duration(_)
        | ExprKind::Name(_) => false,
    }
}

fn is_comparison(op: BinOp) -> bool {
    matches!(
        op,
        BinOp::Eq | BinOp::Ne | BinOp::Lt | BinOp::Le | BinOp::Gt | BinOp::Ge
    )
}

impl<'p> Compiler<'p, '_> {
    fn alloc(&mut self) -> R {
        let r = self.next;
        self.next += 1;
        self.code.nregs = self.code.nregs.max(self.next as usize);
        r
    }

    /// `n` consecutive registers.
    fn alloc_n(&mut self, n: usize) -> R {
        let start = self.next;
        for _ in 0..n {
            self.alloc();
        }
        start
    }

    fn emit(&mut self, op: Op<'p>, span: Span) -> usize {
        self.code.ops.push(op);
        self.code.spans.push(span);
        self.code.ops.len() - 1
    }

    fn here(&self) -> u32 {
        self.code.ops.len() as u32
    }

    /// Points the jump at `at` to the current end.
    fn patch(&mut self, at: usize) {
        let here = self.here();
        match &mut self.code.ops[at] {
            Op::Jump { to }
            | Op::JumpIfNot { to, .. }
            | Op::JumpIf { to, .. }
            | Op::CmpJump { to, .. }
            | Op::CmpIntJump { to, .. }
            | Op::IterNext { to, .. }
            | Op::MatchArm { to, .. } => *to = here,
            _ => unreachable!("not a jump"),
        }
    }

    fn constant(&mut self, dst: R, v: Value, span: Span) {
        let k = self.code.consts.len() as u32;
        self.code.consts.push(v);
        self.emit(Op::Const { dst, k }, span);
    }

    /// Inside a closure: which captured variable `name` is.
    fn captured(&self, name: &str) -> Option<u32> {
        let names = self.captured.as_ref()?;
        names.iter().rposition(|n| &**n == name).map(|i| i as u32)
    }

    fn local(&self, name: &str) -> Option<R> {
        self.scope
            .iter()
            .rev()
            .find(|(n, _)| *n == name)
            .map(|(_, r)| *r)
    }

    /// Ends a scope opened when `scope` had `mark` entries and `next` was
    /// the first free register.
    fn end_scope(&mut self, mark: usize, next: R, span: Span) {
        if self.next > next {
            self.emit(
                Op::Clear {
                    start: next,
                    n: self.next - next,
                },
                span,
            );
        }
        self.scope.truncate(mark);
        self.next = next;
    }

    fn block(&mut self, b: &'p Block) -> C<()> {
        let (mark, next) = (self.scope.len(), self.next);
        for s in &b.stmts {
            self.stmt(s)?;
        }
        self.end_scope(mark, next, b.span);
        Ok(())
    }

    fn stmt(&mut self, s: &'p Stmt) -> C<()> {
        let temps = self.next;
        match &s.kind {
            StmtKind::Let { name, value, .. } => {
                let r = self.alloc();
                let after = self.next;
                self.expr_into(value, r)?;
                self.next = after;
                self.scope.push((&name.name, r));
                return Ok(());
            }
            StmtKind::LetTuple { names, value, .. } => {
                let targets: Vec<Option<R>> = names
                    .iter()
                    .map(|n| (n.name != "_").then(|| self.alloc()))
                    .collect();
                let after = self.next;
                let t = self.alloc();
                self.expr_into(value, t)?;
                let k = self.code.unpacks.len() as u32;
                self.code.unpacks.push(targets.clone());
                self.emit(Op::Unpack { src: t, targets: k }, s.span);
                self.next = after;
                for (n, r) in names.iter().zip(targets) {
                    if let Some(r) = r {
                        self.scope.push((&n.name, r));
                    }
                }
                return Ok(());
            }
            StmtKind::Expr(e) => {
                let t = self.alloc();
                self.expr_into(e, t)?;
            }
            StmtKind::Assign { target, op, value } => self.assign(s, target, *op, value)?,
            StmtKind::If {
                cond,
                then,
                otherwise,
            } => {
                let skip = self.jump_unless(cond)?;
                self.next = temps;
                self.block(then)?;
                match otherwise {
                    None => self.patch(skip),
                    Some(other) => {
                        let end = self.emit(Op::Jump { to: 0 }, s.span);
                        self.patch(skip);
                        match other {
                            Else::Block(b) => self.block(b)?,
                            Else::If(stmt) => self.stmt(stmt)?,
                        }
                        self.patch(end);
                    }
                }
            }
            StmtKind::While { cond, body } => {
                let start = self.here();
                let exit = self.jump_unless(cond)?;
                self.next = temps;
                self.emit(Op::Tick, s.span);
                self.loops.push(Loop {
                    next: start,
                    breaks: Vec::new(),
                });
                self.block(body)?;
                self.emit(Op::Jump { to: start }, s.span);
                let lp = self.loops.pop().expect("loop");
                self.patch(exit);
                for b in lp.breaks {
                    self.patch(b);
                }
            }
            StmtKind::For { vars, iter, body } => self.for_loop(s, vars, iter, body)?,
            StmtKind::Return(value) => match value {
                Some(e) => {
                    let r = self.operand(e)?;
                    self.emit(Op::Return { src: r }, s.span);
                }
                None => {
                    self.emit(Op::ReturnUnit, s.span);
                }
            },
            StmtKind::Break => {
                let at = self.emit(Op::Jump { to: 0 }, s.span);
                self.loops.last_mut().ok_or(Unsupported)?.breaks.push(at);
            }
            StmtKind::Continue => {
                let to = self.loops.last().ok_or(Unsupported)?.next;
                self.emit(Op::Jump { to }, s.span);
            }
            StmtKind::Block(b) => self.block(b)?,
        }
        self.next = temps;
        Ok(())
    }

    fn for_loop(
        &mut self,
        s: &'p Stmt,
        vars: &'p [Ident],
        iter: &'p Expr,
        body: &'p Block,
    ) -> C<()> {
        let temps = self.next;
        let items = self.operand(iter)?;
        self.emit(Op::IterStart { src: items }, iter.span);
        self.next = temps;
        let (mark, outer) = (self.scope.len(), self.next);
        // The loop variables, as the tree walker defines them: one name
        // (even `_`), or each name but `_`.
        let (item, targets) = if vars.len() == 1 {
            (self.alloc(), Vec::new())
        } else {
            let targets: Vec<Option<R>> = vars
                .iter()
                .map(|v| (v.name != "_").then(|| self.alloc()))
                .collect();
            (self.alloc(), targets)
        };
        let next = self.emit(Op::IterNext { dst: item, to: 0 }, s.span);
        self.emit(Op::Tick, s.span);
        if vars.len() == 1 {
            self.scope.push((&vars[0].name, item));
        } else {
            let k = self.code.unpacks.len() as u32;
            self.code.unpacks.push(targets.clone());
            self.emit(
                Op::Unpack {
                    src: item,
                    targets: k,
                },
                s.span,
            );
            for (v, r) in vars.iter().zip(targets) {
                if let Some(r) = r {
                    self.scope.push((&v.name, r));
                }
            }
        }
        self.loops.push(Loop {
            next: next as u32,
            breaks: Vec::new(),
        });
        self.block(body)?;
        self.emit(Op::Jump { to: next as u32 }, s.span);
        let lp = self.loops.pop().expect("loop");
        // `break` leaves through `IterEnd`; the end of the items skips it.
        for b in lp.breaks {
            self.patch(b);
        }
        self.emit(Op::IterEnd, s.span);
        self.patch(next);
        self.end_scope(mark, outer, s.span);
        Ok(())
    }

    fn assign(&mut self, s: &'p Stmt, target: &'p Expr, op: AssignOp, value: &'p Expr) -> C<()> {
        let bin = match op {
            AssignOp::Set => None,
            AssignOp::Add => Some(BinOp::Add),
            AssignOp::Sub => Some(BinOp::Sub),
            AssignOp::Mul => Some(BinOp::Mul),
            AssignOp::Div => Some(BinOp::Div),
        };
        if let ExprKind::Name(n) = &target.kind {
            let slot = self.local(n).ok_or(Unsupported)?;
            match bin {
                None => self.expr_into(value, slot)?,
                Some(op) => {
                    // The value first, then the variable's current value.
                    if let ExprKind::Int(n) = value.kind {
                        self.emit(
                            Op::BinInt {
                                op,
                                dst: slot,
                                a: slot,
                                n,
                            },
                            s.span,
                        );
                    } else {
                        let b = self.operand(value)?;
                        self.emit(
                            Op::Bin {
                                op,
                                dst: slot,
                                a: slot,
                                b,
                            },
                            s.span,
                        );
                    }
                }
            }
            return Ok(());
        }
        let v = self.alloc();
        self.expr_into(value, v)?;
        if let Some(op) = bin {
            let current = self.alloc();
            self.expr_into(target, current)?;
            self.emit(
                Op::Bin {
                    op,
                    dst: v,
                    a: current,
                    b: v,
                },
                s.span,
            );
        }
        let place = self.place(target)?;
        self.emit(
            Op::Assign {
                place,
                src: v,
                insert: bin.is_none(),
            },
            s.span,
        );
        Ok(())
    }

    /// A place rooted in a local variable. Index keys are evaluated in the
    /// tree walker's order: the last step's first.
    fn place(&mut self, target: &'p Expr) -> C<u32> {
        let mut steps = Vec::new();
        let mut cur = target;
        let root = loop {
            match &cur.kind {
                ExprKind::Name(n) => break self.local(n).ok_or(Unsupported)?,
                ExprKind::Field { target, name } => {
                    steps.push(PlaceStep::Field(&name.name));
                    cur = target;
                }
                ExprKind::Index { target, index } => {
                    let r = self.alloc();
                    self.expr_into(index, r)?;
                    steps.push(PlaceStep::Index(r, index.span));
                    cur = target;
                }
                _ => return Err(Unsupported),
            }
        };
        steps.reverse();
        self.code.places.push(Place { root, steps });
        Ok(self.code.places.len() as u32 - 1)
    }

    /// Compiles a condition; returns the jump to patch with where to go
    /// when it is false.
    fn jump_unless(&mut self, cond: &'p Expr) -> C<usize> {
        if let ExprKind::Binary { op, lhs, rhs } = &cond.kind
            && is_comparison(*op)
        {
            let a = self.operand_before(lhs, rhs)?;
            if let ExprKind::Int(n) = rhs.kind {
                return Ok(self.emit(
                    Op::CmpIntJump {
                        op: *op,
                        a,
                        n,
                        to: 0,
                    },
                    cond.span,
                ));
            }
            let b = self.operand(rhs)?;
            return Ok(self.emit(
                Op::CmpJump {
                    op: *op,
                    a,
                    b,
                    to: 0,
                },
                cond.span,
            ));
        }
        let r = self.operand(cond)?;
        Ok(self.emit(Op::JumpIfNot { cond: r, to: 0 }, cond.span))
    }

    /// The register holding `e`'s value: a local variable's own register,
    /// or a new one the value is computed into.
    fn operand(&mut self, e: &'p Expr) -> C<R> {
        if let ExprKind::Name(n) = &e.kind
            && let Some(r) = self.local(n)
        {
            return Ok(r);
        }
        let r = self.alloc();
        self.expr_into(e, r)?;
        Ok(r)
    }

    /// Like [`Compiler::operand`] for `e`, which is evaluated before
    /// `later`: if `later` may assign variables, `e` is copied first.
    fn operand_before(&mut self, e: &'p Expr, later: &Expr) -> C<R> {
        if may_assign(later) {
            let r = self.alloc();
            self.expr_into(e, r)?;
            Ok(r)
        } else {
            self.operand(e)
        }
    }

    /// Each expression into consecutive new registers.
    fn exprs_into(&mut self, items: impl ExactSizeIterator<Item = &'p Expr>) -> C<R> {
        let start = self.alloc_n(items.len());
        for (i, e) in items.enumerate() {
            self.expr_into(e, start + i as R)?;
        }
        Ok(start)
    }

    /// Compiles `e`, writing its value into `dst`. Only the last
    /// instruction writes `dst`, so `dst` may be a variable that `e` reads.
    fn expr_into(&mut self, e: &'p Expr, dst: R) -> C<()> {
        let span = e.span;
        match &e.kind {
            ExprKind::Int(n) => {
                self.emit(Op::Int { dst, n: *n }, span);
            }
            ExprKind::Float(x) => self.constant(dst, Value::Float(*x), span),
            ExprKind::Str(s) => self.constant(dst, Value::str(s.as_str()), span),
            ExprKind::Bool(b) => self.constant(dst, Value::Bool(*b), span),
            ExprKind::Duration(ms) => self.constant(dst, Value::Duration(*ms), span),
            ExprKind::Name(n) => match self.local(n) {
                Some(src) => {
                    if src != dst {
                        self.emit(Op::Move { dst, src }, span);
                    }
                }
                None => match self.captured(n) {
                    Some(i) => {
                        self.emit(Op::Captured { dst, i }, span);
                    }
                    None => {
                        self.emit(Op::Name { dst, name: n }, span);
                    }
                },
            },
            ExprKind::Interp(parts) => {
                let mut out = Vec::with_capacity(parts.len());
                for part in parts {
                    out.push(match part {
                        InterpPart::Lit(text) => Part::Lit(text),
                        InterpPart::Expr(inner) => {
                            let r = self.alloc();
                            self.expr_into(inner, r)?;
                            Part::Expr(r)
                        }
                    });
                }
                self.code.interps.push(out);
                let parts = self.code.interps.len() as u32 - 1;
                self.emit(Op::Interp { dst, parts }, span);
            }
            ExprKind::Tuple(items) => {
                let start = self.exprs_into(items.iter())?;
                let n = items.len() as u32;
                self.emit(Op::Tuple { dst, start, n }, span);
            }
            ExprKind::List(items) => {
                let start = self.exprs_into(items.iter())?;
                let n = items.len() as u32;
                self.emit(Op::List { dst, start, n }, span);
            }
            ExprKind::Record(fields) => {
                let start = self.exprs_into(fields.iter().map(|f| &f.value))?;
                let names: Rc<[Rc<str>]> = fields
                    .iter()
                    .map(|f| Rc::from(f.name.name.as_str()))
                    .collect();
                self.code.records.push(names);
                let names = self.code.records.len() as u32 - 1;
                self.emit(Op::Record { dst, start, names }, span);
            }
            ExprKind::Map(entries) => {
                let start = self.exprs_into(
                    entries
                        .iter()
                        .flat_map(|(k, v)| [k, v])
                        .collect::<Vec<_>>()
                        .into_iter(),
                )?;
                let n = entries.len() as u32;
                self.emit(Op::Map { dst, start, n }, span);
            }
            ExprKind::Unary { op, expr } => {
                let src = self.operand(expr)?;
                match op {
                    UnaryOp::Not => self.emit(Op::Not { dst, src }, span),
                    UnaryOp::Neg => self.emit(Op::Neg { dst, src }, span),
                };
            }
            ExprKind::Binary { op, lhs, rhs } if matches!(op, BinOp::And | BinOp::Or) => {
                let t = self.alloc();
                self.expr_into(lhs, t)?;
                let skip = match op {
                    BinOp::And => self.emit(Op::JumpIfNot { cond: t, to: 0 }, span),
                    _ => self.emit(Op::JumpIf { cond: t, to: 0 }, span),
                };
                self.expr_into(rhs, t)?;
                self.patch(skip);
                self.emit(Op::Move { dst, src: t }, span);
            }
            ExprKind::Binary { op, lhs, rhs } => {
                let a = self.operand_before(lhs, rhs)?;
                if let ExprKind::Int(n) = rhs.kind {
                    self.emit(Op::BinInt { op: *op, dst, a, n }, span);
                } else {
                    let b = self.operand(rhs)?;
                    self.emit(Op::Bin { op: *op, dst, a, b }, span);
                }
            }
            ExprKind::Range {
                start,
                end,
                inclusive,
            } => {
                let s = self.operand_before(start, end)?;
                let e2 = self.operand(end)?;
                self.emit(
                    Op::Range {
                        dst,
                        start: s,
                        end: e2,
                        inclusive: *inclusive,
                    },
                    span,
                );
            }
            ExprKind::Call {
                callee,
                type_args,
                args,
            } => self.call(e, callee, type_args, args, dst)?,
            ExprKind::Field { target, name } => {
                if let ExprKind::Name(module) = &target.kind
                    && module == "std.math"
                {
                    self.constant(dst, stdextra::math_constant(&name.name), span);
                    return Ok(());
                }
                let src = self.operand(target)?;
                self.emit(
                    Op::Field {
                        dst,
                        src,
                        name: &name.name,
                    },
                    span,
                );
            }
            ExprKind::Index { target, index } => {
                let t = self.operand_before(target, index)?;
                let i = self.operand(index)?;
                self.emit(
                    Op::Index {
                        dst,
                        target: t,
                        index: i,
                    },
                    index.span,
                );
            }
            ExprKind::Match { scrutinee, arms } => {
                let src = self.operand(scrutinee)?;
                let mut ends = Vec::new();
                for arm in arms {
                    let (mark, next) = (self.scope.len(), self.next);
                    let mut names = Vec::new();
                    binding_names(&arm.pattern, &mut names);
                    let regs: Vec<R> = names.iter().map(|_| self.alloc()).collect();
                    self.code.binds.push(regs.clone());
                    let binds = self.code.binds.len() as u32 - 1;
                    let test = self.emit(
                        Op::MatchArm {
                            pattern: &arm.pattern,
                            src,
                            binds,
                            to: 0,
                        },
                        arm.span,
                    );
                    for (n, r) in names.into_iter().zip(regs) {
                        self.scope.push((n, r));
                    }
                    match &arm.body {
                        ArmBody::Expr(body) => self.expr_into(body, dst)?,
                        ArmBody::Block(b) => {
                            self.block(b)?;
                            self.constant(dst, Value::Unit, b.span);
                        }
                    }
                    self.end_scope(mark, next, arm.span);
                    ends.push(self.emit(Op::Jump { to: 0 }, arm.span));
                    self.patch(test);
                }
                self.emit(Op::NoArm, span);
                for end in ends {
                    self.patch(end);
                }
            }
            ExprKind::Await(inner) => {
                let src = self.operand(inner)?;
                self.emit(Op::Await { dst, src }, span);
            }
            ExprKind::Try(inner) => {
                let src = self.operand(inner)?;
                self.emit(Op::Try { dst, src }, span);
            }
            ExprKind::Lambda { params, body } => {
                let captured: Vec<(Rc<str>, R)> =
                    self.scope.iter().map(|(n, r)| (Rc::from(*n), *r)).collect();
                // The closure sees the running closure's captured variables,
                // then the variables here.
                let mut names = self.captured.clone().unwrap_or_default();
                names.extend(captured.iter().map(|(n, _)| n.clone()));
                if let Some(code) = compile_lambda(self.vm, params, body, names) {
                    self.code.nested.push((e as *const Expr as usize, code));
                }
                self.code.captures.push(captured);
                let captures = self.code.captures.len() as u32 - 1;
                self.emit(
                    Op::Closure {
                        dst,
                        lambda: e,
                        captures,
                    },
                    span,
                );
            }
            ExprKind::Variant {
                enum_name,
                variant,
                args,
            } => {
                let start = self.exprs_into(args.iter())?;
                self.emit(
                    Op::Variant {
                        dst,
                        enum_name,
                        variant,
                        start,
                        n: args.len() as u32,
                    },
                    span,
                );
            }
        }
        Ok(())
    }

    fn call(
        &mut self,
        e: &'p Expr,
        callee: &'p Expr,
        type_args: &'p [TypeExpr],
        args: &'p [Arg],
        dst: R,
    ) -> C<()> {
        let argc = args.len() as u32;
        let values = || args.iter().map(|a| &a.value);
        match &callee.kind {
            ExprKind::Name(name) => {
                if let Some(local) = self.local(name) {
                    // The function is read before the arguments.
                    let func = if args.iter().any(|a| may_assign(&a.value)) {
                        let t = self.alloc();
                        self.emit(Op::Move { dst: t, src: local }, callee.span);
                        t
                    } else {
                        local
                    };
                    let start = self.exprs_into(values())?;
                    self.emit(
                        Op::CallValue {
                            func,
                            name,
                            args: start,
                            argc,
                            dst,
                        },
                        callee.span,
                    );
                    return Ok(());
                }
                if let Some(i) = self.captured(name) {
                    let func = self.alloc();
                    self.emit(Op::Captured { dst: func, i }, callee.span);
                    let start = self.exprs_into(values())?;
                    self.emit(
                        Op::CallValue {
                            func,
                            name,
                            args: start,
                            argc,
                            dst,
                        },
                        callee.span,
                    );
                    return Ok(());
                }
                if self.vm.const_names.contains_key(name.as_str()) {
                    return Err(Unsupported);
                }
                if let Some(&fi) = self.vm.fn_index.get(name.as_str()) {
                    let f = self.vm.fns[fi as usize];
                    if f.params.len() != args.len() {
                        return Err(Unsupported);
                    }
                    // Arguments run in the order written, each into its
                    // parameter's register.
                    let start = self.alloc_n(args.len());
                    for (i, arg) in args.iter().enumerate() {
                        let slot = match &arg.name {
                            None => i,
                            Some(id) => f
                                .params
                                .iter()
                                .position(|p| p.name.name == id.name)
                                .ok_or(Unsupported)?,
                        };
                        self.expr_into(&arg.value, start + slot as R)?;
                    }
                    let op = if f.is_async {
                        Op::CallAsync {
                            name,
                            args: start,
                            argc,
                            dst,
                        }
                    } else {
                        Op::Call {
                            f: fi,
                            args: start,
                            argc,
                            dst,
                        }
                    };
                    self.emit(op, callee.span);
                    return Ok(());
                }
                let start = self.exprs_into(values())?;
                self.emit(
                    Op::CallBuiltin {
                        name,
                        args: start,
                        argc,
                        dst,
                    },
                    e.span,
                );
            }
            ExprKind::Field { target, name } => {
                let start = self.exprs_into(values())?;
                if let ExprKind::Name(module) = &target.kind
                    && MODULES.contains(&module.as_str())
                {
                    self.code.modules.push(ModuleCall {
                        module,
                        name: &name.name,
                        type_args,
                    });
                    let m = self.code.modules.len() as u32 - 1;
                    self.emit(
                        Op::CallModule {
                            m,
                            args: start,
                            argc,
                            dst,
                        },
                        e.span,
                    );
                    return Ok(());
                }
                let method = name.name.as_str();
                if builtins::is_mutating(method) {
                    let place = self.place(target)?;
                    self.emit(
                        Op::CallMutating {
                            method,
                            place,
                            args: start,
                            argc,
                            dst,
                        },
                        e.span,
                    );
                    return Ok(());
                }
                let recv = self.operand(target)?;
                self.emit(
                    Op::CallMethod {
                        method,
                        recv,
                        args: start,
                        argc,
                        dst,
                    },
                    e.span,
                );
            }
            _ => {
                let func = self.alloc();
                self.expr_into(callee, func)?;
                let start = self.exprs_into(values())?;
                self.emit(
                    Op::CallValue {
                        func,
                        name: "",
                        args: start,
                        argc,
                        dst,
                    },
                    callee.span,
                );
            }
        }
        Ok(())
    }
}

/// The names a pattern binds, in the order [`match_pattern`] binds them.
fn binding_names<'p>(p: &'p Pattern, out: &mut Vec<&'p str>) {
    match &p.kind {
        PatternKind::Name(n) if n != "None" => out.push(n),
        PatternKind::Ctor { args, .. } => {
            if let Some(a) = args.first() {
                binding_names(a, out);
            }
        }
        PatternKind::Variant { args, .. } | PatternKind::Tuple(args) => {
            for a in args {
                binding_names(a, out);
            }
        }
        _ => {}
    }
}

// ----- running -----------------------------------------------------------

impl<'p> Interpreter<'p, '_> {
    /// `f`'s compiled code, compiling it the first time; `None` if the
    /// engine does not handle it.
    pub(crate) fn compiled(&mut self, f: &'p FnDecl) -> Option<Rc<Code<'p>>> {
        let i = *self.vm.fn_index.get(f.name.name.as_str())?;
        if !std::ptr::eq(self.vm.fns[i as usize], f) {
            return None;
        }
        self.compiled_at(i)
    }

    fn compiled_at(&mut self, i: u32) -> Option<Rc<Code<'p>>> {
        let i = i as usize;
        match &self.vm.codes[i] {
            Compiled::Ready(code) => Some(code.clone()),
            Compiled::Unsupported => None,
            Compiled::NotYet => {
                let compiled = match compile(&self.vm, self.vm.fns[i]) {
                    Some(mut code) => {
                        self.install_closures(&mut code);
                        Compiled::Ready(Rc::new(code))
                    }
                    None => {
                        if std::env::var_os("ELOMAR_VM_DEBUG").is_some() {
                            eprintln!("[vm] tree: {}", self.vm.fns[i].name.name);
                        }
                        Compiled::Unsupported
                    }
                };
                self.vm.codes[i] = compiled;
                self.compiled_at(i as u32)
            }
        }
    }

    /// Makes the closures compiled with `code` (and inside them) known.
    fn install_closures(&mut self, code: &mut Code<'p>) {
        for (key, mut inner) in std::mem::take(&mut code.nested) {
            self.install_closures(&mut inner);
            self.vm.lambdas.insert(key, Rc::new(inner));
        }
    }

    /// Compiles the code of closure `f` if it has none yet: a closure copied
    /// into a thread was made by code this interpreter did not compile.
    pub(crate) fn prepare_closure(&mut self, f: &Function) {
        let Function::Closure { key, captured } = f else {
            return;
        };
        if !self.vm.enabled || self.vm.lambdas.contains_key(key) {
            return;
        }
        let Some(&e) = self.lambdas.get(key) else {
            return;
        };
        let ExprKind::Lambda { params, body } = &e.kind else {
            return;
        };
        let names = captured.iter().map(|(n, _)| n.clone()).collect();
        if let Some(mut code) = compile_lambda(&self.vm, params, body, names) {
            self.install_closures(&mut code);
            self.vm.lambdas.insert(*key, Rc::new(code));
        }
    }

    /// Calls a compiled closure.
    pub(crate) fn call_closure_compiled(
        &mut self,
        code: &Code<'p>,
        params: usize,
        captured: &Captured,
        args: Vec<Value>,
        span: Span,
    ) -> Exec<Value> {
        self.check_depth(span)?;
        let base = self.vm.stack.len();
        self.vm.stack.extend(args.into_iter().take(params));
        self.vm.stack.resize(base + code.nregs, Value::Unit);
        self.vm.depth += 1;
        let result = self.run_code(code, base, Some(captured));
        self.vm.depth -= 1;
        self.vm.stack.truncate(base);
        match result {
            Err(Flow::Error(mut err)) => {
                if err.trace.len() < 64 {
                    err.trace.push(("<closure>".into(), span));
                }
                Err(Flow::Error(err))
            }
            other => other,
        }
    }

    /// Runs compiled `f`, whose arguments are the last values on the
    /// register stack.
    pub(crate) fn call_compiled(
        &mut self,
        f: &'p FnDecl,
        code: &Code<'p>,
        call_span: Span,
    ) -> Exec<Value> {
        let base = self.vm.stack.len() - f.params.len();
        if let Err(e) = self.check_depth(call_span) {
            self.vm.stack.truncate(base);
            return Err(e);
        }
        self.vm.stack.resize(base + code.nregs, Value::Unit);
        self.vm.depth += 1;
        let result = self.run_code(code, base, None);
        self.vm.depth -= 1;
        self.vm.stack.truncate(base);
        match result {
            Err(Flow::Error(mut err)) => {
                if err.trace.len() < 64 {
                    err.trace.push((f.name.name.clone(), call_span));
                }
                Err(Flow::Error(err))
            }
            other => other,
        }
    }

    /// The values of `argc` argument registers from `args`, moved out.
    fn take_args(&mut self, base: usize, args: R, argc: u32) -> Vec<Value> {
        let from = base + args as usize;
        self.vm.stack[from..from + argc as usize]
            .iter_mut()
            .map(|v| std::mem::replace(v, Value::Unit))
            .collect()
    }

    /// The value of a name that is not a local variable.
    fn global_name(&self, name: &str, span: Span) -> Exec<Value> {
        if let Some(v) = self.globals.get(name) {
            return Ok(v.clone());
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

    fn run_code(
        &mut self,
        code: &Code<'p>,
        base: usize,
        captured: Option<&Captured>,
    ) -> Exec<Value> {
        macro_rules! reg {
            ($r:expr) => {
                self.vm.stack[base + $r as usize]
            };
        }
        if self.watching() {
            self.check_deadlines()?;
        }
        let ops = &code.ops[..];
        let mut iters: Vec<Iter> = Vec::new();
        let mut pc = 0usize;
        loop {
            let op = ops[pc];
            let span = code.spans[pc];
            pc += 1;
            match op {
                Op::Int { dst, n } => reg!(dst) = Value::Int(n),
                Op::Const { dst, k } => reg!(dst) = code.consts[k as usize].clone(),
                Op::Move { dst, src } => reg!(dst) = reg!(src).clone(),
                Op::Bin { op, dst, a, b } => {
                    if let (Value::Int(x), Value::Int(y)) = (&reg!(a), &reg!(b))
                        && let Some(v) = int_binary(op, *x, *y)
                    {
                        reg!(dst) = v;
                        continue;
                    }
                    // `s = s + t` and `s += t` append in place, so building a
                    // string or list in a loop is not quadratic; one that
                    // other values share is copied once first.
                    if op == BinOp::Add && dst == a && a != b {
                        match (&reg!(a), &reg!(b)) {
                            (Value::Str(_), Value::Str(t)) => {
                                let t = t.clone();
                                if let Value::Str(s) = &mut reg!(a) {
                                    Rc::make_mut(s).push_str(&t);
                                }
                                continue;
                            }
                            (Value::List(_), Value::List(t)) => {
                                let t = t.clone();
                                if let Value::List(s) = &mut reg!(a) {
                                    Rc::make_mut(s).extend(t.iter().cloned());
                                }
                                continue;
                            }
                            _ => {}
                        }
                    }
                    let (l, r) = (reg!(a).clone(), reg!(b).clone());
                    reg!(dst) = self.binary(op, l, r, span)?;
                }
                Op::BinInt { op, dst, a, n } => {
                    if let Value::Int(x) = reg!(a)
                        && let Some(v) = int_binary(op, x, n)
                    {
                        reg!(dst) = v;
                        continue;
                    }
                    let l = reg!(a).clone();
                    reg!(dst) = self.binary(op, l, Value::Int(n), span)?;
                }
                Op::Jump { to } => pc = to as usize,
                Op::JumpIfNot { cond, to } => {
                    if !matches!(reg!(cond), Value::Bool(true)) {
                        pc = to as usize;
                    }
                }
                Op::JumpIf { cond, to } => {
                    if matches!(reg!(cond), Value::Bool(true)) {
                        pc = to as usize;
                    }
                }
                Op::CmpJump { op, a, b, to } => {
                    let holds = match (&reg!(a), &reg!(b)) {
                        (Value::Int(x), Value::Int(y)) => int_compare(op, *x, *y),
                        (l, r) => {
                            let (l, r) = (l.clone(), r.clone());
                            matches!(self.binary(op, l, r, span)?, Value::Bool(true))
                        }
                    };
                    if !holds {
                        pc = to as usize;
                    }
                }
                Op::CmpIntJump { op, a, n, to } => {
                    let holds = match &reg!(a) {
                        Value::Int(x) => int_compare(op, *x, n),
                        l => {
                            let l = l.clone();
                            matches!(self.binary(op, l, Value::Int(n), span)?, Value::Bool(true))
                        }
                    };
                    if !holds {
                        pc = to as usize;
                    }
                }
                Op::Tick => {
                    if self.watching() {
                        self.check_deadlines()?;
                    }
                }
                Op::Call { f, args, argc, dst } => {
                    let callee = self.vm.fns[f as usize];
                    let v = match self.compiled_at(f) {
                        Some(callee_code) => {
                            let from = base + args as usize;
                            for i in 0..argc as usize {
                                let v =
                                    std::mem::replace(&mut self.vm.stack[from + i], Value::Unit);
                                self.vm.stack.push(v);
                            }
                            self.call_compiled(callee, &callee_code, span)?
                        }
                        None => {
                            let values = self.take_args(base, args, argc);
                            self.call_fn(callee, values, span)?
                        }
                    };
                    reg!(dst) = v;
                }
                Op::Field { dst, src, name } => {
                    let v = match &reg!(src) {
                        Value::Record(fields) => fields
                            .iter()
                            .find(|(n, _)| **n == *name)
                            .map(|(_, v)| v.clone())
                            .expect("checked field"),
                        Value::List(items) => Value::Int(items.len() as i64),
                        Value::Map(entries) => Value::Int(entries.len() as i64),
                        Value::Set(items) => Value::Int(items.len() as i64),
                        Value::Str(s) => Value::Int(builtins::str_length(s) as i64),
                        Value::Tuple(items) => {
                            let i: usize = name.parse().expect("checked tuple index");
                            items[i].clone()
                        }
                        _ => unreachable!("checked field access"),
                    };
                    reg!(dst) = v;
                }
                Op::Index { dst, target, index } => {
                    let v = match (&reg!(target), &reg!(index)) {
                        (Value::List(items), Value::Int(i)) => {
                            let idx = builtins::check_index(*i, items.len(), span)?;
                            items[idx].clone()
                        }
                        (Value::Map(entries), key) => match entries.get(&MapKey::from_value(key)) {
                            Some(v) => v.clone(),
                            None => return Err(builtins::missing_key(key, span).into()),
                        },
                        _ => unreachable!("checked indexing"),
                    };
                    reg!(dst) = v;
                }
                Op::Try { dst, src } => {
                    let v = match &reg!(src) {
                        Value::Result(Ok(v)) | Value::Option(Some(v)) => (**v).clone(),
                        err @ (Value::Result(Err(_)) | Value::Option(None)) => {
                            return Ok(err.clone());
                        }
                        _ => unreachable!("checker only allows `?` on Result and Option"),
                    };
                    reg!(dst) = v;
                }
                Op::IterStart { src } => {
                    let v = reg!(src).clone();
                    iters.push(Iter::new(v));
                }
                Op::IterNext { dst, to } => match iters.last_mut().expect("loop").next() {
                    Some(v) => reg!(dst) = v,
                    None => {
                        iters.pop();
                        pc = to as usize;
                    }
                },
                Op::IterEnd => {
                    iters.pop();
                }
                Op::MatchArm {
                    pattern,
                    src,
                    binds,
                    to,
                } => {
                    let mut bindings = Vec::new();
                    if match_pattern(pattern, &reg!(src), &mut bindings) {
                        for (r, (_, v)) in code.binds[binds as usize].iter().zip(bindings) {
                            reg!(*r) = v;
                        }
                    } else {
                        pc = to as usize;
                    }
                }
                Op::Return { src } => return Ok(std::mem::replace(&mut reg!(src), Value::Unit)),
                Op::ReturnUnit => return Ok(Value::Unit),
                other => self.run_other(other, code, base, captured, span)?,
            }
        }
    }

    /// The instructions that are not in the hot loop of
    /// [`Interpreter::run_code`], so that function stays small: it runs
    /// once per call, and a large function is slower to enter.
    #[inline(never)]
    fn run_other(
        &mut self,
        op: Op<'p>,
        code: &Code<'p>,
        base: usize,
        captured: Option<&Captured>,
        span: Span,
    ) -> Exec<()> {
        macro_rules! reg {
            ($r:expr) => {
                self.vm.stack[base + $r as usize]
            };
        }
        match op {
            Op::Name { dst, name } => reg!(dst) = self.global_name(name, span)?,
            Op::Captured { dst, i } => {
                reg!(dst) = captured.expect("closure")[i as usize].1.clone();
            }
            Op::Not { dst, src } => {
                let v = match &reg!(src) {
                    Value::Bool(b) => Value::Bool(!b),
                    v => v.clone(),
                };
                reg!(dst) = v;
            }
            Op::Neg { dst, src } => {
                let v = match &reg!(src) {
                    Value::Int(n) => {
                        Value::Int(n.checked_neg().ok_or_else(|| builtins::overflow(span))?)
                    }
                    Value::Float(x) => Value::Float(-x),
                    v => v.clone(),
                };
                reg!(dst) = v;
            }
            Op::CallAsync {
                name,
                args,
                argc,
                dst,
            } => {
                let values = self.take_args(base, args, argc);
                reg!(dst) = Value::future(Thunk::Call {
                    name: name.to_string(),
                    args: values,
                    span,
                });
            }
            Op::CallValue {
                func,
                name,
                args,
                argc,
                dst,
            } => {
                let values = self.take_args(base, args, argc);
                let v = match reg!(func).clone() {
                    Value::Function(func) => self.call_value(&func, values, span)?,
                    // A variable that is not a function, with the name
                    // of one.
                    _ if self.fns.contains_key(name) => {
                        self.call_value(&Function::Named(name.to_string()), values, span)?
                    }
                    _ => builtins::call_function(&mut *self.host, name, values, span)
                        .map_err(Flow::from)?,
                };
                reg!(dst) = v;
            }
            Op::CallBuiltin {
                name,
                args,
                argc,
                dst,
            } => {
                let values = self.take_args(base, args, argc);
                let v = match name {
                    "all" => {
                        let items = match values.into_iter().next() {
                            Some(Value::List(items)) => items,
                            _ => Rc::new(Vec::new()),
                        };
                        Value::future(Thunk::All(items))
                    }
                    "timeout" => {
                        let mut values = values.into_iter();
                        let future = values.next().unwrap_or(Value::Unit);
                        let limit = match values.next() {
                            Some(Value::Duration(ms)) => ms,
                            _ => 0,
                        };
                        Value::future(Thunk::Timeout(future, limit))
                    }
                    _ => builtins::call_function(&mut *self.host, name, values, span)
                        .map_err(Flow::from)?,
                };
                reg!(dst) = v;
            }
            Op::CallModule { m, args, argc, dst } => {
                let values = self.take_args(base, args, argc);
                let call = &code.modules[m as usize];
                reg!(dst) =
                    self.call_module(call.module, call.name, call.type_args, values, span)?;
            }
            Op::CallMethod {
                method,
                recv,
                args,
                argc,
                dst,
            } => {
                let values = self.take_args(base, args, argc);
                let recv = reg!(recv).clone();
                reg!(dst) = self.call_method_value(recv, method, values, span)?;
            }
            Op::CallMutating {
                method,
                place,
                args,
                argc,
                dst,
            } => {
                let values = self.take_args(base, args, argc);
                let place = &code.places[place as usize];
                let v = self.with_vm_place(base, place, false, |slot| {
                    builtins::call_mutating(slot, method, values, span)
                })?;
                reg!(dst) = v;
            }
            Op::Assign { place, src, insert } => {
                let v = std::mem::replace(&mut reg!(src), Value::Unit);
                let place = &code.places[place as usize];
                self.with_vm_place(base, place, insert, |slot| {
                    *slot = v;
                    Ok(())
                })?;
            }
            Op::List { dst, start, n } => {
                let items = self.take_args(base, start, n);
                reg!(dst) = Value::list(items);
            }
            Op::Tuple { dst, start, n } => {
                let items = self.take_args(base, start, n);
                reg!(dst) = Value::Tuple(Rc::new(items));
            }
            Op::Record { dst, start, names } => {
                let names = &code.records[names as usize];
                let values = self.take_args(base, start, names.len() as u32);
                reg!(dst) = Value::Record(Rc::new(names.iter().cloned().zip(values).collect()));
            }
            Op::Map { dst, start, n } => {
                let items = self.take_args(base, start, n * 2);
                let mut out = std::collections::BTreeMap::new();
                let mut items = items.into_iter();
                while let (Some(k), Some(v)) = (items.next(), items.next()) {
                    out.insert(MapKey::from_value(&k), v);
                }
                reg!(dst) = Value::Map(Rc::new(out));
            }
            Op::Interp { dst, parts } => {
                use std::fmt::Write as _;
                let mut out = String::new();
                for part in &code.interps[parts as usize] {
                    match part {
                        Part::Lit(text) => out.push_str(text),
                        Part::Expr(r) => {
                            let _ = write!(out, "{}", reg!(*r));
                        }
                    }
                }
                reg!(dst) = Value::str(out);
            }
            Op::Range {
                dst,
                start,
                end,
                inclusive,
            } => {
                let (Value::Int(s), Value::Int(e)) = (&reg!(start), &reg!(end)) else {
                    unreachable!("checked range bounds");
                };
                reg!(dst) = Value::Range {
                    start: *s,
                    end: *e,
                    inclusive,
                };
            }
            Op::Variant {
                dst,
                enum_name,
                variant,
                start,
                n,
            } => {
                let fields = self.take_args(base, start, n);
                reg!(dst) = Value::enum_value(enum_name, variant, fields);
            }
            Op::Await { dst, src } => {
                let v = match &reg!(src) {
                    Value::Future(state) => {
                        let state = state.clone();
                        self.force(&state, span)?
                    }
                    _ => unreachable!("checker only awaits futures"),
                };
                reg!(dst) = v;
            }
            Op::Closure {
                dst,
                lambda,
                captures,
            } => {
                let key = lambda as *const Expr as usize;
                self.lambdas.insert(key, lambda);
                let mut copies: Vec<(Rc<str>, Value)> =
                    captured.map(|c| c.to_vec()).unwrap_or_default();
                copies.extend(
                    code.captures[captures as usize]
                        .iter()
                        .map(|(n, r)| (n.clone(), reg!(*r).clone())),
                );
                reg!(dst) = Value::Function(Rc::new(Function::Closure {
                    key,
                    captured: Rc::new(copies),
                }));
            }
            Op::Unpack { src, targets } => {
                let values = unpack(&reg!(src));
                for (t, v) in code.unpacks[targets as usize].iter().zip(values) {
                    if let Some(t) = t {
                        reg!(*t) = v;
                    }
                }
            }
            Op::NoArm => {
                return Err(RuntimeError::new(
                    "R0009",
                    span,
                    Text::new(
                        "internal error: no `match` arm matched",
                        "خطأ داخلي: لم يطابق أي فرع في `match`",
                    ),
                )
                .into());
            }
            Op::Clear { start, n } => {
                let from = base + start as usize;
                for v in &mut self.vm.stack[from..from + n as usize] {
                    *v = Value::Unit;
                }
            }
            _ => unreachable!("handled in run_code"),
        }
        Ok(())
    }

    /// Applies `f` to the slot `place` names in the registers from `base`.
    fn with_vm_place<T>(
        &mut self,
        base: usize,
        place: &Place<'p>,
        insert_missing_key: bool,
        f: impl FnOnce(&mut Value) -> Result<T, RuntimeError>,
    ) -> Exec<T> {
        let steps: Vec<Step<'p>> = place
            .steps
            .iter()
            .map(|s| match s {
                PlaceStep::Field(name) => Step::Field(name),
                PlaceStep::Index(r, span) => Step::Index(
                    std::mem::replace(&mut self.vm.stack[base + *r as usize], Value::Unit),
                    *span,
                ),
            })
            .collect();
        let slot = &mut self.vm.stack[base + place.root as usize];
        let slot = walk_place(slot, &steps, insert_missing_key)?;
        Ok(f(slot)?)
    }
}

#[inline]
fn int_compare(op: BinOp, a: i64, b: i64) -> bool {
    match op {
        BinOp::Eq => a == b,
        BinOp::Ne => a != b,
        BinOp::Lt => a < b,
        BinOp::Le => a <= b,
        BinOp::Gt => a > b,
        _ => a >= b,
    }
}
