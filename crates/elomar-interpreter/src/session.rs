//! A long-lived interpreter for `elomar repl`: variables, functions and
//! started MCP servers survive from one input to the next.
//!
//! Each input is checked as part of a whole program, which the REPL then
//! hands over here. Programs must live for the rest of the session
//! (`&'static`): closures made by earlier inputs point into them.

use std::collections::HashMap;
use std::mem;

use elomar_ai_core::AiRuntime;
use elomar_ast::{Expr, Item, Program, Stmt, StmtKind};

use crate::{Frame, FsPolicy, Host, Interpreter, RuntimeError, Value, mcp};

/// What survives between inputs.
struct Kept {
    globals: HashMap<String, Value>,
    frames: Vec<Frame<'static>>,
    lambdas: HashMap<usize, &'static Expr>,
    mcp: mcp::McpHub,
}

pub struct Session {
    host: Box<dyn Host>,
    ai: AiRuntime,
    fs: FsPolicy,
    kept: Option<Kept>,
}

impl Session {
    pub fn new(host: Box<dyn Host>, ai: AiRuntime, fs: FsPolicy) -> Self {
        Session {
            host,
            ai,
            fs,
            kept: None,
        }
    }

    /// Forgets every variable and stops the MCP servers started so far.
    pub fn clear(&mut self) {
        self.kept = None;
    }

    /// Makes `program`'s functions and types current, evaluates its
    /// file-level constants that are new, then runs `stmt` (a statement of
    /// the REPL's own function) in the session's scope. Returns the value of
    /// an expression statement.
    pub fn step(
        &mut self,
        program: &'static Program,
        stmt: Option<&'static Stmt>,
    ) -> Result<Option<Value>, RuntimeError> {
        let mut interp = Interpreter::new(program, &mut *self.host, &mut self.ai, self.fs.clone());
        if let Some(kept) = self.kept.take() {
            interp.globals = kept.globals;
            interp.frames = kept.frames;
            interp.lambdas = kept.lambdas;
            interp.mcp = kept.mcp;
        }
        let result = run_step(&mut interp, program, stmt);
        // Whatever ran, the scope it ran in may have changed.
        interp.frames.truncate(1);
        self.kept = Some(Kept {
            globals: mem::take(&mut interp.globals),
            frames: mem::take(&mut interp.frames),
            lambdas: mem::take(&mut interp.lambdas),
            mcp: mem::replace(&mut interp.mcp, mcp::McpHub::empty()),
        });
        result
    }
}

fn run_step(
    interp: &mut Interpreter<'static, '_>,
    program: &'static Program,
    stmt: Option<&'static Stmt>,
) -> Result<Option<Value>, RuntimeError> {
    for item in &program.items {
        if let Item::Const(c) = item
            && !interp.globals.contains_key(&c.name.name)
        {
            let v = interp.eval(&c.value).map_err(|f| f.into_error())?;
            interp.globals.insert(c.name.name.clone(), v);
        }
    }
    let Some(stmt) = stmt else {
        return Ok(None);
    };
    match &stmt.kind {
        StmtKind::Expr(e) => interp.eval(e).map(Some).map_err(|f| f.into_error()),
        _ => interp.exec(stmt).map(|_| None).map_err(|f| f.into_error()),
    }
}
