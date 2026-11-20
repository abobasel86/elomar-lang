//! Modules (spec §11.2): `use app.students` makes the `pub` items of
//! `src/students.elo` available as `students.count`, `students.Student`,
//! `students.Level.High`.
//!
//! The loader has already renamed every module item to `module.name`. The
//! checker resolves each reference against the module it appears in and
//! rewrites it to that full name, so the interpreter sees one flat program.

use std::collections::{HashMap, HashSet};

use elomar_ast::*;
use elomar_diagnostics::{Diagnostic, Text};

use crate::{Checker, suggest};

/// The module part of an item's full name: `students` for `students.count`,
/// empty for an item of the main file.
pub(crate) fn module_of(name: &str) -> &str {
    name.rsplit_once('.').map_or("", |(m, _)| m)
}

/// The name as written in its own file: `count` for `students.count`.
pub(crate) fn base_name(name: &str) -> &str {
    name.rsplit_once('.').map_or(name, |(_, n)| n)
}

/// Names `use std.*` binds; an app module cannot reuse them as an alias.
const STD_SHORT_NAMES: [&str; 17] = [
    "ai", "fs", "json", "http", "db", "py", "mcp", "agent", "math", "random", "time", "os",
    "regex", "encoding", "crypto", "server", "process",
];

#[derive(Default)]
pub(crate) struct ModuleScopes {
    /// Module being checked; empty for the main file.
    pub current: String,
    /// Per module: `std` modules it imported, by short name (`ai`, `fs`).
    pub std_imports: HashMap<String, HashSet<String>>,
    /// Per module: alias → imported app module (`students` → `students`).
    pub app_aliases: HashMap<String, HashMap<String, String>>,
    pub loaded: HashSet<String>,
    /// Full names of `pub` items.
    pub public: HashSet<String>,
    /// Full names of declared type aliases.
    pub type_names: HashSet<String>,
    /// Full names of declared traits.
    pub trait_names: HashSet<String>,
    /// Full names of every top-level item, known before checking starts.
    pub all_items: HashSet<String>,
    /// Per module: name → full name, from `use app.x.{name}`.
    pub imported_names: HashMap<String, HashMap<String, String>>,
    /// Package names (from `[dependencies]`).
    pub packages: Vec<String>,
}

impl Checker {
    pub(crate) fn collect_module_info(&mut self, program: &Program) {
        self.scopes_mod.loaded = program.modules.iter().cloned().collect();
        self.scopes_mod.packages = program.packages.clone();
        for item in &program.items {
            let (name, is_pub) = match item {
                Item::Fn(f) => (&f.name.name, f.is_pub),
                Item::Type(t) => {
                    self.scopes_mod.type_names.insert(t.name.name.clone());
                    (&t.name.name, t.is_pub)
                }
                Item::Enum(e) => (&e.name.name, e.is_pub),
                Item::Const(c) => (&c.name.name, c.is_pub),
                Item::Trait(t) => {
                    self.scopes_mod.trait_names.insert(t.name.name.clone());
                    (&t.name.name, t.is_pub)
                }
                Item::Use(_) | Item::Impl(_) => continue,
            };
            self.scopes_mod.all_items.insert(name.clone());
            if is_pub {
                self.scopes_mod.public.insert(name.clone());
            }
        }
    }

    pub(crate) fn enter_module_of(&mut self, item_name: &str) {
        self.scopes_mod.current = module_of(item_name).to_string();
    }

    pub(crate) fn std_imported(&self, short: &str) -> bool {
        self.scopes_mod
            .std_imports
            .get(&self.scopes_mod.current)
            .is_some_and(|s| s.contains(short))
    }

    pub(crate) fn import_std(&mut self, module: &str, short: &str) {
        self.scopes_mod
            .std_imports
            .entry(module.to_string())
            .or_default()
            .insert(short.to_string());
    }

