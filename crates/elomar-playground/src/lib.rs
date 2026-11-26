//! Elomar-Lang in the browser: checks and runs one program and returns its
//! output, for the playground page (`site/playground/`).
//!
//! The page talks to the WebAssembly module through three functions:
//! `elomar_alloc(len)` gives a buffer for the request, `elomar_run(ptr, len)`
//! takes a JSON request `{ "source", "lang", "inputs" }` and returns a
//! pointer to a 4-byte little-endian length followed by a JSON answer
//! `{ "ok", "output", "errors" }`, and `elomar_free(ptr, len)` releases the
//! request buffer. A request with a `"check"` field checks an exercise
//! instead (see [`check_exercise`]). Nothing touches files or the network: `std.fs` is denied,
//! and `std.db` and `std.http` are not built in. The model `mock` answers
//! AI calls offline.

use std::cell::RefCell;

use elomar_ai_core::AiRuntime;
use elomar_ai_provider_mock::MockProvider;
use elomar_ast::{Item, Program};
use elomar_diagnostics::{Diagnostic, Lang, RenderOptions, SourceFile, render};
use elomar_interpreter::{BufferHost, run_limited};
use elomar_policy::FsPolicy;
use serde_json::{Value, json};

/// Parses and type-checks `source`. Returns the program, or the
/// diagnostics if there is an error; warnings are returned either way.
fn compile(source: &str) -> (Option<Program>, Vec<Diagnostic>) {
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
    let ok = diags.iter().all(|d| !d.is_error());
    (ok.then_some(program), diags)
}

/// What running a checked program gave: its output, and the runtime error
/// (rendered) if it failed.
struct Ran {
    output: String,
    error: Option<String>,
}

fn execute(
    program: &Program,
    file: &SourceFile,
    opts: RenderOptions,
    inputs: Vec<String>,
    limit_ms: i64,
) -> Ran {
    let mut host = BufferHost {
        inputs,
        ..Default::default()
    };
    let mut ai = AiRuntime::default();
    ai.register(
        "mock",
        Box::new(MockProvider::new(Vec::new()).expect("no rules to validate")),
    );
    // Stops cleanly, keeping the output so far, before the page's own
    // 10-second limit has to end the worker.
    let error = match run_limited(
        program,
        &mut host,
        &mut ai,
        FsPolicy::deny_all("."),
        limit_ms,
    ) {
        Ok(()) => None,
        Err(e) if e.exit == Some(0) => None,
        Err(e) if e.exit.is_some() => Some(String::new()),
        Err(e) => Some(render(&e.to_diagnostic(), file, opts)),
    };
    Ran {
        output: host.output,
        error,
    }
}

/// Checks and runs `source`. `inputs` answer `input()` in order.
pub fn run_program(source: &str, lang: Lang, inputs: Vec<String>) -> Value {
    run_program_limited(source, lang, inputs, 8_000)
}

/// Like [`run_program`], stopping the program after `limit_ms`.
pub fn run_program_limited(source: &str, lang: Lang, inputs: Vec<String>, limit_ms: i64) -> Value {
    let file = SourceFile::new("main.elo", source);
    let opts = RenderOptions { lang, color: false };
    let (program, diags) = compile(source);
    let rendered: String = diags.iter().map(|d| render(d, &file, opts)).collect();
    let Some(program) = program else {
        return json!({ "ok": false, "output": "", "errors": rendered });
    };
    let ran = execute(&program, &file, opts, inputs, limit_ms);
    let ok = ran.error.is_none();
    let errors = rendered + &ran.error.unwrap_or_default();
    json!({ "ok": ok, "output": ran.output, "errors": errors })
}

/// Output compared without trailing spaces and blank lines at the end.
fn normalize(text: &str) -> String {
    text.lines()
        .map(str::trim_end)
        .collect::<Vec<_>>()
        .join("\n")
        .trim_end()
        .to_string()
}

