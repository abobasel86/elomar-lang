//! Packages: `[dependencies]` in elomar.toml, resolved to folders.
//!
//! ```toml
//! [dependencies]
//! mathx = { path = "../mathx" }
//! textkit = { git = "https://github.com/user/textkit", tag = "v1.0" }
//! strings = "1.2"     # from the registry (see [`registry`]): ^1.2
//! ```
//!
//! - A path dependency is a folder (relative to the project) with its own
//!   elomar.toml and `src/` (or modules next to elomar.toml).
//! - A git dependency is cloned with the `git` command into a shared cache
//!   (`$ELOMAR_HOME/git`, default `~/.elomar/git`), one folder per commit.
//!   `elomar.lock` records the commit of each git package, so every build
//!   uses the same code until `elomar update`.
//! - A registry dependency names a version requirement; the newest
//!   matching version in the project's registry index is used, which is a
//!   git commit. The lock file records the version too.
//! - Dependencies of dependencies are resolved too. One name must always
//!   mean the same package, and a registry package has one version: every
//!   requirement on it must accept the version chosen first.
//!
//! Resolution is [`Mode::Offline`] for editors and checks (never touches
//! the network) and [`Mode::Online`] for `run`, `test`, `add` and `update`.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::process::Command;

use semver::{Version, VersionReq};
use serde::Deserialize;

pub mod registry;

pub use registry::{Index, IndexEntry, IndexVersion};

pub const LOCK_FILE: &str = "elomar.lock";

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Source {
    Path(PathBuf),
    Git {
        url: String,
        /// A tag, branch or commit; `None` for the default branch.
        reference: Option<String>,
    },
    /// A version from the project's registry.
    Registry(VersionReq),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Mode {
    /// Use the lock file and the cache only.
    Offline,
    /// Clone what is missing; resolve unlocked git references.
    Online,
    /// Like `Online`, ignoring the commits in the lock file.
    Update,
}

/// A resolved package.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Package {
    pub name: String,
    /// The package folder (where its elomar.toml is).
    pub root: PathBuf,
    /// The commit, for git and registry packages.
    pub commit: Option<String>,
    /// The version, for registry packages.
    pub version: Option<String>,
}

#[derive(Debug, Default, Deserialize)]
struct Manifest {
    #[serde(default)]
    dependencies: BTreeMap<String, toml::Value>,
}

#[derive(Debug, Default, Deserialize)]
struct LockFile {
    #[serde(default)]
    package: Vec<LockEntry>,
}

#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
struct LockEntry {
    name: String,
    git: String,
    commit: String,
    /// Set for registry packages.
    #[serde(default)]
    version: Option<String>,
}

/// Whether `name` can be a package name: a lowercase identifier that is
/// not `app` or `std`.
pub fn valid_name(name: &str) -> bool {
    let mut chars = name.chars();
    matches!(chars.next(), Some(c) if c.is_ascii_lowercase() || c == '_')
        && chars.all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '_')
        && name != "app"
        && name != "std"
}