    /// `use app.a.b`: binds alias `b` to module `a.b` in the `use`'s module.
    pub(crate) fn check_app_use(&mut self, u: &UseDecl) {
        if u.path.len() < 2 {
            self.error(
                "E1011",
                u.span,
                Text::new(
                    "`use app` needs a module name, e.g. `use app.students`",
                    "يحتاج `use app` إلى اسم وحدة، مثل `use app.students`",
                ),
            );
            return;
        }
        let target = u.path[1..]
            .iter()
            .map(|i| i.name.as_str())
            .collect::<Vec<_>>()
            .join(".");
        let alias = &u.path[u.path.len() - 1];
        if !self.scopes_mod.loaded.contains(&target)
            && let Some((package, rest)) = target.split_once('.')
            && self.scopes_mod.packages.iter().any(|p| p == package)
        {
            let file = format!("src/{}.elo", rest.replace('.', "/"));
            self.push(
                Diagnostic::error(
                    "E1011",
                    u.span,
                    Text::new(
                        format!("package `{package}` has no module `{rest}`"),
                        format!("الحزمة `{package}` ليس فيها الوحدة `{rest}`"),
                    ),
                )
                .with_hint(Text::new(
                    format!("expected the file `{file}` in the package folder"),
                    format!("المتوقع وجود الملف `{file}` في مجلد الحزمة"),
                )),
            );
            return;
        }
        if !self.scopes_mod.loaded.contains(&target) {
            let file = format!("src/{}.elo", target.replace('.', "/"));
            self.push(
                Diagnostic::error(
                    "E1011",
                    u.span,
                    Text::new(
                        format!("cannot find module `app.{target}`"),
                        format!("تعذّر العثور على الوحدة `app.{target}`"),
                    ),
                )
                .with_hint(Text::new(
                    format!(
                        "expected the file `{file}` in the project folder (where elomar.toml is)"
                    ),
                    format!("المتوقع وجود الملف `{file}` في مجلد المشروع (حيث elomar.toml)"),
                )),
            );
            return;
        }
        if !u.names.is_empty() {
            self.import_names(u, &target);
            return;
        }
        let aliases = self
            .scopes_mod
            .app_aliases
            .entry(u.module.clone())
            .or_default();
        let clash = STD_SHORT_NAMES.contains(&alias.name.as_str())
            || aliases.get(&alias.name).is_some_and(|t| *t != target);
        if clash {
            self.error(
                "E1002",
                alias.span,
                Text::new(
                    format!(
                        "the name `{}` is already used by another import",
                        alias.name
                    ),
                    format!("الاسم `{}` مستخدم مسبقًا لاستيراد آخر", alias.name),
                ),
            );
            return;
        }
        aliases.insert(alias.name.clone(), target);
    }

    /// `use app.x.{a, b}`: binds `a` and `b` in the `use`'s module.
    fn import_names(&mut self, u: &UseDecl, target: &str) {
        let alias = &u.path[u.path.len() - 1].name;
        for name in &u.names {
            let full = format!("{target}.{}", name.name);
            if !self.scopes_mod.all_items.contains(&full) {
                let available: Vec<String> = self
                    .scopes_mod
                    .public
                    .iter()
                    .filter(|n| module_of(n) == target)
                    .map(|n| base_name(n).to_string())
                    .collect();
                let mut diag = Diagnostic::error(
                    "E1008",
                    name.span,
                    Text::new(
                        format!("module `{alias}` has no item `{}`", name.name),
                        format!("الوحدة `{alias}` ليس فيها العنصر `{}`", name.name),
                    ),
                );
                if let Some(s) = suggest(&name.name, &available) {
                    diag = diag.with_hint(Text::new(
                        format!("did you mean `{s}`?"),
                        format!("هل تقصد `{s}`؟"),
                    ));
                }
                self.push(diag);
                continue;
            }
            if !self.scopes_mod.public.contains(&full) {
                self.push(
                    Diagnostic::error(
                        "E1012",
                        name.span,
                        Text::new(
                            format!("`{}` is private to module `{alias}`", name.name),
                            format!("`{}` خاص بالوحدة `{alias}`", name.name),
                        ),
                    )
                    .with_hint(Text::new(
                        format!(
                            "mark it `pub` in src/{}.elo to use it from other files",
                            target.replace('.', "/")
                        ),
                        format!(
                            "علّمه بـ `pub` في src/{}.elo لاستخدامه من ملفات أخرى",
                            target.replace('.', "/")
                        ),
                    )),
                );
                continue;
            }
            let own = if u.module.is_empty() {
                name.name.clone()
            } else {
                format!("{}.{}", u.module, name.name)
            };
            let names = self
                .scopes_mod
                .imported_names
                .entry(u.module.clone())
                .or_default();
            let clash = self.scopes_mod.all_items.contains(&own)
                || names.get(&name.name).is_some_and(|f| *f != full);
            if clash {
                self.error(
                    "E1002",
                    name.span,
                    Text::new(
                        format!("the name `{}` is already defined in this file", name.name),
                        format!("الاسم `{}` معرّف مسبقًا في هذا الملف", name.name),
                    ),
                );
                continue;
            }
            names.insert(name.name.clone(), full);
        }
    }

    fn item_exists(&self, full: &str) -> bool {
        self.fns.contains_key(full)
            || self.consts.contains_key(full)
            || self.enums.contains_key(full)
            || self.scopes_mod.type_names.contains(full)
            || self.traits.contains_key(full)
            || self.scopes_mod.trait_names.contains(full)
    }

    /// The full name an unqualified `name` refers to in the current module:
    /// its own item if it has one, else `name` (prelude or main-file names).
    pub(crate) fn local_item(&self, name: &str) -> String {
        let module = &self.scopes_mod.current;
        if name.contains('.') {
            return name.to_string();
        }
        if !module.is_empty() {
            let full = format!("{module}.{name}");
            if self.item_exists(&full) {
                return full;
            }
        }
        // Names imported with `use app.x.{name}`.
        if let Some(full) = self
            .scopes_mod
            .imported_names
            .get(module)
            .and_then(|names| names.get(name))
        {
            return full.clone();
        }
        name.to_string()
    }

