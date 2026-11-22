//! `std.fs` and `std.json` at run time. Every failure is an `Err` value
//! (`FsError` / `JsonError`) the program can `match` on, never a crash.

use std::collections::HashMap;
use std::io::Write as _;
use std::rc::Rc;

use elomar_ast::TypeExpr;
use elomar_policy::{Access, FsPolicy};

use crate::Value;
use crate::ai::{from_json, schema_of, to_json};

fn error_value(enum_name: &str, kind: &str, message: String) -> Value {
    Value::Result(Err(Rc::new(Value::Enum {
        enum_name: enum_name.into(),
        variant: kind.into(),
        fields: Rc::new(vec![Value::str(message)]),
    })))
}

fn ok(v: Value) -> Value {
    Value::Result(Ok(Rc::new(v)))
}

fn fs_error(kind: &str, message: String) -> Value {
    error_value("FsError", kind, message)
}

/// Uses fixed wording for common cases, so messages are the same on every
/// operating system; other errors keep the system's text.
fn io_error(path: &str, verb: &str, e: &std::io::Error) -> Value {
    let (kind, reason) = match e.kind() {
        std::io::ErrorKind::NotFound => ("NotFound", "no such file or folder".to_string()),
        std::io::ErrorKind::PermissionDenied => (
            "PermissionDenied",
            "the operating system refused access".to_string(),
        ),
        std::io::ErrorKind::InvalidData => ("InvalidData", e.to_string()),
        _ => ("Other", e.to_string()),
    };
    fs_error(kind, format!("cannot {verb} `{path}`: {reason}"))
}

fn text_arg(v: &Value) -> &str {
    match v {
        Value::Str(s) => s,
        _ => "",
    }
}

/// Removes a UTF-8 byte-order mark, which some Windows editors add.
fn strip_bom(s: &str) -> &str {
    s.strip_prefix('\u{feff}').unwrap_or(s)
}

pub(crate) fn fs_call(policy: &FsPolicy, func: &str, args: &[Value]) -> Value {
    let path = text_arg(&args[0]);
    let access = match func {
        "write_text" | "append_text" | "create_dir" | "remove" | "remove_dir" | "rename" => {
            Access::Write
        }
        _ => Access::Read,
    };
    let resolved = match policy.check(path, access) {
        Ok(p) => p,
        Err(message) => return fs_error("PolicyDenied", message),
    };
    match func {
        "read_text" => match std::fs::read(&resolved) {
            Ok(bytes) => match String::from_utf8(bytes) {
                Ok(text) => ok(Value::str(strip_bom(&text))),
                Err(_) => fs_error("InvalidData", format!("`{path}` is not UTF-8 text")),
            },
            Err(e) => io_error(path, "read", &e),
        },
        "write_text" => match std::fs::write(&resolved, text_arg(&args[1])) {
            Ok(()) => ok(Value::Unit),
            Err(e) => io_error(path, "write", &e),
        },
        "append_text" => {
            let result = std::fs::OpenOptions::new()
                .create(true)
                .append(true)
                .open(&resolved)
                .and_then(|mut f| f.write_all(text_arg(&args[1]).as_bytes()));
            match result {
                Ok(()) => ok(Value::Unit),
                Err(e) => io_error(path, "append to", &e),
            }
        }
        "exists" => ok(Value::Bool(resolved.exists())),
        "is_dir" => ok(Value::Bool(resolved.is_dir())),
        "size" => match std::fs::metadata(&resolved) {
            Ok(m) if m.is_dir() => fs_error("Other", format!("`{path}` is a folder, not a file")),
            Ok(m) => ok(Value::Int(i64::try_from(m.len()).unwrap_or(i64::MAX))),
            Err(e) => io_error(path, "read", &e),
        },
        "create_dir" => match std::fs::create_dir_all(&resolved) {
            Ok(()) => ok(Value::Unit),
            Err(e) => io_error(path, "create the folder", &e),
        },
        "remove" => {
            if resolved.is_dir() {
                return fs_error(
                    "Other",
                    format!("`{path}` is a folder; use fs.remove_dir to delete folders"),
                );
            }
            match std::fs::remove_file(&resolved) {
                Ok(()) => ok(Value::Unit),
                Err(e) => io_error(path, "delete", &e),
            }
        }
        "remove_dir" => {
            if policy.is_root(path) {
                return fs_error(
                    "PolicyDenied",
                    format!(
                        "`{path}` is listed under `[permissions]` itself; only what is inside it may be deleted"
                    ),
                );
            }
            if resolved.exists() && !resolved.is_dir() {
                return fs_error(
                    "Other",
                    format!("`{path}` is a file; use fs.remove to delete files"),
                );
            }
            match std::fs::remove_dir_all(&resolved) {
                Ok(()) => ok(Value::Unit),
                Err(e) => io_error(path, "delete the folder", &e),
            }
        }
        "rename" | "copy" => {
            let to = text_arg(&args[1]);
            let target = match policy.check(to, Access::Write) {
                Ok(p) => p,
                Err(message) => return fs_error("PolicyDenied", message),
            };
            if func == "rename" {
                match std::fs::rename(&resolved, &target) {
                    Ok(()) => ok(Value::Unit),
                    Err(e) => io_error(path, "rename", &e),
                }
            } else if resolved.is_dir() {
                fs_error(
                    "Other",
                    format!("`{path}` is a folder; fs.copy copies files"),
                )
            } else {
                match std::fs::copy(&resolved, &target) {
                    Ok(_) => ok(Value::Unit),
                    Err(e) => io_error(path, "copy", &e),
                }
            }
        }
        "list_dir" => match std::fs::read_dir(&resolved) {
            Ok(entries) => {
                let mut names: Vec<String> = entries
                    .filter_map(|e| e.ok())
                    .map(|e| e.file_name().to_string_lossy().into_owned())
                    .collect();
                names.sort();
                ok(Value::list(names.into_iter().map(Value::str).collect()))
            }
            Err(e) => io_error(path, "list", &e),
        },
        other => unreachable!("checked fs function `{other}`"),
    }
}

pub(crate) fn json_call(
    func: &str,
    type_args: &[TypeExpr],
    args: &[Value],
    unit_enums: &HashMap<String, Vec<String>>,
) -> Value {
    match func {
        "parse" => {
            let schema = schema_of(&type_args[0], unit_enums);
            let text = strip_bom(text_arg(&args[0]));
            let json: serde_json::Value = match serde_json::from_str(text) {
                Ok(j) => j,
                Err(e) => {
                    return error_value("JsonError", "InvalidJson", format!("invalid JSON: {e}"));
                }
            };
            match schema.validate(&json) {
                Ok(()) => ok(from_json(&schema, &json)),
                Err(problem) => error_value(
                    "JsonError",
                    "TypeMismatch",
                    format!("the JSON does not match `{}`: {problem}", schema.describe()),
                ),
            }
        }
        "stringify" => Value::str(to_json(&args[0]).to_string()),
        "pretty" => {
            Value::str(serde_json::to_string_pretty(&to_json(&args[0])).unwrap_or_default())
        }
        other => unreachable!("checked json function `{other}`"),
    }
}
