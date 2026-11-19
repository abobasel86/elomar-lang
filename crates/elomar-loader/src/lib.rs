//! Loads a program and the modules it imports (spec §11.2).
//!
//! `use app.students` refers to `src/students.elo` in the project folder
//! (or `students.elo` next to `elomar.toml` when there is no `src/`), and
//! `use app.reports.daily` to `src/reports/daily.elo`. Every file is parsed
//! into one shared span space ([`SourceMap`]), and the items of a module are
//! renamed with the module's name as a prefix (`count` in `students` becomes
//! `students.count`), so the checker and interpreter see a single program.
//! Missing modules are left for the checker to report.

use std::collections::{HashSet, VecDeque};
use std::path::{Path, PathBuf};

use elomar_ast::{Item, Program};
use elomar_diagnostics::{Diagnostic, SourceFile, SourceMap};

pub struct Loaded {
    pub program: Program,
    pub sources: SourceMap,
    /// The absolute path of each file in `sources`, in the same order.
    pub paths: Vec<PathBuf>,
    /// Lexical and syntax errors from every file.
    pub diagnostics: Vec<Diagnostic>,
}

/// Where module files live: `<project>/src` if it exists, else `<project>`.
pub fn module_dir(project_root: &Path) -> PathBuf {
    let src = project_root.join("src");
    if src.is_dir() {
        src
    } else {
        project_root.to_path_buf()
    }
}

/// The file for module `a.b`: `<module_dir>/a/b.elo`.
pub fn module_file(module_dir: &Path, module: &str) -> PathBuf {
    let mut path = module_dir.to_path_buf();
    for part in module.split('.') {
        path.push(part);
    }
    path.set_extension("elo");
    path
}

/// A path for messages: relative to the current folder when possible.
fn display_name(path: &Path, entry_display: &str, entry: &Path) -> String {
    if path == entry {
        return entry_display.to_string();
    }
    let shown = std::env::current_dir()
        .ok()
        .and_then(|cwd| path.strip_prefix(cwd).ok().map(Path::to_path_buf))
        .unwrap_or_else(|| path.to_path_buf());
    // Same separators on every system, so messages and tests are stable.
    shown.display().to_string().replace('\\', "/")
}

/// Reads `entry` and, transitively, every `use app.*` module it needs.
/// Items of imported modules come first (dependencies before the modules
/// that use them, so file-level constants are initialized in order); the
/// main file's items come last.
pub fn load(entry: &Path, project_root: &Path) -> std::io::Result<Loaded> {
    load_with(entry, project_root, None)
}

/// Like [`load`], with the main file's text given instead of read from
/// disk (an editor's unsaved buffer).
pub fn load_with(
    entry: &Path,
    project_root: &Path,
    entry_text: Option<String>,
) -> std::io::Result<Loaded> {
    let entry_display = entry.display().to_string();
    let entry_abs = std::path::absolute(entry)?;
    let dir = module_dir(&std::path::absolute(project_root)?);

    let mut sources = SourceMap::default();
    let mut diagnostics = Vec::new();

    let text = match entry_text {
        Some(t) => t,
        None => std::fs::read_to_string(entry)?,
    };
    let base = sources.next_base();
    let (main, diags) = elomar_parser::parse_at(&text, base);
    sources.add(SourceFile::new(entry_display.clone(), text));
    let mut paths = vec![entry_abs.clone()];
    diagnostics.extend(diags);

    let mut queue: VecDeque<String> = imports(&main.items).into();
    let mut seen: HashSet<String> = HashSet::new();
    let mut modules: Vec<(String, Vec<Item>)> = Vec::new();

    while let Some(module) = queue.pop_front() {
        if !seen.insert(module.clone()) {
            continue;
        }
        let file = module_file(&dir, &module);
        if file == entry_abs || !file.is_file() {
            continue;
        }
        let text = std::fs::read_to_string(&file)?;
        let base = sources.next_base();
        let (parsed, diags) = elomar_parser::parse_at(&text, base);
        sources.add(SourceFile::new(
            display_name(&file, &entry_display, &entry_abs),
            text,
        ));
        paths.push(file.clone());
        diagnostics.extend(diags);
        let mut items = parsed.items;
        for item in &mut items {
            prefix(item, &module);
        }
        queue.extend(imports(&items));
        modules.push((module, items));
    }

    let mut program = Program {
        modules: modules.iter().map(|(m, _)| m.clone()).collect(),
        items: Vec::new(),
    };
    for (_, items) in modules.into_iter().rev() {
        program.items.extend(items);
    }
    program.items.extend(main.items);
    Ok(Loaded {
        program,
        sources,
        paths,
        diagnostics,
    })
}

