//! Static checking for Elomar-Lang: name resolution, type inference with
//! unification, and effect inference/checking (spec §10 and §16.2).
//!
//! The checker may rewrite the AST in one way: an integer literal that is
//! expected to be a `Float` (e.g. `var total: Float = 0`) becomes a float
//! literal, so the interpreter needs no type information.

mod ai;
mod builtins;
mod expr;
mod ty;

use std::collections::{BTreeSet, HashMap, HashSet};

use elomar_ast::*;
use elomar_diagnostics::{Diagnostic, Text};

pub use builtins::{BUILTIN_FUNCTIONS, KNOWN_EFFECTS};
pub use ty::{Subst, Ty};

#[derive(Debug, Clone, Default)]
pub struct CheckOptions {
    /// Report `E1005` if there is no `main` function.
    pub require_main: bool,
    /// Models from `elomar.toml` whose provider sends data over the network.
    /// Calling them with `ai.infer` also performs the `net` effect.
    pub cloud_models: Vec<String>,
}

/// Checks a parsed program. Returns errors and warnings.
pub fn check(program: &mut Program, opts: CheckOptions) -> Vec<Diagnostic> {
    let mut checker = Checker::default();
    checker.run(program, opts);
    checker.diags
}

#[derive(Debug, Clone)]
struct FnSig {
    params: Vec<(String, Ty)>,
    ret: Ty,
    declared: Option<(BTreeSet<String>, Span)>,
    is_pub: bool,
    is_async: bool,
    name_span: Span,
}

#[derive(Debug, Clone)]
struct Local {
    ty: Ty,
    mutable: bool,
    span: Span,
    is_param: bool,
}

/// What a function or constant does: effects it performs itself and the
/// user functions it calls (for transitive effect inference).
#[derive(Debug, Default, Clone)]
struct Usage {
    direct: Vec<(String, Span)>,
    calls: Vec<(String, Span)>,
}

#[derive(Default)]
struct Checker {
    diags: Vec<Diagnostic>,
    subst: Subst,
    aliases: HashMap<String, Ty>,
    fns: HashMap<String, FnSig>,
    consts: HashMap<String, (Ty, Span)>,
    scopes: Vec<HashMap<String, Local>>,
    ret_ty: Option<Ty>,
    loop_depth: u32,
    /// Whether the function being checked is `async` (so `await` is allowed).
    in_async: bool,
    /// Short names of imported modules, e.g. `ai` for `use std.ai`.
    modules: HashSet<String>,
    cloud_models: Vec<String>,
    usage: Usage,
}

impl Checker {
    fn run(&mut self, program: &mut Program, opts: CheckOptions) {
        self.aliases.insert("AIError".into(), ai::ai_error_ty());
        self.cloud_models = opts.cloud_models.clone();
        for item in &program.items {
            if let Item::Use(u) = item {
                self.check_use(u);
            }
        }
        self.collect_types(program);
        self.collect_fns(program);

        let mut const_usage = Vec::new();
        for item in &mut program.items {
            if let Item::Const(c) = item {
                self.check_const(c);
                const_usage.push((c.name.clone(), std::mem::take(&mut self.usage)));
            }
        }

        let mut fn_usage: HashMap<String, Usage> = HashMap::new();
        for item in &mut program.items {
            if let Item::Fn(f) = item {
                self.check_fn(f);
                fn_usage.insert(f.name.name.clone(), std::mem::take(&mut self.usage));
            }
        }

        let effects = infer_effects(&fn_usage);
        self.check_fn_effects(&fn_usage, &effects);
        self.check_const_effects(&const_usage, &effects);

        match self.fns.get("main").cloned() {
            Some(main) => {
                if !main.params.is_empty() || !matches!(main.ret, Ty::Unit | Ty::Error) {
                    self.error(
                        "E2011",
                        main.name_span,
                        Text::new(
                            "`main` must take no parameters and return nothing",
                            "يجب ألا تأخذ `main` معاملات وألا تُرجع قيمة",
                        ),
                    );
                }
            }
            None if opts.require_main => {
                let span = Span::default();
                self.error(
                    "E1005",
                    span,
                    Text::new("no `main` function found", "لا توجد دالة `main`"),
                );
                if let Some(d) = self.diags.last_mut() {
                    d.hint = Some(Text::new(
                        "add `fn main() { ... }`; the program starts there",
                        "أضف `fn main() { ... }`؛ يبدأ تنفيذ البرنامج منها",
                    ));
                }
            }
            None => {}
        }
    }

