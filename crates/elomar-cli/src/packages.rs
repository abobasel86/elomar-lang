//! `elomar add`, `elomar remove`, `elomar update`, `elomar search`,
//! `elomar publish`, and downloading packages before `run` and `test`.

use std::path::{Path, PathBuf};
use std::process::ExitCode;

use elomar_pkg::{Index, Mode, registry};

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
    let (name, requirement) = match args.name.split_once('@') {
        Some((n, r)) => (n, Some(r)),
        None => (args.name, None),
    };
    if requirement.is_some() && (args.path.is_some() || args.git.is_some()) {
        reporter.say(
            "error: `name@version` is for registry packages; drop `@...` or `--path`/`--git`",
            "خطأ: الصيغة `name@version` لحزم السجل؛ احذف `@...` أو `--path`/`--git`",
        );
        return ExitCode::from(2);
    }
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
        _ => match registry_requirement(&dir, name, requirement) {
            Ok(req) => quote(&req),
            Err(message) => {
                reporter.say(
                    &format!("error: cannot add `{name}`: {message}"),
                    &format!("خطأ: تعذّرت إضافة `{name}`: {message}"),
                );
                return ExitCode::from(1);
            }
        },
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
            let detail = match p {
                Some(p) if p.version.is_some() => {
                    format!(" {}", p.version.as_deref().unwrap_or_default())
                }
                Some(p) => p
                    .commit
                    .as_deref()
                    .map(|c| format!(" at {}", &c[..c.len().min(10)]))
                    .unwrap_or_default(),
                None => String::new(),
            };
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
                let at = match (&p.version, &p.commit) {
                    (Some(v), _) => v.clone(),
                    (None, Some(c)) => c[..c.len().min(10)].to_string(),
                    (None, None) => p.root.display().to_string(),
                };
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

/// The requirement to write for a registry package: the one given, checked
/// against the registry, or `^` the newest version.
fn registry_requirement(dir: &Path, name: &str, given: Option<&str>) -> Result<String, String> {
    let index = open_registry(dir)?;
    let entry = index
        .entry(name)?
        .ok_or_else(|| format!("`{name}` is not in the registry (try `elomar search`)"))?;
    match given {
        Some(text) => {
            let req = registry::parse_requirement(text)?;
            if entry.best(&req).is_none() {
                return Err(format!("no version of `{name}` matches `{text}`"));
            }
            Ok(text.to_string())
        }
        None => entry
            .latest()
            .map(|(v, _)| format!("^{v}"))
            .ok_or_else(|| format!("`{name}` has no published versions")),
    }
}

fn open_registry(dir: &Path) -> Result<Index, String> {
    let location = registry::location(dir)?.ok_or_else(registry::no_registry_message)?;
    Index::open(&location, true)
}

pub(crate) fn search(query: &str, reporter: &Reporter) -> ExitCode {
    let dir = project_dir().unwrap_or_else(|| std::env::current_dir().unwrap_or_default());
    let index = match open_registry(&dir) {
        Ok(i) => i,
        Err(message) => {
            reporter.say(&format!("error: {message}"), &format!("خطأ: {message}"));
            return ExitCode::from(1);
        }
    };
    let found = index.search(query);
    if found.is_empty() {
        reporter.out("no packages found", "لم تُعثر على حزم");
    }
    for (name, entry) in found {
        let version = entry
            .latest()
            .map(|(v, _)| v.to_string())
            .unwrap_or_else(|| "-".into());
        let line = if entry.description.is_empty() {
            format!("{name} {version}")
        } else {
            format!("{name} {version} - {}", entry.description)
        };
        reporter.out(&line, &line);
    }
    ExitCode::SUCCESS
}

pub(crate) fn publish(index: Option<&str>, git: Option<&str>, reporter: &Reporter) -> ExitCode {
    let fail = |message: String| {
        reporter.say(&format!("error: {message}"), &format!("خطأ: {message}"));
        ExitCode::from(1)
    };
    let Some(dir) = project_dir() else {
        return fail(
            "no elomar.toml here or in a parent folder; run this inside the package".into(),
        );
    };
    let release = match registry::release(&dir, git) {
        Ok(r) => r,
        Err(m) => return fail(m),
    };
    let folder = match index {
        Some(f) => std::path::absolute(f).unwrap_or_else(|_| PathBuf::from(f)),
        None => match registry::location(&dir) {
            Ok(Some(l)) if !l.contains("://") && !l.starts_with("git@") => PathBuf::from(l),
            Ok(_) => {
                return fail(
                    "give a local clone of the registry index with `--index <folder>`".into(),
                );
            }
            Err(m) => return fail(m),
        },
    };
    if !folder.is_dir() {
        return fail(format!("registry folder `{}` not found", folder.display()));
    }
    let written = Index { dir: folder }.publish(
        &release.name,
        &release.description,
        &release.git,
        &release.version,
        &release.commit,
    );
    match written {
        Ok(path) => {
            let short = &release.commit[..release.commit.len().min(10)];
            reporter.out(
                &format!(
                    "added {} {} (commit {short}) to {}\nnext: commit and push the registry index (or open a pull request) to publish it",
                    release.name,
                    release.version,
                    path.display()
                ),
                &format!(
                    "أُضيفت {} {} (commit {short}) إلى {}\nالتالي: اعمل commit وpush لفهرس السجل (أو افتح pull request) لنشرها",
                    release.name,
                    release.version,
                    path.display()
                ),
            );
            if !release.pushed {
                reporter.say(
                    &format!("warning: commit {short} is not on any remote branch yet; push it so others can download it"),
                    &format!("تحذير: الـ commit {short} ليس على أي فرع بعيد بعد؛ ادفعه (push) ليتمكن الآخرون من تنزيله"),
                );
            }
            ExitCode::SUCCESS
        }
        Err(m) => fail(m),
    }
}