/// Checks a learner's answer to an exercise against its cases.
///
/// - `mode` `"function"`: `source` defines functions and no `main`. Each
///   case is an expression such as `total([1, 2])`; the checker adds a
///   `main` that prints it and compares the output with `expect`.
/// - `mode` `"program"`: `source` is a whole program. Each case gives the
///   `inputs` for `input()` and the whole `expect`ed output.
///
/// The answer is `{ "ok", "errors", "note", "cases": [{ "pass", "got",
/// "error" }] }`. `errors` holds compile errors (then no case runs);
/// `note` is `"main"` when a function exercise defines `main`, or
/// `"signature"` when the checker's calls do not compile although the
/// learner's code alone does: the page explains both.
pub fn check_exercise(source: &str, lang: Lang, mode: &str, cases: &[Value]) -> Value {
    let opts = RenderOptions { lang, color: false };
    let function = mode == "function";
    let program_for = |case: &Value| -> String {
        if function {
            let expr = case["expr"].as_str().unwrap_or("");
            format!("{source}\n\nfn main() {{\n    print({expr})\n}}\n")
        } else {
            source.to_string()
        }
    };
    let fail = |errors: String, note: &str| json!({ "ok": false, "errors": errors, "note": note, "cases": [] });

    if function {
        // The learner's code alone, so errors point at what they wrote.
        let (mut program, mut diags) = elomar_parser::parse(source);
        if diags.iter().all(|d| !d.is_error()) {
            diags.extend(elomar_types::check(&mut program, Default::default()));
        }
        let file = SourceFile::new("main.elo", source);
        let errors: String = diags
            .iter()
            .filter(|d| d.is_error())
            .map(|d| render(d, &file, opts))
            .collect();
        if !errors.is_empty() {
            return fail(errors, "");
        }
        if program
            .items
            .iter()
            .any(|item| matches!(item, Item::Fn(f) if f.name.name == "main" || f.name.name == elomar_ast::MAIN_AR))
        {
            return fail(String::new(), "main");
        }
    }

    let mut results = Vec::new();
    let mut all = true;
    for case in cases {
        let full = program_for(case);
        let file = SourceFile::new("main.elo", &full);
        let (program, diags) = compile(&full);
        let Some(program) = program else {
            let errors: String = diags
                .iter()
                .filter(|d| d.is_error())
                .map(|d| render(d, &file, opts))
                .collect();
            return fail(errors, if function { "signature" } else { "" });
        };
        let inputs = case["inputs"]
            .as_array()
            .map(|a| {
                a.iter()
                    .filter_map(|v| v.as_str().map(str::to_string))
                    .collect()
            })
            .unwrap_or_default();
        let ran = execute(&program, &file, opts, inputs, 3_000);
        let expect = normalize(case["expect"].as_str().unwrap_or(""));
        let got = normalize(&ran.output);
        let pass = ran.error.is_none() && got == expect;
        all &= pass;
        results.push(json!({ "pass": pass, "got": got, "error": ran.error }));
    }
    json!({ "ok": all, "errors": "", "note": "", "cases": results })
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
    // The keyword switch: {"keywords": "ar", "sources": [...]} gives the
    // same sources with every keyword in Arabic (or English with "en").
    if let Some(to) = req["keywords"].as_str() {
        let to = match to {
            "ar" => elomar_fmt::Keywords::Arabic,
            _ => elomar_fmt::Keywords::English,
        };
        let sources: Vec<Value> = req["sources"]
            .as_array()
            .map(|a| {
                a.iter()
                    .map(|s| json!(elomar_fmt::convert_keywords(s.as_str().unwrap_or(""), to)))
                    .collect()
            })
            .unwrap_or_default();
        return json!({ "ok": true, "sources": sources }).to_string();
    }
    if let Some(check) = req.get("check") {
        let mode = check["mode"].as_str().unwrap_or("program");
        let cases = check["cases"].as_array().cloned().unwrap_or_default();
        return check_exercise(source, lang, mode, &cases).to_string();
    }
    let limit = req["limit_ms"].as_i64().unwrap_or(8_000);
    run_program_limited(source, lang, inputs, limit).to_string()
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

#[cfg(test)]
mod exercise_tests {
    use super::*;

    /// Every exercise on the website: its solution passes every case, and
    /// the starting code does not.
    #[test]
    fn site_exercises_have_working_solutions() {
        let data: Value =
            serde_json::from_str(include_str!("../../../site/_data/exercises.json")).unwrap();
        let mut count = 0;
        for (lesson, exercises) in data.as_object().unwrap() {
            for ex in exercises.as_array().unwrap() {
                let id = &ex["id"];
                let mode = ex["mode"].as_str().unwrap();
                let cases = ex["cases"].as_array().unwrap();
                for (key, lang) in [("en", Lang::En), ("ar", Lang::Ar)] {
                    assert!(ex["title"][key].is_string(), "{lesson} {id}: title.{key}");
                    assert!(ex["task"][key].is_string(), "{lesson} {id}: task.{key}");
                    assert_eq!(
                        ex["hints"]["en"].as_array().unwrap().len(),
                        ex["hints"][key].as_array().unwrap().len(),
                        "{lesson} {id}: hints.{key}"
                    );
                    let r = check_exercise(ex["solution"].as_str().unwrap(), lang, mode, cases);
                    assert_eq!(r["ok"], true, "{lesson} {id}: the solution fails: {r}");
                }
                let r = check_exercise(ex["starter"].as_str().unwrap(), Lang::En, mode, cases);
                assert_eq!(r["ok"], false, "{lesson} {id}: the starting code passes");
                count += 1;
            }
        }
        assert!(count >= 10, "only {count} exercises");
    }

    /// The website's keyword switch: every gallery program, with Arabic
    /// keywords, prints the same, and switching back gives it unchanged.
    #[test]
    fn keyword_switch_keeps_programs_working() {
        let gallery: Vec<Value> =
            serde_json::from_str(include_str!("../../../site/_data/gallery.json")).unwrap();
        let codes: Vec<&str> = gallery
            .iter()
            .map(|g| g["code"].as_str().unwrap())
            .collect();
        let ask = |to: &str, sources: &[&str]| -> Vec<String> {
            let answer: Value = serde_json::from_str(&run_json(
                &json!({ "keywords": to, "sources": sources }).to_string(),
            ))
            .unwrap();
            answer["sources"]
                .as_array()
                .unwrap()
                .iter()
                .map(|s| s.as_str().unwrap().to_string())
                .collect()
        };
        let ar = ask("ar", &codes);
        let ar_refs: Vec<&str> = ar.iter().map(String::as_str).collect();
        let en = ask("en", &ar_refs);
        for ((g, a), e) in gallery.iter().zip(&ar).zip(&en) {
            let key = g["key"].as_str().unwrap();
            assert!(a.contains("دالة"), "{key}");
            if key != "keywords" {
                assert_eq!(e, g["code"].as_str().unwrap(), "{key}");
            }
            if let Some(expected) = g["output"].as_str() {
                let input: Vec<String> = g["input"]
                    .as_str()
                    .map(|i| i.lines().map(str::to_string).collect())
                    .unwrap_or_default();
                let r = run_program_limited(a, Lang::En, input, 60_000);
                assert_eq!(r["output"], expected, "{key} with Arabic keywords");
            }
        }
    }

    /// Every program of the gallery under the playground (and in its
    /// Examples menu) runs in both languages and prints what the gallery
    /// says it prints.
    #[test]
    fn site_gallery_programs_run() {
        let data: Value =
            serde_json::from_str(include_str!("../../../site/_data/gallery.json")).unwrap();
        let programs = data.as_array().unwrap();
        let mut keys = std::collections::HashSet::new();
        for ex in programs {
            let key = ex["key"].as_str().unwrap();
            assert!(keys.insert(key), "{key}: two programs with this key");
            let inputs: Vec<&str> = ex["input"].as_str().map(|i| vec![i]).unwrap_or_default();
            for lang in ["en", "ar"] {
                assert!(ex["title"][lang].is_string(), "{key}: title.{lang}");
                assert!(ex["desc"][lang].is_string(), "{key}: desc.{lang}");
                let req = json!({ "source": ex["code"], "lang": lang, "inputs": inputs,
                                  "limit_ms": 60_000 });
                let r: Value = serde_json::from_str(&run_json(&req.to_string())).unwrap();
                // Outside the browser this build has no random source (the
                // page gives it one), so `crypto.uuid()` may stop here.
                let no_random = r["errors"].as_str().unwrap_or("").contains("R0006");
                assert!(r["ok"] == true || no_random, "{key} ({lang}): {r}");
                if let Some(expected) = ex["output"].as_str() {
                    assert_eq!(r["output"], expected, "{key} ({lang})");
                }
            }
        }
        assert!(keys.contains("hello"), "the playground starts with `hello`");
        assert!(programs.len() >= 10, "only {} programs", programs.len());
    }

    #[test]
    fn explains_wrong_answers() {
        let cases = [json!({ "expr": "double(2)", "expect": "4" })];
        let r = check_exercise(
            "fn double(n: Int) -> Int {\n    return n\n}",
            Lang::En,
            "function",
            &cases,
        );
        assert_eq!(r["ok"], false);
        assert_eq!(r["cases"][0]["got"], "2");
        let r = check_exercise(
            "fn double(n: Int) -> Int {\n    return n +\n}",
            Lang::En,
            "function",
            &cases,
        );
        assert!(r["errors"].as_str().unwrap().contains("main.elo:3"), "{r}");
        let r = check_exercise(
            "fn twice(n: Int) -> Int {\n    return n * 2\n}",
            Lang::En,
            "function",
            &cases,
        );
        assert_eq!(r["note"], "signature");
        let r = check_exercise(
            "fn double(n: Int) -> Int {\n    return n * 2\n}\nfn main() {\n}",
            Lang::En,
            "function",
            &cases,
        );
        assert_eq!(r["note"], "main");
        let r = check_exercise(
            "fn double(n: Int) -> Int {\n    return n div 0\n}",
            Lang::En,
            "function",
            &cases,
        );
        assert!(
            r["cases"][0]["error"].as_str().unwrap().contains("R0001"),
            "{r}"
        );
        let r = run_json(
            &json!({ "source": "fn double(n: Int) -> Int {\n    return n * 2\n}", "lang": "en",
                     "check": { "mode": "function", "cases": cases } })
            .to_string(),
        );
        assert!(r.contains("\"ok\":true"), "{r}");
    }
}

#[cfg(test)]
mod limit_tests {
    use super::*;

    #[test]
    fn endless_programs_stop_and_keep_their_output() {
        let src = "fn main() {\n    print(\"start\")\n    var i = 0\n    while true {\n        i += 1\n    }\n}";
        let r: Value = serde_json::from_str(&run_json(
            &json!({ "source": src, "lang": "en", "inputs": [], "limit_ms": 200 }).to_string(),
        ))
        .unwrap();
        assert_eq!(r["ok"], false);
        assert_eq!(r["output"], "start\n");
        assert!(r["errors"].as_str().unwrap().contains("R0013"), "{r}");
    }
}