    // ----- diagnostics helpers ---------------------------------------------

    fn error(&mut self, code: &'static str, span: Span, text: Text) {
        self.diags.push(Diagnostic::error(code, span, text));
    }

    fn push(&mut self, diag: Diagnostic) {
        self.diags.push(diag);
    }

    fn mismatch(&mut self, expected: &Ty, found: &Ty, span: Span) {
        let e = self.subst.show(expected);
        let f = self.subst.show(found);
        let mut diag = Diagnostic::error(
            "E2001",
            span,
            Text::new(
                format!("type mismatch: expected `{e}`, found `{f}`"),
                format!("عدم توافق الأنواع: المتوقع `{e}` لكن الموجود `{f}`"),
            ),
        )
        .with_label(Text::new(
            format!("expected `{e}`"),
            format!("المتوقع `{e}`"),
        ));
        let (re, rf) = (self.subst.resolve(expected), self.subst.resolve(found));
        if let (Ty::Record(want), Ty::Record(have)) = (&re, &rf) {
            let missing: Vec<_> = want
                .iter()
                .filter(|(n, _)| !have.iter().any(|(m, _)| m == n))
                .map(|(n, _)| format!("`{n}`"))
                .collect();
            let extra: Vec<_> = have
                .iter()
                .filter(|(n, _)| !want.iter().any(|(m, _)| m == n))
                .map(|(n, _)| format!("`{n}`"))
                .collect();
            if !missing.is_empty() || !extra.is_empty() {
                diag = diag.with_hint(Text::new(
                    format!(
                        "missing fields: [{}]; unexpected fields: [{}]",
                        missing.join(", "),
                        extra.join(", ")
                    ),
                    format!(
                        "حقول ناقصة: [{}]؛ حقول زائدة: [{}]",
                        missing.join(", "),
                        extra.join(", ")
                    ),
                ));
            }
        } else if matches!(re, Ty::Int) && matches!(rf, Ty::Float) {
            diag = diag.with_hint(Text::new(
                "convert explicitly with `to_int(x)` (truncates), or use `div` for integer division",
                "حوّل صراحةً بـ `to_int(x)` (يحذف الكسر)، أو استخدم `div` للقسمة الصحيحة",
            ));
        } else if matches!(re, Ty::Str) && !matches!(rf, Ty::Str | Ty::Error | Ty::Var(_)) {
            diag = diag.with_hint(Text::new(
                "convert with `to_string(x)`",
                "حوّل باستخدام `to_string(x)`",
            ));
        }
        self.push(diag);
    }

    /// Unifies `found` with `expected`, reporting `E2001` on failure.
    fn expect_ty(&mut self, expected: &Ty, found: &Ty, span: Span) {
        if !self.subst.unify(expected, found) {
            self.mismatch(expected, found, span);
        }
    }

    // ----- scopes ----------------------------------------------------------

    fn push_scope(&mut self) {
        self.scopes.push(HashMap::new());
    }

    fn pop_scope(&mut self) {
        self.scopes.pop();
    }

