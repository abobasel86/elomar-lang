//! Checking statements, expressions and patterns.

use elomar_ast::*;
use elomar_diagnostics::{Diagnostic, Text};

use crate::{Checker, FnSig, Ty, suggest};

impl Checker {
    pub(crate) fn check_block(&mut self, block: &mut Block) {
        self.push_scope();
        for stmt in &mut block.stmts {
            self.check_stmt(stmt);
        }
        self.pop_scope();
    }

    fn check_stmt(&mut self, stmt: &mut Stmt) {
        let stmt_span = stmt.span;
        let loop_word = match stmt.kind {
            StmtKind::Break => "break",
            _ => "continue",
        };
        match &mut stmt.kind {
            StmtKind::Let {
                mutable,
                name,
                ty,
                value,
            } => {
                let t = match ty {
                    Some(t) => {
                        let t = self.lower_type(t);
                        self.check_expr_against(value, &t);
                        t
                    }
                    None => self.check_expr(value, None),
                };
                let (name, mutable) = (name.clone(), *mutable);
                self.declare(&name, t, mutable, false);
            }
            StmtKind::LetTuple {
                mutable,
                names,
                value,
            } => {
                let t = self.check_expr(value, None);
                let (names, mutable) = (names.clone(), *mutable);
                let types = self.unpack(&t, names.len(), value.span);
                for (name, ty) in names.iter().zip(types) {
                    if name.name != "_" {
                        self.declare(name, ty, mutable, false);
                    }
                }
            }
            StmtKind::Assign { target, op, value } => {
                let op = *op;
                self.check_mutable_place(target);
                let target_ty = self.check_expr(target, None);
                let bin = match op {
                    AssignOp::Set => {
                        self.check_expr_against(value, &target_ty);
                        return;
                    }
                    AssignOp::Add => BinOp::Add,
                    AssignOp::Sub => BinOp::Sub,
                    AssignOp::Mul => BinOp::Mul,
                    AssignOp::Div => BinOp::Div,
                };
                let hint = matches!(self.subst.shallow(&target_ty), Ty::Float).then_some(Ty::Float);
                let value_ty = self.check_expr(value, hint.as_ref());
                let result = self.arith_type(bin, &target_ty, &value_ty, stmt_span);
                if !self.subst.unify(&target_ty, &result) {
                    self.mismatch(&target_ty, &result, stmt_span);
                    if bin == BinOp::Div
                        && let Some(d) = self.diags.last_mut()
                    {
                        d.hint = Some(Text::new(
                            "`/` always produces a `Float`; use `x = x div n` for integer division",
                            "العامل `/` يُنتج دائمًا `Float`؛ استخدم `x = x div n` للقسمة الصحيحة",
                        ));
                    }
                }
            }
            StmtKind::If {
                cond,
                then,
                otherwise,
            } => {
                self.check_condition(cond);
                self.check_block(then);
                match otherwise {
                    Some(Else::Block(b)) => self.check_block(b),
                    Some(Else::If(s)) => self.check_stmt(s),
                    None => {}
                }
            }
            StmtKind::While { cond, body } => {
                self.check_condition(cond);
                self.loop_depth += 1;
                self.check_block(body);
                self.loop_depth -= 1;
            }
            StmtKind::For { vars, iter, body } => {
                let iter_ty = self.check_expr(iter, None);
                let elem = self.element_type(&iter_ty, iter.span);
                self.push_scope();
                let vars = vars.clone();
                let types = match vars.len() {
                    1 => vec![elem],
                    n => self.unpack(&elem, n, iter.span),
                };
                for (var, ty) in vars.iter().zip(types) {
                    if var.name != "_" {
                        self.declare(var, ty, false, false);
                    }
                }
                self.loop_depth += 1;
                self.check_block(body);
                self.loop_depth -= 1;
                self.pop_scope();
            }
            StmtKind::Return(value) => {
                let ret = self.ret_ty.clone().unwrap_or(Ty::Error);
                match value {
                    Some(v) => self.check_expr_against(v, &ret),
                    None => {
                        if !self.subst.unify(&ret, &Ty::Unit) {
                            self.mismatch(&ret, &Ty::Unit, stmt_span);
                        }
                    }
                }
            }
            StmtKind::Break | StmtKind::Continue => {
                if self.loop_depth == 0 {
                    let word = loop_word;
                    self.error(
                        "E1004",
                        stmt_span,
                        Text::new(
                            format!("`{word}` can only be used inside a loop"),
                            format!("لا يمكن استخدام `{word}` إلا داخل حلقة تكرار"),
                        ),
                    );
                }
            }
            StmtKind::Block(b) => self.check_block(b),
            StmtKind::Expr(e) => {
                if let ExprKind::Match { scrutinee, arms } = &mut e.kind {
                    self.check_match(scrutinee, arms, e.span, None, false);
                    return;
                }
                let ty = self.check_expr(e, None);
                if matches!(self.subst.shallow(&ty), Ty::Future(_)) {
                    self.push(
                        Diagnostic::warning(
                            "W1003",
                            e.span,
                            Text::new(
                                "this future is never awaited, so it never runs",
                                "لا يُنتظر ناتج هذه العملية غير المتزامنة بـ `await`، لذا لن تُنفَّذ",
                            ),
                        )
                        .with_hint(Text::new("add `await` before it", "أضف `await` قبلها")),
                    );
                    return;
                }
                // A call's result may be ignored, also after `?` (`db.execute(...)?`).
                let is_call = match &e.kind {
                    ExprKind::Call { .. } => true,
                    ExprKind::Try(inner) => matches!(inner.kind, ExprKind::Call { .. }),
                    _ => false,
                };
                let unused = !is_call && !matches!(self.subst.shallow(&ty), Ty::Unit | Ty::Error);
                if unused {
                    let mut diag = Diagnostic::warning(
                        "W1002",
                        e.span,
                        Text::new("this value is never used", "هذه القيمة لا تُستخدم"),
                    );
                    if matches!(e.kind, ExprKind::Binary { op: BinOp::Eq, .. }) {
                        diag = diag.with_hint(Text::new(
                            "to assign, use `=` instead of `==`",
                            "للإسناد استخدم `=` بدل `==`",
                        ));
                    }
                    self.push(diag);
                }
            }
        }
    }

    fn check_condition(&mut self, cond: &mut Expr) {
        let ty = self.check_expr(cond, Some(&Ty::Bool));
        if !self.subst.unify(&Ty::Bool, &ty) {
            let found = self.subst.show(&ty);
            self.push(
                Diagnostic::error(
                    "E2003",
                    cond.span,
                    Text::new(
                        format!("condition must be a `Bool`, found `{found}`"),
                        format!("الشرط يجب أن يكون `Bool`، لكن وُجد `{found}`"),
                    ),
                )
                .with_hint(Text::new(
                    "write an explicit comparison, e.g. `count > 0` or `name != \"\"`",
                    "اكتب مقارنة صريحة، مثل `count > 0` أو `name != \"\"`",
                )),
            );
        }
    }

