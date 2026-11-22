//! `elomar dap`: a debugger speaking the Debug Adapter Protocol on standard
//! input/output, for VS Code and other editors.
//!
//! The main thread reads requests. The program runs on its own thread with
//! a [`Debugger`] hook; while the program is stopped, that thread answers
//! the requests that need its state (stack, variables), which the main
//! thread forwards to it.

use std::collections::{HashMap, HashSet};
use std::io::{BufRead, BufReader, Write};
use std::path::{Path, PathBuf};
use std::process::ExitCode;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{Receiver, Sender, channel};
use std::sync::{Arc, Mutex};

use elomar_diagnostics::{Diagnostic, Lang, RenderOptions, SourceMap, Span, render_in};
use elomar_interpreter::{Debugger, Host, Inspect, MapKey, Resume, RuntimeError, Value};
use serde_json::{Value as Json, json};

use crate::{load_ai_runtime, project_root};

/// Sends protocol messages; shared by both threads.
#[derive(Clone)]
struct Out {
    inner: Arc<Mutex<(std::io::Stdout, i64)>>,
}

impl Out {
    fn send(&self, mut message: Json) {
        let mut guard = self.inner.lock().expect("output lock");
        guard.1 += 1;
        message["seq"] = json!(guard.1);
        let body = message.to_string();
        let out = &mut guard.0;
        let _ = write!(out, "Content-Length: {}\r\n\r\n{}", body.len(), body);
        let _ = out.flush();
    }

    fn event(&self, event: &str, body: Json) {
        self.send(json!({ "type": "event", "event": event, "body": body }));
    }

    fn respond(&self, request_seq: i64, command: &str, body: Json) {
        self.send(json!({
            "type": "response",
            "request_seq": request_seq,
            "success": true,
            "command": command,
            "body": body,
        }));
    }

    fn fail(&self, request_seq: i64, command: &str, message: &str) {
        self.send(json!({
            "type": "response",
            "request_seq": request_seq,
            "success": false,
            "command": command,
            "message": message,
            "body": { "error": { "id": 1, "format": message } },
        }));
    }

    fn output(&self, category: &str, text: &str) {
        self.event("output", json!({ "category": category, "output": text }));
    }
}

/// What the program thread is told while it is stopped.
enum Command {
    Continue,
    Next,
    StepIn,
    StepOut,
    /// A request that needs the program's state.
    Request {
        seq: i64,
        command: String,
        args: Json,
    },
    Disconnect,
}

/// State both threads look at.
#[derive(Default)]
struct Shared {
    /// Lines with breakpoints, by normalized file path.
    breakpoints: Mutex<HashMap<String, HashSet<u32>>>,
    stopped: AtomicBool,
    pause_requested: AtomicBool,
    stop_requested: AtomicBool,
    waiting_for_input: AtomicBool,
}

/// A path in one spelling, for comparing the editor's paths with ours.
fn normalize(path: &Path) -> String {
    let absolute = std::path::absolute(path).unwrap_or_else(|_| path.to_path_buf());
    let text = absolute.display().to_string().replace('\\', "/");
    if cfg!(windows) {
        text.to_lowercase()
    } else {
        text
    }
}

fn tr<'a>(lang: Lang, en: &'a str, ar: &'a str) -> &'a str {
    if lang == Lang::Ar { ar } else { en }
}