    fn declare(&mut self, name: &Ident, ty: Ty, mutable: bool, is_param: bool) {
        if CONSTRUCTORS.contains(&name.name.as_str()) || name.name == "_" {
            self.error(
                "E1002",
                name.span,
                Text::new(
                    format!(
                        "`{}` is reserved and cannot be used as a variable name",
                        name.name
                    ),
                    format!("الاسم `{}` محجوز ولا يمكن استخدامه اسمًا لمتغير", name.name),
                ),
            );
            return;
        }
        if let Some(prev) = self.scopes.last().and_then(|s| s.get(&name.name)) {
            let prev_span = prev.span;
            self.push(
                Diagnostic::error(
                    "E1002",
                    name.span,
                    Text::new(
                        format!("`{}` is already defined in this scope", name.name),
                        format!("الاسم `{}` معرّف مسبقًا في هذا النطاق", name.name),
                    ),
                )
                .with_note(
                    prev_span,
                    Text::new("first defined here", "عُرّف أول مرة هنا"),
                ),
            );
            return;
        }
        let shadowed = self
            .scopes
            .iter()
            .rev()
            .skip(1)
            .find_map(|s| s.get(&name.name))
            .filter(|l| !l.is_param)
            .map(|l| l.span);
        if let Some(prev_span) = shadowed {
            self.push(
                Diagnostic::warning(
                    "W1001",
                    name.span,
                    Text::new(
                        format!("`{}` shadows a variable from an outer scope", name.name),
                        format!("الاسم `{}` يحجب متغيرًا من نطاق خارجي", name.name),
                    ),
                )
                .with_note(
                    prev_span,
                    Text::new("outer variable defined here", "المتغير الخارجي معرّف هنا"),
                ),
            );
        }
        self.scopes.last_mut().expect("scope").insert(
            name.name.clone(),
            Local {
                ty,
                mutable,
                span: name.span,
                is_param,
            },
        );
    }

    fn lookup_local(&self, name: &str) -> Option<&Local> {
        self.scopes.iter().rev().find_map(|s| s.get(name))
    }

    /// Names visible at this point, for "did you mean" suggestions.
    fn visible_names(&self) -> Vec<String> {
        let mut names: Vec<String> = self
            .scopes
            .iter()
            .flat_map(|s| s.keys().cloned())
            .chain(self.consts.keys().cloned())
            .chain(self.fns.keys().cloned())
            .chain(BUILTIN_FUNCTIONS.iter().map(|s| s.to_string()))
            .chain(CONSTRUCTORS.iter().map(|s| s.to_string()))
            .chain(self.modules.iter().cloned())
            .collect();
        names.sort();
        names.dedup();
        names
    }

    fn unknown_name(&mut self, name: &str, span: Span) {
        let mut diag = Diagnostic::error(
            "E1001",
            span,
            Text::new(
                format!("unknown name `{name}`"),
                format!("اسم غير معروف `{name}`"),
            ),
        );
        if let Some(s) = suggest(name, &self.visible_names()) {
            diag = diag.with_hint(Text::new(
                format!("did you mean `{s}`?"),
                format!("هل تقصد `{s}`؟"),
            ));
        }
        self.push(diag);
    }

    // ----- items -----------------------------------------------------------

    fn collect_types(&mut self, program: &Program) {
        let decls: HashMap<String, &TypeDecl> = program
            .items
            .iter()
            .filter_map(|i| match i {
                Item::Type(t) => Some((t.name.name.clone(), t)),
                _ => None,
            })
            .collect();
        let mut seen: HashMap<String, Span> = HashMap::new();
        for item in &program.items {
            let Item::Type(t) = item else { continue };
            if builtins::is_builtin_type(&t.name.name) {
                self.error(
                    "E1002",
                    t.name.span,
                    Text::new(
                        format!(
                            "`{}` is a built-in type and cannot be redefined",
                            t.name.name
                        ),
                        format!("`{}` نوع أساسي ولا يمكن إعادة تعريفه", t.name.name),
                    ),
                );
                continue;
            }
            if let Some(prev) = seen.insert(t.name.name.clone(), t.name.span) {
                self.push(
                    Diagnostic::error(
                        "E1002",
                        t.name.span,
                        Text::new(
                            format!("type `{}` is defined more than once", t.name.name),
                            format!("النوع `{}` معرّف أكثر من مرة", t.name.name),
                        ),
                    )
                    .with_note(prev, Text::new("first defined here", "عُرّف أول مرة هنا")),
                );
                continue;
            }
            let mut stack = Vec::new();
            let ty = self.resolve_alias(&t.name.name, &decls, &mut stack);
            self.aliases.insert(t.name.name.clone(), ty);
        }
    }

