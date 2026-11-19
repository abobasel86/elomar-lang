//! `use`, `await`, and the `std.ai` module: `ai.infer<T>(...)` (spec §13).
//!
//! The checker rewrites two things for the interpreter, which has no types:
//! the module name becomes `std.ai` (a name users cannot write), and `T` is
//! replaced by its alias-free structure. Arguments are reordered into
//! `model, instructions, input[, timeout]` with names removed.

use elomar_ast::*;
use elomar_diagnostics::{Diagnostic, Text};

use crate::{Checker, Ty, suggest};

/// Modules that `use` accepts, with the short name they bind (if any).
/// `std.io` is accepted for compatibility; its functions are in the prelude.
const KNOWN_MODULES: [(&str, Option<&str>); 4] = [
    ("std.io", None),
    ("std.ai", Some("ai")),
    ("std.fs", Some("fs")),
    ("std.json", Some("json")),
];

/// Where a value crosses into JSON, for error messages.
#[derive(Clone, Copy)]
pub(crate) enum DataRole {
    AiOutput,
    AiInput,
    JsonParse,
    JsonValue,
}

/// Parameters of `ai.infer`: (name, required).
const INFER_PARAMS: [(&str, bool); 4] = [
    ("model", true),
    ("instructions", true),
    ("input", true),
    ("timeout", false),
];

/// `AIError` is a built-in enum; every variant carries a message.
pub(crate) fn ai_error_ty() -> Ty {
    Ty::Enum(crate::enums::AI_ERROR.into())
}

impl Checker {
    pub(crate) fn check_use(&mut self, u: &UseDecl) {
        if u.path.first().is_some_and(|p| p.name == "app") {
            self.check_app_use(u);
            return;
        }
        let path = u
            .path
            .iter()
            .map(|i| i.name.as_str())
            .collect::<Vec<_>>()
            .join(".");
        match KNOWN_MODULES.iter().find(|(p, _)| *p == path) {
            Some((_, Some(short))) => {
                self.import_std(&u.module, short);
            }
            Some((_, None)) => {}
            None => {
                let mut known: Vec<String> =
                    KNOWN_MODULES.iter().map(|(p, _)| p.to_string()).collect();
                known.push("app.<file>".into());
                self.push(
                    Diagnostic::error(
                        "E1009",
                        u.span,
                        Text::new(
                            format!("unknown module `{path}`"),
                            format!("وحدة غير معروفة `{path}`"),
                        ),
                    )
                    .with_hint(Text::new(
                        format!("available modules: {}", known.join(", ")),
                        format!("الوحدات المتاحة: {}", known.join("، ")),
                    )),
                );
            }
        }
    }

    pub(crate) fn check_await(&mut self, inner: &mut Expr, span: Span) -> Ty {
        if !self.in_async {
            self.push(
                Diagnostic::error(
                    "E2013",
                    span,
                    Text::new(
                        "`await` can only be used inside an `async fn`",
                        "لا يمكن استخدام `await` إلا داخل دالة `async fn`",
                    ),
                )
                .with_hint(Text::new(
                    "mark the enclosing function `async`, e.g. `async fn main() { ... }`",
                    "اجعل الدالة المحيطة `async`، مثل `async fn main() { ... }`",
                )),
            );
        }
        let t = self.check_expr(inner, None);
        match self.subst.shallow(&t) {
            Ty::Future(result) => *result,
            Ty::Error => Ty::Error,
            Ty::Var(_) => {
                let result = self.subst.fresh();
                self.subst.unify(&t, &Ty::future(result.clone()));
                result
            }
            other => {
                let shown = self.subst.show(&other);
                self.push(
                    Diagnostic::error(
                        "E2014",
                        inner.span,
                        Text::new(
                            format!("only a `Future` can be awaited, but this is `{shown}`"),
                            format!("لا يُنتظر بـ `await` إلا `Future`، لكن هذه القيمة `{shown}`"),
                        ),
                    )
                    .with_hint(Text::new(
                        "only `async fn` calls and `ai.infer` produce futures; remove `await`",
                        "وحدها استدعاءات `async fn` و`ai.infer` تُنتج `Future`؛ احذف `await`",
                    )),
                );
                Ty::Error
            }
        }
    }

