//! Traits, `impl` blocks and methods (v0.3).
//!
//! - `impl` blocks are desugared before checking: each method becomes an
//!   ordinary function named `Type::method` (`students.Student::describe`
//!   inside a module) whose first parameter is `self`.
//! - `x.describe()` on a value whose type has such a method is rewritten
//!   to a call `Student::describe(x)`, so the interpreter needs no types.
//! - A trait bound `fn show<T: Describe>(x: T)` adds a hidden parameter per
//!   trait method (a "dictionary" of functions). Each call passes the
//!   implementations for the actual type, and `x.describe()` inside calls
//!   the hidden parameter. Calling a function value costs no effect; the
//!   implementation's effects are charged where it is passed.

use std::collections::HashSet;

use elomar_ast::*;
use elomar_diagnostics::{Diagnostic, Text};

use crate::{Checker, Ty, modules};

#[derive(Debug, Clone)]
pub(crate) struct TraitSig {
    pub name: String,
    /// Parameters after `self`; `Self` is `Ty::Param("Self")`.
    pub params: Vec<Ty>,
    pub ret: Ty,
    pub is_async: bool,
}

#[derive(Debug, Clone)]
pub(crate) struct TraitInfo {
    pub methods: Vec<TraitSig>,
}

/// A method callable as `value.name(...)` on values of type `target`.
#[derive(Debug, Clone)]
pub(crate) struct MethodEntry {
    pub target: Ty,
    pub name: String,
    pub fn_name: String,
    pub span: Span,
}

/// One `impl` block after desugaring.
#[derive(Debug, Clone)]
pub(crate) struct ImplRecord {
    pub trait_name: Option<Ident>,
    pub target: TypeExpr,
    pub module: String,
    /// (method name, function name, name span)
    pub methods: Vec<(String, String, Span)>,
    pub span: Span,
}

/// The hidden parameter holding method `method` of trait `tr` for type
/// parameter `param`. Users cannot write this name.
pub(crate) fn dict_name(param: &str, tr: &str, method: &str) -> String {
    format!("{param}: {tr}.{method}")
}

/// The default methods of the trait an impl names: by its full name, by its
/// name inside the impl's module, or by a base name only one trait has.
fn find_trait<'a>(
    defaults: &'a [(String, Vec<TraitMethod>)],
    written: &str,
    module: &str,
) -> Option<&'a Vec<TraitMethod>> {
    let in_module = format!("{module}.{written}");
    if let Some((_, m)) = defaults
        .iter()
        .find(|(full, _)| *full == written || (!module.is_empty() && *full == in_module))
    {
        return Some(m);
    }
    let base = written.rsplit('.').next().unwrap_or(written);
    let mut same_base = defaults
        .iter()
        .filter(|(full, _)| full.rsplit('.').next() == Some(base));
    match (same_base.next(), same_base.next()) {
        (Some((_, m)), None) => Some(m),
        _ => None,
    }
}

/// The written name of an impl's target, for its methods' function names.
fn target_base(t: &TypeExpr) -> String {
    match &t.kind {
        TypeExprKind::Named { name, .. } => modules::base_name(&name.name).to_string(),
        TypeExprKind::Record(_) => "record".into(),
        TypeExprKind::Fn { .. } => "fn".into(),
        TypeExprKind::Tuple(_) => "tuple".into(),
    }
}

/// Replaces `Self` in a method signature by the impl's target.
fn replace_self(t: &mut TypeExpr, target: &TypeExpr) {
    match &mut t.kind {
        TypeExprKind::Named { name, args } => {
            if name.name == "Self" && args.is_empty() {
                *t = TypeExpr {
                    kind: target.kind.clone(),
                    span: t.span,
                };
                return;
            }
            for a in args {
                replace_self(a, target);
            }
        }
        TypeExprKind::Record(fields) => {
            for (_, f) in fields {
                replace_self(f, target);
            }
        }
        TypeExprKind::Fn { params, ret } => {
            for p in params {
                replace_self(p, target);
            }
            replace_self(ret, target);
        }
        TypeExprKind::Tuple(items) => {
            for t in items {
                replace_self(t, target);
            }
        }
    }
}

