//! Types of `std.math`, `std.random`, `std.time` and `std.os`.
//! The interpreter implements the same names in its `stdextra` module.

use elomar_ast::*;

use crate::{Checker, Ty};

pub(crate) const MATH_FUNCTIONS: [&str; 16] = [
    "sqrt", "cbrt", "pow", "exp", "ln", "log10", "log2", "sin", "cos", "tan", "asin", "acos",
    "atan", "atan2", "hypot", "is_nan",
];
/// `math.pi`, `math.e`, `math.tau`, `math.inf`: values, not functions.
pub(crate) const MATH_CONSTANTS: [&str; 4] = ["pi", "e", "tau", "inf"];
pub(crate) const RANDOM_FUNCTIONS: [&str; 6] =
    ["int", "float", "bool", "choice", "shuffle", "seed"];
pub(crate) const TIME_FUNCTIONS: [&str; 7] =
    ["now", "sleep", "format", "parse", "today", "date", "make"];
pub(crate) const OS_FUNCTIONS: [&str; 4] = ["args", "env", "platform", "exit"];

/// What `time.date(ms)` returns (fields in alphabetical order).
pub(crate) fn date_ty() -> Ty {
    Ty::Record(
        [
            "day", "hour", "minute", "month", "second", "weekday", "year",
        ]
        .iter()
        .map(|n| (n.to_string(), Ty::Int))
        .collect(),
    )
}

impl Checker {
    /// `math.pi` and the other constants. Returns `None` when `target.name`
    /// is not one (the field is then checked as usual).
    pub(crate) fn try_module_constant(&mut self, target: &mut Expr, name: &Ident) -> Option<Ty> {
        let ExprKind::Name(module) = &target.kind else {
            return None;
        };
        if module != "math"
            || !MATH_CONSTANTS.contains(&name.name.as_str())
            || self.lookup_local(module).is_some()
            || self.consts.contains_key(module.as_str())
            || !self.std_imported("math")
        {
            return None;
        }
        target.kind = ExprKind::Name("std.math".into());
        Some(Ty::Float)
    }

    /// `math.*`: pure functions on numbers. `Int` arguments are accepted
    /// and used as `Float`.
    pub(crate) fn check_math_call(
        &mut self,
        func: &Ident,
        type_args: &[TypeExpr],
        args: &mut [Arg],
        span: Span,
    ) -> Ty {
        let name = func.name.as_str();
        if !MATH_FUNCTIONS.contains(&name) {
            let mut all: Vec<&str> = MATH_FUNCTIONS.to_vec();
            all.extend(MATH_CONSTANTS);
            return self.unknown_module_function("math", func, &all, args);
        }
        let what = format!("math.{name}");
        self.no_type_args(&what, type_args);
        self.reject_named_args(args);
        let n = if matches!(name, "pow" | "atan2" | "hypot") {
            2
        } else {
            1
        };
        if self.arity(&what, args, n, n, span) {
            for arg in args.iter_mut() {
                self.numeric_arg(&mut arg.value);
            }
        }
        if name == "is_nan" {
            Ty::Bool
        } else {
            Ty::Float
        }
    }

    /// `random.*`: pseudo-random numbers (not for passwords or keys).
    pub(crate) fn check_random_call(
        &mut self,
        func: &Ident,
        type_args: &[TypeExpr],
        args: &mut [Arg],
        span: Span,
    ) -> Ty {
        let name = func.name.as_str();
        if !RANDOM_FUNCTIONS.contains(&name) {
            return self.unknown_module_function("random", func, &RANDOM_FUNCTIONS, args);
        }
        let what = format!("random.{name}");
        self.no_type_args(&what, type_args);
        self.reject_named_args(args);
        match name {
            "int" => {
                self.typed_args(&what, args, &[Ty::Int, Ty::Int], span);
                Ty::Int
            }
            "float" => {
                self.typed_args(&what, args, &[], span);
                Ty::Float
            }
            "bool" => {
                self.typed_args(&what, args, &[], span);
                Ty::Bool
            }
            "seed" => {
                self.typed_args(&what, args, &[Ty::Int], span);
                Ty::Unit
            }
            _ => {
                let elem = self.subst.fresh();
                self.typed_args(&what, args, &[Ty::list(elem.clone())], span);
                if name == "choice" {
                    Ty::option(elem)
                } else {
                    Ty::list(elem)
                }
            }
        }
    }

