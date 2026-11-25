//! `elomar new <name>`: creates a project that runs and passes its tests.

use std::path::Path;
use std::process::ExitCode;

use crate::Reporter;

const MAIN: &str = r#"// The program starts in `main`. Run it with `elomar run src/main.elo`.
use app.greeting

fn main() {
    print(greeting.greet("world"))
}
"#;

const GREETING: &str = r#"// A module: `use app.greeting` loads this file, and its `pub` items are
// used from other files as `greeting.greet(...)`.

pub fn greet(name: String) -> String {
    if name.is_empty() {
        return "Hello!"
    }
    return "Hello, " + name + "!"
}
"#;

const TEST: &str = r#"// Run with `elomar test`. Every `fn test_...()` in tests/ is a test.
use app.greeting

fn test_greets_by_name() {
    assert_eq(greeting.greet("Lina"), "Hello, Lina!")
}

fn test_empty_name() {
    assert_eq(greeting.greet(""), "Hello!")
}
"#;

fn config(name: &str) -> String {
    format!(
        r#"[package]
name = "{name}"
version = "0.1.0"
entry = "src/main.elo"

# Everything is denied unless listed here (spec §16.3).
[permissions]
filesystem_read = []
filesystem_write = []
network_hosts = []
# `std.py` runs Python, which is not sandboxed; enable only if needed.
python = false

# AI models for `use std.ai`. The built-in model "mock" needs no setup.
# [ai.models."assistant"]
# provider = "mock"
"#
    )
}

/// A folder name that is safe to create and to put in `elomar.toml`.
fn valid_name(name: &str) -> bool {
    !name.is_empty()
        && name != "."
        && name != ".."
        && !name.contains(['/', '\\', ':', '"', '*', '?', '<', '>', '|'])
        && !name.starts_with('.')
}

pub(crate) fn new_project(name: &str, ai: Option<&str>, reporter: &Reporter) -> ExitCode {
    if let Some(provider) = ai
        && !crate::scaffold_ai::PROVIDERS.contains(&provider)
    {
        let list = crate::scaffold_ai::PROVIDERS.join(", ");
        reporter.say(
            &format!("error: `--ai {provider}` is not known; use one of: {list}"),
            &format!("خطأ: `--ai {provider}` غير معروف؛ استخدم أحد: {list}"),
        );
        return ExitCode::from(2);
    }
    if !valid_name(name) {
        reporter.say(
            &format!(
                "error: `{name}` is not a valid project name; use letters, digits, `-` or `_`"
            ),
            &format!("خطأ: `{name}` ليس اسم مشروع صالحًا؛ استخدم الحروف والأرقام و`-` و`_`"),
        );
        return ExitCode::from(2);
    }
    let dir = Path::new(name);
    let not_empty = std::fs::read_dir(dir).is_ok_and(|mut entries| entries.next().is_some());
    if dir.is_file() || not_empty {
        reporter.say(
            &format!("error: `{name}` already exists and is not empty"),
            &format!("خطأ: `{name}` موجود مسبقًا وليس فارغًا"),
        );
        return ExitCode::from(2);
    }
    let files = match ai {
        Some(provider) => crate::scaffold_ai::files(name, provider),
        None => vec![
            ("elomar.toml", config(name)),
            ("src/main.elo", MAIN.to_string()),
            ("src/greeting.elo", GREETING.to_string()),
            ("tests/greeting_test.elo", TEST.to_string()),
        ],
    };
    for (path, text) in &files {
        let p = dir.join(path);
        let written = p
            .parent()
            .map_or(Ok(()), std::fs::create_dir_all)
            .and_then(|()| std::fs::write(&p, text));
        if let Err(e) = written {
            reporter.say(
                &format!("error: cannot write `{}`: {e}", p.display()),
                &format!("خطأ: تعذّرت كتابة `{}`: {e}", p.display()),
            );
            return ExitCode::from(2);
        }
    }
    let list: String = files
        .iter()
        .map(|(p, _)| format!("  {name}/{p}\n"))
        .collect();
    let (setup_en, setup_ar) = ai.map_or(("", ""), crate::scaffold_ai::before_running);
    let doctor = if ai.is_some() {
        "  elomar doctor\n"
    } else {
        ""
    };
    reporter.out(
        &format!(
            "Created project `{name}`:\n{list}\nNext:\n  cd {name}\n{setup_en}{doctor}  elomar run src/main.elo\n  elomar test"
        ),
        &format!(
            "أُنشئ المشروع `{name}`:\n{list}\nالخطوات التالية:\n  cd {name}\n{setup_ar}{doctor}  elomar run src/main.elo\n  elomar test"
        ),
    );
    ExitCode::SUCCESS
}