impl Checker {
    /// Turns every `impl` method into a top-level function.
    pub(crate) fn desugar_impls(&mut self, program: &mut Program) {
        let mut taken: HashSet<String> = program
            .items
            .iter()
            .filter_map(|i| match i {
                Item::Fn(f) => Some(f.name.name.clone()),
                _ => None,
            })
            .collect();
        // Default method bodies by trait (full name).
        let defaults: Vec<(String, Vec<TraitMethod>)> = program
            .items
            .iter()
            .filter_map(|i| match i {
                Item::Trait(t) => Some((
                    t.name.name.clone(),
                    t.methods
                        .iter()
                        .filter(|m| m.body.is_some())
                        .cloned()
                        .collect(),
                )),
                _ => None,
            })
            .collect();
        let mut out = Vec::with_capacity(program.items.len());
        for item in std::mem::take(&mut program.items) {
            let Item::Impl(mut imp) = item else {
                out.push(item);
                continue;
            };
            if let Some(written) = &imp.trait_name
                && let Some(methods) = find_trait(&defaults, &written.name, &imp.module)
            {
                for m in methods {
                    if imp.methods.iter().any(|f| f.name.name == m.name.name) {
                        continue;
                    }
                    imp.methods.push(FnDecl {
                        is_pub: true,
                        is_async: m.is_async,
                        name: m.name.clone(),
                        type_params: Vec::new(),
                        bounds: Vec::new(),
                        params: m.params.clone(),
                        ret: m.ret.clone(),
                        effects: m.effects.clone(),
                        body: m.body.clone().expect("default method"),
                        span: m.span,
                    });
                }
            }
            let prefix = if imp.module.is_empty() {
                String::new()
            } else {
                format!("{}.", imp.module)
            };
            let base = target_base(&imp.target);
            let mut methods = Vec::new();
            for mut f in imp.methods {
                let method = f.name.name.clone();
                let mut full = format!("{prefix}{base}::{method}");
                let mut n = 2;
                while taken.contains(&full) {
                    full = format!("{prefix}{base}::{method}#{n}");
                    n += 1;
                }
                taken.insert(full.clone());
                for p in &mut f.params {
                    replace_self(&mut p.ty, &imp.target);
                }
                if let Some(r) = &mut f.ret {
                    replace_self(r, &imp.target);
                }
                methods.push((method, full.clone(), f.name.span));
                f.name.name = full;
                out.push(Item::Fn(f));
            }
            self.impls.push(ImplRecord {
                trait_name: imp.trait_name,
                target: imp.target,
                module: imp.module,
                methods,
                span: imp.span,
            });
        }
        program.items = out;
    }

    pub(crate) fn collect_traits(&mut self, program: &Program) {
        for item in &program.items {
            let Item::Trait(t) = item else { continue };
            self.enter_module_of(&t.name.name);
            let name = &t.name.name;
            if self.traits.contains_key(name)
                || self.enums.contains_key(name)
                || self.scopes_mod.type_names.contains(name)
            {
                self.error(
                    "E1002",
                    t.name.span,
                    Text::new(
                        format!("`{name}` is defined more than once"),
                        format!("`{name}` معرّف أكثر من مرة"),
                    ),
                );
                continue;
            }
            let saved = std::mem::replace(&mut self.type_params, vec!["Self".into()]);
            let mut methods: Vec<TraitSig> = Vec::new();
            for m in &t.methods {
                if methods.iter().any(|s| s.name == m.name.name) {
                    self.error(
                        "E1002",
                        m.name.span,
                        Text::new(
                            format!("method `{}` is listed twice", m.name.name),
                            format!("الدالة `{}` مذكورة مرتين", m.name.name),
                        ),
                    );
                    continue;
                }
                let params = m.params[1..]
                    .iter()
                    .map(|p| self.lower_type(&p.ty))
                    .collect();
                let ret = m.ret.as_ref().map_or(Ty::Unit, |r| self.lower_type(r));
                methods.push(TraitSig {
                    name: m.name.name.clone(),
                    params,
                    ret,
                    is_async: m.is_async,
                });
            }
            self.type_params = saved;
            self.type_spans.insert(name.clone(), t.name.span);
            self.note_text(t.name.span, format!("trait {}", modules::base_name(name)));
            self.traits.insert(name.clone(), TraitInfo { methods });
        }
    }

