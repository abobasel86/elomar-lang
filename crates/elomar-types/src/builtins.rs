//! Types of the prelude's functions and of the methods on built-in types.
//! The interpreter implements the same names; keep both in sync.

use elomar_ast::*;
use elomar_diagnostics::{Diagnostic, Text};

use crate::expr::place_root;
use crate::{Checker, Ty, suggest};

/// Effects a function may declare in `!{...}` (spec §16.2).
pub const KNOWN_EFFECTS: [&str; 6] = ["io", "fs", "net", "ai", "db", "py"];

/// Functions available without `use`.
pub const BUILTIN_FUNCTIONS: [&str; 13] = [
    "print",
    "input",
    "to_float",
    "to_int",
    "to_string",
    "parse_int",
    "parse_float",
    "assert",
    "assert_eq",
    "set",
    "min",
    "max",
    "abs",
];

pub(crate) const BUILTIN_TYPES: [&str; 20] = [
    "Int",
    "Float",
    "Bool",
    "String",
    "Unit",
    "Range",
    "Duration",
    "List",
    "Option",
    "Result",
    "Future",
    "Map",
    "Set",
    "AIError",
    "FsError",
    "JsonError",
    "HttpError",
    "Db",
    "DbError",
    "PyError",
];

pub(crate) fn is_builtin_type(name: &str) -> bool {
    BUILTIN_TYPES.contains(&name)
}

const LIST_METHODS: &[&str] = &[
    "push",
    "pop",
    "insert",
    "remove",
    "contains",
    "index_of",
    "is_empty",
    "first",
    "last",
    "reverse",
    "sorted",
    "join",
    "sum",
    "slice",
    "map",
    "filter",
    "any",
    "all",
    "find",
    "count",
    "fold",
    "sort_by",
    "to_string",
];
const STR_METHODS: &[&str] = &[
    "upper",
    "lower",
    "trim",
    "contains",
    "starts_with",
    "ends_with",
    "split",
    "chars",
    "replace",
    "repeat",
    "parse_int",
    "parse_float",
    "is_empty",
    "to_string",
];
const INT_METHODS: &[&str] = &["to_float", "abs", "to_string"];
const FLOAT_METHODS: &[&str] = &[
    "to_int",
    "round",
    "floor",
    "ceil",
    "abs",
    "sqrt",
    "to_string",
];
const OPTION_METHODS: &[&str] = &["is_some", "is_none", "unwrap_or", "to_string"];
const RESULT_METHODS: &[&str] = &["is_ok", "is_err", "unwrap_or", "to_string"];
const DURATION_METHODS: &[&str] = &["millis", "seconds", "to_string"];
const MAP_METHODS: &[&str] = &[
    "get",
    "contains_key",
    "remove",
    "keys",
    "values",
    "is_empty",
    "to_string",
];
const ERROR_METHODS: &[&str] = &["kind", "message", "to_string"];
const SET_METHODS: &[&str] = &[
    "contains",
    "add",
    "remove",
    "is_empty",
    "to_list",
    "union",
    "intersection",
    "difference",
    "is_subset",
    "to_string",
];
const OTHER_METHODS: &[&str] = &["to_string"];

fn methods_of(ty: &Ty) -> &'static [&'static str] {
    match ty {
        Ty::List(_) => LIST_METHODS,
        Ty::Str => STR_METHODS,
        Ty::Int => INT_METHODS,
        Ty::Float => FLOAT_METHODS,
        Ty::Option(_) => OPTION_METHODS,
        Ty::Result(..) => RESULT_METHODS,
        Ty::Duration => DURATION_METHODS,
        Ty::Map(..) => MAP_METHODS,
        Ty::Set(_) => SET_METHODS,
        Ty::Enum(name, _) if crate::enums::is_error_enum(name) => ERROR_METHODS,
        _ => OTHER_METHODS,
    }
}

pub(crate) fn has_method(ty: &Ty, name: &str) -> bool {
    methods_of(ty).contains(&name)
}

impl Checker {
    /// Checks the argument count; on mismatch reports `E2002`.
    pub(crate) fn arity(
        &mut self,
        what: &str,
        args: &mut [Arg],
        min: usize,
        max: usize,
        span: Span,
    ) -> bool {
        if (min..=max).contains(&args.len()) {
            return true;
        }
        let want = if min == max {
            min.to_string()
        } else {
            format!("{min}–{max}")
        };
        self.wrong_arg_count(what, &want, args.len(), span);
        self.check_args_loosely(args);
        false
    }