    /// The types of the `n` names in `let (a, b) = ...` or `for (k, v) in`:
    /// the elements of a tuple, or a map entry's key and value.
    fn unpack(&mut self, t: &Ty, n: usize, span: Span) -> Vec<Ty> {
        match self.subst.shallow(t) {
            Ty::Tuple(items) if items.len() == n => return items,
            Ty::Record(fields)
                if n == 2
                    && fields.len() == 2
                    && fields[0].0 == "key"
                    && fields[1].0 == "value" =>
            {
                return fields.into_iter().map(|(_, t)| t).collect();
            }
            Ty::Var(_) => {
                let items: Vec<Ty> = (0..n).map(|_| self.subst.fresh()).collect();
                self.subst.unify(t, &Ty::Tuple(items.clone()));
                return items;
            }
            Ty::Error => {}
            other => {
                let shown = self.subst.show(&other);
                self.error(
                    "E2026",
                    span,
                    Text::new(
                        format!(
                            "cannot unpack `{shown}` into {n} names; it needs a tuple of {n} values"
                        ),
                        format!("لا يمكن تفريق `{shown}` إلى {n} أسماء؛ يلزم صف (tuple) من {n} قيم"),
                    ),
                );
            }
        }
        vec![Ty::Error; n]
    }

    fn element_type(&mut self, iter_ty: &Ty, span: Span) -> Ty {
        match self.subst.shallow(iter_ty) {
            Ty::List(t) => *t,
            Ty::Range => Ty::Int,
            Ty::Str => Ty::Str,
            Ty::Map(k, v) => Ty::record(vec![("key".into(), *k), ("value".into(), *v)]),
            Ty::Set(t) => *t,
            Ty::Error => Ty::Error,
            Ty::Var(_) => {
                self.cannot_infer(span);
                Ty::Error
            }
            other => {
                let shown = self.subst.show(&other);
                self.push(
                    Diagnostic::error(
                        "E2012",
                        span,
                        Text::new(
                            format!("cannot iterate over a value of type `{shown}`"),
                            format!("لا يمكن التكرار على قيمة من النوع `{shown}`"),
                        ),
                    )
                    .with_hint(Text::new(
                        "use a `List`, a range such as `0..n`, or a `String`",
                        "استخدم `List` أو نطاقًا مثل `0..n` أو `String`",
                    )),
                );
                Ty::Error
            }
        }
    }

    pub(crate) fn cannot_infer(&mut self, span: Span) {
        self.push(
            Diagnostic::error(
                "E2008",
                span,
                Text::new(
                    "cannot infer the type of this value",
                    "تعذّر استنتاج نوع هذه القيمة",
                ),
            )
            .with_hint(Text::new(
                "add a type annotation, e.g. `let xs: List<Int> = []`",
                "أضف تصريحًا بالنوع، مثل `let xs: List<Int> = []`",
            )),
        );
    }

    /// Reports `E1003` if `target` is not rooted at a `var`.
    pub(crate) fn check_mutable_place(&mut self, target: &Expr) {
        let Some((name, span)) = place_root(target) else {
            return;
        };
        let captured = self.closure_scope.is_some_and(|boundary| {
            self.scopes
                .iter()
                .rposition(|s| s.contains_key(name))
                .is_some_and(|i| i < boundary)
        });
        if captured {
            self.push(
                Diagnostic::error(
                    "E1013",
                    span,
                    Text::new(
                        format!("`{name}` cannot be changed inside a closure: closures get a copy of the variables they use"),
                        format!("لا يمكن تعديل `{name}` داخل دالة مجهولة: الدالة المجهولة تأخذ نسخة من المتغيرات التي تستخدمها"),
                    ),
                )
                .with_hint(Text::new(
                    "return the new value from the closure instead, or use `fold`",
                    "أرجع القيمة الجديدة من الدالة المجهولة بدلًا من ذلك، أو استخدم `fold`",
                )),
            );
            return;
        }
        if let Some(local) = self.lookup_local(name) {
            if local.mutable {
                return;
            }
            let (decl_span, is_param) = (local.span, local.is_param);
            let hint = if is_param {
                Text::new(
                    format!("parameters cannot be modified; copy it first: `var {name} = {name}`"),
                    format!("لا يمكن تعديل المعاملات؛ انسخه أولًا: `var {name} = {name}`"),
                )
            } else {
                Text::new(
                    format!("declare it with `var {name}` to allow changes"),
                    format!("عرّفه بـ `var {name}` للسماح بالتعديل"),
                )
            };
            self.push(
                Diagnostic::error(
                    "E1003",
                    span,
                    Text::new(
                        format!("cannot modify `{name}` because it is not declared with `var`"),
                        format!("لا يمكن تعديل `{name}` لأنه غير معرّف بـ `var`"),
                    ),
                )
                .with_note(decl_span, Text::new("declared here", "عُرّف هنا"))
                .with_hint(hint),
            );
        } else if self.consts.contains_key(name) {
            self.error(
                "E1003",
                span,
                Text::new(
                    format!("cannot modify the constant `{name}`"),
                    format!("لا يمكن تعديل الثابت `{name}`"),
                ),
            );
        }
    }

    pub(crate) fn check_expr_against(&mut self, e: &mut Expr, expected: &Ty) {
        let found = self.check_expr(e, Some(expected));
        self.expect_ty(expected, &found, e.span);
    }

