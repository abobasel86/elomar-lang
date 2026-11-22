//! `std.process.run`: starts a program listed in `[permissions] commands`
//! directly (no shell, so arguments are never interpreted as shell syntax),
//! in the project folder, and waits for it with a time limit.

use std::io::Read;
use std::path::Path;
use std::process::{Command, Stdio};
use std::rc::Rc;
use std::time::{Duration, Instant};

use elomar_ai_core::Permissions;

use crate::Value;

const DEFAULT_TIMEOUT_MS: i64 = 60_000;
/// Output beyond this is dropped, so a chatty program cannot exhaust memory.
const MAX_OUTPUT: u64 = 16 * 1024 * 1024;

fn error(kind: &str, message: String) -> Value {
    Value::Result(Err(Rc::new(Value::enum_value(
        "ProcessError",
        kind,
        vec![Value::str(message)],
    ))))
}

fn text(v: &Value) -> &str {
    match v {
        Value::Str(s) => s,
        _ => "",
    }
}

fn read_all(pipe: Option<impl Read + Send + 'static>) -> std::thread::JoinHandle<Vec<u8>> {
    std::thread::spawn(move || {
        let mut buf = Vec::new();
        if let Some(p) = pipe {
            let _ = p.take(MAX_OUTPUT).read_to_end(&mut buf);
        }
        buf
    })
}

pub(crate) fn run(permissions: &Permissions, base: &Path, args: &[Value]) -> Value {
    let command = text(&args[0]);
    let arguments: Vec<String> = match &args[1] {
        Value::List(items) => items.iter().map(|v| text(v).to_string()).collect(),
        _ => Vec::new(),
    };
    let timeout_ms = match args.get(2) {
        Some(Value::Duration(ms)) => (*ms).max(1),
        _ => DEFAULT_TIMEOUT_MS,
    };
    if !permissions.allows_command(command) {
        return error(
            "PolicyDenied",
            format!(
                "running `{command}` is not permitted; add \"{command}\" to `commands` under `[permissions]` in elomar.toml"
            ),
        );
    }
    let mut child = match Command::new(command)
        .args(&arguments)
        .current_dir(base)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
    {
        Ok(c) => c,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
            return error("NotFound", format!("program `{command}` was not found"));
        }
        Err(e) => return error("Failed", format!("cannot start `{command}`: {e}")),
    };
    let out = read_all(child.stdout.take());
    let err = read_all(child.stderr.take());
    let deadline = Instant::now() + Duration::from_millis(timeout_ms as u64);
    let status = loop {
        match child.try_wait() {
            Ok(Some(status)) => break status,
            Ok(None) if Instant::now() >= deadline => {
                let _ = child.kill();
                let _ = child.wait();
                return error(
                    "Timeout",
                    format!("`{command}` did not finish within {timeout_ms} ms and was stopped"),
                );
            }
            Ok(None) => std::thread::sleep(Duration::from_millis(5)),
            Err(e) => return error("Failed", format!("waiting for `{command}` failed: {e}")),
        }
    };
    let stdout = String::from_utf8_lossy(&out.join().unwrap_or_default()).into_owned();
    let stderr = String::from_utf8_lossy(&err.join().unwrap_or_default()).into_owned();
    // A program stopped by a signal has no code; report -1.
    let code = status.code().unwrap_or(-1);
    Value::Result(Ok(Rc::new(Value::Record(Rc::new(vec![
        (Rc::from("status"), Value::Int(code as i64)),
        (Rc::from("stdout"), Value::str(stdout)),
        (Rc::from("stderr"), Value::str(stderr)),
    ])))))
}