    /// Checks `args` against `params` exactly.
    pub(crate) fn typed_args(
        &mut self,
        what: &str,
        args: &mut [Arg],
        params: &[Ty],
        span: Span,
    ) -> bool {
        if !self.arity(what, args, params.len(), params.len(), span) {
            return false;
        }
        for (arg, ty) in args.iter_mut().zip(params) {
            self.check_expr_against(&mut arg.value, ty);
        }
        true
    }

    /// Checks an argument that must be `Int` or `Float`, returning its type.
    fn numeric_arg(&mut self, e: &mut Expr) -> Ty {
        let t = self.check_expr(e, None);
        match self.subst.shallow(&t) {
            Ty::Int | Ty::Float | Ty::Error => t,
            Ty::Var(_) => {
                self.subst.unify(&t, &Ty::Int);
                Ty::Int
            }
            other => {
                let shown = self.subst.show(&other);
                self.error(
                    "E2001",
                    e.span,
                    Text::new(
                        format!(
                            "type mismatch: expected a number (`Int` or `Float`), found `{shown}`"
                        ),
                        format!(
                            "عدم توافق الأنواع: المتوقع عدد (`Int` أو `Float`) لكن الموجود `{shown}`"
                        ),
                    ),
                );
                Ty::Error
            }
        }
    }

    pub(crate) fn check_builtin_call(
        &mut self,
        name: &str,
        args: &mut [Arg],
        span: Span,
        callee_span: Span,
        _hint: Option<&Ty>,
    ) -> Ty {
        self.reject_named_args(args);
        match name {
            "print" => {
                self.usage.direct.push(("io".into(), callee_span));
                self.check_args_loosely(args);
                Ty::Unit
            }
            "input" => {
                self.usage.direct.push(("io".into(), callee_span));
                if self.arity(name, args, 0, 1, span) {
                    for arg in args.iter_mut() {
                        self.check_expr_against(&mut arg.value, &Ty::Str);
                    }
                }
                Ty::Str
            }
            "to_float" | "to_int" | "abs" => {
                if !self.arity(name, args, 1, 1, span) {
                    return Ty::Error;
                }
                let t = self.numeric_arg(&mut args[0].value);
                match name {
                    "to_float" => Ty::Float,
                    "to_int" => Ty::Int,
                    _ => t,
                }
            }
            "min" | "max" => {
                if !self.arity(name, args, 2, 2, span) {
                    return Ty::Error;
                }
                let a = self.numeric_arg(&mut args[0].value);
                let b = self.numeric_arg(&mut args[1].value);
                match (self.subst.shallow(&a), self.subst.shallow(&b)) {
                    (Ty::Error, _) | (_, Ty::Error) => Ty::Error,
                    (Ty::Int, Ty::Int) => Ty::Int,
                    _ => Ty::Float,
                }
            }
            "to_string" => {
                if self.arity(name, args, 1, 1, span) {
                    self.check_expr(&mut args[0].value, None);
                }
                Ty::Str
            }
            "parse_int" | "parse_float" => {
                self.typed_args(name, args, &[Ty::Str], span);
                Ty::option(if name == "parse_int" {
                    Ty::Int
                } else {
                    Ty::Float
                })
            }
            "assert" => {
                if self.arity(name, args, 1, 2, span) {
                    self.check_expr_against(&mut args[0].value, &Ty::Bool);
                    if let Some(msg) = args.get_mut(1) {
                        self.check_expr_against(&mut msg.value, &Ty::Str);
                    }
                }
                Ty::Unit
            }
            "set" => {
                if !self.arity(name, args, 1, 1, span) {
                    return Ty::Error;
                }
                let elem = self.subst.fresh();
                self.check_expr_against(&mut args[0].value, &Ty::list(elem.clone()));
                self.check_set_element(&elem, args[0].value.span);
                Ty::set(elem)
            }
            "assert_eq" => {
                if self.arity(name, args, 2, 2, span) {
                    let actual = self.check_expr(&mut args[0].value, None);
                    let expected = self.check_expr(&mut args[1].value, Some(&actual));
                    let (a, e) = (self.subst.shallow(&actual), self.subst.shallow(&expected));
                    let both_numeric = a.is_numeric() && e.is_numeric();
                    if !both_numeric && !self.subst.unify(&actual, &expected) {
                        self.mismatch(&actual, &expected, args[1].value.span);
                    }
                }
                Ty::Unit
            }
            _ => unreachable!("not a builtin: {name}"),
        }
    }

