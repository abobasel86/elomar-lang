//! Package registries: an index of published packages and their versions.
//!
//! A registry is a git repository (or a plain folder) with one file per
//! package, `packages/<name>.toml`:
//!
//! ```toml
//! description = "Text helpers"
//! git = "https://github.com/user/textkit"
//!
//! [[versions]]
//! version = "1.0.0"
//! commit = "3f2a...40 hex digits"
//! ```
//!
//! A version points at an exact commit, so its code cannot change after
//! publishing. `yanked = true` hides a version from new resolutions (lock
//! files that already use it keep working). Which registry a project uses
//! is set in its elomar.toml, `[registry] index = "<git URL or folder>"`,
//! or by the `ELOMAR_REGISTRY` environment variable.

use std::path::{Path, PathBuf};

use semver::{Version, VersionReq};
use serde::Deserialize;

use crate::{cache_dir, git, url_key};

pub const REGISTRY_ENV: &str = "ELOMAR_REGISTRY";

#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
pub struct IndexVersion {
    pub version: String,
    pub commit: String,
    #[serde(default)]
    pub yanked: bool,
}

#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
pub struct IndexEntry {
    #[serde(default)]
    pub description: String,
    pub git: String,
    #[serde(default)]
    pub versions: Vec<IndexVersion>,
}

impl IndexEntry {
    /// The newest version that is not yanked and matches `req`.
    pub fn best(&self, req: &VersionReq) -> Option<(Version, &IndexVersion)> {
        self.versions
            .iter()
            .filter(|v| !v.yanked)
            .filter_map(|v| Version::parse(&v.version).ok().map(|p| (p, v)))
            .filter(|(p, _)| req.matches(p))
            .max_by(|a, b| a.0.cmp(&b.0))
    }

    /// The newest version that is not yanked.
    pub fn latest(&self) -> Option<(Version, &IndexVersion)> {
        self.best(&VersionReq::STAR).or_else(|| {
            // `*` skips pre-releases; fall back to any version.
            self.versions
                .iter()
                .filter(|v| !v.yanked)
                .filter_map(|v| Version::parse(&v.version).ok().map(|p| (p, v)))
                .max_by(|a, b| a.0.cmp(&b.0))
        })
    }
}

#[derive(Debug, Default, Deserialize)]
struct RegistryManifest {
    #[serde(default)]
    registry: Option<RegistrySection>,
}

#[derive(Debug, Deserialize)]
struct RegistrySection {
    index: String,
}

/// Where the project's registry is: a git URL, or an absolute folder.
pub fn location(project_root: &Path) -> Result<Option<String>, String> {
    let path = project_root.join("elomar.toml");
    if let Ok(text) = std::fs::read_to_string(&path) {
        let m: RegistryManifest =
            toml::from_str(&text).map_err(|e| format!("{}: {e}", path.display()))?;
        if let Some(r) = m.registry {
            return Ok(Some(absolute(project_root, &r.index)));
        }
    }
    Ok(std::env::var(REGISTRY_ENV)
        .ok()
        .filter(|s| !s.trim().is_empty())
        .map(|s| absolute(&std::env::current_dir().unwrap_or_default(), &s)))
}

fn is_git_url(s: &str) -> bool {
    s.contains("://") || s.starts_with("git@")
}

fn absolute(base: &Path, s: &str) -> String {
    if is_git_url(s) || Path::new(s).is_absolute() {
        s.to_string()
    } else {
        base.join(s).to_string_lossy().to_string()
    }
}

pub fn no_registry_message() -> String {
    format!(
        "no package registry is configured; add `[registry]` with `index = \"<git URL or folder>\"` to elomar.toml, or set {REGISTRY_ENV}"
    )
}

/// An index on disk.
#[derive(Debug, Clone)]
pub struct Index {
    pub dir: PathBuf,
}

