//! A Model Context Protocol client over stdio, for `std.mcp` and the MCP
//! tools of `agent.run`.
//!
//! - The server is a subprocess; messages are newline-delimited JSON-RPC
//!   on its stdin and stdout. What it writes to stderr is kept (the last
//!   few kilobytes) to explain a crash.
//! - Modern servers (protocol 2026-07-28) have no handshake: every request
//!   carries the protocol version and client capabilities in `_meta`.
//!   Older servers need the `initialize` handshake. As the stdio binding
//!   recommends, the client first probes with `server/discover` and falls
//!   back to `initialize` on any error that is not a modern one, or on
//!   silence.
//! - Only tools are used: `tools/list` and `tools/call`. The client
//!   declares no capabilities, so servers must not ask it for input
//!   (sampling, elicitation); such a result is reported as an error.

use std::collections::VecDeque;
use std::io::{BufRead, BufReader, Read, Write};
use std::path::Path;
use std::process::{Child, ChildStdin, Command, Stdio};
use std::sync::mpsc::{Receiver, RecvTimeoutError, channel};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use serde_json::{Value, json};

/// The protocol revision without a handshake.
pub const MODERN_VERSION: &str = "2026-07-28";
/// The newest revision with the `initialize` handshake.
pub const LEGACY_VERSION: &str = "2025-11-25";
/// How long to wait for an answer to the `server/discover` probe before
/// assuming a legacy server that ignores unknown requests.
const PROBE_TIMEOUT: Duration = Duration::from_secs(3);
/// JSON-RPC errors defined by the modern protocol.
const UNSUPPORTED_VERSION: i64 = -32022;
const MISSING_CAPABILITY: i64 = -32021;
const STDERR_TAIL: usize = 4096;

#[derive(Debug, Clone, PartialEq)]
pub enum McpError {
    /// The server could not be started, exited, or did not answer in time.
    Unavailable(String),
    /// The server answered with a JSON-RPC error or something malformed.
    Protocol(String),
}

impl McpError {
    pub fn message(&self) -> &str {
        match self {
            McpError::Unavailable(m) | McpError::Protocol(m) => m,
        }
    }
}

impl std::fmt::Display for McpError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.message())
    }
}

/// How to start a server.
#[derive(Debug, Clone, Default)]
pub struct Launch {
    pub command: String,
    pub args: Vec<String>,
    pub env: Vec<(String, String)>,
    /// Limit for each request.
    pub timeout: Duration,
}

/// A tool as the server describes it.
#[derive(Debug, Clone, PartialEq)]
pub struct Tool {
    pub name: String,
    pub description: String,
    /// A JSON Schema object for the arguments.
    pub input_schema: Value,
}

/// What `tools/call` returned.
#[derive(Debug, Clone, PartialEq)]
pub struct CallResult {
    /// The text content, joined with newlines. Non-text content is shown
    /// as a short placeholder; if there is no content at all, this is the
    /// structured content as JSON.
    pub text: String,
    /// The tool reported a failure (`isError`).
    pub is_error: bool,
    pub structured: Option<Value>,
}

#[derive(Debug, Clone, PartialEq)]
enum Era {
    Modern,
    Legacy,
}

/// An error answer to a request.
enum RpcFailure {
    Error {
        code: i64,
        message: String,
        data: Value,
    },
    Transport(McpError),
    /// No answer in time.
    Timeout(McpError),
}

pub struct Client {
    name: String,
    child: Child,
    stdin: Option<ChildStdin>,
    lines: Receiver<String>,
    stderr: Arc<Mutex<VecDeque<u8>>>,
    next_id: i64,
    era: Era,
    timeout: Duration,
}

