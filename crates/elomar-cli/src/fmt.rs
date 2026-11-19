//! `elomar fmt`: formats `.elo` files in place (see the `elomar-fmt` crate).

use std::io::Read as _;
use std::path::{Path, PathBuf};
use std::process::ExitCode;

use elomar_diagnostics::{SourceFile, SourceMap};
use elomar_fmt::FormatError;

use crate::Reporter;

/// Formats `src`, keeping Windows line endings if the file uses them.
fn format_text(src: &str) -> Result<String, FormatError> {
    let crlf = src.contains("\r\n");
    let out = elomar_fmt::format(&src.replace("\r\n", "\n"))?;
    Ok(if crlf { out.replace('\n', "\r\n") } else { out })
}

/// `.elo` files under the given paths, skipping `target/` and hidden folders.
fn collect(paths: &[PathBuf]) -> Vec<PathBuf> {
    fn walk(dir: &Path, out: &mut Vec<PathBuf>) {
        let Ok(entries) = std::fs::read_dir(dir) else {
            return;
        };
        for entry in entries.flatten() {
            let p = entry.path();
            let name = entry.file_name().to_string_lossy().into_owned();
            if p.is_dir() {
                if name != "target" && !name.starts_with('.') {
                    walk(&p, out);
                }
            } else if p.extension().is_some_and(|e| e == "elo") {
                out.push(p);
            }
        }
    }
    let mut files = Vec::new();
    for p in paths {
        if p.is_dir() {
            walk(p, &mut files);
        } else {
            files.push(p.clone());
        }
    }
    files.sort();
    files
}

fn shown(path: &Path) -> String {
    path.display().to_string().replace('\\', "/")
}

/// Reports why a file could not be formatted.
fn report_error(reporter: &Reporter, name: &str, src: &str, err: FormatError) {
    match err {
        FormatError::Lex(diags) => {
            let map = SourceMap::single(SourceFile::new(name, src.replace("\r\n", "\n")));
            for d in &diags {
                reporter.emit(d, &map);
            }
            reporter.say(
                &format!("error: `{name}` was not formatted because it has errors"),
                &format!("خطأ: لم يُنسَّق `{name}` لأن فيه أخطاء"),
            );
        }
        FormatError::Unsafe => reporter.say(
            &format!("error: formatting `{name}` would change its meaning; it was left unchanged (please report this)"),
            &format!("خطأ: تنسيق `{name}` كان سيغيّر معناه؛ تُرك دون تغيير (يرجى الإبلاغ عن ذلك)"),
        ),
    }
}

pub(crate) fn run_fmt(
    paths: &[PathBuf],
    check: bool,
    stdin: bool,
    reporter: &Reporter,
) -> ExitCode {
    if stdin {
        let mut src = String::new();
        if let Err(e) = std::io::stdin().read_to_string(&mut src) {
            reporter.say(
                &format!("error: cannot read standard input: {e}"),
                &format!("خطأ: تعذّرت قراءة الإدخال القياسي: {e}"),
            );
            return ExitCode::from(2);
        }
        return match format_text(&src) {
            Ok(out) => {
                print!("{out}");
                ExitCode::SUCCESS
            }
            Err(e) => {
                report_error(reporter, "<stdin>", &src, e);
                ExitCode::from(1)
            }
        };
    }

    let files = collect(paths);
    let (mut changed, mut failed) = (Vec::new(), 0);
    for file in &files {
        let name = shown(file);
        let src = match std::fs::read_to_string(file) {
            Ok(s) => s,
            Err(e) => {
                reporter.say(
                    &format!("error: cannot read `{name}`: {e}"),
                    &format!("خطأ: تعذّرت قراءة `{name}`: {e}"),
                );
                failed += 1;
                continue;
            }
        };
        match format_text(&src) {
            Ok(out) if out == src => {}
            Ok(out) => {
                if !check && let Err(e) = std::fs::write(file, out) {
                    reporter.say(
                        &format!("error: cannot write `{name}`: {e}"),
                        &format!("خطأ: تعذّرت كتابة `{name}`: {e}"),
                    );
                    failed += 1;
                    continue;
                }
                changed.push(name);
            }
            Err(e) => {
                report_error(reporter, &name, &src, e);
                failed += 1;
            }
        }
    }

    let n = changed.len();
    if check {
        for name in &changed {
            reporter.out(
                &format!("would reformat {name}"),
                &format!("يحتاج إلى تنسيق: {name}"),
            );
        }
        reporter.out(
            &format!("{n} of {} file(s) would be reformatted", files.len()),
            &format!("{n} من {} من الملفات تحتاج إلى تنسيق", files.len()),
        );
    } else {
        for name in &changed {
            reporter.out(&format!("formatted {name}"), &format!("نُسِّق {name}"));
        }
        reporter.out(
            &format!("{n} of {} file(s) reformatted", files.len()),
            &format!("نُسِّق {n} من {} من الملفات", files.len()),
        );
    }
    if failed > 0 || (check && n > 0) {
        ExitCode::from(1)
    } else {
        ExitCode::SUCCESS
    }
}