/// The `[dependencies]` of the project or package in `root`.
pub fn dependencies(root: &Path) -> Result<Vec<(String, Source)>, String> {
    let path = root.join("elomar.toml");
    let Ok(text) = std::fs::read_to_string(&path) else {
        return Ok(Vec::new());
    };
    let manifest: Manifest =
        toml::from_str(&text).map_err(|e| format!("{}: {e}", path.display()))?;
    let mut out = Vec::new();
    for (name, value) in manifest.dependencies {
        if !valid_name(&name) {
            return Err(format!(
                "{}: `{name}` is not a valid package name (lowercase letters, digits and `_`; not `app` or `std`)",
                path.display()
            ));
        }
        if let Some(text) = value.as_str() {
            let req = registry::parse_requirement(text)
                .map_err(|e| format!("{}: dependency `{name}`: {e}", path.display()))?;
            out.push((name, Source::Registry(req)));
            continue;
        }
        let table = value.as_table().ok_or_else(|| {
            format!(
                "{}: dependency `{name}` needs a source, e.g. `{name} = \"1.0\"` (registry), `{name} = {{ path = \"../{name}\" }}` or `{name} = {{ git = \"https://...\" }}`",
                path.display()
            )
        })?;
        let get = |k: &str| {
            table
                .get(k)
                .and_then(toml::Value::as_str)
                .map(str::to_string)
        };
        let source = match (get("path"), get("git")) {
            (None, None) if get("version").is_some() => {
                let text = get("version").unwrap_or_default();
                Source::Registry(
                    registry::parse_requirement(&text)
                        .map_err(|e| format!("{}: dependency `{name}`: {e}", path.display()))?,
                )
            }
            _ if get("version").is_some() => {
                return Err(format!(
                    "{}: dependency `{name}`: `version` is for registry packages; drop it, or drop `path`/`git`",
                    path.display()
                ));
            }
            (Some(p), None) => Source::Path(root.join(p)),
            (None, Some(url)) => {
                let refs: Vec<String> = ["tag", "branch", "rev"]
                    .iter()
                    .filter_map(|k| get(k))
                    .collect();
                if refs.len() > 1 {
                    return Err(format!(
                        "{}: dependency `{name}`: give only one of `tag`, `branch` and `rev`",
                        path.display()
                    ));
                }
                Source::Git {
                    url,
                    reference: refs.into_iter().next(),
                }
            }
            _ => {
                return Err(format!(
                    "{}: dependency `{name}` needs exactly one of `path` and `git`",
                    path.display()
                ));
            }
        };
        out.push((name, source));
    }
    Ok(out)
}

/// Where git packages are cached.
pub fn cache_dir() -> PathBuf {
    if let Some(home) = std::env::var_os("ELOMAR_HOME").filter(|h| !h.is_empty()) {
        return PathBuf::from(home);
    }
    let home = std::env::var_os("HOME")
        .or_else(|| std::env::var_os("USERPROFILE"))
        .map(PathBuf::from)
        .unwrap_or_else(std::env::temp_dir);
    home.join(".elomar")
}

fn read_lock(root: &Path) -> Vec<LockEntry> {
    std::fs::read_to_string(root.join(LOCK_FILE))
        .ok()
        .and_then(|t| toml::from_str::<LockFile>(&t).ok())
        .map(|l| l.package)
        .unwrap_or_default()
}

fn write_lock(root: &Path, entries: &[LockEntry]) -> Result<(), String> {
    let path = root.join(LOCK_FILE);
    if entries.is_empty() {
        if path.exists() {
            std::fs::remove_file(&path).map_err(|e| format!("{}: {e}", path.display()))?;
        }
        return Ok(());
    }
    let mut text =
        String::from("# Generated by `elomar`; records the exact commit of each git package.\n");
    for e in entries {
        text.push_str(&format!("\n[[package]]\nname = {:?}\n", e.name));
        if let Some(v) = &e.version {
            text.push_str(&format!("version = {v:?}\n"));
        }
        text.push_str(&format!("git = {:?}\ncommit = {:?}\n", e.git, e.commit));
    }
    if std::fs::read_to_string(&path).ok().as_deref() == Some(text.as_str()) {
        return Ok(());
    }
    std::fs::write(&path, text).map_err(|e| format!("{}: {e}", path.display()))
}

pub(crate) fn git(args: &[&str], dir: Option<&Path>) -> Result<String, String> {
    let mut cmd = Command::new("git");
    if let Some(d) = dir {
        cmd.current_dir(d);
    }
    // Never ask for credentials on the terminal.
    cmd.env("GIT_TERMINAL_PROMPT", "0");
    let out = cmd
        .args(args)
        .output()
        .map_err(|e| format!("cannot run `git` ({e}); install git to use git dependencies"))?;
    if out.status.success() {
        Ok(String::from_utf8_lossy(&out.stdout).trim().to_string())
    } else {
        Err(format!(
            "`git {}` failed: {}",
            args.join(" "),
            String::from_utf8_lossy(&out.stderr).trim()
        ))
    }
}