    /// Handles `module.func(...)`. Returns `None` if `module` is not a
    /// module here (e.g. a local variable), so the call is a method call.
    pub(crate) fn try_module_call(
        &mut self,
        module: &mut String,
        func: &Ident,
        type_args: &mut [TypeExpr],
        args: &mut Vec<Arg>,
        span: Span,
    ) -> Option<Ty> {
        if self.lookup_local(module).is_some() || self.consts.contains_key(module.as_str()) {
            return None;
        }
        let (path, _) = KNOWN_MODULES
            .iter()
            .find(|(_, short)| *short == Some(module.as_str()))?;
        if !self.std_imported(module.as_str()) {
            self.push(
                Diagnostic::error(
                    "E1001",
                    func.span,
                    Text::new(
                        format!("unknown name `{module}`"),
                        format!("اسم غير معروف `{module}`"),
                    ),
                )
                .with_hint(Text::new(
                    format!("add `use {path}` at the top of the file"),
                    format!("أضف `use {path}` في أعلى الملف"),
                )),
            );
            self.check_args_loosely(args);
            return Some(Ty::Error);
        }
        *module = (*path).to_string();
        Some(match *path {
            "std.fs" => self.check_fs_call(func, type_args, args, span),
            "std.json" => self.check_json_call(func, type_args, args, span),
            _ if func.name != "infer" => self.unknown_module_function("ai", func, &["infer"], args),
            _ => self.check_ai_infer(func.span, type_args, args, span),
        })
    }

    fn check_ai_infer(
        &mut self,
        callee_span: Span,
        type_args: &mut [TypeExpr],
        args: &mut Vec<Arg>,
        span: Span,
    ) -> Ty {
        self.usage.direct.push(("ai".into(), callee_span));

        let out = match type_args {
            [t] => {
                let ty = self.lower_type(t);
                let resolved = self.subst.resolve(&ty);
                if let Some(bad) = self.non_json_part(&resolved, false) {
                    self.not_json_data(&resolved, &bad, t.span, DataRole::AiOutput);
                    Ty::Error
                } else {
                    *t = type_expr_of(&resolved, t.span);
                    resolved
                }
            }
            [] => {
                self.push(
                    Diagnostic::error(
                        "E2008",
                        callee_span,
                        Text::new(
                            "`ai.infer` needs the type of the answer",
                            "تحتاج `ai.infer` إلى نوع الإجابة المطلوبة",
                        ),
                    )
                    .with_hint(Text::new(
                        "write it in angle brackets, e.g. `ai.infer<Ticket>(...)` or `ai.infer<String>(...)`",
                        "اكتبه بين قوسين زاويين، مثل `ai.infer<Ticket>(...)` أو `ai.infer<String>(...)`",
                    )),
                );
                Ty::Error
            }
            _ => {
                self.wrong_arg_count("ai.infer", "1 type", type_args.len(), span);
                Ty::Error
            }
        };

        let slots = self.bind_infer_args(args, span);
        // A cloud model also sends data over the network (spec §13.1). If the
        // model name is not a literal, assume the worst when any cloud model
        // is configured.
        let model_arg = slots[0].map(|i| &args[i].value);
        let uses_network = match model_arg.map(|e| &e.kind) {
            Some(ExprKind::Str(name)) => self.cloud_models.contains(name),
            _ => !self.cloud_models.is_empty(),
        };
        if uses_network {
            let site = model_arg.map_or(callee_span, |e| e.span);
            self.usage.direct.push(("net".into(), site));
        }
        for (i, arg) in args.iter_mut().enumerate() {
            match slots.iter().position(|s| *s == Some(i)) {
                Some(0 | 1) => self.check_expr_against(&mut arg.value, &Ty::Str),
                Some(2) => {
                    let t = self.check_expr(&mut arg.value, None);
                    let resolved = self.subst.resolve(&t);
                    if let Some(bad) = self.non_json_part(&resolved, false) {
                        self.not_json_data(&resolved, &bad, arg.value.span, DataRole::AiInput);
                    }
                }
                Some(_) => self.check_expr_against(&mut arg.value, &Ty::Duration),
                None => {
                    self.check_expr(&mut arg.value, None);
                }
            }
        }
        if slots[..3].iter().all(Option::is_some) {
            let mut old: Vec<Option<Arg>> = std::mem::take(args).into_iter().map(Some).collect();
            for slot in slots.iter().flatten() {
                let mut arg = old[*slot].take().expect("each argument fills one slot");
                arg.name = None;
                args.push(arg);
            }
        }
        Ty::future(Ty::result(out, ai_error_ty()))
    }