    /// Infers the type of `e`. `hint` is the expected type when known; it is
    /// used to type empty lists, constructors and int-as-float literals.
    pub(crate) fn check_expr(&mut self, e: &mut Expr, hint: Option<&Ty>) -> Ty {
        let span = e.span;
        let hint_resolved = hint.map(|h| self.subst.shallow(h));
        if let ExprKind::Int(n) = e.kind {
            if matches!(hint_resolved, Some(Ty::Float)) {
                e.kind = ExprKind::Float(n as f64);
                return Ty::Float;
            }
            return Ty::Int;
        }
        if self.resolve_refs(&mut e.kind) {
            return Ty::Error;
        }
        if let Some(enum_name) = self.enum_target(&e.kind) {
            return self.check_variant_expr(e, enum_name);
        }
        match &mut e.kind {
            ExprKind::Int(_) => unreachable!("handled above"),
            ExprKind::Float(_) => Ty::Float,
            ExprKind::Str(_) => Ty::Str,
            ExprKind::Tuple(items) => {
                let hints: Vec<Option<Ty>> = match hint_resolved {
                    Some(Ty::Tuple(h)) if h.len() == items.len() => {
                        h.into_iter().map(Some).collect()
                    }
                    _ => vec![None; items.len()],
                };
                let mut out = Vec::with_capacity(items.len());
                for (item, hint) in items.iter_mut().zip(hints) {
                    out.push(match hint {
                        Some(h) => {
                            self.check_expr_against(item, &h);
                            h
                        }
                        None => self.check_expr(item, None),
                    });
                }
                Ty::Tuple(out)
            }
            ExprKind::Interp(parts) => {
                // Any value can be shown as text, as with `to_string`.
                for part in parts {
                    if let InterpPart::Expr(inner) = part {
                        self.check_expr(inner, None);
                    }
                }
                Ty::Str
            }
            ExprKind::Bool(_) => Ty::Bool,
            ExprKind::Name(name) => {
                let name = name.clone();
                self.check_name(&name, span)
            }
            ExprKind::List(items) => {
                let elem = match hint_resolved {
                    Some(Ty::List(t)) => *t,
                    _ => self.subst.fresh(),
                };
                for item in items {
                    self.check_expr_against(item, &elem);
                }
                Ty::list(elem)
            }
            ExprKind::Record(fields) => {
                let hint_fields = match hint_resolved {
                    Some(Ty::Record(f)) => f,
                    _ => Vec::new(),
                };
                let mut out: Vec<(String, Ty)> = Vec::new();
                for field in fields {
                    if out.iter().any(|(n, _)| n == &field.name.name) {
                        self.error(
                            "E1002",
                            field.name.span,
                            Text::new(
                                format!("field `{}` is given twice", field.name.name),
                                format!("الحقل `{}` مذكور مرتين", field.name.name),
                            ),
                        );
                        continue;
                    }
                    let fhint = hint_fields
                        .iter()
                        .find(|(n, _)| n == &field.name.name)
                        .map(|(_, t)| t.clone());
                    let t = self.check_expr(&mut field.value, fhint.as_ref());
                    out.push((field.name.name.clone(), t));
                }
                Ty::record(out)
            }
            ExprKind::Unary { op, expr } => match op {
                UnaryOp::Neg => {
                    let t = self.check_expr(expr, hint);
                    let resolved = self.subst.shallow(&t);
                    if !resolved.is_numeric() && !matches!(resolved, Ty::Error) {
                        self.bad_operand("-", &t, None, span);
                        return Ty::Error;
                    }
                    t
                }
                UnaryOp::Not => {
                    let t = self.check_expr(expr, Some(&Ty::Bool));
                    if !self.subst.unify(&Ty::Bool, &t) {
                        self.bad_operand("!", &t, None, span);
                    }
                    Ty::Bool
                }
            },
            ExprKind::Binary { op, lhs, rhs } => {
                let op = *op;
                self.check_binary(op, lhs, rhs, hint_resolved.as_ref(), span)
            }
            ExprKind::Range { start, end, .. } => {
                self.check_expr_against(start, &Ty::Int);
                self.check_expr_against(end, &Ty::Int);
                Ty::Range
            }
            ExprKind::Call {
                callee,
                type_args,
                args,
            } => self.check_call(callee, type_args, args, span, hint_resolved.as_ref()),
            ExprKind::Duration(_) => Ty::Duration,
            ExprKind::Await(inner) => self.check_await(inner, span),
            ExprKind::Try(inner) => self.check_try(inner, span),
            ExprKind::Lambda { params, body } => {
                self.check_lambda(params, body, hint_resolved.as_ref())
            }
            ExprKind::Variant { enum_name, .. } => self.enum_instance(&enum_name.clone()).0,
            ExprKind::Map(entries) => {
                let (k, v) = match hint_resolved {
                    Some(Ty::Map(k, v)) => (*k, *v),
                    _ => (self.subst.fresh(), self.subst.fresh()),
                };
                for (key, value) in entries {
                    self.check_expr_against(key, &k);
                    self.check_expr_against(value, &v);
                }
                self.check_map_key(&k, span);
                Ty::map(k, v)
            }
            ExprKind::Field { target, name } => {
                let name = name.clone();
                if let Some(ty) = self.try_module_constant(target, &name) {
                    return ty;
                }
                let t = self.check_expr(target, None);
                let ty = self.field_type(&t, &name);
                self.note_typed(name.span, format!("{}: ", name.name), &ty);
                ty
            }
            ExprKind::Index { target, index } => {
                let t = self.check_expr(target, None);
                let target_span = target.span;
                match self.subst.shallow(&t) {
                    Ty::List(elem) => {
                        self.check_expr_against(index, &Ty::Int);
                        *elem
                    }
                    Ty::Map(k, v) => {
                        self.check_expr_against(index, &k);
                        *v
                    }
                    Ty::Error => {
                        self.check_expr(index, None);
                        Ty::Error
                    }
                    Ty::Var(_) => {
                        self.cannot_infer(target_span);
                        Ty::Error
                    }
                    other => {
                        self.check_expr(index, None);
                        let shown = self.subst.show(&other);
                        let mut diag = Diagnostic::error(
                            "E2006",
                            span,
                            Text::new(
                                format!("a value of type `{shown}` cannot be indexed with `[]`"),
                                format!("لا يمكن الفهرسة بـ `[]` في قيمة من النوع `{shown}`"),
                            ),
                        );
                        if other == Ty::Str {
                            diag = diag.with_hint(Text::new(
                                "strings are not indexed by number; use `s.chars()[i]`",
                                "النصوص لا تُفهرس بالأرقام؛ استخدم `s.chars()[i]`",
                            ));
                        }
                        self.push(diag);
                        Ty::Error
                    }
                }
            }
            ExprKind::Match { scrutinee, arms } => {
                self.check_match(scrutinee, arms, span, hint_resolved.as_ref(), true)
            }
        }
    }

    fn check_name(&mut self, name: &str, span: Span) -> Ty {
        if let Some(local) = self.lookup_local(name).cloned() {
            self.note_local(name, &local, span);
            return local.ty;
        }
        if let Some((ty, def)) = self.consts.get(name).cloned() {
            self.note_typed(
                span,
                format!("let {}: ", crate::modules::base_name(name)),
                &ty,
            );
            self.note_def(span, def);
            return ty;
        }
        if name == "None" {
            return Ty::option(self.subst.fresh());
        }
        if matches!(name, "Some" | "Ok" | "Err") {
            self.error(
                "E2002",
                span,
                Text::new(
                    format!("`{name}` needs a value, e.g. `{name}(x)`"),
                    format!("`{name}` يحتاج إلى قيمة، مثل `{name}(x)`"),
                ),
            );
            return Ty::Error;
        }
        if self.app_target(name).is_some() {
            self.error(
                "E2004",
                span,
                Text::new(
                    format!("`{name}` is a module; use one of its items, e.g. `{name}.something`"),
                    format!("`{name}` وحدة؛ استخدم أحد عناصرها، مثل `{name}.something`"),
                ),
            );
            return Ty::Error;
        }
        if self.enums.contains_key(name) {
            self.error(
                "E2004",
                span,
                Text::new(
                    format!("`{name}` is a type; use one of its variants, e.g. `{name}.Variant`"),
                    format!("`{name}` نوع؛ استخدم إحدى حالاته، مثل `{name}.Variant`"),
                ),
            );
            return Ty::Error;
        }
        if self.std_imported(name) {
            self.error(
                "E2004",
                span,
                Text::new(
                    format!("`{name}` is a module, not a value; call one of its functions, e.g. `{name}.infer(...)`"),
                    format!("`{name}` وحدة وليست قيمة؛ استدعِ إحدى دوالها، مثل `{name}.infer(...)`"),
                ),
            );
            return Ty::Error;
        }
        if let Some(sig) = self.fns.get(name).cloned().filter(|s| !s.bounds.is_empty()) {
            let _ = sig;
            self.push(
                Diagnostic::error(
                    "E0103",
                    span,
                    Text::new(
                        "a function with trait bounds cannot be used as a value yet",
                        "لا يمكن بعد استخدام دالة ذات قيود واجهات كقيمة",
                    ),
                )
                .with_hint(Text::new(
                    format!("wrap it in a closure: `|x| {name}(x)`"),
                    format!("ضعها داخل دالة مجهولة: `|x| {name}(x)`"),
                )),
            );
            return Ty::Error;
        }
        if let Some(sig) = self.fns.get(name).cloned() {
            self.note_fn(span, name);
            self.note_def(span, sig.name_span);
            // A named function used as a value. Its effects are charged
            // here, where it is referred to (see `check_lambda`).
            self.usage.calls.push((name.to_string(), span));
            let sig = self.instantiate(name, sig, &[], span);
            let params = sig.params.iter().map(|(_, t)| t.clone()).collect();
            let ret = if sig.is_async {
                Ty::future(sig.ret)
            } else {
                sig.ret
            };
            return Ty::Fn(params, Box::new(ret));
        }
        if crate::BUILTIN_FUNCTIONS.contains(&name) {
            self.push(
                Diagnostic::error(
                    "E0103",
                    span,
                    Text::new(
                        "a built-in function cannot be used as a value yet",
                        "لا يمكن استخدام الدوال المدمجة كقيم بعد",
                    ),
                )
                .with_hint(Text::new(
                    format!("wrap it in a closure: `|x| {name}(x)`"),
                    format!("ضعها داخل دالة مجهولة: `|x| {name}(x)`"),
                )),
            );
            return Ty::Error;
        }
        self.unknown_name(name, span);
        Ty::Error
    }