    fn resolve_alias(
        &mut self,
        name: &str,
        decls: &HashMap<String, &TypeDecl>,
        stack: &mut Vec<String>,
    ) -> Ty {
        if let Some(ty) = self.aliases.get(name) {
            return ty.clone();
        }
        let decl = decls[name];
        if stack.iter().any(|s| s == name) {
            self.error(
                "E1006",
                decl.name.span,
                Text::new(
                    format!("type `{name}` refers to itself"),
                    format!("النوع `{name}` يشير إلى نفسه"),
                ),
            );
            return Ty::Error;
        }
        stack.push(name.to_string());
        let ty = self.lower_type_with(&decl.ty, &mut |c, n| {
            if decls.contains_key(n) {
                Some(c.resolve_alias(n, decls, stack))
            } else {
                c.aliases.get(n).cloned()
            }
        });
        stack.pop();
        self.aliases.insert(name.to_string(), ty.clone());
        ty
    }

    /// Converts a written type into a [`Ty`].
    fn lower_type(&mut self, t: &TypeExpr) -> Ty {
        self.lower_type_with(t, &mut |c, n| c.aliases.get(n).cloned())
    }

    fn lower_type_with(
        &mut self,
        t: &TypeExpr,
        alias: &mut dyn FnMut(&mut Checker, &str) -> Option<Ty>,
    ) -> Ty {
        match &t.kind {
            TypeExprKind::Record(fields) => {
                let mut out: Vec<(String, Ty)> = Vec::new();
                for (name, fty) in fields {
                    if out.iter().any(|(n, _)| n == &name.name) {
                        self.error(
                            "E1002",
                            name.span,
                            Text::new(
                                format!("field `{}` is listed twice", name.name),
                                format!("الحقل `{}` مذكور مرتين", name.name),
                            ),
                        );
                        continue;
                    }
                    let ty = self.lower_type_with(fty, alias);
                    out.push((name.name.clone(), ty));
                }
                Ty::record(out)
            }
            TypeExprKind::Named { name, args } => {
                let mut lowered: Vec<Ty> = args
                    .iter()
                    .map(|a| self.lower_type_with(a, alias))
                    .collect();
                let arity = match name.name.as_str() {
                    "Int" | "Float" | "Bool" | "String" | "Unit" | "Range" | "Duration" => 0,
                    "List" | "Option" | "Future" => 1,
                    "Result" => 2,
                    other => {
                        if let Some(ty) = alias(self, other) {
                            if !args.is_empty() {
                                self.wrong_type_args(name, 0, args.len(), t.span);
                            }
                            return ty;
                        }
                        let mut diag = Diagnostic::error(
                            "E1006",
                            name.span,
                            Text::new(
                                format!("unknown type `{other}`"),
                                format!("نوع غير معروف `{other}`"),
                            ),
                        );
                        let mut known: Vec<String> = builtins::BUILTIN_TYPES
                            .iter()
                            .map(|s| s.to_string())
                            .collect();
                        known.extend(self.aliases.keys().cloned());
                        if let Some(s) = suggest(other, &known) {
                            diag = diag.with_hint(Text::new(
                                format!("did you mean `{s}`?"),
                                format!("هل تقصد `{s}`؟"),
                            ));
                        }
                        self.push(diag);
                        return Ty::Error;
                    }
                };
                if lowered.len() != arity {
                    self.wrong_type_args(name, arity, lowered.len(), t.span);
                    return Ty::Error;
                }
                match name.name.as_str() {
                    "Int" => Ty::Int,
                    "Float" => Ty::Float,
                    "Bool" => Ty::Bool,
                    "String" => Ty::Str,
                    "Unit" => Ty::Unit,
                    "Range" => Ty::Range,
                    "Duration" => Ty::Duration,
                    "List" => Ty::list(lowered.remove(0)),
                    "Option" => Ty::option(lowered.remove(0)),
                    "Future" => Ty::future(lowered.remove(0)),
                    _ => {
                        let e = lowered.pop().expect("arity 2");
                        Ty::result(lowered.pop().expect("arity 2"), e)
                    }
                }
            }
        }
    }

    fn wrong_type_args(&mut self, name: &Ident, want: usize, got: usize, span: Span) {
        self.error(
            "E1006",
            span,
            Text::new(
                format!(
                    "type `{}` takes {want} type argument(s) but {got} were given",
                    name.name
                ),
                format!(
                    "النوع `{}` يأخذ {want} من معاملات النوع لكن أُعطي {got}",
                    name.name
                ),
            ),
        );
    }