    pub(crate) fn check_method(
        &mut self,
        recv: &mut Expr,
        recv_ty: &Ty,
        name: &Ident,
        args: &mut [Arg],
        span: Span,
    ) -> Ty {
        self.reject_named_args(args);
        let r = self.subst.shallow(recv_ty);
        let m = name.name.as_str();
        match &r {
            Ty::Error => {
                self.check_args_loosely(args);
                return Ty::Error;
            }
            Ty::Var(_) => {
                self.cannot_infer(recv.span);
                self.check_args_loosely(args);
                return Ty::Error;
            }
            _ => {}
        }
        if m == "to_string" {
            self.arity(m, args, 0, 0, span);
            return Ty::Str;
        }
        if m == "length" && matches!(r, Ty::List(_) | Ty::Str | Ty::Map(..) | Ty::Set(_)) {
            self.push(
                Diagnostic::error(
                    "E2004",
                    span,
                    Text::new(
                        "`length` is a property, not a method",
                        "`length` خاصية وليست دالة",
                    ),
                )
                .with_hint(Text::new(
                    "write `.length` without `()`",
                    "اكتب `.length` بدون `()`",
                )),
            );
            return Ty::Int;
        }
        let result = match (&r, m) {
            (Ty::List(elem), _) => self.list_method(recv, elem, m, args, span),
            (Ty::Str, _) => self.str_method(m, args, span),
            (Ty::Int, "to_float") => self.no_args(m, args, span, Ty::Float),
            (Ty::Int, "abs") => self.no_args(m, args, span, Ty::Int),
            (Ty::Float, "to_int" | "round" | "floor" | "ceil") => {
                self.no_args(m, args, span, Ty::Int)
            }
            (Ty::Float, "abs" | "sqrt") => self.no_args(m, args, span, Ty::Float),
            (Ty::Map(k, v), _) => self.map_method(recv, k, v, m, args, span),
            (Ty::Set(t), _) => self.set_method(recv, t, m, args, span),
            (Ty::Enum(name, _), "kind" | "message") if crate::enums::is_error_enum(name) => {
                self.no_args(m, args, span, Ty::Str)
            }
            (Ty::Duration, "millis") => self.no_args(m, args, span, Ty::Int),
            (Ty::Duration, "seconds") => self.no_args(m, args, span, Ty::Float),
            (Ty::Option(_), "is_some" | "is_none") | (Ty::Result(..), "is_ok" | "is_err") => {
                self.no_args(m, args, span, Ty::Bool)
            }
            (Ty::Option(t) | Ty::Result(t, _), "unwrap_or") => {
                let t = (**t).clone();
                self.typed_args(m, args, std::slice::from_ref(&t), span);
                Some(t)
            }
            _ => None,
        };
        if let Some(t) = result {
            return t;
        }
        if let Ty::Record(fields) = &r
            && fields.iter().any(|(n, _)| n == m)
        {
            self.error(
                "E2004",
                span,
                Text::new(
                    format!("field `{m}` is a value, not a method"),
                    format!("الحقل `{m}` قيمة وليس دالة"),
                ),
            );
            self.check_args_loosely(args);
            return Ty::Error;
        }
        let shown = self.subst.show(&r);
        let available: Vec<String> = methods_of(&r).iter().map(|s| s.to_string()).collect();
        let mut diag = Diagnostic::error(
            "E1008",
            name.span,
            Text::new(
                format!("type `{shown}` has no method `{m}`"),
                format!("النوع `{shown}` ليس له دالة تابعة باسم `{m}`"),
            ),
        );
        diag = match suggest(m, &available) {
            Some(s) => diag.with_hint(Text::new(
                format!("did you mean `{s}`?"),
                format!("هل تقصد `{s}`؟"),
            )),
            None => diag.with_hint(Text::new(
                format!("available methods: {}", available.join(", ")),
                format!("الدوال المتاحة: {}", available.join("، ")),
            )),
        };
        self.push(diag);
        self.check_args_loosely(args);
        Ty::Error
    }