    fn check_call(
        &mut self,
        callee: &mut Expr,
        type_args: &mut Vec<TypeExpr>,
        args: &mut Vec<Arg>,
        span: Span,
        hint: Option<&Ty>,
    ) -> Ty {
        let callee_span = callee.span;
        if let ExprKind::Field { target, name } = &mut callee.kind
            && let ExprKind::Name(module) = &mut target.kind
            && let Some(ty) = self.try_module_call(module, name, type_args, args, span)
        {
            return ty;
        }
        let generic_callee = matches!(&callee.kind, ExprKind::Name(n)
            if self.lookup_local(n).is_none()
                && self.fns.get(n).is_some_and(|s| !s.type_params.is_empty()));
        if let Some(first) = type_args.first().filter(|_| !generic_callee) {
            let targs_span = first.span.to(type_args.last().expect("non-empty").span);
            self.push(
                Diagnostic::error(
                    "E0103",
                    targs_span,
                    Text::new(
                        "explicit type arguments are not supported yet",
                        "معاملات النوع الصريحة غير مدعومة بعد",
                    ),
                )
                .with_hint(Text::new(
                    "generics are planned for v0.3; today only `ai.infer<T>` and `json.parse<T>` take a type argument",
                    "الأنواع العامة مخطط لها في v0.3؛ حاليًا تأخذ `ai.infer<T>` و`json.parse<T>` وحدهما معامل نوع",
                )),
            );
        }
        match &mut callee.kind {
            ExprKind::Name(name) => {
                let name = name.clone();
                if let Some(local) = self.lookup_local(&name).cloned() {
                    self.note_local(&name, &local, callee_span);
                    return self.check_value_call(&name, &local.ty, args, callee_span);
                }
                if let Some((ty, def)) = self.consts.get(&name).cloned() {
                    self.note_typed(
                        callee_span,
                        format!("let {}: ", crate::modules::base_name(&name)),
                        &ty,
                    );
                    self.note_def(callee_span, def);
                    return self.check_value_call(&name, &ty, args, callee_span);
                }
                if let Some(sig) = self.fns.get(&name).cloned() {
                    self.note_fn(callee_span, &name);
                    self.note_def(callee_span, sig.name_span);
                    let bounds = sig.bounds.clone();
                    let (sig, map) = self.instantiate_with_map(&name, sig, type_args, span);
                    let ty = self.check_user_call(&name, &sig, args, span, callee_span);
                    if !bounds.is_empty() {
                        self.pass_dictionaries(&name, &bounds, &map, args, callee_span);
                    }
                    return ty;
                }
                if CONSTRUCTORS.contains(&name.as_str()) {
                    return self.check_ctor(&name, args, span, hint);
                }
                if crate::BUILTIN_FUNCTIONS.contains(&name.as_str()) {
                    return self.check_builtin_call(&name, args, span, callee_span, hint);
                }
                self.unknown_name(&name, callee_span);
                self.check_args_loosely(args);
                Ty::Error
            }
            ExprKind::Field { .. } => {
                let ExprKind::Field { target, name } = &mut callee.kind else {
                    unreachable!()
                };
                let method = name.clone();
                let recv_ty = self.check_expr(target, None);
                if let Some(ty) = self.try_user_method(callee, &recv_ty, &method, args, span) {
                    return ty;
                }
                let ExprKind::Field { target, name } = &mut callee.kind else {
                    unreachable!()
                };
                let ty = self.check_method(target, &recv_ty, &method, args, span);
                // An empty list's sum must still be a Float when the list is
                // `List<Float>`; tell the interpreter, which has no types.
                if method.name == "sum"
                    && matches!(self.subst.resolve(&recv_ty), Ty::List(e) if *e == Ty::Float)
                {
                    name.name = "sum.float".into();
                }
                ty
            }
            _ => {
                let t = self.check_expr(callee, None);
                self.check_value_call("this value", &t, args, callee_span)
            }
        }
    }

    /// Calling a value of function type: a closure, a function stored in a
    /// variable, or one returned by another call.
    fn check_value_call(&mut self, what: &str, ty: &Ty, args: &mut [Arg], span: Span) -> Ty {
        let mut fty = self.subst.shallow(ty);
        if let Ty::Var(_) = fty {
            // Called before its type is known: it is a function of these arguments.
            let guess = Ty::Fn(
                args.iter().map(|_| self.subst.fresh()).collect(),
                Box::new(self.subst.fresh()),
            );
            self.subst.unify(ty, &guess);
            fty = guess;
        }
        match fty {
            Ty::Fn(params, ret) => {
                self.reject_named_args(args);
                if args.len() != params.len() {
                    self.wrong_arg_count(what, &params.len().to_string(), args.len(), span);
                    self.check_args_loosely(args);
                } else {
                    for (arg, p) in args.iter_mut().zip(&params) {
                        self.check_expr_against(&mut arg.value, p);
                    }
                }
                *ret
            }
            Ty::Error => {
                self.check_args_loosely(args);
                Ty::Error
            }
            other => {
                self.not_callable(what, &other, span);
                self.check_args_loosely(args);
                Ty::Error
            }
        }
    }

    /// `|x, y| body`. Parameter types come from annotations, from the type
    /// the context expects (e.g. `xs.map(...)` on a `List<Int>`), or are
    /// inferred from the body.
    ///
    /// Effects: the body's effects are charged to the function that creates
    /// the closure, as if the body ran there; calling a function value is
    /// effect-free. Captured variables are copies and cannot be changed.
    fn check_lambda(
        &mut self,
        params: &[LambdaParam],
        body: &mut ArmBody,
        hint: Option<&Ty>,
    ) -> Ty {
        let (hint_params, hint_ret) = match hint {
            Some(Ty::Fn(ps, r)) if ps.len() == params.len() => (ps.clone(), Some((**r).clone())),
            _ => (Vec::new(), None),
        };
        let mut types = Vec::with_capacity(params.len());
        for (i, p) in params.iter().enumerate() {
            let expected = hint_params.get(i).cloned();
            let ty = match (&p.ty, expected) {
                (Some(t), expected) => {
                    let ty = self.lower_type(t);
                    if let Some(e) = expected
                        && !self.subst.unify(&e, &ty)
                    {
                        self.mismatch(&e, &ty, t.span);
                    }
                    ty
                }
                (None, Some(e)) => e,
                (None, None) => self.subst.fresh(),
            };
            types.push(ty);
        }
        let ret = hint_ret.unwrap_or_else(|| self.subst.fresh());

        let saved_ret = self.ret_ty.replace(ret.clone());
        let saved = (self.loop_depth, self.in_async, self.closure_scope);
        self.loop_depth = 0;
        self.in_async = false;
        self.push_scope();
        self.closure_scope = Some(self.scopes.len() - 1);
        for (p, t) in params.iter().zip(&types) {
            self.declare(&p.name, t.clone(), false, true);
        }
        match body {
            ArmBody::Expr(e) => self.check_expr_against(e, &ret),
            ArmBody::Block(b) => {
                self.check_block(b);
                if !block_diverges(b) && !self.subst.unify(&ret, &Ty::Unit) {
                    self.mismatch(&ret, &Ty::Unit, b.span);
                }
            }
        }
        self.pop_scope();
        self.ret_ty = saved_ret;
        (self.loop_depth, self.in_async, self.closure_scope) = saved;
        Ty::Fn(types, Box::new(ret))
    }