    /// The full name of the trait `id` refers to here, or `E1006`.
    pub(crate) fn resolve_trait_name(&mut self, id: &Ident) -> Option<String> {
        let full = match id.name.rsplit_once('.') {
            Some((alias, member)) => self
                .app_target(alias)
                .map_or_else(|| id.name.clone(), |m| format!("{m}.{member}")),
            None => self.local_item(&id.name),
        };
        if self.traits.contains_key(&full) {
            if let Some(def) = self.type_spans.get(&full).copied() {
                self.note_def(id.span, def);
            }
            return Some(full);
        }
        let mut diag = Diagnostic::error(
            "E1006",
            id.span,
            Text::new(
                format!("unknown trait `{}`", id.name),
                format!("واجهة غير معروفة `{}`", id.name),
            ),
        );
        let known: Vec<String> = self.traits.keys().cloned().collect();
        if let Some(s) = crate::suggest(&id.name, &known) {
            diag = diag.with_hint(Text::new(
                format!("did you mean `{s}`?"),
                format!("هل تقصد `{s}`؟"),
            ));
        }
        self.push(diag);
        None
    }

    /// Bounds of a generic function: `(type parameter, trait full name)`.
    pub(crate) fn resolve_bounds(&mut self, f: &FnDecl) -> Vec<(String, String)> {
        let mut out = Vec::new();
        for (p, tr) in &f.bounds {
            if !f.type_params.iter().any(|t| t.name == p.name) {
                self.error(
                    "E1006",
                    p.span,
                    Text::new(
                        format!("unknown type parameter `{}`", p.name),
                        format!("معامل نوع غير معروف `{}`", p.name),
                    ),
                );
                continue;
            }
            if let Some(full) = self.resolve_trait_name(tr)
                && !out.contains(&(p.name.clone(), full.clone()))
            {
                out.push((p.name.clone(), full));
            }
        }
        out
    }

    /// Registers methods and checks each `impl Trait for Type` against its
    /// trait. Runs after all function signatures are known.
    pub(crate) fn resolve_impls(&mut self) {
        let impls = self.impls.clone();
        for (idx, imp) in impls.iter().enumerate() {
            self.scopes_mod.current = imp.module.clone();
            let target = self.lower_type(&imp.target);
            if target == Ty::Error {
                continue;
            }
            let shown = self.subst.show(&target);
            let tr = match &imp.trait_name {
                Some(id) => match self.resolve_trait_name(id) {
                    Some(tr) => Some(tr),
                    None => continue,
                },
                None => None,
            };
            if let Some(tr) = &tr
                && self
                    .trait_impls
                    .iter()
                    .any(|(t, ty, _)| t == tr && *ty == target)
            {
                self.error(
                    "E1002",
                    imp.span,
                    Text::new(
                        format!("`{tr}` is already implemented for `{shown}`"),
                        format!("الواجهة `{tr}` منفذة مسبقًا للنوع `{shown}`"),
                    ),
                );
                continue;
            }
            for (method, fn_name, span) in &imp.methods {
                if let Some(prev) = self
                    .methods
                    .iter()
                    .find(|e| &e.name == method && e.target == target)
                {
                    let prev = prev.span;
                    self.push(
                        Diagnostic::error(
                            "E1002",
                            *span,
                            Text::new(
                                format!("type `{shown}` already has a method `{method}`"),
                                format!("النوع `{shown}` له دالة باسم `{method}` مسبقًا"),
                            ),
                        )
                        .with_note(prev, Text::new("first defined here", "عُرّفت أول مرة هنا")),
                    );
                    continue;
                }
                self.methods.push(MethodEntry {
                    target: target.clone(),
                    name: method.clone(),
                    fn_name: fn_name.clone(),
                    span: *span,
                });
            }
            let Some(tr) = tr else {
                continue;
            };
            self.check_impl_against_trait(imp, &tr, &target);
            self.trait_impls.push((tr, target, idx));
        }
    }

