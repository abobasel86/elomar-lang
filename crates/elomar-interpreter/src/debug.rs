//! Hooks for a debugger (`elomar dap`): the interpreter reports each
//! statement before it runs and each runtime error where it starts, and the
//! debugger may look at the call stack and the variables meanwhile.

use elomar_diagnostics::Span;

use crate::{RuntimeError, Value};

/// One call on the stack.
#[derive(Debug, Clone)]
pub struct StackFrame {
    /// The function's name, or `<closure>`.
    pub name: String,
    /// The statement this call is running.
    pub span: Span,
}

/// What a debugger may look at while the program waits for it.
pub trait Inspect {
    /// The number of calls on the stack; cheap.
    fn depth(&self) -> usize;
    /// The calls, innermost first.
    fn stack(&self) -> Vec<StackFrame>;
    /// The variables visible in call `frame` (an index into [`Inspect::stack`]),
    /// each name once, with the value it currently has.
    fn locals(&self, frame: usize) -> Vec<(String, Value)>;
    /// File-level constants.
    fn globals(&self) -> Vec<(String, Value)>;
}

/// What the program does after a hook returns.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Resume {
    Run,
    /// End the program now (the debugger disconnected).
    Stop,
}

pub trait Debugger {
    /// Called before each statement runs.
    fn statement(&mut self, inspect: &dyn Inspect, span: Span) -> Resume;
    /// Called once per runtime error, at the statement it happened in,
    /// before the program unwinds.
    fn error(&mut self, inspect: &dyn Inspect, span: Span, error: &RuntimeError) -> Resume;
}
