//! `elomar add`, `elomar remove`, `elomar update`, and downloading git
//! packages before `run`, `check` and `test`.

use std::path::{Path, PathBuf};
use std::process::ExitCode;

use elomar_pkg::Mode;

use crate::Reporter;

/// The project folder: the nearest elomar.toml at or above the current folder.
fn project_dir() -> Option<PathBuf> {
    let cwd = std::env::current_dir().ok()?;
    cwd.ancestors()
        .find(|d| d.join("elomar.toml").is_file())
        .map(Path::to_path_buf)
}

/// Downloads missing git packages of the project that `source` belongs
/// to, so that loading (which is offline) finds them. Errors are reported
/// by the loader as well, with context, so they are not repeated here.
pub(crate) fn fetch_for(source: &Path, reporter: &Reporter) {
    let root = crate::project_root(source);
    if let Err(message) = elomar_pkg::resolve(&root, Mode::Online) {
        reporter.say(
            &format!("error: dependencies: {message}"),
            &format!("خطأ: الاعتماديات: {message}"),
        );
    }
}

pub(crate) struct AddArgs<'a> {
    pub name: &'a str,
    pub path: Option<&'a str>,
    pub git: Option<&'a str>,
    pub tag: Option<&'a str>,
    pub branch: Option<&'a str>,
    pub rev: Option<&'a str>,
}

fn quote(s: &str) -> String {
    format!("{s:?}")
}

pub(crate) fn add(args: &AddArgs, reporter: &Reporter) -> ExitCode {
    let Some(dir) = project_dir() else {
        reporter.say(
            "error: no elomar.toml here or in a parent folder; run this inside a project (see `elomar new`)",
            "خطأ: لا يوجد elomar.toml هنا أو في مجلد أعلى؛ شغّل الأمر داخل مشروع (انظر `elomar new`)",
        );
        return ExitCode::from(2);
    };
    let name = args.name;
    if !elomar_pkg::valid_name(name) {
        reporter.say(
            &format!("error: `{name}` is not a valid package name (lowercase letters, digits and `_`; not `app` or `std`)"),
            &format!("خطأ: `{name}` ليس اسم حزمة صالحًا (حروف لاتينية صغيرة وأرقام و`_`؛ وليس `app` أو `std`)"),
        );
        return ExitCode::from(2);
    }
    let spec = match (args.path, args.git) {
        (Some(p), None) => format!("{{ path = {} }}", quote(&p.replace('\\', "/"))),
        (None, Some(url)) => {
            let mut s = format!("{{ git = {}", quote(url));
            for (key, value) in [
                ("tag", args.tag),
                ("branch", args.branch),
                ("rev", args.rev),
            ] {
                if let Some(v) = value {
                    s.push_str(&format!(", {key} = {}", quote(v)));
                }
            }
            s.push_str(" }");
            s
        }
        _ => {
            reporter.say(
                "error: give the package's source: `--path <folder>` or `--git <url>`",
                "خطأ: حدد مصدر الحزمة: `--path <folder>` أو `--git <url>`",
            );
            return ExitCode::from(2);
        }
    };
    let manifest = dir.join("elomar.toml");
    let text = std::fs::read_to_string(&manifest).unwrap_or_default();
    let updated = elomar_pkg::add_to_manifest(&text, name, &spec);
    if let Err(e) = std::fs::write(&manifest, &updated) {
        reporter.say(&format!("error: {e}"), &format!("خطأ: {e}"));
        return ExitCode::from(2);
    }
    match elomar_pkg::resolve(&dir, Mode::Online) {
        Ok(packages) => {
            let p = packages.iter().find(|p| p.name == name);
            let detail = p
                .and_then(|p| p.commit.as_deref())
                .map(|c| format!(" at {}", &c[..c.len().min(10)]))
                .unwrap_or_default();
            reporter.out(
                &format!("added `{name}`{detail}; use it with `use {name}.<module>`"),
                &format!("أُضيفت `{name}`{detail}؛ استخدمها بـ `use {name}.<module>`"),
            );
            ExitCode::SUCCESS
        }
        Err(message) => {
            // Leave elomar.toml as it was.
            let _ = std::fs::write(&manifest, &text);
            reporter.say(
                &format!("error: cannot add `{name}`: {message}"),
                &format!("خطأ: تعذّرت إضافة `{name}`: {message}"),
            );
            ExitCode::from(1)
        }
    }
}

pub(crate) fn remove(name: &str, reporter: &Reporter) -> ExitCode {
    let Some(dir) = project_dir() else {
        reporter.say(
            "error: no elomar.toml here or in a parent folder",
            "خطأ: لا يوجد elomar.toml هنا أو في مجلد أعلى",
        );
        return ExitCode::from(2);
    };
    let manifest = dir.join("elomar.toml");
    let text = std::fs::read_to_string(&manifest).unwrap_or_default();
    let Some(updated) = elomar_pkg::remove_from_manifest(&text, name) else {
        reporter.say(
            &format!("error: `{name}` is not in [dependencies]"),
            &format!("خطأ: `{name}` ليست في [dependencies]"),
        );
        return ExitCode::from(1);
    };
    if let Err(e) = std::fs::write(&manifest, updated) {
        reporter.say(&format!("error: {e}"), &format!("خطأ: {e}"));
        return ExitCode::from(2);
    }
    // Rewrites the lock file without the removed package.
    let _ = elomar_pkg::resolve(&dir, Mode::Online);
    reporter.out(&format!("removed `{name}`"), &format!("أُزيلت `{name}`"));
    ExitCode::SUCCESS
}

pub(crate) fn update(reporter: &Reporter) -> ExitCode {
    let Some(dir) = project_dir() else {
        reporter.say(
            "error: no elomar.toml here or in a parent folder",
            "خطأ: لا يوجد elomar.toml هنا أو في مجلد أعلى",
        );
        return ExitCode::from(2);
    };
    match elomar_pkg::resolve(&dir, Mode::Update) {
        Ok(packages) => {
            if packages.is_empty() {
                reporter.out("no dependencies", "لا توجد اعتماديات");
            }
            for p in packages {
                let at = p
                    .commit
                    .as_deref()
                    .map(|c| c[..c.len().min(10)].to_string())
                    .unwrap_or_else(|| p.root.display().to_string());
                reporter.out(&format!("{} {at}", p.name), &format!("{} {at}", p.name));
            }
            ExitCode::SUCCESS
        }
        Err(message) => {
            reporter.say(&format!("error: {message}"), &format!("خطأ: {message}"));
            ExitCode::from(1)
        }
    }
}