    fn collect_fns(&mut self, program: &Program) {
        for item in &program.items {
            let Item::Fn(f) = item else { continue };
            let name = &f.name.name;
            if BUILTIN_FUNCTIONS.contains(&name.as_str()) || CONSTRUCTORS.contains(&name.as_str()) {
                self.error(
                    "E1002",
                    f.name.span,
                    Text::new(
                        format!("`{name}` is a built-in function and cannot be redefined"),
                        format!("`{name}` دالة مدمجة ولا يمكن إعادة تعريفها"),
                    ),
                );
                continue;
            }
            if let Some(prev) = self.fns.get(name) {
                let prev = prev.name_span;
                self.push(
                    Diagnostic::error(
                        "E1002",
                        f.name.span,
                        Text::new(
                            format!("function `{name}` is defined more than once"),
                            format!("الدالة `{name}` معرّفة أكثر من مرة"),
                        ),
                    )
                    .with_note(prev, Text::new("first defined here", "عُرّفت أول مرة هنا")),
                );
                continue;
            }
            let mut params: Vec<(String, Ty)> = Vec::new();
            for p in &f.params {
                let ty = self.lower_type(&p.ty);
                if params.iter().any(|(n, _)| n == &p.name.name) {
                    self.error(
                        "E1002",
                        p.name.span,
                        Text::new(
                            format!("parameter `{}` is listed twice", p.name.name),
                            format!("المعامل `{}` مذكور مرتين", p.name.name),
                        ),
                    );
                }
                params.push((p.name.name.clone(), ty));
            }
            let ret = f.ret.as_ref().map_or(Ty::Unit, |t| self.lower_type(t));
            let declared = f.effects.as_ref().map(|set| {
                let mut names = BTreeSet::new();
                for e in &set.effects {
                    if KNOWN_EFFECTS.contains(&e.name.as_str()) {
                        names.insert(e.name.clone());
                    } else {
                        self.push(
                            Diagnostic::error(
                                "E3003",
                                e.span,
                                Text::new(
                                    format!("unknown effect `{}`", e.name),
                                    format!("تأثير غير معروف `{}`", e.name),
                                ),
                            )
                            .with_hint(Text::new(
                                format!("known effects: {}", KNOWN_EFFECTS.join(", ")),
                                format!("التأثيرات المعروفة: {}", KNOWN_EFFECTS.join("، ")),
                            )),
                        );
                    }
                }
                (names, set.span)
            });
            self.fns.insert(
                name.clone(),
                FnSig {
                    params,
                    ret,
                    declared,
                    is_pub: f.is_pub,
                    is_async: f.is_async,
                    name_span: f.name.span,
                },
            );
        }
    }

    fn check_const(&mut self, c: &mut ConstDecl) {
        let name = &c.name.name;
        let clash = self.fns.contains_key(name)
            || BUILTIN_FUNCTIONS.contains(&name.as_str())
            || CONSTRUCTORS.contains(&name.as_str());
        let ty = match &c.ty {
            Some(t) => {
                let ty = self.lower_type(t);
                self.check_expr_against(&mut c.value, &ty);
                ty
            }
            None => self.check_expr(&mut c.value, None),
        };
        if clash {
            self.error(
                "E1002",
                c.name.span,
                Text::new(
                    format!("`{name}` is already defined as a function"),
                    format!("الاسم `{name}` معرّف مسبقًا كدالة"),
                ),
            );
        } else if let Some((_, prev)) = self.consts.get(name) {
            let prev = *prev;
            self.push(
                Diagnostic::error(
                    "E1002",
                    c.name.span,
                    Text::new(
                        format!("constant `{name}` is defined more than once"),
                        format!("الثابت `{name}` معرّف أكثر من مرة"),
                    ),
                )
                .with_note(prev, Text::new("first defined here", "عُرّف أول مرة هنا")),
            );
        } else {
            self.consts.insert(name.clone(), (ty, c.name.span));
        }
    }