/// Modules named by `use app.*` items.
fn imports(items: &[Item]) -> Vec<String> {
    items
        .iter()
        .filter_map(|item| match item {
            Item::Use(u) if u.path.len() > 1 && u.path[0].name == "app" => Some(
                u.path[1..]
                    .iter()
                    .map(|i| i.name.as_str())
                    .collect::<Vec<_>>()
                    .join("."),
            ),
            _ => None,
        })
        .collect()
}

/// Renames a module's item to `module.name` and records where a `use` is.
fn prefix(item: &mut Item, module: &str) {
    let name = match item {
        Item::Fn(f) => &mut f.name.name,
        Item::Type(t) => &mut t.name.name,
        Item::Enum(e) => &mut e.name.name,
        Item::Const(c) => &mut c.name.name,
        Item::Trait(t) => &mut t.name.name,
        Item::Impl(i) => {
            i.module = module.to_string();
            return;
        }
        Item::Use(u) => {
            u.module = module.to_string();
            return;
        }
    };
    *name = format!("{module}.{name}");
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicU32, Ordering};

    fn project(files: &[(&str, &str)]) -> PathBuf {
        static N: AtomicU32 = AtomicU32::new(0);
        let dir = std::env::temp_dir().join(format!(
            "elomar-loader-{}-{}",
            std::process::id(),
            N.fetch_add(1, Ordering::SeqCst)
        ));
        let _ = std::fs::remove_dir_all(&dir);
        for (path, text) in files {
            let p = dir.join(path);
            std::fs::create_dir_all(p.parent().unwrap()).unwrap();
            std::fs::write(p, text).unwrap();
        }
        dir
    }

    fn names(program: &Program) -> Vec<String> {
        program
            .items
            .iter()
            .map(|i| match i {
                Item::Fn(f) => f.name.name.clone(),
                Item::Type(t) => t.name.name.clone(),
                Item::Enum(e) => e.name.name.clone(),
                Item::Const(c) => c.name.name.clone(),
                Item::Use(u) => format!("use@{}", u.module),
                Item::Trait(t) => t.name.name.clone(),
                Item::Impl(i) => format!("impl@{}", i.module),
            })
            .collect()
    }

    #[test]
    fn loads_imported_modules_with_prefixes() {
        let dir = project(&[
            ("src/main.elo", "use app.students\nfn main() {}\n"),
            (
                "src/students.elo",
                "use app.reports.text\npub fn count() -> Int {\n  return 1\n}\npub type Student = { name: String }\n",
            ),
            ("src/reports/text.elo", "pub let TITLE = \"Report\"\n"),
        ]);
        let loaded = load(&dir.join("src/main.elo"), &dir).unwrap();
        assert!(loaded.diagnostics.is_empty());
        assert_eq!(loaded.program.modules, vec!["students", "reports.text"]);
        assert_eq!(
            names(&loaded.program),
            vec![
                "reports.text.TITLE",
                "use@students",
                "students.count",
                "students.Student",
                "use@",
                "main"
            ]
        );
        let _ = std::fs::remove_dir_all(dir);
    }

    #[test]
    fn spans_point_into_the_right_file() {
        let dir = project(&[
            ("main.elo", "use app.lib\nfn main() {}\n"),
            ("lib.elo", "fn oops( {}\n"),
        ]);
        let loaded = load(&dir.join("main.elo"), &dir).unwrap();
        assert_eq!(loaded.diagnostics.len(), 1);
        let (file, _) = loaded
            .sources
            .file_at(loaded.diagnostics[0].label.span.start as usize);
        assert!(file.name.ends_with("lib.elo"), "{}", file.name);
        let _ = std::fs::remove_dir_all(dir);
    }

    #[test]
    fn missing_and_cyclic_modules() {
        let dir = project(&[
            ("main.elo", "use app.a\nuse app.missing\nfn main() {}\n"),
            ("a.elo", "use app.b\npub fn fa() {}\n"),
            ("b.elo", "use app.a\npub fn fb() {}\n"),
        ]);
        let loaded = load(&dir.join("main.elo"), &dir).unwrap();
        assert_eq!(loaded.program.modules, vec!["a", "b"]);
        let _ = std::fs::remove_dir_all(dir);
    }
}
