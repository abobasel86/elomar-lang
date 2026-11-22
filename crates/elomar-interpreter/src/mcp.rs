//! `std.mcp` at run time, and the MCP servers used by `agent.run`.
//!
//! - Only servers listed under `[mcp.servers]` in elomar.toml can be
//!   started; that list is the permission. A server's program runs with
//!   the user's rights, like `std.py`.
//! - A server is started on first use, in the project folder, and kept for
//!   the rest of the run. If it stops, the next use starts it again.
//! - Failures are `Err(McpError...)` values.

use std::collections::HashMap;
use std::path::PathBuf;
use std::rc::Rc;
use std::time::Duration;

use elomar_ai_core::McpConfig;
use elomar_mcp::{Client, Launch, McpError, Tool};

use crate::Value;
use crate::ai::to_json;

/// Why a server cannot be used: the `McpError` kind and a message.
pub(crate) type Failure = (&'static str, String);

pub(crate) struct McpHub {
    config: McpConfig,
    base: PathBuf,
    clients: HashMap<String, Client>,
}

fn failure(e: &McpError) -> Failure {
    match e {
        McpError::Unavailable(m) => ("Unavailable", m.clone()),
        McpError::Protocol(m) => ("Protocol", m.clone()),
    }
}

impl McpHub {
    pub(crate) fn new(config: McpConfig, base: PathBuf) -> Self {
        McpHub {
            config,
            base,
            clients: HashMap::new(),
        }
    }

    /// A hub that may start no server; a placeholder while the real one is
    /// kept elsewhere (the REPL's session).
    pub(crate) fn empty() -> Self {
        McpHub::new(McpConfig::default(), PathBuf::new())
    }

    /// The running client of `server`, started if needed.
    pub(crate) fn client(&mut self, server: &str) -> Result<&mut Client, Failure> {
        if !self.clients.contains_key(server) {
            let Some(cfg) = self.config.servers.get(server) else {
                return Err((
                    "NotConfigured",
                    format!(
                        "MCP server `{server}` is not configured; add `[mcp.servers.{server}]` with its `command` to elomar.toml"
                    ),
                ));
            };
            let launch = Launch {
                command: cfg.command.clone(),
                args: cfg.args.clone(),
                env: cfg
                    .env
                    .iter()
                    .map(|(k, v)| (k.clone(), v.clone()))
                    .collect(),
                timeout: Duration::from_millis(cfg.timeout_ms.unwrap_or(30_000)),
            };
            let client = Client::start(server, &launch, &self.base).map_err(|e| failure(&e))?;
            self.clients.insert(server.to_string(), client);
        }
        Ok(self.clients.get_mut(server).expect("just started"))
    }

    /// Forgets a server that stopped, so the next use restarts it.
    fn forget_if_down(&mut self, server: &str, e: &McpError) {
        if matches!(e, McpError::Unavailable(_)) {
            self.clients.remove(server);
        }
    }

    pub(crate) fn tools(&mut self, server: &str) -> Result<Vec<Tool>, Failure> {
        let result = self.client(server)?.list_tools();
        result.map_err(|e| {
            self.forget_if_down(server, &e);
            failure(&e)
        })
    }

    /// Calls a tool; the result is its text, or `ToolFailed` when the tool
    /// reported an error.
    pub(crate) fn call(
        &mut self,
        server: &str,
        tool: &str,
        arguments: serde_json::Value,
    ) -> Result<String, Failure> {
        let result = self.client(server)?.call_tool(tool, arguments);
        match result {
            Ok(r) if r.is_error => Err(("ToolFailed", r.text)),
            Ok(r) => Ok(r.text),
            Err(e) => {
                self.forget_if_down(server, &e);
                Err(failure(&e))
            }
        }
    }
}

fn error((kind, message): Failure) -> Value {
    Value::Result(Err(Rc::new(Value::enum_value(
        "McpError",
        kind,
        vec![Value::str(message)],
    ))))
}

fn text(v: Option<&Value>) -> &str {
    match v {
        Some(Value::Str(s)) => s,
        _ => "",
    }
}

/// `mcp.tools(server)` and `mcp.call(server, tool, arguments)`.
pub(crate) fn call(hub: &mut McpHub, func: &str, args: &[Value]) -> Value {
    let server = text(args.first());
    match func {
        "tools" => match hub.tools(server) {
            Ok(tools) => Value::Result(Ok(Rc::new(Value::list(
                tools
                    .into_iter()
                    .map(|t| {
                        Value::Record(Rc::new(vec![
                            (Rc::from("name"), Value::str(t.name)),
                            (Rc::from("description"), Value::str(t.description)),
                        ]))
                    })
                    .collect(),
            )))),
            Err(f) => error(f),
        },
        _ => {
            let tool = text(args.get(1));
            let arguments = match args.get(2).map(to_json) {
                Some(serde_json::Value::Object(o)) => serde_json::Value::Object(o),
                _ => serde_json::json!({}),
            };
            match hub.call(server, tool, arguments) {
                Ok(t) => Value::Result(Ok(Rc::new(Value::str(t)))),
                Err(f) => error(f),
            }
        }
    }
}