    /// `time.*`: times are `Int` milliseconds since 1970-01-01 UTC.
    pub(crate) fn check_time_call(
        &mut self,
        func: &Ident,
        type_args: &[TypeExpr],
        args: &mut [Arg],
        span: Span,
    ) -> Ty {
        let name = func.name.as_str();
        if !TIME_FUNCTIONS.contains(&name) {
            return self.unknown_module_function("time", func, &TIME_FUNCTIONS, args);
        }
        let what = format!("time.{name}");
        self.no_type_args(&what, type_args);
        self.reject_named_args(args);
        match name {
            "now" => {
                self.typed_args(&what, args, &[], span);
                Ty::Int
            }
            "today" => {
                self.typed_args(&what, args, &[], span);
                Ty::Str
            }
            "sleep" => {
                self.typed_args(&what, args, &[Ty::Duration], span);
                Ty::Unit
            }
            "format" => {
                self.typed_args(&what, args, &[Ty::Int], span);
                Ty::Str
            }
            "parse" => {
                self.typed_args(&what, args, &[Ty::Str], span);
                Ty::option(Ty::Int)
            }
            "date" => {
                self.typed_args(&what, args, &[Ty::Int], span);
                date_ty()
            }
            _ => {
                // make(year, month, day[, hour[, minute[, second]]])
                if self.arity(&what, args, 3, 6, span) {
                    for arg in args.iter_mut() {
                        self.check_expr_against(&mut arg.value, &Ty::Int);
                    }
                }
                Ty::Int
            }
        }
    }

    /// `os.*`: the program's arguments and environment. These perform `io`.
    pub(crate) fn check_os_call(
        &mut self,
        func: &Ident,
        type_args: &[TypeExpr],
        args: &mut [Arg],
        span: Span,
    ) -> Ty {
        let name = func.name.as_str();
        if !OS_FUNCTIONS.contains(&name) {
            return self.unknown_module_function("os", func, &OS_FUNCTIONS, args);
        }
        let what = format!("os.{name}");
        self.no_type_args(&what, type_args);
        self.reject_named_args(args);
        if name != "platform" {
            self.usage.direct.push(("io".into(), func.span));
        }
        match name {
            "args" => {
                self.typed_args(&what, args, &[], span);
                Ty::list(Ty::Str)
            }
            "env" => {
                self.typed_args(&what, args, &[Ty::Str], span);
                Ty::option(Ty::Str)
            }
            "platform" => {
                self.typed_args(&what, args, &[], span);
                Ty::Str
            }
            _ => {
                self.typed_args(&what, args, &[Ty::Int], span);
                Ty::Unit
            }
        }
    }
}

pub(crate) const REGEX_FUNCTIONS: [&str; 6] = [
    "is_match", "find", "find_all", "captures", "replace", "split",
];
pub(crate) const ENCODING_FUNCTIONS: [&str; 6] = [
    "base64_encode",
    "base64_decode",
    "hex_encode",
    "hex_decode",
    "url_encode",
    "url_decode",
];
pub(crate) const CRYPTO_FUNCTIONS: [&str; 4] = ["sha256", "hmac_sha256", "uuid", "token"];

impl Checker {
    /// `regex.*(pattern, text, ...)`. A pattern written as a literal is
    /// compiled here, so a mistake in it is reported before the program runs.
    pub(crate) fn check_regex_call(
        &mut self,
        func: &Ident,
        type_args: &[TypeExpr],
        args: &mut [Arg],
        span: Span,
    ) -> Ty {
        let name = func.name.as_str();
        if !REGEX_FUNCTIONS.contains(&name) {
            return self.unknown_module_function("regex", func, &REGEX_FUNCTIONS, args);
        }
        let what = format!("regex.{name}");
        self.no_type_args(&what, type_args);
        self.reject_named_args(args);
        let n = if name == "replace" { 3 } else { 2 };
        let params = vec![Ty::Str; n];
        if self.typed_args(&what, args, &params, span)
            && let ExprKind::Str(pattern) = &args[0].value.kind
            && let Err(e) = regex::Regex::new(pattern)
        {
            let detail = e.to_string();
            let short = detail.lines().last().unwrap_or("").trim().to_string();
            self.error(
                "E2023",
                args[0].value.span,
                elomar_diagnostics::Text::new(
                    format!("invalid regular expression: {short}"),
                    format!("تعبير نمطي غير صالح: {short}"),
                ),
            );
        }
        match name {
            "is_match" => Ty::Bool,
            "find" => Ty::option(Ty::Str),
            "find_all" | "split" => Ty::list(Ty::Str),
            "captures" => Ty::option(Ty::list(Ty::Str)),
            _ => Ty::Str,
        }
    }

