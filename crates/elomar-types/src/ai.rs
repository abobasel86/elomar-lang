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
const KNOWN_MODULES: [(&str, Option<&str>); 16] = [
    ("std.io", None),
    ("std.ai", Some("ai")),
    ("std.fs", Some("fs")),
    ("std.json", Some("json")),
    ("std.http", Some("http")),
    ("std.db", Some("db")),
    ("std.py", Some("py")),
    ("std.mcp", Some("mcp")),
    ("std.agent", Some("agent")),
    ("std.math", Some("math")),
    ("std.random", Some("random")),
    ("std.time", Some("time")),
    ("std.os", Some("os")),
    ("std.regex", Some("regex")),
    ("std.encoding", Some("encoding")),
    ("std.crypto", Some("crypto")),
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

/// Parameters of `ai.run`: (name, required).
const RUN_PARAMS: [(&str, bool); 6] = [
    ("model", true),
    ("instructions", true),
    ("input", true),
    ("tools", true),
    ("max_steps", false),
    ("timeout", false),
];

/// Parameters of `agent.run`: (name, required).
const AGENT_PARAMS: [(&str, bool); 10] = [
    ("model", true),
    ("instructions", true),
    ("input", true),
    ("tools", false),
    ("mcp", false),
    ("require_approval", false),
    ("approve", false),
    ("on_step", false),
    ("max_steps", false),
    ("timeout", false),
];

/// What `approve` receives in `agent.run`.
pub(crate) fn tool_request_ty() -> Ty {
    Ty::record(vec![
        ("tool".into(), Ty::Str),
        ("arguments".into(), Ty::Str),
    ])
}

/// What `on_step` receives in `agent.run`.
pub(crate) fn agent_step_ty() -> Ty {
    Ty::record(vec![
        ("tool".into(), Ty::Str),
        ("arguments".into(), Ty::Str),
        ("result".into(), Ty::Str),
        ("approved".into(), Ty::Bool),
    ])
}

/// `AIError` is a built-in enum; every variant carries a message.
pub(crate) fn ai_error_ty() -> Ty {
    Ty::Enum(crate::enums::AI_ERROR.into(), Vec::new())
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
        type_args: &mut Vec<TypeExpr>,
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
            "std.http" => self.check_http_call(func, type_args, args, span),
            "std.db" => self.check_db_call(func, type_args, args, span),
            "std.py" => self.check_py_call(func, type_args, args, span),
            "std.mcp" => self.check_mcp_call(func, type_args, args, span),
            "std.math" => self.check_math_call(func, type_args, args, span),
            "std.random" => self.check_random_call(func, type_args, args, span),
            "std.time" => self.check_time_call(func, type_args, args, span),
            "std.os" => self.check_os_call(func, type_args, args, span),
            "std.regex" => self.check_regex_call(func, type_args, args, span),
            "std.encoding" => self.check_encoding_call(func, type_args, args, span),
            "std.crypto" => self.check_crypto_call(func, type_args, args, span),
            "std.agent" if func.name == "run" => {
                self.check_ai_call("agent.run", func.span, type_args, args, span)
            }
            "std.agent" => self.unknown_module_function("agent", func, &["run"], args),
            _ if func.name == "infer" || func.name == "run" => {
                let what = format!("ai.{}", func.name);
                self.check_ai_call(&what, func.span, type_args, args, span)
            }
            _ => self.unknown_module_function("ai", func, &["infer", "run"], args),
        })
    }

    /// `ai.infer<T>(model, instructions, input[, timeout])`,
    /// `ai.run<T>(model, instructions, input, tools[, max_steps][, timeout])`
    /// and `agent.run<T>(...)` (see [`AGENT_PARAMS`]).
    ///
    /// Arguments are reordered into the full parameter list with names
    /// removed; for `ai.run` and `agent.run` missing optional ones are
    /// filled in (`max_steps` 5 or 10, empty lists, and `false` for "not
    /// given"), and the parameter and result types of each tool are
    /// appended to the type arguments.
    fn check_ai_call(
        &mut self,
        what: &str,
        callee_span: Span,
        type_args: &mut Vec<TypeExpr>,
        args: &mut Vec<Arg>,
        span: Span,
    ) -> Ty {
        let is_run = what != "ai.infer";
        let is_agent = what == "agent.run";
        let params: &[(&str, bool)] = match what {
            "ai.run" => &RUN_PARAMS,
            "agent.run" => &AGENT_PARAMS,
            _ => &INFER_PARAMS,
        };
        self.usage.direct.push(("ai".into(), callee_span));

        let out = match type_args.as_mut_slice() {
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
                            format!("`{what}` needs the type of the answer"),
                            format!("تحتاج `{what}` إلى نوع الإجابة المطلوبة"),
                        ),
                    )
                    .with_hint(Text::new(
                        format!("write it in angle brackets, e.g. `{what}<Ticket>(...)` or `{what}<String>(...)`"),
                        format!("اكتبه بين قوسين زاويين، مثل `{what}<Ticket>(...)` أو `{what}<String>(...)`"),
                    )),
                );
                Ty::Error
            }
            _ => {
                self.wrong_arg_count(what, "1 type", type_args.len(), span);
                Ty::Error
            }
        };

        let slots = self.bind_named_args(what, params, args, span);
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
        let mut tools = Vec::new();
        // Literal server names of `mcp:` (`None` if not a literal list).
        let mut servers: Option<Vec<String>> = Some(Vec::new());
        let mut approvals: Vec<(String, Span)> = Vec::new();
        for (i, arg) in args.iter_mut().enumerate() {
            let param = slots
                .iter()
                .position(|s| *s == Some(i))
                .map(|j| params[j].0);
            match param {
                Some("model" | "instructions") => self.check_expr_against(&mut arg.value, &Ty::Str),
                Some("input") => {
                    let t = self.check_expr(&mut arg.value, None);
                    let resolved = self.subst.resolve(&t);
                    if let Some(bad) = self.non_json_part(&resolved, false) {
                        self.not_json_data(&resolved, &bad, arg.value.span, DataRole::AiInput);
                    }
                }
                Some("tools") => tools = self.check_tools(&mut arg.value),
                Some("mcp") => {
                    self.check_expr_against(&mut arg.value, &Ty::list(Ty::Str));
                    servers = literal_strings(&arg.value)
                        .map(|v| v.into_iter().map(|(s, _)| s).collect());
                    if servers.as_ref().is_none_or(|s| !s.is_empty()) {
                        self.usage.direct.push(("mcp".into(), arg.value.span));
                    }
                }
                Some("require_approval") => {
                    self.check_expr_against(&mut arg.value, &Ty::list(Ty::Str));
                    approvals = literal_strings(&arg.value).unwrap_or_default();
                }
                Some("approve") => self.check_expr_against(
                    &mut arg.value,
                    &Ty::Fn(vec![tool_request_ty()], Box::new(Ty::Bool)),
                ),
                Some("on_step") => self.check_expr_against(
                    &mut arg.value,
                    &Ty::Fn(vec![agent_step_ty()], Box::new(Ty::Unit)),
                ),
                Some("max_steps") => self.check_expr_against(&mut arg.value, &Ty::Int),
                Some(_) => self.check_expr_against(&mut arg.value, &Ty::Duration),
                None => {
                    self.check_expr(&mut arg.value, None);
                }
            }
        }
        for (name, at) in approvals {
            let known = tools.iter().any(|(n, _, _)| *n == name)
                || name.split_once('.').is_some_and(|(server, tool)| {
                    !tool.is_empty()
                        && servers
                            .as_ref()
                            .is_none_or(|s| s.iter().any(|x| x == server))
                });
            if !known {
                self.push(
                    Diagnostic::error(
                        "E2022",
                        at,
                        Text::new(
                            format!("`{name}` is not one of the tools of this call"),
                            format!("`{name}` ليست من أدوات هذا الاستدعاء"),
                        ),
                    )
                    .with_hint(Text::new(
                        "name a function from `tools`, or an MCP tool as `server.tool` with the server in `mcp`",
                        "اذكر دالة من `tools`، أو أداة MCP بالصيغة `server.tool` والخادم مذكور في `mcp`",
                    )),
                );
            }
        }
        let required = params.iter().filter(|(_, r)| *r).count();
        if slots[..required].iter().all(Option::is_some) {
            let mut old: Vec<Option<Arg>> = std::mem::take(args).into_iter().map(Some).collect();
            for (j, slot) in slots.iter().enumerate() {
                let arg = match slot {
                    Some(i) => {
                        let mut arg = old[*i].take().expect("each argument fills one slot");
                        arg.name = None;
                        arg
                    }
                    // `ai.infer` keeps its old shape: no placeholders.
                    None if !is_run => continue,
                    None => Arg {
                        name: None,
                        value: Expr {
                            kind: match params[j].0 {
                                "max_steps" => ExprKind::Int(if is_agent { 10 } else { 5 }),
                                "tools" | "mcp" => ExprKind::List(Vec::new()),
                                _ => ExprKind::Bool(false),
                            },
                            span: callee_span,
                        },
                    },
                };
                args.push(arg);
            }
        }
        if is_run && type_args.len() == 1 {
            for (_, params, ret) in tools {
                let unit = TypeExpr {
                    kind: TypeExprKind::Named {
                        name: Ident {
                            name: "Unit".into(),
                            span: callee_span,
                        },
                        args: Vec::new(),
                    },
                    span: callee_span,
                };
                type_args.push(type_expr_of(&Ty::record(params), callee_span));
                type_args.push(if ret == Ty::Unit {
                    unit
                } else {
                    type_expr_of(&ret, callee_span)
                });
            }
        }
        Ty::future(Ty::result(out, ai_error_ty()))
    }

    /// `tools: [find_student, average]`: names of plain functions whose
    /// parameters and result are JSON data. Returns each tool's parameter
    /// types and result type. Using a function here charges its effects to
    /// the caller, as for any function value.
    fn check_tools(&mut self, e: &mut Expr) -> Vec<Tool> {
        let ExprKind::List(items) = &mut e.kind else {
            self.check_expr(e, None);
            self.push(Diagnostic::error(
                "E2022",
                e.span,
                Text::new(
                    "`tools` must be a list of function names, e.g. `tools: [find_student]`",
                    "يجب أن تكون `tools` قائمة بأسماء دوال، مثل `tools: [find_student]`",
                ),
            ));
            return Vec::new();
        };
        let mut out = Vec::new();
        let mut names: Vec<String> = Vec::new();
        for item in items.iter_mut() {
            self.check_expr(item, None);
            let fn_name = match &item.kind {
                ExprKind::Name(n) if self.lookup_local(n).is_none() && self.fns.contains_key(n) => {
                    n.clone()
                }
                _ => {
                    self.error(
                        "E2022",
                        item.span,
                        Text::new(
                            "each tool must be the name of a function declared with `fn`",
                            "كل أداة يجب أن تكون اسم دالة معرّفة بـ `fn`",
                        ),
                    );
                    continue;
                }
            };
            let sig = self.fns[&fn_name].clone();
            let tool = crate::modules::base_name(&fn_name).to_string();
            let problem = if !sig.type_params.is_empty() {
                Some((
                    "a generic function cannot be a tool".to_string(),
                    "لا يمكن أن تكون الدالة العامة أداة".to_string(),
                ))
            } else if sig.is_async {
                Some((
                    "an `async fn` cannot be a tool yet".into(),
                    "لا يمكن بعد أن تكون `async fn` أداة".into(),
                ))
            } else if tool.is_empty()
                || !tool.chars().all(|c| c.is_ascii_alphanumeric() || c == '_')
            {
                Some((
                    format!(
                        "tool `{tool}`: model APIs accept only ASCII letters, digits and `_` in tool names"
                    ),
                    format!(
                        "الأداة `{tool}`: واجهات النماذج تقبل في أسماء الأدوات الحروف اللاتينية والأرقام و`_` فقط"
                    ),
                ))
            } else if names.contains(&tool) {
                Some((
                    format!("tool `{tool}` is listed twice"),
                    format!("الأداة `{tool}` مذكورة مرتين"),
                ))
            } else {
                None
            };
            if let Some((en, ar)) = problem {
                self.error("E2022", item.span, Text::new(en, ar));
                continue;
            }
            let mut bad = false;
            for (_, t) in &sig.params {
                if let Some(part) = self.non_json_part(t, false) {
                    self.not_json_data(t, &part, item.span, DataRole::AiInput);
                    bad = true;
                }
            }
            if sig.ret != Ty::Unit
                && let Some(part) = self.non_json_part(&sig.ret, false)
            {
                self.not_json_data(&sig.ret, &part, item.span, DataRole::AiInput);
                bad = true;
            }
            if bad {
                continue;
            }
            names.push(tool.clone());
            out.push((tool, sig.params.clone(), sig.ret.clone()));
        }
        out
    }

    /// Maps each parameter of `what` to the index of its argument.
    fn bind_named_args(
        &mut self,
        what: &str,
        params: &[(&str, bool)],
        args: &[Arg],
        span: Span,
    ) -> Vec<Option<usize>> {
        let mut slots = vec![None; params.len()];
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
                    match params.iter().position(|(n, _)| *n == id.name) {
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
                                params.iter().map(|(n, _)| n.to_string()).collect();
                            let mut diag = Diagnostic::error(
                                "E2010",
                                id.span,
                                Text::new(
                                    format!("`{what}` has no parameter named `{}`", id.name),
                                    format!("`{what}` ليس لها معامل باسم `{}`", id.name),
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
        if args.len() > params.len() {
            let required = params.iter().filter(|(_, r)| *r).count();
            self.wrong_arg_count(
                what,
                &format!("{required}–{}", params.len()),
                args.len(),
                span,
            );
        }
        for (j, (name, required)) in params.iter().enumerate() {
            if *required && slots[j].is_none() {
                self.error(
                    "E2002",
                    span,
                    Text::new(
                        format!("`{what}` is missing the argument `{name}`"),
                        format!("ينقص `{what}` المعامل `{name}`"),
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
            Ty::Enum(name, _) if self.enums.get(name).is_some_and(|e| e.is_unit_only()) => None,
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

/// The strings of a list literal of string literals, with their spans.
fn literal_strings(e: &Expr) -> Option<Vec<(String, Span)>> {
    let ExprKind::List(items) = &e.kind else {
        return None;
    };
    items
        .iter()
        .map(|i| match &i.kind {
            ExprKind::Str(s) => Some((s.to_string(), i.span)),
            _ => None,
        })
        .collect()
}

/// A checked tool: its name, parameters and result type.
type Tool = (String, Vec<(String, Ty)>, Ty);

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
        Ty::Enum(name, args) => named(name, args.iter().map(|a| type_expr_of(a, span)).collect()),
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