    /// Maps each parameter of `ai.infer` to the index of its argument.
    fn bind_infer_args(&mut self, args: &[Arg], span: Span) -> [Option<usize>; 4] {
        let mut slots = [None; 4];
        let mut seen_named = false;
        for (i, arg) in args.iter().enumerate() {
            match &arg.name {
                None if seen_named => {
                    self.error(
                        "E2010",
                        arg.value.span,
                        Text::new(
                            "positional arguments must come before named ones",
                            "المعاملات الموضعية يجب أن تسبق المعاملات المسماة",
                        ),
                    );
                }
                None if i < slots.len() => slots[i] = Some(i),
                None => {}
                Some(id) => {
                    seen_named = true;
                    match INFER_PARAMS.iter().position(|(n, _)| *n == id.name) {
                        Some(j) if slots[j].is_some() => self.error(
                            "E2010",
                            id.span,
                            Text::new(
                                format!("parameter `{}` is given more than once", id.name),
                                format!("المعامل `{}` أُعطي أكثر من مرة", id.name),
                            ),
                        ),
                        Some(j) => slots[j] = Some(i),
                        None => {
                            let names: Vec<String> =
                                INFER_PARAMS.iter().map(|(n, _)| n.to_string()).collect();
                            let mut diag = Diagnostic::error(
                                "E2010",
                                id.span,
                                Text::new(
                                    format!("`ai.infer` has no parameter named `{}`", id.name),
                                    format!("`ai.infer` ليس لها معامل باسم `{}`", id.name),
                                ),
                            );
                            diag = match suggest(&id.name, &names) {
                                Some(s) => diag.with_hint(Text::new(
                                    format!("did you mean `{s}`?"),
                                    format!("هل تقصد `{s}`؟"),
                                )),
                                None => diag.with_hint(Text::new(
                                    format!("parameters: {}", names.join(", ")),
                                    format!("المعاملات: {}", names.join("، ")),
                                )),
                            };
                            self.push(diag);
                        }
                    }
                }
            }
        }
        if args.len() > INFER_PARAMS.len() {
            self.wrong_arg_count("ai.infer", "3–4", args.len(), span);
        }
        for (j, (name, required)) in INFER_PARAMS.iter().enumerate() {
            if *required && slots[j].is_none() {
                self.error(
                    "E2002",
                    span,
                    Text::new(
                        format!("`ai.infer` is missing the argument `{name}`"),
                        format!("ينقص `ai.infer` المعامل `{name}`"),
                    ),
                );
            }
        }
        slots
    }