    fn check_impl_against_trait(&mut self, imp: &ImplRecord, tr: &str, target: &Ty) {
        let info = self.traits[tr].clone();
        let shown = self.subst.show(target);
        let self_map = [("Self".to_string(), target.clone())];
        for sig in &info.methods {
            let Some((_, fn_name, span)) = imp.methods.iter().find(|(m, _, _)| *m == sig.name)
            else {
                self.push(
                    Diagnostic::error(
                        "E1014",
                        imp.span,
                        Text::new(
                            format!("`impl {tr} for {shown}` is missing method `{}`", sig.name),
                            format!("`impl {tr} for {shown}` ينقصها الدالة `{}`", sig.name),
                        ),
                    )
                    .with_hint(Text::new(
                        format!(
                            "add `fn {}(self, ...)` with the signature from the trait",
                            sig.name
                        ),
                        format!(
                            "أضف `fn {}(self, ...)` بالتوقيع المذكور في الواجهة",
                            sig.name
                        ),
                    )),
                );
                continue;
            };
            let Some(have) = self.fns.get(fn_name).cloned() else {
                continue;
            };
            let want_params: Vec<Ty> = sig.params.iter().map(|t| t.substitute(&self_map)).collect();
            let want_ret = sig.ret.substitute(&self_map);
            let have_params: Vec<Ty> = have.params[1..].iter().map(|(_, t)| t.clone()).collect();
            if have_params != want_params || have.ret != want_ret || have.is_async != sig.is_async {
                let want = Ty::Fn(want_params, Box::new(want_ret));
                let found = Ty::Fn(have_params, Box::new(have.ret.clone()));
                let (w, f) = (self.subst.show(&want), self.subst.show(&found));
                self.error(
                    "E2001",
                    *span,
                    Text::new(
                        format!("method `{}` does not match trait `{tr}`: expected `{w}`, found `{f}` (after `self`)", sig.name),
                        format!("الدالة `{}` لا تطابق الواجهة `{tr}`: المتوقع `{w}` والموجود `{f}` (بعد `self`)", sig.name),
                    ),
                );
            }
        }
        for (m, _, span) in &imp.methods {
            if !info.methods.iter().any(|s| &s.name == m) {
                self.push(
                    Diagnostic::error(
                        "E1008",
                        *span,
                        Text::new(
                            format!("trait `{tr}` has no method `{m}`"),
                            format!("الواجهة `{tr}` ليس فيها الدالة `{m}`"),
                        ),
                    )
                    .with_hint(Text::new(
                        format!("put other methods in a separate `impl {shown} {{ ... }}`"),
                        format!("ضع الدوال الأخرى في `impl {shown} {{ ... }}` مستقلة"),
                    )),
                );
            }
        }
    }

    /// Adds the hidden dictionary parameters of bounded generic functions.
    pub(crate) fn add_dictionary_params(&mut self, program: &mut Program) {
        for item in &mut program.items {
            let Item::Fn(f) = item else { continue };
            let Some(sig) = self.fns.get(&f.name.name) else {
                continue;
            };
            for (p, tr) in &sig.bounds {
                for m in &self.traits[tr].methods {
                    let span = f.name.span;
                    f.params.push(Param {
                        name: Ident {
                            name: dict_name(p, tr, &m.name),
                            span,
                        },
                        ty: TypeExpr {
                            kind: TypeExprKind::Named {
                                name: Ident {
                                    name: "Unit".into(),
                                    span,
                                },
                                args: Vec::new(),
                            },
                            span,
                        },
                    });
                }
            }
        }
    }