    fn check_fn(&mut self, f: &mut FnDecl) {
        let Some(sig) = self.fns.get(&f.name.name).cloned() else {
            return;
        };
        self.ret_ty = Some(sig.ret.clone());
        self.in_async = sig.is_async;
        self.loop_depth = 0;
        self.push_scope();
        for (p, (_, ty)) in f.params.iter().zip(&sig.params) {
            self.declare(&p.name, ty.clone(), false, true);
        }
        self.check_block(&mut f.body);
        self.pop_scope();
        self.ret_ty = None;

        if !matches!(sig.ret, Ty::Unit | Ty::Error) && !expr::block_diverges(&f.body) {
            let ret = self.subst.show(&sig.ret);
            let close = Span {
                start: f.body.span.end.saturating_sub(1),
                end: f.body.span.end,
            };
            self.push(
                Diagnostic::error(
                    "E2007",
                    close,
                    Text::new(
                        format!(
                            "function `{}` may finish without returning a `{ret}`",
                            f.name.name
                        ),
                        format!(
                            "قد تنتهي الدالة `{}` دون إرجاع قيمة من النوع `{ret}`",
                            f.name.name
                        ),
                    ),
                )
                .with_note(
                    f.name.span,
                    Text::new("function declared here", "الدالة معرّفة هنا"),
                )
                .with_hint(Text::new(
                    "make sure every path ends with `return`",
                    "تأكد أن كل مسار ينتهي بـ `return`",
                )),
            );
        }
    }

    // ----- effects -----------------------------------------------------------

    fn check_fn_effects(
        &mut self,
        usage: &HashMap<String, Usage>,
        effects: &HashMap<String, BTreeSet<String>>,
    ) {
        let mut names: Vec<&String> = usage.keys().collect();
        names.sort_by_key(|n| self.fns.get(*n).map(|s| s.name_span.start));
        for name in names {
            let Some(sig) = self.fns.get(name).cloned() else {
                continue;
            };
            let actual = &effects[name];
            match &sig.declared {
                Some((declared, set_span)) => {
                    for eff in actual.difference(declared) {
                        let site = effect_site(eff, &usage[name], effects);
                        let listed = format_effects(declared);
                        self.push(
                            Diagnostic::error(
                                "E3001",
                                site,
                                Text::new(
                                    format!(
                                        "function `{name}` performs effect `{eff}` but declares only `!{{{listed}}}`"
                                    ),
                                    format!(
                                        "الدالة `{name}` تُحدث التأثير `{eff}` لكنها تصرّح فقط بـ `!{{{listed}}}`"
                                    ),
                                ),
                            )
                            .with_label(Text::new(
                                format!("`{eff}` happens here"),
                                format!("التأثير `{eff}` يحدث هنا"),
                            ))
                            .with_note(
                                *set_span,
                                Text::new("declared effects", "التأثيرات المصرّح بها"),
                            )
                            .with_hint(Text::new(
                                format!("add `{eff}` to the effect list"),
                                format!("أضف `{eff}` إلى قائمة التأثيرات"),
                            )),
                        );
                    }
                }
                None if sig.is_pub && !actual.is_empty() => {
                    let all = format_effects(actual);
                    self.push(
                        Diagnostic::error(
                            "E3002",
                            sig.name_span,
                            Text::new(
                                format!("public function `{name}` must declare its effects"),
                                format!("الدالة العامة `{name}` يجب أن تصرّح بتأثيراتها"),
                            ),
                        )
                        .with_hint(Text::new(
                            format!("add `!{{{all}}}` after the parameter list (and return type)"),
                            format!("أضف `!{{{all}}}` بعد قائمة المعاملات (ونوع الإرجاع)"),
                        )),
                    );
                }
                None => {}
            }
        }
    }