    /// The first part of `ty` that has no JSON form, if any.
    /// The first part of `ty` that has no JSON form, if any. `Map<String, V>`
    /// is JSON for `std.json` but not for models (strict schemas forbid it).
    pub(crate) fn non_json_part(&self, ty: &Ty, allow_maps: bool) -> Option<Ty> {
        match ty {
            Ty::Str | Ty::Int | Ty::Float | Ty::Bool | Ty::Error => None,
            Ty::List(t) | Ty::Option(t) => self.non_json_part(t, allow_maps),
            Ty::Map(k, v) if allow_maps && matches!(**k, Ty::Str) => {
                self.non_json_part(v, allow_maps)
            }
            Ty::Record(fields) => fields
                .iter()
                .find_map(|(_, t)| self.non_json_part(t, allow_maps)),
            // An enum of plain names travels as one of its names (a string).
            Ty::Enum(name) if self.enums.get(name).is_some_and(|e| e.is_unit_only()) => None,
            other => Some(other.clone()),
        }
    }

    pub(crate) fn not_json_data(&mut self, ty: &Ty, bad: &Ty, span: Span, role: DataRole) {
        if matches!(bad, Ty::Var(_)) {
            self.cannot_infer(span);
            return;
        }
        let shown = self.subst.show(ty);
        let part = self.subst.show(bad);
        let (en, ar) = match role {
            DataRole::AiOutput => (
                format!("`ai.infer` cannot produce a `{shown}`: `{part}` has no JSON form"),
                format!(
                    "لا يمكن أن تُنتج `ai.infer` قيمة `{shown}`: النوع `{part}` ليس له تمثيل JSON"
                ),
            ),
            DataRole::AiInput => (
                format!("a `{shown}` cannot be sent to a model: `{part}` has no JSON form"),
                format!("لا يمكن إرسال `{shown}` إلى نموذج: النوع `{part}` ليس له تمثيل JSON"),
            ),
            DataRole::JsonParse => (
                format!("`json.parse` cannot produce a `{shown}`: `{part}` has no JSON form"),
                format!(
                    "لا يمكن أن تُنتج `json.parse` قيمة `{shown}`: النوع `{part}` ليس له تمثيل JSON"
                ),
            ),
            DataRole::JsonValue => (
                format!("a `{shown}` cannot be written as JSON: `{part}` has no JSON form"),
                format!("لا يمكن كتابة `{shown}` بصيغة JSON: النوع `{part}` ليس له تمثيل JSON"),
            ),
        };
        self.push(Diagnostic::error("E2015", span, Text::new(en, ar)).with_hint(Text::new(
            "JSON data is String, Int, Float, Bool, List<T>, Option<T>, enums of plain names, records of these, and (outside `ai.infer`) Map<String, T>",
            "بيانات JSON هي String وInt وFloat وBool وList<T> وOption<T> والأنواع المعدودة الاسمية والسجلات المكونة منها، و(خارج `ai.infer`) Map<String, T>",
        )));
    }
}

/// Writes a resolved type back as syntax, with aliases expanded.
pub(crate) fn type_expr_of(ty: &Ty, span: Span) -> TypeExpr {
    let named = |name: &str, args: Vec<TypeExpr>| TypeExpr {
        kind: TypeExprKind::Named {
            name: Ident {
                name: name.into(),
                span,
            },
            args,
        },
        span,
    };
    match ty {
        Ty::Str => named("String", vec![]),
        Ty::Int => named("Int", vec![]),
        Ty::Float => named("Float", vec![]),
        Ty::Bool => named("Bool", vec![]),
        Ty::List(t) => named("List", vec![type_expr_of(t, span)]),
        Ty::Option(t) => named("Option", vec![type_expr_of(t, span)]),
        Ty::Enum(name) => named(name, vec![]),
        Ty::Map(k, v) => named("Map", vec![type_expr_of(k, span), type_expr_of(v, span)]),
        Ty::Record(fields) => TypeExpr {
            kind: TypeExprKind::Record(
                fields
                    .iter()
                    .map(|(n, t)| {
                        (
                            Ident {
                                name: n.clone(),
                                span,
                            },
                            type_expr_of(t, span),
                        )
                    })
                    .collect(),
            ),
            span,
        },
        other => unreachable!("not JSON data: {other:?}"),
    }
}