impl Index {
    /// Opens the index at `location`. A git index is cloned into the cache;
    /// with `refresh` an existing clone is brought up to date.
    pub fn open(location: &str, refresh: bool) -> Result<Index, String> {
        if !is_git_url(location) {
            let dir = PathBuf::from(location);
            if !dir.is_dir() {
                return Err(format!("registry folder `{location}` not found"));
            }
            return Ok(Index { dir });
        }
        let dir = cache_dir().join("registry").join(url_key(location));
        if dir.join(".git").is_dir() {
            if refresh {
                git(
                    &["fetch", "--quiet", "--depth", "1", "origin", "HEAD"],
                    Some(&dir),
                )?;
                git(&["reset", "--quiet", "--hard", "FETCH_HEAD"], Some(&dir))?;
            }
            return Ok(Index { dir });
        }
        if !refresh {
            return Err(format!(
                "the registry `{location}` has not been downloaded yet; run `elomar update` once"
            ));
        }
        let parent = dir.parent().expect("has a parent");
        std::fs::create_dir_all(parent).map_err(|e| format!("{}: {e}", parent.display()))?;
        let tmp = parent.join(format!(
            "{}.tmp-{}",
            dir.file_name().unwrap().to_string_lossy(),
            std::process::id()
        ));
        let _ = std::fs::remove_dir_all(&tmp);
        git(
            &[
                "clone",
                "--quiet",
                "--depth",
                "1",
                location,
                &tmp.to_string_lossy(),
            ],
            None,
        )?;
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::rename(&tmp, &dir).map_err(|e| format!("{}: {e}", dir.display()))?;
        Ok(Index { dir })
    }

    fn file(&self, name: &str) -> PathBuf {
        self.dir.join("packages").join(format!("{name}.toml"))
    }

    /// The entry of package `name`, if it is published here.
    pub fn entry(&self, name: &str) -> Result<Option<IndexEntry>, String> {
        let path = self.file(name);
        let Ok(text) = std::fs::read_to_string(&path) else {
            return Ok(None);
        };
        toml::from_str(&text)
            .map(Some)
            .map_err(|e| format!("registry entry {}: {e}", path.display()))
    }

    /// Packages whose name or description contains `query` (all if empty),
    /// sorted by name. Broken entries are skipped.
    pub fn search(&self, query: &str) -> Vec<(String, IndexEntry)> {
        let q = query.to_lowercase();
        let mut out = Vec::new();
        let Ok(dir) = std::fs::read_dir(self.dir.join("packages")) else {
            return out;
        };
        for f in dir.flatten() {
            let path = f.path();
            let Some(name) = path
                .file_name()
                .and_then(|n| n.to_str())
                .and_then(|n| n.strip_suffix(".toml"))
                .map(str::to_string)
            else {
                continue;
            };
            let Ok(Some(entry)) = self.entry(&name) else {
                continue;
            };
            if q.is_empty() || name.contains(&q) || entry.description.to_lowercase().contains(&q) {
                out.push((name, entry));
            }
        }
        out.sort_by(|a, b| a.0.cmp(&b.0));
        out
    }

    /// Adds `version` of package `name` to the index folder. The first
    /// version creates the entry; later ones must come from the same git
    /// repository and must be new.
    pub fn publish(
        &self,
        name: &str,
        description: &str,
        git_url: &str,
        version: &Version,
        commit: &str,
    ) -> Result<PathBuf, String> {
        if !crate::valid_name(name) {
            return Err(format!("`{name}` is not a valid package name"));
        }
        if !is_commit(commit) {
            return Err(format!("`{commit}` is not a full commit hash"));
        }
        let path = self.file(name);
        let block = format!(
            "\n[[versions]]\nversion = {:?}\ncommit = {:?}\n",
            version.to_string(),
            commit
        );
        let text = match self.entry(name)? {
            Some(entry) => {
                if entry.git != git_url {
                    return Err(format!(
                        "package `{name}` is already published from `{}`, not `{git_url}`",
                        entry.git
                    ));
                }
                if entry
                    .versions
                    .iter()
                    .any(|v| Version::parse(&v.version).ok().as_ref() == Some(version))
                {
                    return Err(format!(
                        "version {version} of `{name}` is already published; raise `version` in elomar.toml"
                    ));
                }
                let mut text = std::fs::read_to_string(&path).map_err(|e| e.to_string())?;
                if !text.ends_with('\n') {
                    text.push('\n');
                }
                text + &block
            }
            None => format!(
                "description = {:?}\ngit = {:?}\n{block}",
                description, git_url
            ),
        };
        let parent = path.parent().expect("has a parent");
        std::fs::create_dir_all(parent).map_err(|e| format!("{}: {e}", parent.display()))?;
        std::fs::write(&path, text).map_err(|e| format!("{}: {e}", path.display()))?;
        Ok(path)
    }
}