    fn no_args(&mut self, m: &str, args: &mut [Arg], span: Span, ret: Ty) -> Option<Ty> {
        self.arity(m, args, 0, 0, span);
        Some(ret)
    }

    fn require_place(&mut self, recv: &Expr, method: &str) {
        if place_root(recv).is_some() {
            self.check_mutable_place(recv);
        } else {
            self.error(
                "E1003",
                recv.span,
                Text::new(
                    format!("`{method}` changes its collection, but this value is temporary"),
                    format!("`{method}` تعدّل المجموعة، لكن هذه القيمة مؤقتة"),
                ),
            );
        }
    }

    fn list_method(
        &mut self,
        recv: &Expr,
        elem: &Ty,
        m: &str,
        args: &mut [Arg],
        span: Span,
    ) -> Option<Ty> {
        let elem = elem.clone();
        let list = Ty::list(elem.clone());
        Some(match m {
            "push" => {
                self.require_place(recv, m);
                self.typed_args(m, args, std::slice::from_ref(&elem), span);
                Ty::Unit
            }
            "pop" => {
                self.require_place(recv, m);
                self.arity(m, args, 0, 0, span);
                Ty::option(elem)
            }
            "insert" => {
                self.require_place(recv, m);
                self.typed_args(m, args, &[Ty::Int, elem], span);
                Ty::Unit
            }
            "remove" => {
                self.require_place(recv, m);
                self.typed_args(m, args, &[Ty::Int], span);
                elem
            }
            "contains" => {
                self.typed_args(m, args, std::slice::from_ref(&elem), span);
                Ty::Bool
            }
            "index_of" => {
                self.typed_args(m, args, std::slice::from_ref(&elem), span);
                Ty::option(Ty::Int)
            }
            "is_empty" => {
                self.arity(m, args, 0, 0, span);
                Ty::Bool
            }
            "first" | "last" => {
                self.arity(m, args, 0, 0, span);
                Ty::option(elem)
            }
            "reverse" => {
                self.arity(m, args, 0, 0, span);
                list
            }
            "slice" => {
                self.typed_args(m, args, &[Ty::Int, Ty::Int], span);
                list
            }
            "sorted" | "sum" => {
                self.arity(m, args, 0, 0, span);
                let resolved = self.subst.shallow(&elem);
                let ok = match m {
                    "sorted" => matches!(resolved, Ty::Int | Ty::Float | Ty::Str | Ty::Error),
                    _ => matches!(resolved, Ty::Int | Ty::Float | Ty::Error),
                };
                if matches!(resolved, Ty::Var(_)) {
                    self.cannot_infer(recv.span);
                    return Some(Ty::Error);
                }
                if !ok {
                    let shown = self.subst.show(&list);
                    let (need_en, need_ar) = if m == "sorted" {
                        ("numbers or strings", "أعداد أو نصوص")
                    } else {
                        ("numbers", "أعداد")
                    };
                    self.error(
                        "E2005",
                        span,
                        Text::new(
                            format!("`{m}` needs a list of {need_en}, but this is `{shown}`"),
                            format!("`{m}` تحتاج قائمة {need_ar}، لكن هذه `{shown}`"),
                        ),
                    );
                    return Some(Ty::Error);
                }
                if m == "sorted" { list } else { elem }
            }
            "map" => {
                let out = self.subst.fresh();
                let f = Ty::Fn(vec![elem], Box::new(out.clone()));
                self.typed_args(m, args, &[f], span);
                Ty::list(out)
            }
            "filter" | "any" | "all" | "find" | "count" => {
                let f = Ty::Fn(vec![elem.clone()], Box::new(Ty::Bool));
                self.typed_args(m, args, &[f], span);
                match m {
                    "filter" => list,
                    "find" => Ty::option(elem),
                    "count" => Ty::Int,
                    _ => Ty::Bool,
                }
            }
            "fold" => {
                let acc = self.subst.fresh();
                let f = Ty::Fn(vec![acc.clone(), elem], Box::new(acc.clone()));
                self.typed_args(m, args, &[acc.clone(), f], span);
                acc
            }
            "sort_by" => {
                let key = self.subst.fresh();
                let f = Ty::Fn(vec![elem], Box::new(key.clone()));
                self.typed_args(m, args, &[f], span);
                let k = self.subst.shallow(&key);
                if !matches!(k, Ty::Int | Ty::Float | Ty::Str | Ty::Error | Ty::Var(_)) {
                    let shown = self.subst.show(&k);
                    self.error(
                        "E2005",
                        span,
                        Text::new(
                            format!("`sort_by` needs a key of type `Int`, `Float` or `String`, not `{shown}`"),
                            format!("`sort_by` تحتاج مفتاحًا من النوع `Int` أو `Float` أو `String`، لا `{shown}`"),
                        ),
                    );
                }
                list
            }
            "join" => {
                self.typed_args(m, args, &[Ty::Str], span);
                if !self.subst.unify(&elem, &Ty::Str) {
                    let shown = self.subst.show(&list);
                    self.error(
                        "E2005",
                        span,
                        Text::new(
                            format!("`join` needs a `List<String>`, but this is `{shown}`"),
                            format!("`join` تحتاج `List<String>`، لكن هذه `{shown}`"),
                        ),
                    );
                }
                Ty::Str
            }
            _ => return None,
        })
    }