    /// `encoding.*`: text to and from Base64, hex and URL encoding.
    pub(crate) fn check_encoding_call(
        &mut self,
        func: &Ident,
        type_args: &[TypeExpr],
        args: &mut [Arg],
        span: Span,
    ) -> Ty {
        let name = func.name.as_str();
        if !ENCODING_FUNCTIONS.contains(&name) {
            return self.unknown_module_function("encoding", func, &ENCODING_FUNCTIONS, args);
        }
        let what = format!("encoding.{name}");
        self.no_type_args(&what, type_args);
        self.reject_named_args(args);
        self.typed_args(&what, args, &[Ty::Str], span);
        if name.ends_with("_decode") {
            Ty::option(Ty::Str)
        } else {
            Ty::Str
        }
    }

    /// `crypto.*`: SHA-256, HMAC and secure random values.
    pub(crate) fn check_crypto_call(
        &mut self,
        func: &Ident,
        type_args: &[TypeExpr],
        args: &mut [Arg],
        span: Span,
    ) -> Ty {
        let name = func.name.as_str();
        if !CRYPTO_FUNCTIONS.contains(&name) {
            return self.unknown_module_function("crypto", func, &CRYPTO_FUNCTIONS, args);
        }
        let what = format!("crypto.{name}");
        self.no_type_args(&what, type_args);
        self.reject_named_args(args);
        let params: &[Ty] = match name {
            "sha256" => &[Ty::Str],
            "hmac_sha256" => &[Ty::Str, Ty::Str],
            "token" => &[Ty::Int],
            _ => &[],
        };
        self.typed_args(&what, args, params, span);
        Ty::Str
    }
}

pub(crate) const SERVER_FUNCTIONS: [&str; 5] = ["serve", "text", "html", "json", "redirect"];

/// `HttpRequest`: what a `server.serve` handler receives.
pub(crate) fn http_request_ty() -> Ty {
    let text_map = Ty::map(Ty::Str, Ty::Str);
    Ty::record(vec![
        ("method".into(), Ty::Str),
        ("path".into(), Ty::Str),
        ("query".into(), text_map.clone()),
        ("headers".into(), text_map),
        ("body".into(), Ty::Str),
    ])
}

/// `HttpResponse`: what a handler returns.
pub(crate) fn http_reply_ty() -> Ty {
    Ty::record(vec![
        ("status".into(), Ty::Int),
        ("headers".into(), Ty::map(Ty::Str, Ty::Str)),
        ("body".into(), Ty::Str),
    ])
}

impl Checker {
    /// `server.serve(address, handler[, max_requests])` and the response
    /// helpers `server.text`, `html`, `json` and `redirect`.
    pub(crate) fn check_server_call(
        &mut self,
        func: &Ident,
        type_args: &[TypeExpr],
        args: &mut [Arg],
        span: Span,
    ) -> Ty {
        let name = func.name.as_str();
        if !SERVER_FUNCTIONS.contains(&name) {
            return self.unknown_module_function("server", func, &SERVER_FUNCTIONS, args);
        }
        let what = format!("server.{name}");
        self.no_type_args(&what, type_args);
        self.reject_named_args(args);
        match name {
            "serve" => {
                self.usage.direct.push(("net".into(), func.span));
                if self.arity(&what, args, 2, 3, span) {
                    self.check_expr_against(&mut args[0].value, &Ty::Str);
                    let handler = Ty::Fn(vec![http_request_ty()], Box::new(http_reply_ty()));
                    self.check_expr_against(&mut args[1].value, &handler);
                    if let Some(max) = args.get_mut(2) {
                        self.check_expr_against(&mut max.value, &Ty::Int);
                    }
                }
                Ty::result(
                    Ty::Unit,
                    Ty::Enum(crate::enums::HTTP_ERROR.into(), Vec::new()),
                )
            }
            "text" | "html" => {
                self.typed_args(&what, args, &[Ty::Int, Ty::Str], span);
                http_reply_ty()
            }
            "json" => {
                if self.arity(&what, args, 2, 2, span) {
                    self.check_expr_against(&mut args[0].value, &Ty::Int);
                    self.check_expr(&mut args[1].value, None);
                }
                http_reply_ty()
            }
            _ => {
                self.typed_args(&what, args, &[Ty::Str], span);
                http_reply_ty()
            }
        }
    }
}