pub(crate) fn serve(lang: Lang) -> ExitCode {
    let out = Out {
        inner: Arc::new(Mutex::new((std::io::stdout(), 0))),
    };
    let shared = Arc::new(Shared::default());
    let mut reader = BufReader::new(std::io::stdin().lock());
    let mut commands: Option<Sender<Command>> = None;
    let mut input: Option<Sender<String>> = None;
    let mut launch: Option<Launch> = None;

    while let Some(message) = read_message(&mut reader) {
        let seq = message["seq"].as_i64().unwrap_or(0);
        let command = message["command"].as_str().unwrap_or("").to_string();
        let args = message["arguments"].clone();
        match command.as_str() {
            "initialize" => {
                out.respond(
                    seq,
                    &command,
                    json!({
                        "supportsConfigurationDoneRequest": true,
                        "supportsEvaluateForHovers": true,
                        "supportsTerminateRequest": true,
                    }),
                );
                out.event("initialized", json!({}));
            }
            "launch" => match prepare(&args, lang, &out) {
                Ok(l) => {
                    launch = Some(l);
                    out.respond(seq, &command, json!({}));
                }
                Err(message) => {
                    out.fail(seq, &command, &message);
                    out.event("terminated", json!({}));
                }
            },
            "setBreakpoints" => {
                let path = args["source"]["path"].as_str().unwrap_or("");
                let lines: Vec<u32> = args["breakpoints"]
                    .as_array()
                    .map(|bps| {
                        bps.iter()
                            .filter_map(|b| b["line"].as_u64().map(|l| l as u32))
                            .collect()
                    })
                    .unwrap_or_default();
                shared
                    .breakpoints
                    .lock()
                    .expect("breakpoints lock")
                    .insert(normalize(Path::new(path)), lines.iter().copied().collect());
                let verified: Vec<Json> = lines
                    .iter()
                    .map(|l| json!({ "verified": true, "line": l }))
                    .collect();
                out.respond(seq, &command, json!({ "breakpoints": verified }));
            }
            "setExceptionBreakpoints" => out.respond(seq, &command, json!({})),
            "configurationDone" => {
                out.respond(seq, &command, json!({}));
                if let Some(l) = launch.take() {
                    let (command_tx, command_rx) = channel();
                    let (input_tx, input_rx) = channel();
                    commands = Some(command_tx);
                    input = Some(input_tx);
                    start(l, lang, out.clone(), shared.clone(), command_rx, input_rx);
                }
            }
            "threads" => out.respond(
                seq,
                &command,
                json!({ "threads": [{ "id": 1, "name": "main" }] }),
            ),
            "continue" | "next" | "stepIn" | "stepOut" => {
                let body = if command == "continue" {
                    json!({ "allThreadsContinued": true })
                } else {
                    json!({})
                };
                out.respond(seq, &command, body);
                if shared.stopped.load(Ordering::SeqCst)
                    && let Some(tx) = &commands
                {
                    let _ = tx.send(match command.as_str() {
                        "continue" => Command::Continue,
                        "next" => Command::Next,
                        "stepIn" => Command::StepIn,
                        _ => Command::StepOut,
                    });
                }
            }
            "pause" => {
                shared.pause_requested.store(true, Ordering::SeqCst);
                out.respond(seq, &command, json!({}));
            }
            "stackTrace" | "scopes" | "variables" | "evaluate" => {
                // Text typed in the Debug Console while the program waits
                // for `input(...)` is the answer.
                if command == "evaluate" && shared.waiting_for_input.load(Ordering::SeqCst) {
                    if let Some(tx) = &input {
                        let _ = tx.send(args["expression"].as_str().unwrap_or("").to_string());
                    }
                    out.respond(
                        seq,
                        &command,
                        json!({ "result": "", "variablesReference": 0 }),
                    );
                } else if shared.stopped.load(Ordering::SeqCst)
                    && let Some(tx) = &commands
                {
                    let _ = tx.send(Command::Request {
                        seq,
                        command: command.clone(),
                        args,
                    });
                } else {
                    match command.as_str() {
                        "stackTrace" => out.respond(
                            seq,
                            &command,
                            json!({ "stackFrames": [], "totalFrames": 0 }),
                        ),
                        "scopes" => out.respond(seq, &command, json!({ "scopes": [] })),
                        "variables" => out.respond(seq, &command, json!({ "variables": [] })),
                        _ => out.fail(
                            seq,
                            &command,
                            tr(
                                lang,
                                "The program is running; pause it to look at values.",
                                "البرنامج يعمل؛ أوقفه مؤقتًا لترى القيم.",
                            ),
                        ),
                    }
                }
            }
            "disconnect" | "terminate" => {
                shared.stop_requested.store(true, Ordering::SeqCst);
                if let Some(tx) = &commands {
                    let _ = tx.send(Command::Disconnect);
                }
                out.respond(seq, &command, json!({}));
                if command == "disconnect" {
                    break;
                }
                out.event("terminated", json!({}));
            }
            _ => out.respond(seq, &command, json!({})),
        }
    }
    // Ends the program thread, child processes and MCP servers too.
    std::process::exit(0)
}

