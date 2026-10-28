//! Permission checks at real resource boundaries (spec §16.3).
//!
//! [`FsPolicy`] decides whether a program may read or write a path, from the
//! `filesystem_read` / `filesystem_write` lists in `elomar.toml`. Everything
//! not listed is denied. Paths are resolved against the project directory
//! and canonicalized, so `..` segments and symbolic links cannot escape an
//! allowed folder.

use std::path::{Component, Path, PathBuf};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Access {
    Read,
    Write,
}

impl Access {
    fn config_key(self) -> &'static str {
        match self {
            Access::Read => "filesystem_read",
            Access::Write => "filesystem_write",
        }
    }
}

#[derive(Debug, Clone)]
pub struct FsPolicy {
    /// The project directory: where `elomar.toml` is, or the program's own
    /// folder when there is none. Relative paths are resolved against it.
    base: PathBuf,
    read: Vec<PathBuf>,
    write: Vec<PathBuf>,
}

impl FsPolicy {
    /// No file may be read or written.
    pub fn deny_all(base: impl Into<PathBuf>) -> Self {
        FsPolicy {
            base: base.into(),
            read: Vec::new(),
            write: Vec::new(),
        }
    }

    /// `read` and `write` are entries as written in `elomar.toml`: files or
    /// folders, relative to `base` or absolute.
    pub fn new(base: impl Into<PathBuf>, read: &[String], write: &[String]) -> Self {
        let base = base.into();
        let roots = |entries: &[String]| -> Vec<PathBuf> {
            entries
                .iter()
                .map(|e| canonical_or_lexical(&base.join(e)))
                .collect()
        };
        FsPolicy {
            read: roots(read),
            write: roots(write),
            base,
        }
    }

    pub fn base(&self) -> &Path {
        &self.base
    }

    /// The path a program's `path` argument refers to.
    pub fn resolve(&self, path: &str) -> PathBuf {
        self.base.join(path)
    }

    /// Returns the resolved path if `access` to `path` is allowed, or a
    /// message explaining how to allow it.
    pub fn check(&self, path: &str, access: Access) -> Result<PathBuf, String> {
        let resolved = self.resolve(path);
        let target = canonical_target(&resolved);
        let roots = match access {
            Access::Read => &self.read,
            Access::Write => &self.write,
        };
        if roots.iter().any(|root| target.starts_with(root)) {
            Ok(resolved)
        } else {
            let verb = match access {
                Access::Read => "reading",
                Access::Write => "writing",
            };
            Err(format!(
                "{verb} `{path}` is not permitted; add its folder to `{}` under `[permissions]` in elomar.toml",
                access.config_key()
            ))
        }
    }
}

/// The canonical form of `path`. For a path that does not exist yet (a file
/// about to be written), canonicalizes the nearest existing ancestor and
/// appends the rest, refusing `..` in that rest so it cannot climb out.
fn canonical_target(path: &Path) -> PathBuf {
    if let Ok(p) = path.canonicalize() {
        return p;
    }
    let mut existing = path.to_path_buf();
    let mut rest: Vec<std::ffi::OsString> = Vec::new();
    while !existing.exists() {
        match (existing.file_name(), existing.parent()) {
            (Some(name), Some(parent)) => {
                rest.push(name.to_os_string());
                existing = parent.to_path_buf();
            }
            _ => return lexical(path),
        }
    }
    let Ok(mut out) = existing.canonicalize() else {
        return lexical(path);
    };
    for part in rest.iter().rev() {
        if part == ".." {
            // Cannot be resolved safely; this never matches an allowed root.
            return PathBuf::from("..");
        }
        out.push(part);
    }
    out
}

fn canonical_or_lexical(path: &Path) -> PathBuf {
    path.canonicalize().unwrap_or_else(|_| lexical(path))
}

/// Removes `.` and folds `..` without touching the file system.
fn lexical(path: &Path) -> PathBuf {
    let mut out = PathBuf::new();
    for c in path.components() {
        match c {
            Component::CurDir => {}
            Component::ParentDir => {
                out.pop();
            }
            other => out.push(other),
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicU32, Ordering};

    /// A fresh project folder with `data/a.txt` and an empty `out/`.
    fn project() -> PathBuf {
        static N: AtomicU32 = AtomicU32::new(0);
        let dir = std::env::temp_dir().join(format!(
            "elomar-policy-{}-{}",
            std::process::id(),
            N.fetch_add(1, Ordering::SeqCst)
        ));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(dir.join("data")).unwrap();
        std::fs::create_dir_all(dir.join("out")).unwrap();
        std::fs::write(dir.join("data/a.txt"), "a").unwrap();
        std::fs::write(dir.join("secret.txt"), "s").unwrap();
        dir
    }

    fn policy(dir: &Path) -> FsPolicy {
        FsPolicy::new(dir, &["./data".into()], &["out".into()])
    }

    #[test]
    fn allows_only_listed_folders() {
        let dir = project();
        let p = policy(&dir);
        assert!(p.check("data/a.txt", Access::Read).is_ok());
        assert!(p.check("./data/../data/a.txt", Access::Read).is_ok());
        assert!(p.check("secret.txt", Access::Read).is_err());
        assert!(
            p.check("data/a.txt", Access::Write).is_err(),
            "read does not imply write"
        );
        assert!(
            p.check("out/new.txt", Access::Write).is_ok(),
            "new files in an allowed folder"
        );
        assert!(p.check("out/sub/new.txt", Access::Write).is_ok());
        let _ = std::fs::remove_dir_all(dir);
    }

    #[test]
    fn dot_dot_cannot_escape() {
        let dir = project();
        let p = policy(&dir);
        assert!(p.check("data/../secret.txt", Access::Read).is_err());
        assert!(p.check("out/../secret.txt", Access::Write).is_err());
        assert!(
            p.check("out/missing/../../secret.txt", Access::Write)
                .is_err()
        );
        let outside = dir.parent().unwrap().join("elsewhere.txt");
        assert!(p.check(outside.to_str().unwrap(), Access::Write).is_err());
        let _ = std::fs::remove_dir_all(dir);
    }

    #[test]
    fn deny_all_and_messages() {
        let dir = project();
        let p = FsPolicy::deny_all(&dir);
        let msg = p.check("data/a.txt", Access::Read).unwrap_err();
        assert!(msg.contains("`filesystem_read`"), "{msg}");
        let _ = std::fs::remove_dir_all(dir);
    }
}