    fn map_method(
        &mut self,
        recv: &Expr,
        k: &Ty,
        v: &Ty,
        m: &str,
        args: &mut [Arg],
        span: Span,
    ) -> Option<Ty> {
        let (k, v) = (k.clone(), v.clone());
        Some(match m {
            "get" => {
                self.typed_args(m, args, std::slice::from_ref(&k), span);
                Ty::option(v)
            }
            "contains_key" => {
                self.typed_args(m, args, std::slice::from_ref(&k), span);
                Ty::Bool
            }
            "remove" => {
                self.require_place(recv, m);
                self.typed_args(m, args, std::slice::from_ref(&k), span);
                Ty::option(v)
            }
            "keys" => {
                self.arity(m, args, 0, 0, span);
                Ty::list(k)
            }
            "values" => {
                self.arity(m, args, 0, 0, span);
                Ty::list(v)
            }
            "is_empty" => {
                self.arity(m, args, 0, 0, span);
                Ty::Bool
            }
            _ => return None,
        })
    }

    fn set_method(
        &mut self,
        recv: &Expr,
        elem: &Ty,
        m: &str,
        args: &mut [Arg],
        span: Span,
    ) -> Option<Ty> {
        let elem = elem.clone();
        let set = Ty::set(elem.clone());
        Some(match m {
            "contains" => {
                self.typed_args(m, args, std::slice::from_ref(&elem), span);
                Ty::Bool
            }
            "add" => {
                self.require_place(recv, m);
                self.typed_args(m, args, std::slice::from_ref(&elem), span);
                Ty::Unit
            }
            "remove" => {
                self.require_place(recv, m);
                self.typed_args(m, args, std::slice::from_ref(&elem), span);
                Ty::Bool
            }
            "is_empty" => {
                self.arity(m, args, 0, 0, span);
                Ty::Bool
            }
            "to_list" => {
                self.arity(m, args, 0, 0, span);
                Ty::list(elem)
            }
            "union" | "intersection" | "difference" => {
                self.typed_args(m, args, std::slice::from_ref(&set), span);
                set
            }
            "is_subset" => {
                self.typed_args(m, args, std::slice::from_ref(&set), span);
                Ty::Bool
            }
            _ => return None,
        })
    }

    fn str_method(&mut self, m: &str, args: &mut [Arg], span: Span) -> Option<Ty> {
        Some(match m {
            "upper" | "lower" | "trim" => {
                self.arity(m, args, 0, 0, span);
                Ty::Str
            }
            "contains" | "starts_with" | "ends_with" => {
                self.typed_args(m, args, &[Ty::Str], span);
                Ty::Bool
            }
            "split" => {
                self.typed_args(m, args, &[Ty::Str], span);
                Ty::list(Ty::Str)
            }
            "chars" => {
                self.arity(m, args, 0, 0, span);
                Ty::list(Ty::Str)
            }
            "replace" => {
                self.typed_args(m, args, &[Ty::Str, Ty::Str], span);
                Ty::Str
            }
            "repeat" => {
                self.typed_args(m, args, &[Ty::Int], span);
                Ty::Str
            }
            "parse_int" => {
                self.arity(m, args, 0, 0, span);
                Ty::option(Ty::Int)
            }
            "parse_float" => {
                self.arity(m, args, 0, 0, span);
                Ty::option(Ty::Float)
            }
            "is_empty" => {
                self.arity(m, args, 0, 0, span);
                Ty::Bool
            }
            _ => return None,
        })
    }
}