    fn not_callable(&mut self, name: &str, ty: &Ty, span: Span) {
        let shown = self.subst.show(ty);
        self.error(
            "E2004",
            span,
            Text::new(
                format!("`{name}` is a value of type `{shown}`, not a function"),
                format!("`{name}` قيمة من النوع `{shown}` وليست دالة"),
            ),
        );
    }

    /// Checks arguments without expectations, after a call error.
    pub(crate) fn check_args_loosely(&mut self, args: &mut [Arg]) {
        for arg in args {
            self.check_expr(&mut arg.value, None);
        }
    }

    pub(crate) fn reject_named_args(&mut self, args: &[Arg]) {
        for arg in args {
            if let Some(name) = &arg.name {
                self.error(
                    "E2010",
                    name.span,
                    Text::new(
                        "named arguments are only supported for functions you define",
                        "المعاملات المسماة مدعومة فقط للدوال التي تعرّفها بنفسك",
                    ),
                );
            }
        }
    }

    pub(crate) fn wrong_arg_count(&mut self, what: &str, want: &str, got: usize, span: Span) {
        self.error(
            "E2002",
            span,
            Text::new(
                format!("`{what}` takes {want} argument(s) but {got} were given"),
                format!("`{what}` يأخذ {want} من المعاملات لكن أُعطي {got}"),
            ),
        );
    }

    /// A generic function's signature with its type parameters replaced by
    /// the explicit type arguments, or by fresh variables to be inferred.
    pub(crate) fn instantiate(
        &mut self,
        name: &str,
        sig: FnSig,
        type_args: &[TypeExpr],
        span: Span,
    ) -> FnSig {
        self.instantiate_with_map(name, sig, type_args, span).0
    }

    /// Like `instantiate`, also returning what each type parameter became.
    fn instantiate_with_map(
        &mut self,
        name: &str,
        sig: FnSig,
        type_args: &[TypeExpr],
        span: Span,
    ) -> (FnSig, Vec<(String, Ty)>) {
        if sig.type_params.is_empty() {
            return (sig, Vec::new());
        }
        if !type_args.is_empty() && type_args.len() != sig.type_params.len() {
            self.wrong_arg_count(
                name,
                &format!("{} type", sig.type_params.len()),
                type_args.len(),
                span,
            );
        }
        let map: Vec<(String, Ty)> = sig
            .type_params
            .iter()
            .enumerate()
            .map(|(i, p)| {
                let ty = match type_args.get(i) {
                    Some(t) if type_args.len() == sig.type_params.len() => self.lower_type(t),
                    _ => self.subst.fresh(),
                };
                (p.clone(), ty)
            })
            .collect();
        let sig = FnSig {
            type_params: Vec::new(),
            params: sig
                .params
                .iter()
                .map(|(n, t)| (n.clone(), t.substitute(&map)))
                .collect(),
            ret: sig.ret.substitute(&map),
            ..sig
        };
        (sig, map)
    }

    /// Hover and definition for a use of a local variable.
    fn note_local(&mut self, name: &str, local: &crate::Local, span: Span) {
        let kind = if local.is_param {
            "param"
        } else if local.mutable {
            "var"
        } else {
            "let"
        };
        self.note_typed(span, format!("{kind} {name}: "), &local.ty);
        self.note_def(span, local.span);
    }

    pub(crate) fn check_user_call(
        &mut self,
        name: &str,
        sig: &FnSig,
        args: &mut [Arg],
        span: Span,
        callee_span: Span,
    ) -> Ty {
        self.usage.calls.push((name.to_string(), callee_span));
        let mut filled = vec![false; sig.params.len()];
        let mut seen_named = false;
        for (i, arg) in args.iter_mut().enumerate() {
            let slot = match &arg.name {
                None => {
                    if seen_named {
                        self.error(
                            "E2010",
                            arg.value.span,
                            Text::new(
                                "positional arguments must come before named ones",
                                "المعاملات الموضعية يجب أن تسبق المعاملات المسماة",
                            ),
                        );
                    }
                    (i < sig.params.len()).then_some(i)
                }
                Some(id) => {
                    seen_named = true;
                    match sig.params.iter().position(|(n, _)| n == &id.name) {
                        Some(j) if filled[j] => {
                            self.error(
                                "E2010",
                                id.span,
                                Text::new(
                                    format!("parameter `{}` is given more than once", id.name),
                                    format!("المعامل `{}` أُعطي أكثر من مرة", id.name),
                                ),
                            );
                            None
                        }
                        Some(j) => Some(j),
                        None => {
                            let names: Vec<String> =
                                sig.params.iter().map(|(n, _)| n.clone()).collect();
                            let mut diag = Diagnostic::error(
                                "E2010",
                                id.span,
                                Text::new(
                                    format!("`{name}` has no parameter named `{}`", id.name),
                                    format!("الدالة `{name}` ليس لها معامل باسم `{}`", id.name),
                                ),
                            );
                            if let Some(s) = suggest(&id.name, &names) {
                                diag = diag.with_hint(Text::new(
                                    format!("did you mean `{s}`?"),
                                    format!("هل تقصد `{s}`؟"),
                                ));
                            }
                            self.push(diag);
                            None
                        }
                    }
                }
            };
            match slot {
                Some(j) => {
                    filled[j] = true;
                    let ty = sig.params[j].1.clone();
                    self.check_expr_against(&mut arg.value, &ty);
                }
                None => {
                    self.check_expr(&mut arg.value, None);
                }
            }
        }
        if args.len() != sig.params.len() || filled.contains(&false) {
            self.wrong_arg_count(name, &sig.params.len().to_string(), args.len(), span);
            if let Some(d) = self.diags.last_mut() {
                let list: Vec<String> = sig
                    .params
                    .iter()
                    .map(|(n, t)| format!("{n}: {}", self.subst.show(t)))
                    .collect();
                d.hint = Some(Text::new(
                    format!("signature: {name}({})", list.join(", ")),
                    format!("التوقيع: {name}({})", list.join(", ")),
                ));
            }
        }
        if sig.is_async {
            Ty::future(sig.ret.clone())
        } else {
            sig.ret.clone()
        }
    }