/// Reads one `Content-Length`-framed message; `None` at end of input.
fn read_message(reader: &mut impl BufRead) -> Option<Json> {
    let mut length = None;
    loop {
        let mut line = String::new();
        if reader.read_line(&mut line).ok()? == 0 {
            return None;
        }
        let line = line.trim_end();
        if line.is_empty() {
            break;
        }
        if let Some(value) = line.strip_prefix("Content-Length:") {
            length = value.trim().parse::<usize>().ok();
        }
    }
    let mut body = vec![0; length?];
    reader.read_exact(&mut body).ok()?;
    serde_json::from_slice(&body).ok()
}

/// A checked program, ready to run.
struct Launch {
    program: elomar_ast::Program,
    sources: SourceMap,
    paths: Vec<String>,
    files: Vec<PathBuf>,
    args: Vec<String>,
    stop_on_entry: bool,
    ai: crate::LoadedAi,
}

/// Loads and checks the program named in the `launch` request. Errors in
/// the program are shown in the Debug Console.
fn prepare(args: &Json, lang: Lang, out: &Out) -> Result<Launch, String> {
    let Some(program) = args["program"].as_str().filter(|p| !p.is_empty()) else {
        return Err(tr(
            lang,
            "No program to debug: set `program` in launch.json.",
            "لا يوجد برنامج لتنقيحه: اضبط `program` في launch.json.",
        )
        .into());
    };
    let path = PathBuf::from(program);
    let loaded = elomar_loader::load(&path, &project_root(&path))
        .map_err(|e| format!("{}: {e}", path.display()))?;
    let ai = load_ai_runtime(&path).map_err(|(p, m)| format!("{}: {m}", p.display()))?;
    let mut program = loaded.program;
    let mut diags = loaded.diagnostics;
    if diags.iter().all(|d| !d.is_error()) {
        diags.extend(elomar_types::check(
            &mut program,
            elomar_types::CheckOptions {
                require_main: true,
                cloud_models: ai.cloud_models.clone(),
            },
        ));
    }
    let opts = RenderOptions { lang, color: false };
    let errors: Vec<&Diagnostic> = diags.iter().filter(|d| d.is_error()).collect();
    if !errors.is_empty() {
        for d in &errors {
            out.output(
                "stderr",
                &format!("{}\n", render_in(d, &loaded.sources, opts)),
            );
        }
        return Err(tr(
            lang,
            "The program has errors; see the Debug Console.",
            "في البرنامج أخطاء؛ انظر وحدة تحكم التنقيح (Debug Console).",
        )
        .into());
    }
    Ok(Launch {
        program,
        sources: loaded.sources,
        paths: loaded.paths.iter().map(|p| normalize(p)).collect(),
        files: loaded.paths.clone(),
        args: args["args"]
            .as_array()
            .map(|a| {
                a.iter()
                    .filter_map(|x| x.as_str().map(String::from))
                    .collect()
            })
            .unwrap_or_default(),
        stop_on_entry: args["stopOnEntry"].as_bool().unwrap_or(false),
        ai,
    })
}