pub(crate) fn is_commit(s: &str) -> bool {
    s.len() == 40 && s.chars().all(|c| c.is_ascii_hexdigit())
}

/// Parses a version requirement as in Cargo: `1.2` means `^1.2`.
pub fn parse_requirement(text: &str) -> Result<VersionReq, String> {
    VersionReq::parse(text.trim()).map_err(|e| {
        format!(
            "`{text}` is not a version requirement ({e}); e.g. \"1.2\", \"^1.2.3\", \">=1, <3\""
        )
    })
}

/// What `elomar publish` needs from a package folder.
#[derive(Debug, Clone)]
pub struct Release {
    pub name: String,
    pub version: Version,
    pub description: String,
    pub git: String,
    pub commit: String,
    /// Whether the commit is on a remote branch (as of the last fetch).
    pub pushed: bool,
}

#[derive(Debug, Default, Deserialize)]
struct PackageManifest {
    #[serde(default)]
    package: Option<PackageSection>,
}

#[derive(Debug, Default, Deserialize)]
struct PackageSection {
    #[serde(default)]
    name: Option<String>,
    #[serde(default)]
    version: Option<String>,
    #[serde(default)]
    description: Option<String>,
}

/// Reads `[package]` of `dir` and its git state. The work tree must be
/// clean, so the published commit is exactly what is in the folder.
pub fn release(dir: &Path, git_url: Option<&str>) -> Result<Release, String> {
    let path = dir.join("elomar.toml");
    let text = std::fs::read_to_string(&path).map_err(|e| format!("{}: {e}", path.display()))?;
    let m: PackageManifest =
        toml::from_str(&text).map_err(|e| format!("{}: {e}", path.display()))?;
    let p = m.package.unwrap_or_default();
    let name = p.name.ok_or("elomar.toml needs `name` under `[package]`")?;
    if !crate::valid_name(&name) {
        return Err(format!(
            "`{name}` is not a valid package name (lowercase letters, digits and `_`; not `app` or `std`)"
        ));
    }
    let version_text = p
        .version
        .ok_or("elomar.toml needs `version` under `[package]`, e.g. `version = \"1.0.0\"`")?;
    let version = Version::parse(&version_text).map_err(|e| {
        format!("`version = \"{version_text}\"` is not a version like \"1.0.0\" ({e})")
    })?;
    let status = git(&["status", "--porcelain"], Some(dir))
        .map_err(|e| format!("the package must be a git repository: {e}"))?;
    if !status.is_empty() {
        return Err("the folder has uncommitted changes; commit them first so the published version matches the code".into());
    }
    let commit = git(&["rev-parse", "HEAD"], Some(dir))?;
    let git_url = match git_url {
        Some(u) => u.to_string(),
        None => git(&["remote", "get-url", "origin"], Some(dir)).map_err(|_| {
            "the repository has no `origin` remote; push it somewhere and pass `--git <URL>`"
                .to_string()
        })?,
    };
    let pushed =
        git(&["branch", "-r", "--contains", "HEAD"], Some(dir)).is_ok_and(|s| !s.is_empty());
    Ok(Release {
        name,
        version,
        description: p.description.unwrap_or_default(),
        git: git_url,
        commit,
        pushed,
    })
}