    /// Whether a value of type `recv` can use methods defined for `target`.
    /// Unknown parts of `recv` match anything.
    pub(crate) fn ty_matches(&self, target: &Ty, recv: &Ty) -> bool {
        let recv = self.subst.shallow(recv);
        match (target, &recv) {
            (_, Ty::Var(_)) | (Ty::Error, _) | (_, Ty::Error) => true,
            (Ty::List(a), Ty::List(b))
            | (Ty::Option(a), Ty::Option(b))
            | (Ty::Future(a), Ty::Future(b))
            | (Ty::Set(a), Ty::Set(b)) => self.ty_matches(a, b),
            (Ty::Result(a, b), Ty::Result(c, d)) | (Ty::Map(a, b), Ty::Map(c, d)) => {
                self.ty_matches(a, c) && self.ty_matches(b, d)
            }
            (Ty::Enum(n, a), Ty::Enum(m, b)) => {
                n == m && a.len() == b.len() && a.iter().zip(b).all(|(x, y)| self.ty_matches(x, y))
            }
            (Ty::Record(f), Ty::Record(g)) => {
                f.len() == g.len()
                    && f.iter()
                        .zip(g)
                        .all(|((n, x), (m, y))| n == m && self.ty_matches(x, y))
            }
            (Ty::Fn(p, r), Ty::Fn(q, s)) => {
                p.len() == q.len()
                    && p.iter().zip(q).all(|(x, y)| self.ty_matches(x, y))
                    && self.ty_matches(r, s)
            }
            (a, b) => a == b,
        }
    }

    /// `value.method(args)` for a method from an `impl`, or a trait method
    /// on a bounded type parameter. Rewrites the call into a plain call
    /// with the value as the first argument.
    pub(crate) fn try_user_method(
        &mut self,
        callee: &mut Expr,
        recv_ty: &Ty,
        method: &Ident,
        args: &mut Vec<Arg>,
        span: Span,
    ) -> Option<Ty> {
        let recv = self.subst.shallow(recv_ty);
        let (fn_ref, ty) = match &recv {
            Ty::Var(_) | Ty::Error => return None,
            Ty::Param(p) => {
                let (tr, sig) = self.bounds.iter().find_map(|(bp, tr)| {
                    (bp == p)
                        .then(|| self.traits.get(tr))
                        .flatten()
                        .and_then(|info| info.methods.iter().find(|s| s.name == method.name))
                        .map(|s| (tr.clone(), s.clone()))
                })?;
                let map = [("Self".to_string(), recv.clone())];
                let params: Vec<Ty> = sig.params.iter().map(|t| t.substitute(&map)).collect();
                self.reject_named_args(args);
                let what = format!("{p}.{}", method.name);
                if self.arity(&what, args, params.len(), params.len(), span) {
                    for (arg, t) in args.iter_mut().zip(&params) {
                        self.check_expr_against(&mut arg.value, t);
                    }
                }
                let ret = sig.ret.substitute(&map);
                let ret = if sig.is_async { Ty::future(ret) } else { ret };
                (dict_name(p, &tr, &method.name), ret)
            }
            _ => {
                let entry = self
                    .methods
                    .iter()
                    .find(|e| e.name == method.name && self.ty_matches(&e.target, &recv))
                    .cloned()?;
                self.subst.unify(&recv, &entry.target);
                self.note_fn(method.span, &entry.fn_name);
                self.note_def(method.span, entry.span);
                let mut sig = self.fns.get(&entry.fn_name)?.clone();
                sig.params.remove(0);
                let ty = self.check_user_call(&entry.fn_name, &sig, args, span, method.span);
                (entry.fn_name, ty)
            }
        };
        let ExprKind::Field { target, .. } =
            std::mem::replace(&mut callee.kind, ExprKind::Name(fn_ref))
        else {
            unreachable!("method calls have a field callee");
        };
        args.insert(
            0,
            Arg {
                name: None,
                value: *target,
            },
        );
        Some(ty)
    }