/// Runs the program on its own thread; reports how it ended.
fn start(
    launch: Launch,
    lang: Lang,
    out: Out,
    shared: Arc<Shared>,
    commands: Receiver<Command>,
    input: Receiver<String>,
) {
    std::thread::spawn(move || {
        let Launch {
            program,
            sources,
            paths,
            files,
            args,
            stop_on_entry,
            ai,
        } = launch;
        let opts = RenderOptions { lang, color: false };
        let result = elomar_interpreter::with_large_stack(|| {
            let mut host = DapHost {
                out: out.clone(),
                shared: shared.clone(),
                input,
                args,
            };
            let debugger = DapDebugger {
                out: out.clone(),
                shared: shared.clone(),
                commands,
                sources: sources.clone(),
                paths,
                files,
                lang,
                mode: Mode::Run,
                stop_on_entry,
                last_line: None,
                nodes: Vec::new(),
            };
            let mut runtime = ai.runtime;
            elomar_interpreter::run_debug(
                &program,
                &mut host,
                &mut runtime,
                ai.fs,
                Box::new(debugger),
            )
            .map_err(|e| (e.exit, render_in(&e.to_diagnostic(), &sources, opts)))
        });
        let code = match result {
            Ok(()) => 0,
            Err((Some(code), _)) => code,
            Err((None, text)) => {
                out.output("stderr", &format!("{text}\n"));
                1
            }
        };
        out.event("exited", json!({ "exitCode": code }));
        out.event("terminated", json!({}));
    });
}

/// The program's output goes to the Debug Console; `input` reads what is
/// typed there.
struct DapHost {
    out: Out,
    shared: Arc<Shared>,
    input: Receiver<String>,
    args: Vec<String>,
}

impl Host for DapHost {
    fn print(&mut self, text: &str) {
        self.out.output("stdout", text);
    }

    fn input(&mut self, prompt: &str) -> Option<String> {
        self.out.output("stdout", prompt);
        self.shared.waiting_for_input.store(true, Ordering::SeqCst);
        let line = self.input.recv().ok();
        self.shared.waiting_for_input.store(false, Ordering::SeqCst);
        if let Some(l) = &line {
            self.out.output("stdout", &format!("{l}\n"));
        }
        line
    }

    fn args(&self) -> Vec<String> {
        self.args.clone()
    }
}

/// Where to stop next, besides breakpoints.
#[derive(Clone, Copy)]
enum Mode {
    Run,
    /// The next new line, in any call.
    StepIn,
    /// The next new line in this call or a caller.
    Over(usize),
    /// The next line in a caller.
    Out(usize),
}

/// A `variablesReference` handed to the editor during one stop.
enum Node {
    Locals(usize),
    Globals,
    Value(Value),
}

struct DapDebugger {
    out: Out,
    shared: Arc<Shared>,
    commands: Receiver<Command>,
    sources: SourceMap,
    /// Normalized path of each file in `sources`.
    paths: Vec<String>,
    /// Absolute path of each file in `sources`, as the editor knows it.
    files: Vec<PathBuf>,
    lang: Lang,
    mode: Mode,
    stop_on_entry: bool,
    /// (file, line, depth) of the statement that ran last.
    last_line: Option<(usize, u32, usize)>,
    nodes: Vec<Node>,
}

impl DapDebugger {
    /// The file index and 1-based line and column of a span.
    fn position(&self, span: Span) -> (usize, u32, u32) {
        let offset = span.start as usize;
        let index = self.sources.index_at(offset);
        let (file, base) = self.sources.file_at(offset);
        let (line, col) = file.line_col(offset - base);
        (index, line as u32, col as u32)
    }