    /// The app module that `alias` names here, unless a variable or
    /// constant of that name hides it.
    pub(crate) fn app_target(&self, alias: &str) -> Option<String> {
        if self.lookup_local(alias).is_some() || self.consts.contains_key(&self.local_item(alias)) {
            return None;
        }
        self.scopes_mod
            .app_aliases
            .get(&self.scopes_mod.current)?
            .get(alias)
            .cloned()
    }

    /// Resolves `alias.member`. Reports `E1008` (no such item) or `E1012`
    /// (not `pub`) and returns `None` in those cases.
    fn qualified(&mut self, alias: &str, target: &str, member: &Ident) -> Option<String> {
        let full = format!("{target}.{}", member.name);
        if !self.item_exists(&full) {
            let available: Vec<String> = self
                .scopes_mod
                .public
                .iter()
                .filter(|n| crate::modules::module_of(n) == target)
                .map(|n| base_name(n).to_string())
                .collect();
            let mut diag = Diagnostic::error(
                "E1008",
                member.span,
                Text::new(
                    format!("module `{alias}` has no item `{}`", member.name),
                    format!("الوحدة `{alias}` ليس فيها العنصر `{}`", member.name),
                ),
            );
            if let Some(s) = suggest(&member.name, &available) {
                diag = diag.with_hint(Text::new(
                    format!("did you mean `{alias}.{s}`?"),
                    format!("هل تقصد `{alias}.{s}`؟"),
                ));
            }
            self.push(diag);
            return None;
        }
        if !self.scopes_mod.public.contains(&full) {
            self.push(
                Diagnostic::error(
                    "E1012",
                    member.span,
                    Text::new(
                        format!("`{}` is private to module `{alias}`", member.name),
                        format!("`{}` خاص بالوحدة `{alias}`", member.name),
                    ),
                )
                .with_hint(Text::new(
                    format!(
                        "mark it `pub` in src/{}.elo to use it from other files",
                        target.replace('.', "/")
                    ),
                    format!(
                        "علّمه بـ `pub` في src/{}.elo لاستخدامه من ملفات أخرى",
                        target.replace('.', "/")
                    ),
                )),
            );
            return None;
        }
        Some(full)
    }

    /// Resolves a type name as written (`Student` or `students.Student`) to
    /// its full name. With `report`, unknown modules and private types are
    /// reported and give `None`.
    pub(crate) fn resolve_type_name(&mut self, name: &Ident, report: bool) -> Option<String> {
        let Some((alias, member)) = name.name.split_once('.') else {
            return Some(self.local_item(&name.name));
        };
        let target = self
            .scopes_mod
            .app_aliases
            .get(&self.scopes_mod.current)
            .and_then(|a| a.get(alias))
            .cloned();
        let Some(target) = target else {
            if report {
                self.push(
                    Diagnostic::error(
                        "E1006",
                        name.span,
                        Text::new(
                            format!("unknown type `{}`", name.name),
                            format!("نوع غير معروف `{}`", name.name),
                        ),
                    )
                    .with_hint(Text::new(
                        format!("to use a type from another file, add `use app.{alias}`"),
                        format!("لاستخدام نوع من ملف آخر أضف `use app.{alias}`"),
                    )),
                );
            }
            return None;
        };
        let member = Ident {
            name: member.to_string(),
            span: name.span,
        };
        if report {
            self.qualified(alias, &target, &member)
        } else {
            Some(format!("{target}.{}", member.name))
        }
    }

    /// Rewrites references to module items in `kind` (and in the target of
    /// a field access or call) to full names: `count` inside `students`
    /// becomes `students.count`, and `students.count` written in another
    /// file becomes the name `students.count`. Returns `true` if an error
    /// was reported, so the caller can stop checking this expression.
    pub(crate) fn resolve_refs(&mut self, kind: &mut ExprKind) -> bool {
        match kind {
            ExprKind::Name(n) => {
                if self.lookup_local(n).is_none() {
                    let full = self.local_item(n);
                    if full != *n {
                        *n = full;
                    }
                }
                false
            }
            ExprKind::Field { target, name } => {
                if let ExprKind::Name(alias) = &target.kind
                    && let Some(module) = self.app_target(alias)
                {
                    let alias = alias.clone();
                    return match self.qualified(&alias, &module, name) {
                        Some(full) => {
                            *kind = ExprKind::Name(full);
                            false
                        }
                        None => true,
                    };
                }
                self.resolve_refs(&mut target.kind)
            }
            ExprKind::Call { callee, .. } => self.resolve_refs(&mut callee.kind),
            _ => false,
        }
    }
}
