//! `elomar test`: runs the `test_*` functions in the project's `tests/`
//! folder (spec §20.1).
//!
//! Every `.elo` file under `tests/` is a test file. Each top-level function
//! whose name starts with `test_` and that takes no parameters is a test. A
//! test fails if it hits a runtime error (such as a failed `assert` or
//! `assert_eq`) or returns `Err(...)`. Tests run one by one, each in a fresh
//! interpreter with its own AI budget; their output is shown only when they
//! fail. Test files reach the project's code with `use app.<module>`.

use std::path::{Path, PathBuf};
use std::process::ExitCode;

use elomar_ast::Item;
use elomar_diagnostics::Lang;
use elomar_interpreter::{BufferHost, RuntimeError, Value};

use crate::{Reporter, find_config, load_ai_runtime, project_root};

enum Outcome {
    Passed,
    /// The test returned `Err(...)`; the text is the error value.
    ReturnedErr(String),
    Crashed(RuntimeError),
}

/// Test files under `<project>/tests`, or just `path` if it is a file.
fn test_files(path: &Path) -> (PathBuf, Vec<PathBuf>) {
    if path.is_file() {
        return (project_root(path), vec![path.to_path_buf()]);
    }
    let root = find_config(&path.join("_.elo"))
        .and_then(|c| c.parent().map(Path::to_path_buf))
        .unwrap_or_else(|| path.to_path_buf());
    let mut files = Vec::new();
    collect(&root.join("tests"), &mut files);
    files.sort();
    (root, files)
}

fn collect(dir: &Path, out: &mut Vec<PathBuf>) {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        let p = entry.path();
        if p.is_dir() {
            collect(&p, out);
        } else if p.extension().is_some_and(|e| e == "elo") {
            out.push(p);
        }
    }
}

/// A short path for messages: relative to `root` when possible, with `/`.
fn shown(path: &Path, root: &Path) -> String {
    path.strip_prefix(root)
        .unwrap_or(path)
        .display()
        .to_string()
        .replace('\\', "/")
}