    /// Stops the program and serves the editor until it says how to go on.
    fn stop(&mut self, inspect: &dyn Inspect, reason: &str, text: Option<String>) -> Resume {
        self.mode = Mode::Run;
        self.nodes.clear();
        self.shared.stopped.store(true, Ordering::SeqCst);
        let mut body = json!({ "reason": reason, "threadId": 1, "allThreadsStopped": true });
        if let Some(t) = text {
            body["text"] = json!(t);
            body["description"] = json!(t);
        }
        self.out.event("stopped", body);
        let resume = loop {
            let Ok(command) = self.commands.recv() else {
                break Resume::Stop;
            };
            match command {
                Command::Continue => break Resume::Run,
                Command::Next => {
                    self.mode = Mode::Over(inspect.depth());
                    break Resume::Run;
                }
                Command::StepIn => {
                    self.mode = Mode::StepIn;
                    break Resume::Run;
                }
                Command::StepOut => {
                    self.mode = Mode::Out(inspect.depth());
                    break Resume::Run;
                }
                Command::Disconnect => break Resume::Stop,
                Command::Request { seq, command, args } => {
                    self.answer(inspect, seq, &command, &args)
                }
            }
        };
        self.shared.stopped.store(false, Ordering::SeqCst);
        resume
    }

    fn answer(&mut self, inspect: &dyn Inspect, seq: i64, command: &str, args: &Json) {
        match command {
            "stackTrace" => {
                let frames: Vec<Json> = inspect
                    .stack()
                    .iter()
                    .enumerate()
                    .map(|(i, f)| {
                        let (file, line, column) = self.position(f.span);
                        let (source, _) = self.sources.file_at(f.span.start as usize);
                        let path = self.files.get(file).map(|p| p.display().to_string());
                        json!({
                            "id": i + 1,
                            "name": f.name,
                            "line": line,
                            "column": column,
                            "source": { "name": source.name, "path": path },
                        })
                    })
                    .collect();
                let total = frames.len();
                self.out.respond(
                    seq,
                    command,
                    json!({ "stackFrames": frames, "totalFrames": total }),
                );
            }
            "scopes" => {
                let frame = args["frameId"].as_u64().unwrap_or(1).saturating_sub(1) as usize;
                self.nodes.push(Node::Locals(frame));
                let mut scopes = vec![json!({
                    "name": tr(self.lang, "Locals", "المتغيرات"),
                    "variablesReference": self.nodes.len(),
                    "expensive": false,
                })];
                if !inspect.globals().is_empty() {
                    self.nodes.push(Node::Globals);
                    scopes.push(json!({
                        "name": tr(self.lang, "Constants", "الثوابت"),
                        "variablesReference": self.nodes.len(),
                        "expensive": false,
                    }));
                }
                self.out.respond(seq, command, json!({ "scopes": scopes }));
            }
            "variables" => {
                let reference = args["variablesReference"].as_u64().unwrap_or(0) as usize;
                let children = match reference.checked_sub(1).and_then(|i| self.nodes.get(i)) {
                    Some(Node::Locals(frame)) => inspect.locals(*frame),
                    Some(Node::Globals) => inspect.globals(),
                    Some(Node::Value(v)) => children(v),
                    None => Vec::new(),
                };
                let variables: Vec<Json> = children
                    .into_iter()
                    .map(|(name, value)| self.variable(&name, value))
                    .collect();
                self.out
                    .respond(seq, command, json!({ "variables": variables }));
            }
            "evaluate" => {
                let expression = args["expression"].as_str().unwrap_or("").trim();
                let frame = args["frameId"].as_u64().unwrap_or(1).saturating_sub(1) as usize;
                match lookup(inspect, frame, expression) {
                    Some(value) => {
                        let v = self.variable(expression, value);
                        self.out.respond(
                            seq,
                            command,
                            json!({ "result": v["value"], "variablesReference": v["variablesReference"] }),
                        );
                    }
                    None => self.out.fail(
                        seq,
                        command,
                        tr(
                            self.lang,
                            "Only variables and their fields can be shown here, such as `total` or `student.name`.",
                            "يمكن هنا عرض المتغيرات وحقولها فقط، مثل `total` أو `student.name`.",
                        ),
                    ),
                }
            }
            _ => self.out.respond(seq, command, json!({})),
        }
    }

