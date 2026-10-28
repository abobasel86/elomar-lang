//! The `std.fs` and `std.json` modules (spec §17).
//!
//! As with `std.ai`, the checker rewrites the module name to its full path
//! (`std.fs`, `std.json`) and, for `json.parse<T>`, writes `T` back with
//! aliases expanded so the interpreter can build a schema from it.

use elomar_ast::*;
use elomar_diagnostics::{Diagnostic, Text};

use crate::ai::{DataRole, type_expr_of};
use crate::{Checker, Ty};

pub(crate) const FS_FUNCTIONS: [&str; 5] = [
    "read_text",
    "write_text",
    "append_text",
    "exists",
    "list_dir",
];
pub(crate) const JSON_FUNCTIONS: [&str; 3] = ["parse", "stringify", "pretty"];

impl Checker {
    pub(crate) fn unknown_module_function(
        &mut self,
        module: &str,
        func: &Ident,
        available: &[&str],
        args: &mut [Arg],
    ) -> Ty {
        let list = available.join(", ");
        self.push(
            Diagnostic::error(
                "E1008",
                func.span,
                Text::new(
                    format!("module `{module}` has no function `{}`", func.name),
                    format!("الوحدة `{module}` ليس فيها دالة `{}`", func.name),
                ),
            )
            .with_hint(Text::new(
                format!("available: {list}"),
                format!("المتاح: {list}"),
            )),
        );
        self.check_args_loosely(args);
        Ty::Error
    }

    pub(crate) fn no_type_args(&mut self, what: &str, type_args: &[TypeExpr]) {
        if let Some(first) = type_args.first() {
            self.error(
                "E0103",
                first.span,
                Text::new(
                    format!("`{what}` takes no type argument"),
                    format!("`{what}` لا تأخذ معامل نوع"),
                ),
            );
        }
    }

    /// `fs.*`: every function performs `fs` and returns `Result<_, FsError>`.
    pub(crate) fn check_fs_call(
        &mut self,
        func: &Ident,
        type_args: &[TypeExpr],
        args: &mut [Arg],
        span: Span,
    ) -> Ty {
        if !FS_FUNCTIONS.contains(&func.name.as_str()) {
            return self.unknown_module_function("fs", func, &FS_FUNCTIONS, args);
        }
        self.usage.direct.push(("fs".into(), func.span));
        self.no_type_args(&format!("fs.{}", func.name), type_args);
        self.reject_named_args(args);
        let what = format!("fs.{}", func.name);
        let ok = match func.name.as_str() {
            "read_text" => {
                self.typed_args(&what, args, &[Ty::Str], span);
                Ty::Str
            }
            "write_text" | "append_text" => {
                self.typed_args(&what, args, &[Ty::Str, Ty::Str], span);
                Ty::Unit
            }
            "exists" => {
                self.typed_args(&what, args, &[Ty::Str], span);
                Ty::Bool
            }
            _ => {
                self.typed_args(&what, args, &[Ty::Str], span);
                Ty::list(Ty::Str)
            }
        };
        Ty::result(ok, Ty::Enum(crate::enums::FS_ERROR.into()))
    }

    /// `json.parse<T>(text)`, `json.stringify(value)`, `json.pretty(value)`.
    pub(crate) fn check_json_call(
        &mut self,
        func: &Ident,
        type_args: &mut [TypeExpr],
        args: &mut [Arg],
        span: Span,
    ) -> Ty {
        if !JSON_FUNCTIONS.contains(&func.name.as_str()) {
            return self.unknown_module_function("json", func, &JSON_FUNCTIONS, args);
        }
        self.reject_named_args(args);
        let what = format!("json.{}", func.name);
        if func.name != "parse" {
            self.no_type_args(&what, type_args);
            if self.arity(&what, args, 1, 1, span) {
                let t = self.check_expr(&mut args[0].value, None);
                let resolved = self.subst.resolve(&t);
                if let Some(bad) = self.non_json_part(&resolved, true) {
                    self.not_json_data(&resolved, &bad, args[0].value.span, DataRole::JsonValue);
                }
            }
            return Ty::Str;
        }
        let out = match type_args {
            [t] => {
                let ty = self.lower_type(t);
                let resolved = self.subst.resolve(&ty);
                if let Some(bad) = self.non_json_part(&resolved, true) {
                    self.not_json_data(&resolved, &bad, t.span, DataRole::JsonParse);
                    Ty::Error
                } else {
                    *t = type_expr_of(&resolved, t.span);
                    resolved
                }
            }
            [] => {
                self.push(Diagnostic::error(
                    "E2008",
                    func.span,
                    Text::new(
                        "`json.parse` needs the type to read, e.g. `json.parse<Student>(text)`",
                        "تحتاج `json.parse` إلى النوع المطلوب، مثل `json.parse<Student>(text)`",
                    ),
                ));
                Ty::Error
            }
            _ => {
                self.wrong_arg_count("json.parse", "1 type", type_args.len(), span);
                Ty::Error
            }
        };
        self.typed_args(&what, args, &[Ty::Str], span);
        Ty::result(out, Ty::Enum(crate::enums::JSON_ERROR.into()))
    }
}