pub(crate) fn run_tests(path: &Path, filter: Option<&str>, reporter: &Reporter) -> ExitCode {
    let ar = reporter.opts.lang == Lang::Ar;
    let (root, files) = test_files(path);
    if files.is_empty() {
        reporter.out(
            "no test files found: put files with `fn test_...()` functions in `tests/`",
            "لا توجد ملفات اختبار: ضع ملفات فيها دوال `fn test_...()` داخل المجلد `tests/`",
        );
        return ExitCode::SUCCESS;
    }

    let (mut passed, mut failed, mut filtered_out, mut broken_files) = (0, 0, 0, 0);
    for file in &files {
        let name = shown(file, &root);
        let Ok(ai) = load_ai_runtime(file) else {
            reporter.say(
                "error: invalid elomar.toml; run `elomar check` for details",
                "خطأ: الملف elomar.toml غير صالح؛ شغّل `elomar check` للتفاصيل",
            );
            return ExitCode::from(2);
        };
        // Paths in messages: relative to the current folder when possible.
        let cwd = std::env::current_dir().unwrap_or_default();
        let entry = PathBuf::from(
            file.strip_prefix(&cwd)
                .unwrap_or(file)
                .display()
                .to_string()
                .replace('\\', "/"),
        );
        crate::packages::fetch_for(file, reporter);
        let loaded = match elomar_loader::load(&entry, &project_root(file)) {
            Ok(l) => l,
            Err(e) => {
                reporter.say(
                    &format!("error: cannot read `{name}`: {e}"),
                    &format!("خطأ: تعذّرت قراءة `{name}`: {e}"),
                );
                broken_files += 1;
                continue;
            }
        };
        let sources = loaded.sources;
        let mut program = loaded.program;
        let mut diags = loaded.diagnostics;
        if diags.iter().all(|d| !d.is_error()) {
            diags.extend(elomar_types::check(
                &mut program,
                elomar_types::CheckOptions {
                    require_main: false,
                    cloud_models: ai.cloud_models.clone(),
                },
            ));
        }
        for d in &diags {
            reporter.emit(d, &sources);
        }
        if diags.iter().any(|d| d.is_error()) {
            reporter.out(
                &format!("{name}: has errors; its tests were not run"),
                &format!("{name}: فيه أخطاء؛ لم تُشغَّل اختباراته"),
            );
            broken_files += 1;
            continue;
        }

        let tests: Vec<String> = program
            .items
            .iter()
            .filter_map(|item| match item {
                Item::Fn(f)
                    if (f.name.name.starts_with("test_") || f.name.name.starts_with("اختبار_"))
                        && !f.name.name.contains('.')
                        && f.params.is_empty() =>
                {
                    Some(f.name.name.clone())
                }
                _ => None,
            })
            .collect();
        let selected: Vec<&String> = tests
            .iter()
            .filter(|t| filter.is_none_or(|f| t.contains(f)))
            .collect();
        filtered_out += tests.len() - selected.len();
        if selected.is_empty() {
            continue;
        }
        reporter.out(
            &format!("\nrunning {} test(s) from {name}", selected.len()),
            &format!("\nتشغيل {} من الاختبارات من {name}", selected.len()),
        );

        let mut failures = Vec::new();
        for test in selected {
            let outcome = run_one(&program, test, file);
            let ok = matches!(outcome.0, Outcome::Passed);
            let status = match (ok, ar) {
                (true, false) => "ok",
                (true, true) => "نجح",
                (false, false) => "FAILED",
                (false, true) => "فشل",
            };
            reporter.out(
                &format!("test {test} ... {status}"),
                &format!("اختبار {test} ... {status}"),
            );
            if ok {
                passed += 1;
            } else {
                failed += 1;
                failures.push((test.clone(), outcome));
            }
        }
        for (test, (outcome, output)) in failures {
            reporter.out(
                &format!("\n---- {test} ----"),
                &format!("\n---- {test} ----"),
            );
            if !output.is_empty() {
                print!("{output}");
            }
            match outcome {
                Outcome::Crashed(err) => reporter.emit_out(&err.to_diagnostic(), &sources),
                Outcome::ReturnedErr(value) => reporter.out(
                    &format!("the test returned {value}"),
                    &format!("أرجع الاختبار {value}"),
                ),
                Outcome::Passed => {}
            }
        }
    }

    let all_ok = failed == 0 && broken_files == 0;
    let (word_en, word_ar) = if all_ok {
        ("ok", "نجاح")
    } else {
        ("FAILED", "فشل")
    };
    let mut en = format!("\ntest result: {word_en}. {passed} passed; {failed} failed");
    let mut ar_text = format!("\nنتيجة الاختبارات: {word_ar}. نجح {passed}، فشل {failed}");
    if filtered_out > 0 {
        en.push_str(&format!("; {filtered_out} filtered out"));
        ar_text.push_str(&format!("، استُبعد {filtered_out} بالمرشِّح"));
    }
    if broken_files > 0 {
        en.push_str(&format!("; {broken_files} file(s) with errors"));
        ar_text.push_str(&format!("، {broken_files} من الملفات فيها أخطاء"));
    }
    reporter.out(&en, &ar_text);
    if all_ok {
        ExitCode::SUCCESS
    } else {
        ExitCode::from(1)
    }
}

/// Runs one test in a fresh interpreter; returns the outcome and what it printed.
fn run_one(program: &elomar_ast::Program, test: &str, file: &Path) -> (Outcome, String) {
    // A fresh runtime per test, so call budgets do not carry over.
    let Ok(ai) = load_ai_runtime(file) else {
        unreachable!("elomar.toml was valid a moment ago");
    };
    let (mut runtime, fs) = (ai.runtime, ai.fs);
    elomar_interpreter::with_large_stack(|| {
        let mut host = BufferHost::default();
        let outcome =
            match elomar_interpreter::run_function(program, test, &mut host, &mut runtime, fs) {
                Ok(v @ Value::Result(Err(_))) => Outcome::ReturnedErr(v.to_string()),
                Ok(_) => Outcome::Passed,
                Err(err) => Outcome::Crashed(err),
            };
        (outcome, host.output)
    })
}