    /// A variable for the editor; values with parts get a reference to
    /// expand them.
    fn variable(&mut self, name: &str, value: Value) -> Json {
        let shown = show(&value);
        let reference = if children(&value).is_empty() {
            0
        } else {
            self.nodes.push(Node::Value(value));
            self.nodes.len()
        };
        json!({ "name": name, "value": shown, "variablesReference": reference })
    }
}

impl Debugger for DapDebugger {
    fn statement(&mut self, inspect: &dyn Inspect, span: Span) -> Resume {
        if self.shared.stop_requested.load(Ordering::SeqCst) {
            return Resume::Stop;
        }
        let depth = inspect.depth();
        let (file, line, _) = self.position(span);
        let key = (file, line, depth);
        let new_line = self.last_line != Some(key);
        self.last_line = Some(key);

        let reason = if std::mem::take(&mut self.stop_on_entry) {
            Some("entry")
        } else if self.shared.pause_requested.swap(false, Ordering::SeqCst) {
            Some("pause")
        } else if !new_line {
            None
        } else if match self.mode {
            Mode::Run => false,
            Mode::StepIn => true,
            Mode::Over(d) => depth <= d,
            Mode::Out(d) => depth < d,
        } {
            Some("step")
        } else if self
            .shared
            .breakpoints
            .lock()
            .expect("breakpoints lock")
            .get(&self.paths[file])
            .is_some_and(|lines| lines.contains(&line))
        {
            Some("breakpoint")
        } else {
            None
        };
        match reason {
            Some(r) => self.stop(inspect, r, None),
            None => Resume::Run,
        }
    }

    fn error(&mut self, inspect: &dyn Inspect, _span: Span, error: &RuntimeError) -> Resume {
        let text = format!("[{}] {}", error.code, error.message.get(self.lang));
        self.stop(inspect, "exception", Some(text))
    }
}

/// A value as the editor shows it; strings get quotes.
fn show(value: &Value) -> String {
    match value {
        Value::Str(s) => format!("{s:?}"),
        v => v.to_string(),
    }
}

/// The parts of a value: list items, fields, map entries, payloads.
fn children(value: &Value) -> Vec<(String, Value)> {
    match value {
        Value::List(items) | Value::Tuple(items) => items
            .iter()
            .enumerate()
            .map(|(i, v)| (format!("[{i}]"), v.clone()))
            .collect(),
        Value::Record(fields) => fields
            .iter()
            .map(|(n, v)| (n.to_string(), v.clone()))
            .collect(),
        Value::Map(entries) => entries
            .iter()
            .map(|(k, v)| (show(&k.to_value()), v.clone()))
            .collect(),
        Value::Set(items) => items
            .iter()
            .enumerate()
            .map(|(i, k): (usize, &MapKey)| (format!("[{i}]"), k.to_value()))
            .collect(),
        Value::Enum { fields, .. } => fields
            .iter()
            .enumerate()
            .map(|(i, v)| (format!("[{i}]"), v.clone()))
            .collect(),
        Value::Option(Some(v)) | Value::Result(Ok(v)) | Value::Result(Err(v)) => {
            let inner = (**v).clone();
            let mut parts = children(&inner);
            if parts.is_empty() {
                parts.push(("value".into(), inner));
            }
            parts
        }
        _ => Vec::new(),
    }
}

/// `name` or `name.field.field` in the variables of `frame`.
fn lookup(inspect: &dyn Inspect, frame: usize, expression: &str) -> Option<Value> {
    let mut parts = expression.split('.');
    let first = parts.next()?.trim();
    let mut value = inspect
        .locals(frame)
        .into_iter()
        .chain(inspect.globals())
        .find(|(n, _)| n == first)
        .map(|(_, v)| v)?;
    for part in parts {
        let part = part.trim();
        value = match &value {
            Value::Record(fields) => fields.iter().find(|(n, _)| &**n == part)?.1.clone(),
            Value::List(items) | Value::Tuple(items) => {
                items.get(part.parse::<usize>().ok()?)?.clone()
            }
            _ => return None,
        };
    }
    Some(value)
}
