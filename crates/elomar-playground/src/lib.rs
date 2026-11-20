//! Elomar-Lang in the browser: checks and runs one program and returns its
//! output, for the playground page (`site/playground/`).
//!
//! The page talks to the WebAssembly module through three functions:
//! `elomar_alloc(len)` gives a buffer for the request, `elomar_run(ptr, len)`
//! takes a JSON request `{ "source", "lang", "inputs" }` and returns a
//! pointer to a 4-byte little-endian length followed by a JSON answer
//! `{ "ok", "output", "errors" }`, and `elomar_free(ptr, len)` releases the
//! request buffer. Nothing touches files or the network: `std.fs` is denied,
//! and `std.db` and `std.http` are not built in. The model `mock` answers
//! AI calls offline.

use std::cell::RefCell;

use elomar_ai_core::AiRuntime;
use elomar_ai_provider_mock::MockProvider;
use elomar_diagnostics::{Lang, RenderOptions, SourceFile, render};
use elomar_interpreter::{BufferHost, run_with};
use elomar_policy::FsPolicy;
use serde_json::{Value, json};

/// Checks and runs `source`. `inputs` answer `input()` in order.
pub fn run_program(source: &str, lang: Lang, inputs: Vec<String>) -> Value {
    let file = SourceFile::new("main.elo", source);
    let opts = RenderOptions { lang, color: false };
    let (mut program, mut diags) = elomar_parser::parse(source);
    if diags.iter().all(|d| !d.is_error()) {
        diags.extend(elomar_types::check(
            &mut program,
            elomar_types::CheckOptions {
                require_main: true,
                ..Default::default()
            },
        ));
    }
    diags.sort_by_key(|d| (d.label.span.start, !d.is_error()));
    let rendered: String = diags.iter().map(|d| render(d, &file, opts)).collect();
    if diags.iter().any(|d| d.is_error()) {
        return json!({ "ok": false, "output": "", "errors": rendered });
    }
    let mut host = BufferHost {
        inputs,
        ..Default::default()
    };
    let mut ai = AiRuntime::default();
    ai.register(
        "mock",
        Box::new(MockProvider::new(Vec::new()).expect("no rules to validate")),
    );
    let result = run_with(&program, &mut host, &mut ai, FsPolicy::deny_all("."));
    match result {
        Ok(()) => json!({ "ok": true, "output": host.output, "errors": rendered }),
        Err(e) if e.exit.is_some() => {
            json!({ "ok": e.exit == Some(0), "output": host.output, "errors": rendered })
        }
        Err(e) => {
            let error = render(&e.to_diagnostic(), &file, opts);
            json!({ "ok": false, "output": host.output, "errors": rendered + &error })
        }
    }
}

/// Runs a JSON request (see the crate docs) and returns the JSON answer.
pub fn run_json(request: &str) -> String {
    let req: Value = serde_json::from_str(request).unwrap_or(Value::Null);
    let source = req["source"].as_str().unwrap_or("");
    let lang = match req["lang"].as_str() {
        Some("ar") => Lang::Ar,
        _ => Lang::En,
    };
    let inputs = req["inputs"]
        .as_array()
        .map(|a| {
            a.iter()
                .filter_map(|v| v.as_str().map(str::to_string))
                .collect()
        })
        .unwrap_or_default();
    run_program(source, lang, inputs).to_string()
}

thread_local! {
    /// The last answer, kept alive until the next call so the page can read it.
    static ANSWER: RefCell<Vec<u8>> = const { RefCell::new(Vec::new()) };
}

/// A buffer of `len` bytes for the page to write a request into.
#[unsafe(no_mangle)]
pub extern "C" fn elomar_alloc(len: usize) -> *mut u8 {
    let mut buf = Vec::<u8>::with_capacity(len.max(1));
    let ptr = buf.as_mut_ptr();
    std::mem::forget(buf);
    ptr
}

/// Frees a buffer from [`elomar_alloc`].
///
/// # Safety
/// `ptr` and `len` must come from one `elomar_alloc(len)` call, freed once.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn elomar_free(ptr: *mut u8, len: usize) {
    // SAFETY: the caller guarantees `ptr` came from `elomar_alloc(len)`.
    drop(unsafe { Vec::from_raw_parts(ptr, 0, len.max(1)) });
}

/// Runs the UTF-8 JSON request at `ptr..ptr+len`. Returns a pointer to the
/// answer's length (u32, little-endian) followed by its UTF-8 bytes.
///
/// # Safety
/// `ptr..ptr+len` must be readable memory holding the request.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn elomar_run(ptr: *const u8, len: usize) -> *const u8 {
    // SAFETY: the caller guarantees the range is valid for reads.
    let bytes = unsafe { std::slice::from_raw_parts(ptr, len) };
    let request = String::from_utf8_lossy(bytes);
    let answer = run_json(&request);
    ANSWER.with(|cell| {
        let mut buf = cell.borrow_mut();
        buf.clear();
        buf.extend_from_slice(&(answer.len() as u32).to_le_bytes());
        buf.extend_from_slice(answer.as_bytes());
        buf.as_ptr()
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn run(source: &str, lang: &str, inputs: &[&str]) -> Value {
        let req = json!({ "source": source, "lang": lang, "inputs": inputs });
        serde_json::from_str(&run_json(&req.to_string())).unwrap()
    }

    #[test]
    fn runs_programs() {
        let r = run(
            "fn main() {\n    let name = input(\"name? \")\n    print(\"Hi {name}\")\n}",
            "en",
            &["Lina"],
        );
        assert_eq!(r["ok"], true);
        assert_eq!(r["output"], "name? Hi Lina\n");
        assert_eq!(r["errors"], "");
    }

    #[test]
    fn reports_errors_in_the_chosen_language() {
        let r = run("fn main() {\n    print(1 + \"a\")\n}", "ar", &[]);
        assert_eq!(r["ok"], false);
        let errors = r["errors"].as_str().unwrap();
        assert!(
            errors.contains("E2005") && errors.contains("لا يمكن"),
            "{errors}"
        );
        let r = run("fn main() {\n    print(1 div 0)\n}", "en", &[]);
        assert!(r["errors"].as_str().unwrap().contains("R0001"));
    }

    #[test]
    fn sandboxed_but_useful() {
        // (SQLite and HTTP are left out only in the WebAssembly build; when
        // the whole workspace is tested, the CLI turns them on.)
        let src = "use std.ai\nuse std.fs\nasync fn main() {\n    print(fs.read_text(\"x\").is_err())\n    print(await ai.infer<String>(\"mock\", \"Say hi\", \"x\"))\n}";
        let r = run(src, "en", &[]);
        assert_eq!(r["ok"], true, "{r}");
        assert!(r["output"].as_str().unwrap().starts_with("true\nOk("));
    }
}