impl Client {
    /// Starts the server `name` (used in messages) in the folder `cwd` and
    /// agrees on a protocol version.
    pub fn start(name: &str, launch: &Launch, cwd: &Path) -> Result<Client, McpError> {
        let mut cmd = Command::new(&launch.command);
        cmd.args(&launch.args)
            .current_dir(cwd)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());
        for (k, v) in &launch.env {
            cmd.env(k, v);
        }
        let mut child = cmd.spawn().map_err(|e| {
            McpError::Unavailable(format!(
                "MCP server `{name}`: cannot run `{}`: {e}",
                launch.command
            ))
        })?;
        let stdout = child.stdout.take().expect("piped stdout");
        let stderr_pipe = child.stderr.take().expect("piped stderr");
        let (tx, lines) = channel();
        std::thread::spawn(move || {
            for line in BufReader::new(stdout).lines() {
                let Ok(line) = line else { break };
                if tx.send(line).is_err() {
                    break;
                }
            }
        });
        let stderr = Arc::new(Mutex::new(VecDeque::new()));
        let tail = Arc::clone(&stderr);
        std::thread::spawn(move || {
            let mut pipe = stderr_pipe;
            let mut buf = [0u8; 1024];
            while let Ok(n) = pipe.read(&mut buf) {
                if n == 0 {
                    break;
                }
                let mut t = tail.lock().expect("stderr buffer");
                t.extend(&buf[..n]);
                while t.len() > STDERR_TAIL {
                    t.pop_front();
                }
            }
        });
        let mut client = Client {
            name: name.to_string(),
            stdin: child.stdin.take(),
            child,
            lines,
            stderr,
            next_id: 1,
            era: Era::Modern,
            timeout: if launch.timeout.is_zero() {
                Duration::from_secs(30)
            } else {
                launch.timeout
            },
        };
        client.negotiate()?;
        Ok(client)
    }

    /// Whether the server speaks the modern protocol (no handshake).
    pub fn is_modern(&self) -> bool {
        self.era == Era::Modern
    }

    fn negotiate(&mut self) -> Result<(), McpError> {
        let probe = self.request_within(
            "server/discover",
            json!({}),
            PROBE_TIMEOUT.min(self.timeout),
        );
        let supported: Vec<String> = match probe {
            Ok(result) => strings(&result["supportedVersions"]),
            Err(RpcFailure::Error { code, data, .. }) if code == UNSUPPORTED_VERSION => {
                strings(&data["supported"])
            }
            Err(RpcFailure::Transport(e)) => return Err(e),
            // Any other error, or silence: a server from before 2026-07-28.
            Err(_) => return self.initialize(),
        };
        if supported.iter().any(|v| v == MODERN_VERSION) {
            return Ok(());
        }
        // A dual-era server that lists only handshake versions we know.
        if supported.iter().any(|v| v.as_str() <= LEGACY_VERSION) {
            return self.initialize();
        }
        Err(McpError::Protocol(format!(
            "MCP server `{}` supports protocol version(s) {}; Elomar supports {MODERN_VERSION} and {LEGACY_VERSION}",
            self.name,
            supported.join(", ")
        )))
    }

    fn initialize(&mut self) -> Result<(), McpError> {
        self.era = Era::Legacy;
        self.request(
            "initialize",
            json!({
                "protocolVersion": LEGACY_VERSION,
                "capabilities": {},
                "clientInfo": client_info(),
            }),
        )?;
        self.send(&json!({"jsonrpc": "2.0", "method": "notifications/initialized"}))
    }

    /// All tools of the server (following pagination).
    pub fn list_tools(&mut self) -> Result<Vec<Tool>, McpError> {
        let mut tools = Vec::new();
        let mut cursor: Option<String> = None;
        for _ in 0..100 {
            let params = match &cursor {
                Some(c) => json!({ "cursor": c }),
                None => json!({}),
            };
            let result = self.request("tools/list", params)?;
            let Some(list) = result["tools"].as_array() else {
                return Err(self.malformed("tools/list", "no `tools` list"));
            };
            for t in list {
                let Some(name) = t["name"].as_str() else {
                    continue;
                };
                let schema = match &t["inputSchema"] {
                    Value::Object(_) => t["inputSchema"].clone(),
                    _ => json!({ "type": "object" }),
                };
                tools.push(Tool {
                    name: name.to_string(),
                    description: t["description"]
                        .as_str()
                        .or(t["title"].as_str())
                        .unwrap_or("")
                        .to_string(),
                    input_schema: schema,
                });
            }
            match result["nextCursor"].as_str() {
                Some(c) if !c.is_empty() => cursor = Some(c.to_string()),
                _ => break,
            }
        }
        Ok(tools)
    }

    /// Calls a tool. `arguments` must be a JSON object.
    pub fn call_tool(&mut self, tool: &str, arguments: Value) -> Result<CallResult, McpError> {
        let result = self.request(
            "tools/call",
            json!({ "name": tool, "arguments": arguments }),
        )?;
        if result["resultType"].as_str() == Some("input_required") {
            return Err(McpError::Protocol(format!(
                "MCP server `{}`: tool `{tool}` asked for more input (elicitation or sampling), which Elomar does not support yet",
                self.name
            )));
        }
        let mut parts = Vec::new();
        for item in result["content"].as_array().into_iter().flatten() {
            match item["type"].as_str() {
                Some("text") => parts.push(item["text"].as_str().unwrap_or("").to_string()),
                Some("resource") => {
                    let r = &item["resource"];
                    parts.push(match r["text"].as_str() {
                        Some(t) => t.to_string(),
                        None => format!("[resource {}]", r["uri"].as_str().unwrap_or("")),
                    });
                }
                Some("resource_link") => {
                    parts.push(format!("[link {}]", item["uri"].as_str().unwrap_or("")));
                }
                Some(other) => parts.push(format!("[{other}]")),
                None => {}
            }
        }
        let structured = match &result["structuredContent"] {
            Value::Null => None,
            v => Some(v.clone()),
        };
        let text = match (&structured, parts.is_empty()) {
            (Some(s), true) => s.to_string(),
            _ => parts.join("\n"),
        };
        Ok(CallResult {
            text,
            is_error: result["isError"].as_bool().unwrap_or(false),
            structured,
        })
    }

    fn malformed(&self, method: &str, what: &str) -> McpError {
        McpError::Protocol(format!(
            "MCP server `{}`: malformed answer to `{method}`: {what}",
            self.name
        ))
    }

    fn request(&mut self, method: &str, params: Value) -> Result<Value, McpError> {
        match self.request_within(method, params, self.timeout) {
            Ok(v) => Ok(v),
            Err(RpcFailure::Transport(e) | RpcFailure::Timeout(e)) => Err(e),
            Err(RpcFailure::Error { code, message, .. }) => {
                let hint = if code == MISSING_CAPABILITY {
                    " (the server needs a client capability Elomar does not offer)"
                } else {
                    ""
                };
                Err(McpError::Protocol(format!(
                    "MCP server `{}`: `{method}` failed: {message} (code {code}){hint}",
                    self.name
                )))
            }
        }
    }

    fn request_within(
        &mut self,
        method: &str,
        mut params: Value,
        timeout: Duration,
    ) -> Result<Value, RpcFailure> {
        if self.era == Era::Modern {
            params["_meta"] = json!({
                "io.modelcontextprotocol/protocolVersion": MODERN_VERSION,
                "io.modelcontextprotocol/clientInfo": client_info(),
                "io.modelcontextprotocol/clientCapabilities": {},
            });
        }
        let id = self.next_id;
        self.next_id += 1;
        self.send(&json!({ "jsonrpc": "2.0", "id": id, "method": method, "params": params }))
            .map_err(RpcFailure::Transport)?;
        let deadline = Instant::now() + timeout;
        loop {
            let left = deadline.saturating_duration_since(Instant::now());
            let line = match self.lines.recv_timeout(left) {
                Ok(line) => line,
                Err(RecvTimeoutError::Timeout) => {
                    return Err(RpcFailure::Timeout(McpError::Unavailable(format!(
                        "MCP server `{}`: no answer to `{method}` within {} ms",
                        self.name,
                        timeout.as_millis()
                    ))));
                }
                Err(RecvTimeoutError::Disconnected) => {
                    return Err(RpcFailure::Transport(self.exited()));
                }
            };
            let Ok(msg) = serde_json::from_str::<Value>(line.trim()) else {
                continue;
            };
            if msg.get("method").is_some() {
                // A request from a legacy server (e.g. `ping`); answer so
                // it does not wait. Notifications need no answer.
                if let Some(rid) = msg.get("id") {
                    let reply = if msg["method"] == "ping" {
                        json!({ "jsonrpc": "2.0", "id": rid, "result": {} })
                    } else {
                        json!({ "jsonrpc": "2.0", "id": rid, "error": { "code": -32601, "message": "not supported by this client" } })
                    };
                    let _ = self.send(&reply);
                }
                continue;
            }
            if msg.get("id").and_then(Value::as_i64) != Some(id) {
                continue;
            }
            if let Some(err) = msg.get("error") {
                return Err(RpcFailure::Error {
                    code: err["code"].as_i64().unwrap_or(0),
                    message: err["message"].as_str().unwrap_or("error").to_string(),
                    data: err.get("data").cloned().unwrap_or(Value::Null),
                });
            }
            return Ok(msg.get("result").cloned().unwrap_or(Value::Null));
        }
    }

    fn send(&mut self, msg: &Value) -> Result<(), McpError> {
        let Some(stdin) = self.stdin.as_mut() else {
            return Err(self.exited());
        };
        let mut line = msg.to_string();
        line.push('\n');
        if stdin
            .write_all(line.as_bytes())
            .and_then(|()| stdin.flush())
            .is_err()
        {
            return Err(self.exited());
        }
        Ok(())
    }

    fn exited(&mut self) -> McpError {
        // Give the stderr reader a moment to collect the last words.
        let _ = self.child.try_wait();
        std::thread::sleep(Duration::from_millis(50));
        let tail = {
            let t = self.stderr.lock().expect("stderr buffer");
            String::from_utf8_lossy(&t.iter().copied().collect::<Vec<u8>>()).to_string()
        };
        let tail = tail.trim();
        let last = tail.lines().rev().take(5).collect::<Vec<_>>();
        let detail = if last.is_empty() {
            String::new()
        } else {
            format!(
                ": {}",
                last.into_iter().rev().collect::<Vec<_>>().join(" | ")
            )
        };
        McpError::Unavailable(format!("MCP server `{}` stopped{detail}", self.name))
    }
}

impl Drop for Client {
    /// Closes the server's input, waits briefly, then kills it.
    fn drop(&mut self) {
        self.stdin = None;
        let deadline = Instant::now() + Duration::from_millis(500);
        while Instant::now() < deadline {
            if let Ok(Some(_)) = self.child.try_wait() {
                return;
            }
            std::thread::sleep(Duration::from_millis(10));
        }
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

fn client_info() -> Value {
    json!({ "name": "elomar", "version": env!("CARGO_PKG_VERSION") })
}

fn strings(v: &Value) -> Vec<String> {
    v.as_array()
        .into_iter()
        .flatten()
        .filter_map(|s| s.as_str().map(str::to_string))
        .collect()
}

#[cfg(test)]
mod tests;
