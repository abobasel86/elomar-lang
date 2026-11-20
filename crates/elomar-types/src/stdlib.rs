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
pub(crate) const HTTP_FUNCTIONS: [&str; 2] = ["get", "post"];
pub(crate) const DB_FUNCTIONS: [&str; 4] = ["open", "execute", "query", "query_one"];
pub(crate) const PY_FUNCTIONS: [&str; 1] = ["call"];
pub(crate) const MCP_FUNCTIONS: [&str; 2] = ["tools", "call"];

/// What `mcp.tools` lists.
pub(crate) fn mcp_tool_ty() -> Ty {
    Ty::record(vec![
        ("name".into(), Ty::Str),
        ("description".into(), Ty::Str),
    ])
}

/// A value that maps to one SQLite column: `Int`, `Float`, `String`,
/// `Bool`, or an `Option` of one (NULL).
fn db_scalar(t: &Ty) -> bool {
    match t {
        Ty::Int | Ty::Float | Ty::Str | Ty::Bool | Ty::Error | Ty::Var(_) => true,
        Ty::Option(inner) => !matches!(**inner, Ty::Option(_)) && db_scalar(inner),
        _ => false,
    }
}

/// What `http.get` and `http.post` return on success.
pub(crate) fn http_response_ty() -> Ty {
    Ty::Record(vec![("body".into(), Ty::Str), ("status".into(), Ty::Int)])
}

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
        Ty::result(ok, Ty::Enum(crate::enums::FS_ERROR.into(), Vec::new()))
    }

    /// `http.get(url)` and `http.post(url, body[, content_type])`: the `net`
    /// effect, `Result<{ status: Int, body: String }, HttpError>`.
    pub(crate) fn check_http_call(
        &mut self,
        func: &Ident,
        type_args: &[TypeExpr],
        args: &mut [Arg],
        span: Span,
    ) -> Ty {
        if !HTTP_FUNCTIONS.contains(&func.name.as_str()) {
            return self.unknown_module_function("http", func, &HTTP_FUNCTIONS, args);
        }
        self.usage.direct.push(("net".into(), func.span));
        let what = format!("http.{}", func.name);
        self.no_type_args(&what, type_args);
        self.reject_named_args(args);
        let (min, max) = if func.name == "get" { (1, 1) } else { (2, 3) };
        if self.arity(&what, args, min, max, span) {
            for arg in args.iter_mut() {
                self.check_expr_against(&mut arg.value, &Ty::Str);
            }
        }
        Ty::result(
            http_response_ty(),
            Ty::Enum(crate::enums::HTTP_ERROR.into(), Vec::new()),
        )
    }

    /// `db.open(path)`, `db.execute(db, sql[, params])`,
    /// `db.query<T>(db, sql[, params])`, `db.query_one<T>(...)`: the `db`
    /// effect, results `Result<_, DbError>`.
    pub(crate) fn check_db_call(
        &mut self,
        func: &Ident,
        type_args: &mut [TypeExpr],
        args: &mut [Arg],
        span: Span,
    ) -> Ty {
        if !DB_FUNCTIONS.contains(&func.name.as_str()) {
            return self.unknown_module_function("db", func, &DB_FUNCTIONS, args);
        }
        self.usage.direct.push(("db".into(), func.span));
        self.reject_named_args(args);
        let what = format!("db.{}", func.name);
        let err = Ty::Enum(crate::enums::DB_ERROR.into(), Vec::new());
        match func.name.as_str() {
            "open" => {
                self.no_type_args(&what, type_args);
                self.typed_args(&what, args, &[Ty::Str], span);
                Ty::result(Ty::Db, err)
            }
            "execute" => {
                self.no_type_args(&what, type_args);
                self.db_args(&what, args, span);
                Ty::result(Ty::Int, err)
            }
            _ => {
                let row = match type_args {
                    [t] => {
                        let ty = self.lower_type(t);
                        let resolved = self.subst.resolve(&ty);
                        let ok = match &resolved {
                            Ty::Record(fields) => fields.iter().all(|(_, f)| db_scalar(f)),
                            other => db_scalar(other) && !matches!(other, Ty::Var(_)),
                        };
                        if ok {
                            *t = crate::ai::type_expr_of(&resolved, t.span);
                            resolved
                        } else {
                            let shown = self.subst.show(&resolved);
                            self.push(
                                Diagnostic::error(
                                    "E2021",
                                    t.span,
                                    Text::new(
                                        format!("`{what}` cannot read rows as `{shown}`"),
                                        format!("لا يمكن أن تقرأ `{what}` الصفوف بالنوع `{shown}`"),
                                    ),
                                )
                                .with_hint(Text::new(
                                    "a row is a record of Int, Float, String, Bool or Option of these (matched to columns by name), or one such value for a single column",
                                    "الصف سجل حقوله Int أو Float أو String أو Bool أو Option منها (تُطابق الأعمدة بالاسم)، أو قيمة واحدة منها لعمود واحد",
                                )),
                            );
                            Ty::Error
                        }
                    }
                    [] => {
                        self.push(Diagnostic::error(
                            "E2008",
                            func.span,
                            Text::new(
                                format!(
                                    "`{what}` needs the row type, e.g. `{what}<Student>(conn, sql)`"
                                ),
                                format!(
                                    "تحتاج `{what}` إلى نوع الصف، مثل `{what}<Student>(conn, sql)`"
                                ),
                            ),
                        ));
                        Ty::Error
                    }
                    _ => {
                        self.wrong_arg_count(&what, "1 type", type_args.len(), span);
                        Ty::Error
                    }
                };
                self.db_args(&what, args, span);
                let out = if func.name == "query" {
                    Ty::list(row)
                } else {
                    Ty::option(row)
                };
                Ty::result(out, err)
            }
        }
    }

    /// `(db, sql[, params])`; `params` is a record of column values.
    fn db_args(&mut self, what: &str, args: &mut [Arg], span: Span) {
        if !self.arity(what, args, 2, 3, span) {
            return;
        }
        self.check_expr_against(&mut args[0].value, &Ty::Db);
        self.check_expr_against(&mut args[1].value, &Ty::Str);
        if let Some(p) = args.get_mut(2) {
            let t = self.check_expr(&mut p.value, None);
            let resolved = self.subst.resolve(&t);
            let ok = match &resolved {
                Ty::Record(fields) => fields.iter().all(|(_, f)| db_scalar(f)),
                Ty::Error => true,
                _ => false,
            };
            if !ok {
                let shown = self.subst.show(&resolved);
                self.push(
                    Diagnostic::error(
                        "E2021",
                        p.value.span,
                        Text::new(
                            format!("query parameters must be a record of column values, not `{shown}`"),
                            format!("معاملات الاستعلام يجب أن تكون سجلًا من قيم الأعمدة، لا `{shown}`"),
                        ),
                    )
                    .with_hint(Text::new(
                        "write `{ min: 60 }` and use `:min` in the SQL; fields are Int, Float, String, Bool or Option of these",
                        "اكتب `{ min: 60 }` واستخدم `:min` في SQL؛ والحقول Int أو Float أو String أو Bool أو Option منها",
                    )),
                );
            }
        }
    }

    /// `py.call<T>(module, function, args[, timeout])`: the `py` effect,
    /// `Result<T, PyError>`. `args` and `T` are JSON data.
    pub(crate) fn check_py_call(
        &mut self,
        func: &Ident,
        type_args: &mut [TypeExpr],
        args: &mut [Arg],
        span: Span,
    ) -> Ty {
        if !PY_FUNCTIONS.contains(&func.name.as_str()) {
            return self.unknown_module_function("py", func, &PY_FUNCTIONS, args);
        }
        self.usage.direct.push(("py".into(), func.span));
        self.reject_named_args(args);
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
                        "`py.call` needs the type of the result, e.g. `py.call<Float>(\"statistics\", \"mean\", { data: xs })`",
                        "تحتاج `py.call` إلى نوع النتيجة، مثل `py.call<Float>(\"statistics\", \"mean\", { data: xs })`",
                    ),
                ));
                Ty::Error
            }
            _ => {
                self.wrong_arg_count("py.call", "1 type", type_args.len(), span);
                Ty::Error
            }
        };
        if self.arity("py.call", args, 3, 4, span) {
            self.check_expr_against(&mut args[0].value, &Ty::Str);
            self.check_expr_against(&mut args[1].value, &Ty::Str);
            let t = self.check_expr(&mut args[2].value, None);
            let resolved = self.subst.resolve(&t);
            if let Some(bad) = self.non_json_part(&resolved, true) {
                self.not_json_data(&resolved, &bad, args[2].value.span, DataRole::JsonValue);
            }
            if let Some(timeout) = args.get_mut(3) {
                self.check_expr_against(&mut timeout.value, &Ty::Duration);
            }
        }
        Ty::result(out, Ty::Enum(crate::enums::PY_ERROR.into(), Vec::new()))
    }

    /// `mcp.tools(server)` and `mcp.call(server, tool, arguments)`: the
    /// `mcp` effect, `Result<_, McpError>`. Arguments are JSON data,
    /// usually a record; the answer of a call is its text.
    pub(crate) fn check_mcp_call(
        &mut self,
        func: &Ident,
        type_args: &mut [TypeExpr],
        args: &mut [Arg],
        span: Span,
    ) -> Ty {
        if !MCP_FUNCTIONS.contains(&func.name.as_str()) {
            return self.unknown_module_function("mcp", func, &MCP_FUNCTIONS, args);
        }
        self.usage.direct.push(("mcp".into(), func.span));
        self.reject_named_args(args);
        let what = format!("mcp.{}", func.name);
        self.no_type_args(&what, type_args);
        let error = Ty::Enum(crate::enums::MCP_ERROR.into(), Vec::new());
        if func.name == "tools" {
            self.typed_args(&what, args, &[Ty::Str], span);
            return Ty::result(Ty::list(mcp_tool_ty()), error);
        }
        if self.arity(&what, args, 3, 3, span) {
            self.check_expr_against(&mut args[0].value, &Ty::Str);
            self.check_expr_against(&mut args[1].value, &Ty::Str);
            let t = self.check_expr(&mut args[2].value, None);
            let resolved = self.subst.resolve(&t);
            if !matches!(
                resolved,
                Ty::Record(_) | Ty::Map(..) | Ty::Error | Ty::Var(_)
            ) {
                let shown = self.subst.show(&resolved);
                self.push(
                    Diagnostic::error(
                        "E2001",
                        args[2].value.span,
                        Text::new(
                            format!("the arguments of an MCP tool must be a record, not `{shown}`"),
                            format!("معاملات أداة MCP يجب أن تكون سجلًا، لا `{shown}`"),
                        ),
                    )
                    .with_hint(Text::new(
                        "e.g. `mcp.call(\"notes\", \"add_note\", { text: \"hi\" })`; use `{}` for none",
                        "مثل `mcp.call(\"notes\", \"add_note\", { text: \"hi\" })`؛ واستخدم `{}` إن لم توجد",
                    )),
                );
            } else if let Some(bad) = self.non_json_part(&resolved, true) {
                self.not_json_data(&resolved, &bad, args[2].value.span, DataRole::JsonValue);
            }
        }
        Ty::result(Ty::Str, error)
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
        Ty::result(out, Ty::Enum(crate::enums::JSON_ERROR.into(), Vec::new()))
    }
}