    fn check_ctor(&mut self, name: &str, args: &mut [Arg], span: Span, hint: Option<&Ty>) -> Ty {
        self.reject_named_args(args);
        if name == "None" {
            self.error(
                "E2002",
                span,
                Text::new(
                    "`None` takes no arguments; write `None`",
                    "`None` لا يأخذ معاملات؛ اكتب `None`",
                ),
            );
            self.check_args_loosely(args);
            return Ty::option(self.subst.fresh());
        }
        if args.len() != 1 {
            self.wrong_arg_count(name, "1", args.len(), span);
            self.check_args_loosely(args);
            return Ty::Error;
        }
        let hint = hint.map(|h| self.subst.shallow(h));
        let value = &mut args[0].value;
        match name {
            "Some" => {
                let inner = match hint {
                    Some(Ty::Option(t)) => *t,
                    _ => self.subst.fresh(),
                };
                self.check_expr_against(value, &inner);
                Ty::option(inner)
            }
            _ => {
                let (ok, err) = match hint {
                    Some(Ty::Result(t, e)) => (*t, *e),
                    _ => (self.subst.fresh(), self.subst.fresh()),
                };
                let target = if name == "Ok" { &ok } else { &err };
                let target = target.clone();
                self.check_expr_against(value, &target);
                Ty::result(ok, err)
            }
        }
    }

    // ----- operators -------------------------------------------------------

    fn check_binary(
        &mut self,
        op: BinOp,
        lhs: &mut Expr,
        rhs: &mut Expr,
        hint: Option<&Ty>,
        span: Span,
    ) -> Ty {
        match op {
            BinOp::And | BinOp::Or => {
                for side in [lhs, rhs] {
                    let t = self.check_expr(side, Some(&Ty::Bool));
                    if !self.subst.unify(&Ty::Bool, &t) {
                        self.bad_operand(op.symbol(), &t, None, side.span);
                    }
                }
                Ty::Bool
            }
            BinOp::Eq | BinOp::Ne => {
                let lt = self.check_expr(lhs, None);
                let rt = self.check_expr(rhs, Some(&lt));
                let (l, r) = (self.subst.shallow(&lt), self.subst.shallow(&rt));
                let both_numeric = l.is_numeric() && r.is_numeric();
                if !both_numeric && !self.subst.unify(&lt, &rt) {
                    self.bad_operand(op.symbol(), &lt, Some(&rt), span);
                }
                Ty::Bool
            }
            BinOp::Lt | BinOp::Le | BinOp::Gt | BinOp::Ge => {
                let lt = self.check_expr(lhs, None);
                let rt = self.check_expr(rhs, Some(&lt));
                self.infer_from_other(&lt, &rt);
                let (l, r) = (self.subst.shallow(&lt), self.subst.shallow(&rt));
                let ok = (l.is_numeric() && r.is_numeric())
                    || (l == Ty::Str && r == Ty::Str)
                    || (l == Ty::Duration && r == Ty::Duration)
                    || matches!(l, Ty::Error)
                    || matches!(r, Ty::Error);
                if !ok {
                    self.bad_operand(op.symbol(), &lt, Some(&rt), span);
                }
                Ty::Bool
            }
            _ => {
                let float_hint = matches!(hint, Some(Ty::Float))
                    && matches!(op, BinOp::Add | BinOp::Sub | BinOp::Mul | BinOp::Rem);
                let h = float_hint.then_some(Ty::Float);
                let lt = self.check_expr(lhs, h.as_ref());
                let rt = self.check_expr(rhs, h.as_ref());
                self.arith_type(op, &lt, &rt, span)
            }
        }
    }

    /// If one side is still unknown, assume it has the other side's type.
    fn infer_from_other(&mut self, a: &Ty, b: &Ty) {
        let (l, r) = (self.subst.shallow(a), self.subst.shallow(b));
        if matches!(l, Ty::Var(_)) != matches!(r, Ty::Var(_)) {
            self.subst.unify(a, b);
        }
    }

    /// Result type of an arithmetic operator (spec §10.3).
    pub(crate) fn arith_type(&mut self, op: BinOp, lt: &Ty, rt: &Ty, span: Span) -> Ty {
        self.infer_from_other(lt, rt);
        let (l, r) = (self.subst.shallow(lt), self.subst.shallow(rt));
        match (&l, &r) {
            (Ty::Error, _) | (_, Ty::Error) => Ty::Error,
            (Ty::Int, Ty::Int) => {
                if op == BinOp::Div {
                    Ty::Float
                } else {
                    Ty::Int
                }
            }
            (Ty::Int | Ty::Float, Ty::Int | Ty::Float) => {
                if op == BinOp::IntDiv {
                    self.bad_operand("div", lt, Some(rt), span);
                    if let Some(d) = self.diags.last_mut() {
                        d.hint = Some(Text::new(
                            "`div` is integer division; use `/` for floats, or `to_int(x)` first",
                            "`div` قسمة صحيحة؛ استخدم `/` للأعداد العشرية، أو `to_int(x)` أولًا",
                        ));
                    }
                    Ty::Error
                } else {
                    Ty::Float
                }
            }
            (Ty::Str, Ty::Str) if op == BinOp::Add => Ty::Str,
            (Ty::Duration, Ty::Duration) if matches!(op, BinOp::Add | BinOp::Sub) => Ty::Duration,
            (Ty::List(a), Ty::List(b)) if op == BinOp::Add => {
                if !self.subst.unify(a, b) {
                    self.mismatch(lt, rt, span);
                }
                l.clone()
            }
            (Ty::Var(_), Ty::Var(_)) => {
                self.subst.unify(lt, rt);
                if op == BinOp::Div {
                    Ty::Float
                } else {
                    l.clone()
                }
            }
            _ => {
                self.bad_operand(op.symbol(), lt, Some(rt), span);
                if op == BinOp::Add
                    && (l == Ty::Str || r == Ty::Str)
                    && let Some(d) = self.diags.last_mut()
                {
                    d.hint = Some(Text::new(
                        "to put a value in text, write it inside the text: `\"Total: {n}\"`, or convert it: `\"Total: \" + to_string(n)`",
                        "لوضع قيمة داخل نص اكتبها بين قوسين داخله: `\"المجموع: {n}\"`، أو حوّلها: `\"المجموع: \" + to_string(n)`",
                    ));
                }
                Ty::Error
            }
        }
    }

    fn bad_operand(&mut self, op: &str, lt: &Ty, rt: Option<&Ty>, span: Span) {
        let l = self.subst.show(lt);
        let (en, ar) = match rt {
            Some(rt) => {
                let r = self.subst.show(rt);
                (
                    format!("operator `{op}` cannot be applied to `{l}` and `{r}`"),
                    format!("لا يمكن تطبيق العامل `{op}` على `{l}` و`{r}`"),
                )
            }
            None => (
                format!("operator `{op}` cannot be applied to `{l}`"),
                format!("لا يمكن تطبيق العامل `{op}` على `{l}`"),
            ),
        };
        self.error("E2005", span, Text::new(en, ar));
    }

    // ----- fields ----------------------------------------------------------

