//! `std.py` at run time: calling Python functions in a subprocess.
//!
//! - Needs `python = true` under `[permissions]` in elomar.toml. Python
//!   code runs with the user's rights: Elomar's file and network
//!   permissions do not apply inside it, which is why it is opt-in.
//! - Each call starts `python -X utf8 -c <driver>` (or `$ELOMAR_PYTHON`),
//!   sends `{module, function, args}` as JSON on stdin and reads the JSON
//!   result from stdout. Modules are looked up in `<project>/python`, the
//!   project folder, and the installed packages.
//! - A record argument becomes keyword arguments; any other value is one
//!   positional argument. What the function prints is shown after the
//!   call. Failures are `Err(PyError...)` values.

use std::collections::HashMap;
use std::io::{Read, Write};
use std::path::Path;
use std::process::{Command, Stdio};
use std::rc::Rc;
use std::time::{Duration, Instant};

use elomar_ai_core::Permissions;
use elomar_ast::TypeExpr;
use serde_json::{Value as Json, json};

use crate::Value;
use crate::ai::{from_json, schema_of, to_json};

pub const DEFAULT_TIMEOUT: Duration = Duration::from_secs(60);

/// Runs one call inside Python. Prints from the function go to stderr so
/// that stdout carries only the result.
const DRIVER: &str = r#"
import importlib, json, sys, traceback
request = json.loads(sys.stdin.read())
for p in reversed(request["path"]):
    sys.path.insert(0, p)
out = sys.stdout
sys.stdout = sys.stderr
try:
    fn = getattr(importlib.import_module(request["module"]), request["function"])
    args = request["args"]
    result = fn(**args) if request["keywords"] else fn(args)
    text = json.dumps({"ok": result}, allow_nan=False, ensure_ascii=False)
except BaseException as e:
    text = json.dumps({"error": type(e).__name__, "message": str(e),
                       "trace": traceback.format_exc()}, ensure_ascii=False)
out.write(text)
out.flush()
"#;

fn error(kind: &str, message: String) -> Value {
    Value::Result(Err(Rc::new(Value::Enum {
        enum_name: "PyError".into(),
        variant: kind.into(),
        fields: Rc::new(vec![Value::str(message)]),
    })))
}

fn text(v: &Value) -> &str {
    match v {
        Value::Str(s) => s,
        _ => "",
    }
}

fn executable() -> String {
    std::env::var("ELOMAR_PYTHON")
        .ok()
        .filter(|p| !p.trim().is_empty())
        .unwrap_or_else(|| if cfg!(windows) { "python" } else { "python3" }.to_string())
}

/// Returns the call's value and what the Python function printed.
pub(crate) fn call(
    permissions: &Permissions,
    base: &Path,
    type_args: &[TypeExpr],
    args: &[Value],
    unit_enums: &HashMap<String, Vec<String>>,
) -> (Value, String) {
    if !permissions.python {
        return (
            error(
                "PolicyDenied",
                "running Python is not permitted; add `python = true` under `[permissions]` in elomar.toml".into(),
            ),
            String::new(),
        );
    }
    let module = text(&args[0]);
    let function = text(&args[1]);
    let keywords = matches!(args[2], Value::Record(_));
    let timeout = match args.get(3) {
        Some(Value::Duration(ms)) => Duration::from_millis((*ms).max(1) as u64),
        _ => DEFAULT_TIMEOUT,
    };
    let request = json!({
        "module": module,
        "function": function,
        "args": to_json(&args[2]),
        "keywords": keywords,
        "path": [base.join("python"), base.to_path_buf()],
    });
    let exe = executable();
    let child = Command::new(&exe)
        .args(["-X", "utf8", "-c", DRIVER])
        .current_dir(base)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn();
    let mut child = match child {
        Ok(c) => c,
        Err(e) => {
            return (
                error(
                    "Unavailable",
                    format!("cannot start `{exe}`: {e}; install Python 3 or set ELOMAR_PYTHON"),
                ),
                String::new(),
            );
        }
    };
    if let Some(mut stdin) = child.stdin.take() {
        let _ = stdin.write_all(request.to_string().as_bytes());
    }
    // Read both pipes on threads so a chatty function cannot block.
    let mut stdout = child.stdout.take().expect("piped");
    let mut stderr = child.stderr.take().expect("piped");
    let out_thread = std::thread::spawn(move || {
        let mut s = String::new();
        let _ = stdout.read_to_string(&mut s);
        s
    });
    let err_thread = std::thread::spawn(move || {
        let mut s = String::new();
        let _ = stderr.read_to_string(&mut s);
        s
    });
    let deadline = Instant::now() + timeout;
    loop {
        match child.try_wait() {
            Ok(Some(_)) => break,
            Ok(None) if Instant::now() >= deadline => {
                let _ = child.kill();
                let _ = child.wait();
                return (
                    error(
                        "Timeout",
                        format!(
                            "`{module}.{function}` did not finish within {} ms",
                            timeout.as_millis()
                        ),
                    ),
                    err_thread.join().unwrap_or_default(),
                );
            }
            Ok(None) => std::thread::sleep(Duration::from_millis(5)),
            Err(e) => return (error("Unavailable", e.to_string()), String::new()),
        }
    }
    let out = out_thread.join().unwrap_or_default();
    // Windows Python writes `\r\n`; programs see `\n` everywhere.
    let printed = err_thread.join().unwrap_or_default().replace("\r\n", "\n");
    let reply: Json = match serde_json::from_str(&out) {
        Ok(j) => j,
        Err(_) => {
            let detail = printed.lines().last().unwrap_or("no output").to_string();
            return (
                error(
                    "Unavailable",
                    format!("Python did not run the call: {detail}"),
                ),
                String::new(),
            );
        }
    };
    if let Some(kind) = reply.get("error").and_then(Json::as_str) {
        let message = reply["message"].as_str().unwrap_or_default();
        return (error("Exception", format!("{kind}: {message}")), printed);
    }
    let schema = schema_of(&type_args[0], unit_enums);
    let result = &reply["ok"];
    let value = match schema.validate(result) {
        Ok(()) => Value::Result(Ok(Rc::new(from_json(&schema, result)))),
        Err(problem) => error(
            "TypeMismatch",
            format!(
                "`{module}.{function}` returned a value that does not match `{}`: {problem}",
                schema.describe()
            ),
        ),
    };
    (value, printed)
}
