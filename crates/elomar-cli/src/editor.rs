//! `elomar check --format json`: diagnostics for editors.
//!
//! Prints one JSON object to standard output:
//!
//! ```json
//! { "diagnostics": [ { "file": "/abs/path/main.elo", "severity": "error",
//!   "code": "E2001", "message": "...", "hint": "...",
//!   "range": { "start": { "line": 2, "character": 17 },
//!              "end": { "line": 2, "character": 21 } },
//!   "related": [ { "file": "...", "range": { ... }, "message": "..." } ] } ] }
//! ```
//!
//! Lines and characters are 0-based, and characters count UTF-16 units, as
//! in the Language Server Protocol. `main` is not required, so a module file
//! can be checked on its own.

use std::path::Path;
use std::process::ExitCode;

use elomar_diagnostics::{Diagnostic, Lang, Severity, SourceMap, Span, Text};
use serde_json::{Value, json};

pub(crate) fn check_json(path: &Path, from_stdin: bool, lang: Lang) -> ExitCode {
    let text = if from_stdin {
        let mut buf = String::new();
        if let Err(e) = std::io::Read::read_to_string(&mut std::io::stdin(), &mut buf) {
            return fail(&format!("cannot read standard input: {e}"));
        }
        Some(buf)
    } else {
        None
    };
    let root = crate::project_root(path);

    let ai = match crate::load_ai_runtime(path) {
        Ok(ai) => ai,
        Err((config, message)) => {
            // Report a broken elomar.toml on its first line.
            let out = json!({ "diagnostics": [{
                "file": config.display().to_string(),
                "severity": "error",
                "code": "config",
                "message": message,
                "hint": Value::Null,
                "range": range(0, 0, 0, 1),
                "related": [],
            }]});
            println!("{out}");
            return ExitCode::from(1);
        }
    };

    let loaded = match elomar_loader::load_with(path, &root, text) {
        Ok(l) => l,
        Err(e) => return fail(&format!("cannot read `{}`: {e}", path.display())),
    };
    let mut program = loaded.program;
    let mut diags = loaded.diagnostics;
    if diags.iter().all(|d| !d.is_error()) {
        diags.extend(elomar_types::check(
            &mut program,
            elomar_types::CheckOptions {
                require_main: false,
                cloud_models: ai.cloud_models,
            },
        ));
    }

    let items: Vec<Value> = diags
        .iter()
        .map(|d| diagnostic_json(d, &loaded.sources, &loaded.paths, lang))
        .collect();
    println!("{}", json!({ "diagnostics": items }));
    if diags.iter().any(Diagnostic::is_error) {
        ExitCode::from(1)
    } else {
        ExitCode::SUCCESS
    }
}

fn fail(message: &str) -> ExitCode {
    println!("{}", json!({ "diagnostics": [], "error": message }));
    ExitCode::from(2)
}

fn range(l1: usize, c1: usize, l2: usize, c2: usize) -> Value {
    json!({
        "start": { "line": l1, "character": c1 },
        "end": { "line": l2, "character": c2 },
    })
}

/// The file path and editor range of a global span.
fn locate(span: Span, sources: &SourceMap, paths: &[std::path::PathBuf]) -> (String, Value) {
    let index = sources.index_at(span.start as usize);
    let (file, base) = sources.file_at(span.start as usize);
    let start = file.position_utf16(span.start as usize - base);
    let end = file.position_utf16(
        (span.end as usize)
            .saturating_sub(base)
            .max(span.start as usize - base),
    );
    let path = paths
        .get(index)
        .map_or_else(|| file.name.clone(), |p| p.display().to_string());
    (path, range(start.0, start.1, end.0, end.1))
}

fn text_in(t: &Text, lang: Lang) -> String {
    t.get(lang).to_string()
}

fn diagnostic_json(
    d: &Diagnostic,
    sources: &SourceMap,
    paths: &[std::path::PathBuf],
    lang: Lang,
) -> Value {
    let (file, range) = locate(d.label.span, sources, paths);
    let related: Vec<Value> = d
        .notes
        .iter()
        .map(|n| {
            let (file, range) = locate(n.span, sources, paths);
            json!({
                "file": file,
                "range": range,
                "message": n.text.as_ref().map(|t| text_in(t, lang)).unwrap_or_default(),
            })
        })
        .collect();
    let mut message = text_in(&d.message, lang);
    if let Some(label) = &d.label.text {
        message.push_str(&format!(" ({})", text_in(label, lang)));
    }
    json!({
        "file": file,
        "severity": match d.severity {
            Severity::Error => "error",
            Severity::Warning => "warning",
        },
        "code": d.code,
        "message": message,
        "hint": d.hint.as_ref().map(|h| text_in(h, lang)),
        "range": range,
        "related": related,
    })
}