/// A folder name for `url` (stable, readable, safe on every system).
pub(crate) fn url_key(url: &str) -> String {
    let readable: String = url
        .rsplit('/')
        .next()
        .unwrap_or("repo")
        .trim_end_matches(".git")
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || c == '-' || c == '_' {
                c
            } else {
                '_'
            }
        })
        .collect();
    // FNV-1a: a short, dependency-free hash of the whole URL.
    let mut h: u64 = 0xcbf2_9ce4_8422_2325;
    for b in url.bytes() {
        h ^= u64::from(b);
        h = h.wrapping_mul(0x0100_0000_01b3);
    }
    format!("{readable}-{h:016x}")
}

/// Clones `url` at `commit` into the cache if it is not there yet.
fn fetch(url: &str, commit: &str) -> Result<PathBuf, String> {
    let dir = cache_dir().join("git").join(url_key(url)).join(commit);
    if dir.join(".elomar-complete").exists() {
        return Ok(dir);
    }
    let parent = dir.parent().expect("has a parent");
    std::fs::create_dir_all(parent).map_err(|e| format!("{}: {e}", parent.display()))?;
    let tmp = parent.join(format!("{commit}.tmp-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&tmp);
    let tmp_s = tmp.to_string_lossy().to_string();
    git(&["clone", "--quiet", url, &tmp_s], None)?;
    git(
        &[
            "-c",
            "advice.detachedHead=false",
            "checkout",
            "--quiet",
            commit,
        ],
        Some(&tmp),
    )?;
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::rename(&tmp, &dir).map_err(|e| format!("{}: {e}", dir.display()))?;
    std::fs::write(dir.join(".elomar-complete"), commit).map_err(|e| e.to_string())?;
    Ok(dir)
}

/// The cached checkout of `url` at `commit`, downloaded unless offline.
fn checkout(name: &str, url: &str, commit: &str, mode: Mode) -> Result<PathBuf, String> {
    let dir = cache_dir().join("git").join(url_key(url)).join(commit);
    if dir.join(".elomar-complete").exists() {
        Ok(dir)
    } else if mode == Mode::Offline {
        Err(format!(
            "package `{name}` is not in the cache; run `elomar run` or `elomar update` once"
        ))
    } else {
        fetch(url, commit)
    }
}

/// The commit that `reference` (or the default branch) names in `url`.
fn resolve_commit(url: &str, reference: Option<&str>) -> Result<String, String> {
    if let Some(r) = reference
        && r.len() == 40
        && r.chars().all(|c| c.is_ascii_hexdigit())
    {
        return Ok(r.to_ascii_lowercase());
    }
    let wanted = reference.unwrap_or("HEAD");
    let listing = git(&["ls-remote", url, wanted], None)?;
    // A tag may appear twice; the peeled `^{}` line is the commit.
    let mut commit = None;
    for line in listing.lines() {
        let mut parts = line.split_whitespace();
        let (Some(sha), Some(name)) = (parts.next(), parts.next()) else {
            continue;
        };
        if name.ends_with("^{}") || commit.is_none() {
            commit = Some(sha.to_string());
        }
    }
    commit.ok_or_else(|| match reference {
        Some(r) => format!("`{url}` has no tag or branch `{r}`"),
        None => format!("`{url}` has no default branch"),
    })
}

/// Resolves the project's dependencies (and theirs) to folders.
pub fn resolve(project_root: &Path, mode: Mode) -> Result<Vec<Package>, String> {
    let lock = if mode == Mode::Update {
        Vec::new()
    } else {
        read_lock(project_root)
    };
    let mut new_lock: Vec<LockEntry> = Vec::new();
    let mut out: Vec<(Package, Source)> = Vec::new();
    // The project's registry index, opened (and refreshed) when needed.
    let mut index: Option<Index> = None;
    let mut queue: Vec<(String, Source, String)> = dependencies(project_root)?
        .into_iter()
        .map(|(n, s)| (n, s, "the project".to_string()))
        .collect();
    while !queue.is_empty() {
        let (name, source, wanted_by) = queue.remove(0);
        if let Some((prev, prev_source)) = out.iter().find(|(p, _)| p.name == name) {
            let same = match (&source, prev_source) {
                (Source::Path(a), Source::Path(_)) => {
                    std::fs::canonicalize(a).ok() == std::fs::canonicalize(&prev.root).ok()
                }
                (Source::Git { url: a, .. }, Source::Git { url: b, .. }) => a == b,
                (Source::Registry(req), Source::Registry(_)) => {
                    let chosen = prev.version.as_deref().and_then(|v| Version::parse(v).ok());
                    if chosen.as_ref().is_some_and(|v| req.matches(v)) {
                        continue;
                    }
                    return Err(format!(
                        "package `{name}`: version {} was chosen, but {wanted_by} requires {req}; one version of a package is used",
                        prev.version.as_deref().unwrap_or("?")
                    ));
                }
                _ => false,
            };
            if !same {
                return Err(format!(
                    "package `{name}` is required from two different sources (one by {wanted_by})"
                ));
            }
            continue;
        }
        let mut version = None;
        let (root, commit) = match &source {
            Source::Path(p) => {
                if !p.is_dir() {
                    return Err(format!(
                        "package `{name}` (required by {wanted_by}): folder `{}` not found",
                        p.display()
                    ));
                }
                (std::fs::canonicalize(p).unwrap_or_else(|_| p.clone()), None)
            }
            Source::Git { url, reference } => {
                let locked = lock.iter().find(|e| e.name == name && e.git == *url);
                let commit = match (locked, mode) {
                    (Some(e), _) => e.commit.clone(),
                    (None, Mode::Offline) => {
                        return Err(format!(
                            "package `{name}` has not been downloaded yet; run `elomar run` or `elomar update` once"
                        ));
                    }
                    (None, _) => resolve_commit(url, reference.as_deref())?,
                };
                let root = checkout(&name, url, &commit, mode)?;
                new_lock.push(LockEntry {
                    name: name.clone(),
                    git: url.clone(),
                    commit: commit.clone(),
                    version: None,
                });
                (root, Some(commit))
            }
            Source::Registry(req) => {
                let locked = lock.iter().find(|e| {
                    e.name == name
                        && e.version
                            .as_deref()
                            .and_then(|v| Version::parse(v).ok())
                            .is_some_and(|v| req.matches(&v))
                });
                let (url, commit, v) = match (locked, mode) {
                    (Some(e), _) => (
                        e.git.clone(),
                        e.commit.clone(),
                        e.version.clone().unwrap_or_default(),
                    ),
                    (None, Mode::Offline) => {
                        return Err(format!(
                            "package `{name}` has not been downloaded yet; run `elomar run` or `elomar update` once"
                        ));
                    }
                    (None, _) => {
                        if index.is_none() {
                            let at = registry::location(project_root)?;
                            index = Some(Index::open(&at, true)?);
                        }
                        let idx = index.as_ref().expect("opened");
                        let Some(entry) = idx.entry(&name)? else {
                            return Err(format!(
                                "package `{name}` (required by {wanted_by}) is not in the registry"
                            ));
                        };
                        let Some((v, found)) = entry.best(req) else {
                            let available: Vec<&str> = entry
                                .versions
                                .iter()
                                .filter(|v| !v.yanked)
                                .map(|v| v.version.as_str())
                                .collect();
                            return Err(format!(
                                "package `{name}`: no version matches {req} (required by {wanted_by}); available: {}",
                                if available.is_empty() {
                                    "none".to_string()
                                } else {
                                    available.join(", ")
                                }
                            ));
                        };
                        if !registry::is_commit(&found.commit) {
                            return Err(format!(
                                "registry entry of `{name}` {v}: `commit` must be a full commit hash"
                            ));
                        }
                        (
                            entry.git.clone(),
                            found.commit.to_ascii_lowercase(),
                            v.to_string(),
                        )
                    }
                };
                let root = checkout(&name, &url, &commit, mode)?;
                new_lock.push(LockEntry {
                    name: name.clone(),
                    git: url,
                    commit: commit.clone(),
                    version: Some(v.clone()),
                });
                version = Some(v);
                (root, Some(commit))
            }
        };
        for (n, s) in dependencies(&root)? {
            queue.push((n, s, format!("package `{name}`")));
        }
        out.push((
            Package {
                name,
                root,
                commit,
                version,
            },
            source,
        ));
    }
    if mode != Mode::Offline {
        new_lock.sort_by(|a, b| a.name.cmp(&b.name));
        write_lock(project_root, &new_lock)?;
    }
    Ok(out.into_iter().map(|(p, _)| p).collect())
}

/// Adds or replaces `name = { ... }` under `[dependencies]` in the text
/// of an elomar.toml, keeping the rest of the file as it is.
pub fn add_to_manifest(text: &str, name: &str, spec: &str) -> String {
    let line = format!("{name} = {spec}");
    let without = remove_from_manifest(text, name).unwrap_or_else(|| text.to_string());
    let mut lines: Vec<String> = without.lines().map(str::to_string).collect();
    match lines.iter().position(|l| l.trim() == "[dependencies]") {
        Some(i) => {
            // After the last entry of the section.
            let mut end = i + 1;
            while end < lines.len() && !lines[end].trim_start().starts_with('[') {
                end += 1;
            }
            while end > i + 1 && lines[end - 1].trim().is_empty() {
                end -= 1;
            }
            lines.insert(end, line);
        }
        None => {
            if lines.last().is_some_and(|l| !l.trim().is_empty()) {
                lines.push(String::new());
            }
            lines.push("[dependencies]".into());
            lines.push(line);
        }
    }
    let mut out = lines.join("\n");
    out.push('\n');
    out
}

/// Removes `name = ...` from `[dependencies]`; `None` if it is not there.
pub fn remove_from_manifest(text: &str, name: &str) -> Option<String> {
    let mut in_deps = false;
    let mut found = false;
    let mut out = Vec::new();
    for l in text.lines() {
        let t = l.trim();
        if t.starts_with('[') {
            in_deps = t == "[dependencies]";
        } else if in_deps
            && t.split('=')
                .next()
                .is_some_and(|k| k.trim().trim_matches('"') == name)
        {
            found = true;
            continue;
        }
        out.push(l);
    }
    found.then(|| {
        let mut s = out.join("\n");
        s.push('\n');
        s
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp(name: &str) -> PathBuf {
        let d = std::env::temp_dir().join(format!("elomar-pkg-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&d);
        std::fs::create_dir_all(&d).unwrap();
        d
    }

    /// One cache for all tests, so that setting it from parallel tests
    /// never changes it under another test.
    fn set_test_home() {
        let home = std::env::temp_dir().join(format!("elomar-pkg-home-{}", std::process::id()));
        // SAFETY: every test sets the same value.
        unsafe { std::env::set_var("ELOMAR_HOME", home) };
    }

    /// A git repository in `dir` with package `name`; returns a function
    /// that commits the current files with a tag and gives the commit.
    fn package_repo(dir: &Path, name: &str) -> impl Fn(&str, &str) -> String + use<> {
        std::fs::create_dir_all(dir.join("src")).unwrap();
        std::fs::write(
            dir.join("elomar.toml"),
            format!("[package]\nname = \"{name}\"\n"),
        )
        .unwrap();
        git(&["init", "--quiet"], Some(dir)).unwrap();
        let dir = dir.to_path_buf();
        move |tag: &str, body: &str| {
            std::fs::write(dir.join("src/text.elo"), body).unwrap();
            let id = ["-c", "user.name=t", "-c", "user.email=t@t"];
            git(&[&id[..], &["add", "."]].concat(), Some(&dir)).unwrap();
            git(
                &[&id[..], &["commit", "--quiet", "-m", tag]].concat(),
                Some(&dir),
            )
            .unwrap();
            git(&["tag", "-f", tag], Some(&dir)).unwrap();
            git(&["rev-parse", "HEAD"], Some(&dir)).unwrap()
        }
    }

    fn file_url(p: &Path) -> String {
        let url = format!("file://{}", p.to_string_lossy().replace('\\', "/"));
        if url.starts_with("file:///") {
            url
        } else {
            url.replacen("file://", "file:///", 1)
        }
    }

    #[test]
    fn registry_dependencies() {
        if git(&["--version"], None).is_err() {
            return; // No git on this machine.
        }
        set_test_home();
        let root = temp("registry");
        let commit = package_repo(&root.join("strings"), "strings");
        let c100 = commit("v1.0.0", "pub fn v() -> String {\n  return \"1.0.0\"\n}\n");
        let c120 = commit("v1.2.0", "pub fn v() -> String {\n  return \"1.2.0\"\n}\n");
        let c200 = commit("v2.0.0", "pub fn v() -> String {\n  return \"2.0.0\"\n}\n");
        let url = file_url(&root.join("strings"));

        // Publish three versions into a folder index.
        let index = Index {
            dir: root.join("index"),
        };
        for (v, c) in [("1.0.0", &c100), ("1.2.0", &c120), ("2.0.0", &c200)] {
            index
                .publish(
                    "strings",
                    "Text helpers",
                    &url,
                    &Version::parse(v).unwrap(),
                    c,
                )
                .unwrap();
        }
        let again = index.publish(
            "strings",
            "x",
            &url,
            &Version::parse("1.2.0").unwrap(),
            &c120,
        );
        assert!(again.unwrap_err().contains("already published"));
        let other = index.publish(
            "strings",
            "x",
            "https://x/y",
            &Version::parse("3.0.0").unwrap(),
            &c120,
        );
        assert!(other.unwrap_err().contains("already published from"));
        assert_eq!(index.search("text")[0].0, "strings");
        assert!(index.search("nothing").is_empty());

        let app = root.join("app");
        std::fs::create_dir_all(&app).unwrap();
        let manifest = |deps: &str| {
            std::fs::write(
                app.join("elomar.toml"),
                format!("[registry]\nindex = \"../index\"\n\n[dependencies]\n{deps}\n"),
            )
            .unwrap();
        };
        manifest("strings = \"1\"");
        assert!(
            resolve(&app, Mode::Offline)
                .unwrap_err()
                .contains("not been downloaded")
        );
        let p = resolve(&app, Mode::Online).unwrap();
        assert_eq!(p[0].version.as_deref(), Some("1.2.0"));
        assert_eq!(p[0].commit.as_deref(), Some(c120.as_str()));
        assert!(
            std::fs::read_to_string(p[0].root.join("src/text.elo"))
                .unwrap()
                .contains("1.2.0")
        );
        let lock = std::fs::read_to_string(app.join(LOCK_FILE)).unwrap();
        assert!(lock.contains("version = \"1.2.0\""), "{lock}");
        assert_eq!(resolve(&app, Mode::Offline).unwrap()[0].root, p[0].root);

        // A requirement the lock does not meet picks a new version.
        manifest("strings = { version = \">=2\" }");
        assert_eq!(
            resolve(&app, Mode::Online).unwrap()[0].version.as_deref(),
            Some("2.0.0")
        );
        manifest("strings = \"=1.0.0\"");
        assert_eq!(
            resolve(&app, Mode::Online).unwrap()[0].version.as_deref(),
            Some("1.0.0")
        );
        manifest("strings = \"3\"");
        let e = resolve(&app, Mode::Online).unwrap_err();
        assert!(
            e.contains("no version matches") && e.contains("1.0.0, 1.2.0, 2.0.0"),
            "{e}"
        );
        manifest("nope = \"1\"");
        assert!(
            resolve(&app, Mode::Online)
                .unwrap_err()
                .contains("not in the registry")
        );

        // One version per package: a dependency that needs ^2 conflicts.
        let lib = root.join("lib");
        std::fs::create_dir_all(&lib).unwrap();
        std::fs::write(lib.join("elomar.toml"), "[dependencies]\nstrings = \"2\"\n").unwrap();
        manifest("strings = \"1\"\nlib = { path = \"../lib\" }");
        let e = resolve(&app, Mode::Online).unwrap_err();
        assert!(
            e.contains("version 1.0.0 was chosen") && e.contains("package `lib`"),
            "{e}"
        );

        // A yanked version is skipped by new resolutions.
        let entry_path = root.join("index/packages/strings.toml");
        let text = std::fs::read_to_string(&entry_path).unwrap();
        std::fs::write(
            &entry_path,
            text.replace("version = \"1.2.0\"", "version = \"1.2.0\"\nyanked = true"),
        )
        .unwrap();
        manifest("strings = \"1\"");
        assert_eq!(
            resolve(&app, Mode::Update).unwrap()[0].version.as_deref(),
            Some("1.0.0")
        );

        // Without `[registry]` (or ELOMAR_REGISTRY) the public one is used.
        std::fs::write(app.join("elomar.toml"), "[dependencies]\nstrings = \"1\"\n").unwrap();
        if std::env::var_os(registry::REGISTRY_ENV).is_none() {
            assert_eq!(registry::location(&app).unwrap(), registry::DEFAULT_INDEX);
        }
    }

    #[test]
    fn names() {
        assert!(valid_name("mathx") && valid_name("text_kit2"));
        assert!(
            !valid_name("app") && !valid_name("Std") && !valid_name("2x") && !valid_name("a-b")
        );
    }

    #[test]
    fn editing_the_manifest() {
        let text = "[package]\nname = \"p\"\n\n[permissions]\npython = false\n";
        let added = add_to_manifest(text, "mathx", "{ path = \"../mathx\" }");
        assert!(
            added.ends_with("[dependencies]\nmathx = { path = \"../mathx\" }\n"),
            "{added}"
        );
        let two = add_to_manifest(&added, "kit", "{ git = \"https://x/kit\" }");
        assert!(
            two.contains("mathx = { path = \"../mathx\" }\nkit = { git"),
            "{two}"
        );
        let replaced = add_to_manifest(&two, "mathx", "{ path = \"../m2\" }");
        assert!(
            replaced.contains("mathx = { path = \"../m2\" }") && !replaced.contains("../mathx")
        );
        let removed = remove_from_manifest(&replaced, "kit").unwrap();
        assert!(!removed.contains("kit"));
        assert!(remove_from_manifest(&removed, "nope").is_none());
        // `python = false` in another section is left alone.
        assert!(remove_from_manifest(text, "python").is_none());
    }

    #[test]
    fn path_dependencies_and_conflicts() {
        let root = temp("paths");
        for (dir, toml) in [
            (
                "app",
                "[dependencies]\na = { path = \"../a\" }\nb = { path = \"../b\" }\n",
            ),
            ("a", "[dependencies]\nb = { path = \"../b\" }\n"),
            ("b", ""),
            ("c", "[dependencies]\nb = { path = \"../a\" }\n"),
        ] {
            std::fs::create_dir_all(root.join(dir)).unwrap();
            std::fs::write(root.join(dir).join("elomar.toml"), toml).unwrap();
        }
        let packages = resolve(&root.join("app"), Mode::Offline).unwrap();
        let names: Vec<&str> = packages.iter().map(|p| p.name.as_str()).collect();
        assert_eq!(names, ["a", "b"]);
        // `c` wants a different `b`.
        std::fs::write(
            root.join("app/elomar.toml"),
            "[dependencies]\nb = { path = \"../b\" }\nc = { path = \"../c\" }\n",
        )
        .unwrap();
        let err = resolve(&root.join("app"), Mode::Offline).unwrap_err();
        assert!(err.contains("two different sources"), "{err}");
        std::fs::write(
            root.join("app/elomar.toml"),
            "[dependencies]\nx = { path = \"../none\" }\n",
        )
        .unwrap();
        assert!(
            resolve(&root.join("app"), Mode::Offline)
                .unwrap_err()
                .contains("not found")
        );
        std::fs::write(root.join("app/elomar.toml"), "[dependencies]\nx = 3\n").unwrap();
        assert!(
            resolve(&root.join("app"), Mode::Offline)
                .unwrap_err()
                .contains("needs a source")
        );
        std::fs::write(
            root.join("app/elomar.toml"),
            "[dependencies]\nx = \"one\"\n",
        )
        .unwrap();
        assert!(
            resolve(&root.join("app"), Mode::Offline)
                .unwrap_err()
                .contains("not a version requirement")
        );
    }

    #[test]
    fn git_dependencies_are_cloned_and_locked() {
        if git(&["--version"], None).is_err() {
            return; // No git on this machine.
        }
        let root = temp("git");
        let repo = root.join("repo");
        std::fs::create_dir_all(repo.join("src")).unwrap();
        std::fs::write(repo.join("elomar.toml"), "[package]\nname = \"kit\"\n").unwrap();
        std::fs::write(
            repo.join("src/text.elo"),
            "pub fn hi() -> String {\n  return \"v1\"\n}\n",
        )
        .unwrap();
        let run = |args: &[&str]| git(args, Some(&repo)).unwrap();
        run(&["init", "--quiet"]);
        run(&["-c", "user.name=t", "-c", "user.email=t@t", "add", "."]);
        run(&[
            "-c",
            "user.name=t",
            "-c",
            "user.email=t@t",
            "commit",
            "--quiet",
            "-m",
            "v1",
        ]);
        run(&["tag", "v1"]);
        let first = run(&["rev-parse", "HEAD"]);

        let app = root.join("app");
        std::fs::create_dir_all(&app).unwrap();
        let url = format!("file://{}", repo.to_string_lossy().replace('\\', "/"));
        let url = if url.starts_with("file:///") {
            url
        } else {
            url.replacen("file://", "file:///", 1)
        };
        std::fs::write(
            app.join("elomar.toml"),
            format!("[dependencies]\nkit = {{ git = \"{url}\", tag = \"v1\" }}\n"),
        )
        .unwrap();

        set_test_home();
        assert!(
            resolve(&app, Mode::Offline)
                .unwrap_err()
                .contains("not been downloaded")
        );
        let packages = resolve(&app, Mode::Online).unwrap();
        assert_eq!(packages[0].commit.as_deref(), Some(first.as_str()));
        assert!(packages[0].root.join("src/text.elo").exists());
        assert!(
            std::fs::read_to_string(app.join(LOCK_FILE))
                .unwrap()
                .contains(&first)
        );
        // Offline now works from the lock and cache.
        assert_eq!(
            resolve(&app, Mode::Offline).unwrap()[0].root,
            packages[0].root
        );

        // A new commit is ignored until `update` (the lock pins v1's commit).
        std::fs::write(
            repo.join("src/text.elo"),
            "pub fn hi() -> String {\n  return \"v2\"\n}\n",
        )
        .unwrap();
        run(&[
            "-c",
            "user.name=t",
            "-c",
            "user.email=t@t",
            "commit",
            "--quiet",
            "-am",
            "v2",
        ]);
        run(&["tag", "-f", "v1"]);
        assert_eq!(
            resolve(&app, Mode::Online).unwrap()[0].commit.as_deref(),
            Some(first.as_str())
        );
        let updated = resolve(&app, Mode::Update).unwrap();
        assert_ne!(updated[0].commit.as_deref(), Some(first.as_str()));
    }
}