    /// After a call to a bounded generic function: appends, for each bound,
    /// the implementing functions as hidden arguments.
    pub(crate) fn pass_dictionaries(
        &mut self,
        callee: &str,
        bounds: &[(String, String)],
        map: &[(String, Ty)],
        args: &mut Vec<Arg>,
        span: Span,
    ) {
        for (p, tr) in bounds {
            let Some((_, t)) = map.iter().find(|(n, _)| n == p) else {
                continue;
            };
            let methods: Vec<String> = self.traits[tr]
                .methods
                .iter()
                .map(|m| m.name.clone())
                .collect();
            let resolved = self.subst.resolve(t);
            let names: Vec<String> = match self.subst.shallow(t) {
                Ty::Error => continue,
                Ty::Param(q) => {
                    if !self.bounds.contains(&(q.clone(), tr.clone())) {
                        self.push(
                            Diagnostic::error(
                                "E2019",
                                span,
                                Text::new(
                                    format!("`{callee}` needs `{p}` to implement `{tr}`, but `{q}` is not known to"),
                                    format!("`{callee}` تحتاج أن ينفذ `{p}` الواجهة `{tr}`، و`{q}` غير مضمون أن ينفذها"),
                                ),
                            )
                            .with_hint(Text::new(
                                format!("add the bound: `<{q}: {tr}>`"),
                                format!("أضف القيد: `<{q}: {tr}>`"),
                            )),
                        );
                        continue;
                    }
                    methods.iter().map(|m| dict_name(&q, tr, m)).collect()
                }
                Ty::Var(_) => {
                    self.push(
                        Diagnostic::error(
                            "E2020",
                            span,
                            Text::new(
                                format!("cannot tell which type `{p}` is in this call to `{callee}`"),
                                format!("لا يمكن معرفة النوع `{p}` في هذا الاستدعاء لـ `{callee}`"),
                            ),
                        )
                        .with_hint(Text::new(
                            format!("give the type explicitly: `{callee}<...>(...)`, or annotate the value"),
                            format!("اذكر النوع صراحة: `{callee}<...>(...)`، أو حدد نوع القيمة"),
                        )),
                    );
                    continue;
                }
                _ => {
                    let found = self
                        .trait_impls
                        .iter()
                        .find(|(t2, target, _)| t2 == tr && self.ty_matches(target, &resolved))
                        .map(|(_, target, idx)| (target.clone(), *idx));
                    let Some((target, idx)) = found else {
                        let shown = self.subst.show(&resolved);
                        self.push(
                            Diagnostic::error(
                                "E2019",
                                span,
                                Text::new(
                                    format!("type `{shown}` does not implement `{tr}`, which `{callee}` needs"),
                                    format!("النوع `{shown}` لا ينفذ الواجهة `{tr}` التي تحتاجها `{callee}`"),
                                ),
                            )
                            .with_hint(Text::new(
                                format!("add `impl {tr} for {shown} {{ ... }}`"),
                                format!("أضف `impl {tr} for {shown} {{ ... }}`"),
                            )),
                        );
                        continue;
                    };
                    self.subst.unify(t, &target);
                    let imp = &self.impls[idx];
                    let names: Vec<String> = methods
                        .iter()
                        .filter_map(|m| {
                            imp.methods
                                .iter()
                                .find(|(n, _, _)| n == m)
                                .map(|(_, f, _)| f.clone())
                        })
                        .collect();
                    if names.len() != methods.len() {
                        continue; // E1014 was reported on the impl.
                    }
                    for n in &names {
                        self.usage.calls.push((n.clone(), span));
                    }
                    names
                }
            };
            for n in names {
                args.push(Arg {
                    name: None,
                    value: Expr {
                        kind: ExprKind::Name(n),
                        span,
                    },
                });
            }
        }
    }
}