    fn field_type(&mut self, t: &Ty, name: &Ident) -> Ty {
        let resolved = self.subst.shallow(t);
        match &resolved {
            Ty::Error => Ty::Error,
            Ty::Record(fields) => {
                if let Some((_, ft)) = fields.iter().find(|(n, _)| n == &name.name) {
                    return ft.clone();
                }
                let names: Vec<String> = fields.iter().map(|(n, _)| n.clone()).collect();
                self.no_field(&resolved, name, suggest(&name.name, &names));
                Ty::Error
            }
            Ty::List(_) | Ty::Str | Ty::Map(..) | Ty::Set(_) if name.name == "length" => Ty::Int,
            Ty::Tuple(items) => match name.name.parse::<usize>() {
                Ok(i) if i < items.len() => items[i].clone(),
                _ => {
                    let last = items.len() - 1;
                    self.error(
                        "E2027",
                        name.span,
                        Text::new(
                            format!(
                                "this tuple has {} values; use `.0` to `.{last}`",
                                items.len()
                            ),
                            format!(
                                "هذا الصف فيه {} قيم؛ استخدم من `.0` إلى `.{last}`",
                                items.len()
                            ),
                        ),
                    );
                    Ty::Error
                }
            },
            Ty::Var(_) => {
                self.cannot_infer(name.span);
                Ty::Error
            }
            _ => {
                if crate::builtins::has_method(&resolved, &name.name) {
                    self.no_field(&resolved, name, None);
                    if let Some(d) = self.diags.last_mut() {
                        d.hint = Some(Text::new(
                            format!("`{0}` is a method; call it: `.{0}()`", name.name),
                            format!("`{0}` دالة تابعة؛ استدعها هكذا: `.{0}()`", name.name),
                        ));
                    }
                } else {
                    self.no_field(&resolved, name, None);
                }
                Ty::Error
            }
        }
    }

    fn no_field(&mut self, t: &Ty, name: &Ident, suggestion: Option<String>) {
        let shown = self.subst.show(t);
        let mut diag = Diagnostic::error(
            "E1007",
            name.span,
            Text::new(
                format!("type `{shown}` has no field `{}`", name.name),
                format!("النوع `{shown}` ليس فيه حقل باسم `{}`", name.name),
            ),
        );
        if let Some(s) = suggestion {
            diag = diag.with_hint(Text::new(
                format!("did you mean `{s}`?"),
                format!("هل تقصد `{s}`؟"),
            ));
        }
        self.push(diag);
    }

    // ----- match -----------------------------------------------------------

    fn check_match(
        &mut self,
        scrutinee: &mut Expr,
        arms: &mut [MatchArm],
        span: Span,
        hint: Option<&Ty>,
        value_needed: bool,
    ) -> Ty {
        let st = self.check_expr(scrutinee, None);
        let result = value_needed.then(|| hint.cloned().unwrap_or_else(|| self.subst.fresh()));
        for arm in arms.iter_mut() {
            self.push_scope();
            self.check_pattern(&arm.pattern, &st);
            match &mut arm.body {
                ArmBody::Expr(e) => match &result {
                    Some(r) => {
                        let r = r.clone();
                        self.check_expr_against(e, &r);
                    }
                    None => {
                        if let ExprKind::Match { scrutinee, arms } = &mut e.kind {
                            self.check_match(scrutinee, arms, e.span, None, false);
                        } else {
                            self.check_expr(e, None);
                        }
                    }
                },
                ArmBody::Block(b) => {
                    self.check_block(b);
                    if let Some(r) = &result
                        && !block_diverges(b)
                    {
                        let r = r.clone();
                        self.expect_ty(&r, &Ty::Unit, b.span);
                    }
                }
            }
            self.pop_scope();
        }
        self.check_exhaustive(&st, arms, span);
        result.unwrap_or(Ty::Unit)
    }

    pub(crate) fn check_pattern(&mut self, p: &Pattern, ty: &Ty) {
        let literal = |t: Ty| Some(t);
        let found = match &p.kind {
            PatternKind::Wildcard => None,
            PatternKind::Name(n) if n == "None" => Some(Ty::option(self.subst.fresh())),
            PatternKind::Name(n) if matches!(n.as_str(), "Some" | "Ok" | "Err") => {
                self.error(
                    "E2002",
                    p.span,
                    Text::new(
                        format!("the pattern `{n}` needs a value, e.g. `{n}(x)`"),
                        format!("النمط `{n}` يحتاج إلى قيمة، مثل `{n}(x)`"),
                    ),
                );
                None
            }
            PatternKind::Name(n) => {
                let ident = Ident {
                    name: n.clone(),
                    span: p.span,
                };
                self.declare(&ident, ty.clone(), false, false);
                None
            }
            PatternKind::Ctor { name, args } => {
                let (outer, inner) = match name.name.as_str() {
                    "Some" => {
                        let inner = self.subst.fresh();
                        (Ty::option(inner.clone()), inner)
                    }
                    "Ok" => {
                        let (t, e) = (self.subst.fresh(), self.subst.fresh());
                        (Ty::result(t.clone(), e), t)
                    }
                    "Err" => {
                        let (t, e) = (self.subst.fresh(), self.subst.fresh());
                        (Ty::result(t, e.clone()), e)
                    }
                    "None" => {
                        self.error(
                            "E2002",
                            p.span,
                            Text::new(
                                "the pattern `None` takes no values; write `None`",
                                "النمط `None` لا يأخذ قيمًا؛ اكتب `None`",
                            ),
                        );
                        return;
                    }
                    other => {
                        self.unknown_name(other, name.span);
                        return;
                    }
                };
                if !self.subst.unify(ty, &outer) {
                    self.mismatch(ty, &outer, p.span);
                    return;
                }
                if args.len() != 1 {
                    self.wrong_arg_count(&name.name, "1", args.len(), p.span);
                    return;
                }
                self.check_pattern(&args[0], &inner);
                None
            }
            PatternKind::Variant {
                enum_name,
                variant,
                args,
            } => {
                self.check_variant_pattern(p, enum_name, variant, args, ty);
                None
            }
            PatternKind::Tuple(items) => {
                let fresh: Vec<Ty> = items.iter().map(|_| self.subst.fresh()).collect();
                let tuple = Ty::Tuple(fresh.clone());
                if !self.subst.unify(ty, &tuple) {
                    self.mismatch(ty, &tuple, p.span);
                } else {
                    for (item, t) in items.iter().zip(&fresh) {
                        self.check_pattern(item, t);
                    }
                }
                None
            }
            PatternKind::Int(_) => literal(Ty::Int),
            PatternKind::Range {
                start,
                end,
                inclusive,
            } => {
                if start > end || (start == end && !inclusive) {
                    self.error(
                        "E2025",
                        p.span,
                        Text::new(
                            "this range matches no number; the start must be below the end",
                            "هذا النطاق لا يطابق أي عدد؛ يجب أن تكون البداية أصغر من النهاية",
                        ),
                    );
                }
                literal(Ty::Int)
            }
            PatternKind::Or(alts) => {
                for alt in alts {
                    if binds_names(alt) {
                        self.error(
                            "E2024",
                            alt.span,
                            Text::new(
                                "a pattern with `|` cannot bind names; use `_`, or one arm per case",
                                "النمط الذي فيه `|` لا يربط أسماء؛ استخدم `_`، أو فرعًا لكل حالة",
                            ),
                        );
                    }
                    self.check_pattern(alt, ty);
                }
                None
            }
            PatternKind::Float(_) => literal(Ty::Float),
            PatternKind::Str(_) => literal(Ty::Str),
            PatternKind::Bool(_) => literal(Ty::Bool),
        };
        if let Some(found) = found
            && !self.subst.unify(ty, &found)
        {
            self.mismatch(ty, &found, p.span);
        }
    }

    fn check_exhaustive(&mut self, st: &Ty, arms: &[MatchArm], span: Span) {
        let pats: Vec<&Pattern> = arms.iter().map(|a| &a.pattern).collect();
        let missing = self.missing_cases(&pats, st);
        if missing.is_empty() {
            return;
        }
        let list = missing
            .iter()
            .map(|m| format!("`{m}`"))
            .collect::<Vec<_>>()
            .join(", ");
        self.push(
            Diagnostic::error(
                "E2009",
                span,
                Text::new(
                    format!("`match` does not cover every case; missing {list}"),
                    format!("`match` لا يغطي كل الحالات؛ الناقص: {list}"),
                ),
            )
            .with_hint(Text::new(
                "add an arm for each missing case, or a catch-all arm `_ => ...`",
                "أضف فرعًا لكل حالة ناقصة، أو فرعًا عامًا `_ => ...`",
            )),
        );
    }
}