    fn check_const_effects(
        &mut self,
        usage: &[(Ident, Usage)],
        effects: &HashMap<String, BTreeSet<String>>,
    ) {
        for (name, u) in usage {
            let mut actual = BTreeSet::new();
            for (e, _) in &u.direct {
                actual.insert(e.clone());
            }
            for (callee, _) in &u.calls {
                if let Some(es) = effects.get(callee) {
                    actual.extend(es.iter().cloned());
                }
            }
            if let Some(eff) = actual.iter().next() {
                let site = effect_site(eff, u, effects);
                self.push(
                    Diagnostic::error(
                        "E3004",
                        site,
                        Text::new(
                            format!(
                                "the value of file-level constant `{}` performs effect `{eff}`",
                                name.name
                            ),
                            format!(
                                "قيمة الثابت `{}` على مستوى الملف تُحدث التأثير `{eff}`",
                                name.name
                            ),
                        ),
                    )
                    .with_hint(Text::new(
                        "file-level constants must be pure; compute this inside `main` instead",
                        "الثوابت على مستوى الملف يجب أن تكون خالية من التأثيرات؛ احسبها داخل `main`",
                    )),
                );
            }
        }
    }
}

/// Transitive effects of every function, as a fixed point over the call graph
/// (handles recursion and mutual recursion).
fn infer_effects(usage: &HashMap<String, Usage>) -> HashMap<String, BTreeSet<String>> {
    let mut effects: HashMap<String, BTreeSet<String>> = usage
        .iter()
        .map(|(name, u)| {
            (
                name.clone(),
                u.direct.iter().map(|(e, _)| e.clone()).collect(),
            )
        })
        .collect();
    loop {
        let mut changed = false;
        for (name, u) in usage {
            let mut gained = BTreeSet::new();
            for (callee, _) in &u.calls {
                if let Some(es) = effects.get(callee) {
                    gained.extend(es.iter().cloned());
                }
            }
            let mine = effects.get_mut(name).expect("present");
            let before = mine.len();
            mine.extend(gained);
            changed |= mine.len() != before;
        }
        if !changed {
            return effects;
        }
    }
}

/// The first place in a body where `eff` happens, directly or via a call.
fn effect_site(eff: &str, usage: &Usage, effects: &HashMap<String, BTreeSet<String>>) -> Span {
    let direct = usage.direct.iter().find(|(e, _)| e == eff).map(|(_, s)| *s);
    let via_call = usage
        .calls
        .iter()
        .find(|(callee, _)| effects.get(callee).is_some_and(|es| es.contains(eff)))
        .map(|(_, s)| *s);
    match (direct, via_call) {
        (Some(a), Some(b)) => {
            if a.start <= b.start {
                a
            } else {
                b
            }
        }
        (Some(a), None) | (None, Some(a)) => a,
        (None, None) => Span::default(),
    }
}

fn format_effects(set: &BTreeSet<String>) -> String {
    set.iter().cloned().collect::<Vec<_>>().join(", ")
}

/// The closest candidate within a small edit distance, if any.
fn suggest(name: &str, candidates: &[String]) -> Option<String> {
    let limit = (name.chars().count() / 3).clamp(1, 3);
    candidates
        .iter()
        .filter(|c| c.as_str() != name)
        .map(|c| (edit_distance(name, c), c))
        .filter(|(d, _)| *d <= limit)
        .min_by_key(|(d, _)| *d)
        .map(|(_, c)| c.clone())
}

/// Optimal-string-alignment distance: Levenshtein plus adjacent swaps, so
/// the common typo `cuont` is one edit away from `count`.
fn edit_distance(a: &str, b: &str) -> usize {
    let a: Vec<char> = a.chars().collect();
    let b: Vec<char> = b.chars().collect();
    let mut d = vec![vec![0usize; b.len() + 1]; a.len() + 1];
    for (i, row) in d.iter_mut().enumerate() {
        row[0] = i;
    }
    for (j, cell) in d[0].iter_mut().enumerate() {
        *cell = j;
    }
    for i in 1..=a.len() {
        for j in 1..=b.len() {
            let cost = usize::from(a[i - 1] != b[j - 1]);
            d[i][j] = (d[i - 1][j] + 1)
                .min(d[i][j - 1] + 1)
                .min(d[i - 1][j - 1] + cost);
            if i > 1 && j > 1 && a[i - 1] == b[j - 2] && a[i - 2] == b[j - 1] {
                d[i][j] = d[i][j].min(d[i - 2][j - 2] + 1);
            }
        }
    }
    d[a.len()][b.len()]
}

#[cfg(test)]
mod tests;