/// Whether a pattern introduces a variable.
fn binds_names(p: &Pattern) -> bool {
    match &p.kind {
        PatternKind::Name(n) => !CONSTRUCTORS.contains(&n.as_str()),
        PatternKind::Ctor { args, .. } | PatternKind::Variant { args, .. } => {
            args.iter().any(binds_names)
        }
        PatternKind::Or(alts) => alts.iter().any(binds_names),
        _ => false,
    }
}

pub(crate) fn irrefutable(p: &Pattern) -> bool {
    match &p.kind {
        PatternKind::Wildcard => true,
        PatternKind::Or(alts) => alts.iter().any(irrefutable),
        PatternKind::Name(n) => !CONSTRUCTORS.contains(&n.as_str()),
        _ => false,
    }
}

impl Checker {
    /// `expr?` (spec §10.6): unwraps `Ok`/`Some`, or returns the `Err`/`None`
    /// from the current function, whose return type must allow it.
    pub(crate) fn check_try(&mut self, inner: &mut Expr, span: Span) -> Ty {
        let t = self.check_expr(inner, None);
        let ret = self.ret_ty.clone().map(|r| self.subst.shallow(&r));
        match self.subst.shallow(&t) {
            Ty::Error => Ty::Error,
            Ty::Var(_) => {
                self.cannot_infer(inner.span);
                Ty::Error
            }
            Ty::Result(ok, err) => {
                match ret {
                    Some(Ty::Result(_, ret_err)) => {
                        if !self.subst.unify(&ret_err, &err) {
                            let (have, want) = (self.subst.show(&err), self.subst.show(&ret_err));
                            self.push(
                                Diagnostic::error(
                                    "E2001",
                                    span,
                                    Text::new(
                                        format!("`?` passes on an error of type `{have}`, but this function returns errors of type `{want}`"),
                                        format!("`?` يمرّر خطأً من النوع `{have}`، لكن هذه الدالة تُرجع أخطاء من النوع `{want}`"),
                                    ),
                                )
                                .with_hint(Text::new(
                                    "convert the error with `match` first, or make both error types the same",
                                    "حوّل الخطأ بـ `match` أولًا، أو اجعل نوعي الخطأ متطابقين",
                                )),
                            );
                        }
                    }
                    Some(Ty::Error) => {}
                    _ => self.try_outside(span, "Result"),
                }
                *ok
            }
            Ty::Option(inner_ty) => {
                if !matches!(ret, Some(Ty::Option(_) | Ty::Error)) {
                    self.try_outside(span, "Option");
                }
                *inner_ty
            }
            other => {
                let shown = self.subst.show(&other);
                self.error(
                    "E2017",
                    span,
                    Text::new(
                        format!("`?` works on `Result` or `Option`, not on `{shown}`"),
                        format!("`?` يعمل على `Result` أو `Option` فقط، لا على `{shown}`"),
                    ),
                );
                Ty::Error
            }
        }
    }

    fn try_outside(&mut self, span: Span, what: &str) {
        self.push(
            Diagnostic::error(
                "E2016",
                span,
                Text::new(
                    format!("`?` on a `{what}` can only be used in a function that returns `{what}`"),
                    format!("لا يُستخدم `?` على `{what}` إلا داخل دالة تُرجع `{what}`"),
                ),
            )
            .with_hint(Text::new(
                "handle the case with `match` here, or move this code into a function that returns it",
                "عالج الحالة بـ `match` هنا، أو انقل هذا الكود إلى دالة تُرجع هذا النوع",
            )),
        );
    }

    /// Set elements must be `String` or `Int`, like map keys.
    pub(crate) fn check_set_element(&mut self, t: &Ty, span: Span) {
        match self.subst.shallow(t) {
            Ty::Str | Ty::Int | Ty::Var(_) | Ty::Error => {}
            other => {
                let shown = self.subst.show(&other);
                self.error(
                    "E2018",
                    span,
                    Text::new(
                        format!("`Set` elements must be `String` or `Int`, not `{shown}`"),
                        format!("عناصر `Set` يجب أن تكون `String` أو `Int`، لا `{shown}`"),
                    ),
                );
            }
        }
    }

    /// Map keys must be `String` or `Int` (spec §10.2).
    pub(crate) fn check_map_key(&mut self, k: &Ty, span: Span) {
        match self.subst.shallow(k) {
            Ty::Str | Ty::Int | Ty::Var(_) | Ty::Error => {}
            other => {
                let shown = self.subst.show(&other);
                self.error(
                    "E2018",
                    span,
                    Text::new(
                        format!("`Map` keys must be `String` or `Int`, not `{shown}`"),
                        format!("مفاتيح `Map` يجب أن تكون `String` أو `Int`، لا `{shown}`"),
                    ),
                );
            }
        }
    }
}

/// The variable at the root of an assignable place (`a`, `a.b`, `a[i].c`).
pub(crate) fn place_root(e: &Expr) -> Option<(&str, Span)> {
    match &e.kind {
        ExprKind::Name(n) => Some((n.as_str(), e.span)),
        ExprKind::Field { target, .. } | ExprKind::Index { target, .. } => place_root(target),
        _ => None,
    }
}

/// Whether control never reaches the end of `block` (it always returns).
pub(crate) fn block_diverges(block: &Block) -> bool {
    block.stmts.iter().any(stmt_diverges)
}

fn stmt_diverges(stmt: &Stmt) -> bool {
    match &stmt.kind {
        StmtKind::Return(_) => true,
        StmtKind::If {
            then,
            otherwise: Some(otherwise),
            ..
        } => {
            block_diverges(then)
                && match otherwise {
                    Else::Block(b) => block_diverges(b),
                    Else::If(s) => stmt_diverges(s),
                }
        }
        StmtKind::Block(b) => block_diverges(b),
        StmtKind::While { cond, body } => {
            matches!(cond.kind, ExprKind::Bool(true)) && !contains_break(body)
        }
        StmtKind::Expr(Expr {
            kind: ExprKind::Match { arms, .. },
            ..
        }) => {
            !arms.is_empty()
                && arms
                    .iter()
                    .all(|a| matches!(&a.body, ArmBody::Block(b) if block_diverges(b)))
        }
        _ => false,
    }
}

/// Whether a `break` in `block` exits this loop (not a nested one).
fn contains_break(block: &Block) -> bool {
    block.stmts.iter().any(|s| match &s.kind {
        StmtKind::Break => true,
        StmtKind::If {
            then, otherwise, ..
        } => {
            contains_break(then)
                || match otherwise {
                    Some(Else::Block(b)) => contains_break(b),
                    Some(Else::If(s)) => contains_break(&Block {
                        stmts: vec![(**s).clone()],
                        span: s.span,
                    }),
                    None => false,
                }
        }
        StmtKind::Block(b) => contains_break(b),
        StmtKind::Expr(Expr {
            kind: ExprKind::Match { arms, .. },
            ..
        }) => arms
            .iter()
            .any(|a| matches!(&a.body, ArmBody::Block(b) if contains_break(b))),
        _ => false,
    })
}
