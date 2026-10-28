//! Enums (spec §10.2, v0.2): declarations, `Enum.Variant(...)` values,
//! variant patterns and exhaustiveness, and the built-in `AIError`.
//!
//! Enums are nominal (unlike records): two enums with the same variants are
//! different types. `Enum.Variant` parses as a field access or method call;
//! the checker rewrites it to [`ExprKind::Variant`].

use elomar_ast::*;
use elomar_diagnostics::{Diagnostic, Text};

use crate::{Checker, Ty, suggest};

#[derive(Debug, Clone)]
pub(crate) struct EnumInfo {
    /// Variant name and payload types, in declaration order.
    pub variants: Vec<(String, Vec<Ty>)>,
}

impl EnumInfo {
    /// Whether every variant is a plain name (no payload).
    pub fn is_unit_only(&self) -> bool {
        self.variants.iter().all(|(_, fields)| fields.is_empty())
    }
}

pub(crate) const AI_ERROR: &str = "AIError";

/// The variants of `AIError` (spec §13.4); each carries a message.
pub const AI_ERROR_KINDS: [&str; 11] = [
    "Authentication",
    "ProviderUnavailable",
    "RateLimited",
    "Timeout",
    "Network",
    "UnsupportedCapability",
    "InvalidStructuredOutput",
    "PolicyDenied",
    "BudgetExceeded",
    "ContextLimitExceeded",
    "Cancelled",
];

impl Checker {
    pub(crate) fn register_builtin_enums(&mut self) {
        self.enums.insert(
            AI_ERROR.into(),
            EnumInfo {
                variants: AI_ERROR_KINDS
                    .iter()
                    .map(|k| (k.to_string(), vec![Ty::Str]))
                    .collect(),
            },
        );
    }

    /// Registers every enum name first, so payloads and type aliases can
    /// refer to any enum, including recursively.
    pub(crate) fn declare_enums(&mut self, program: &Program) {
        let mut seen: Vec<(String, Span)> = Vec::new();
        for item in &program.items {
            let Item::Enum(e) = item else { continue };
            let name = &e.name.name;
            if crate::builtins::is_builtin_type(name) {
                self.error(
                    "E1002",
                    e.name.span,
                    Text::new(
                        format!("`{name}` is a built-in type and cannot be redefined"),
                        format!("`{name}` نوع أساسي ولا يمكن إعادة تعريفه"),
                    ),
                );
                continue;
            }
            if let Some((_, prev)) = seen.iter().find(|(n, _)| n == name) {
                let prev = *prev;
                self.push(
                    Diagnostic::error(
                        "E1002",
                        e.name.span,
                        Text::new(
                            format!("type `{name}` is defined more than once"),
                            format!("النوع `{name}` معرّف أكثر من مرة"),
                        ),
                    )
                    .with_note(prev, Text::new("first defined here", "عُرّف أول مرة هنا")),
                );
                continue;
            }
            seen.push((name.clone(), e.name.span));
            self.enums.insert(
                name.clone(),
                EnumInfo {
                    variants: Vec::new(),
                },
            );
        }
    }

    /// Resolves variant payload types once all type names are known.
    pub(crate) fn resolve_enums(&mut self, program: &Program) {
        for item in &program.items {
            let Item::Enum(e) = item else { continue };
            if !self.enums.contains_key(&e.name.name)
                || crate::builtins::is_builtin_type(&e.name.name)
            {
                continue;
            }
            let mut variants: Vec<(String, Vec<Ty>)> = Vec::new();
            for v in &e.variants {
                if variants.iter().any(|(n, _)| n == &v.name.name) {
                    self.error(
                        "E1002",
                        v.name.span,
                        Text::new(
                            format!("variant `{}` is listed twice", v.name.name),
                            format!("الحالة `{}` مذكورة مرتين", v.name.name),
                        ),
                    );
                    continue;
                }
                let fields = v.fields.iter().map(|t| self.lower_type(t)).collect();
                variants.push((v.name.name.clone(), fields));
            }
            if variants.is_empty() {
                self.error(
                    "E1002",
                    e.name.span,
                    Text::new(
                        format!("enum `{}` has no variants", e.name.name),
                        format!("النوع المعدود `{}` بلا حالات", e.name.name),
                    ),
                );
            }
            self.enums
                .insert(e.name.name.clone(), EnumInfo { variants });
        }
    }

    /// If `kind` is `E.V` or `E.V(...)` with `E` an enum (and not shadowed
    /// by a variable), returns `E`.
    pub(crate) fn enum_target(&self, kind: &ExprKind) -> Option<String> {
        let target = match kind {
            ExprKind::Field { target, .. } => target,
            ExprKind::Call { callee, .. } => match &callee.kind {
                ExprKind::Field { target, .. } => target,
                _ => return None,
            },
            _ => return None,
        };
        match &target.kind {
            ExprKind::Name(n)
                if self.enums.contains_key(n)
                    && self.lookup_local(n).is_none()
                    && !self.consts.contains_key(n) =>
            {
                Some(n.clone())
            }
            _ => None,
        }
    }

    fn unknown_variant(&mut self, enum_name: &str, variant: &Ident) {
        let names: Vec<String> = self.enums[enum_name]
            .variants
            .iter()
            .map(|(n, _)| n.clone())
            .collect();
        let mut diag = Diagnostic::error(
            "E1010",
            variant.span,
            Text::new(
                format!("enum `{enum_name}` has no variant `{}`", variant.name),
                format!(
                    "النوع المعدود `{enum_name}` ليس فيه الحالة `{}`",
                    variant.name
                ),
            ),
        );
        diag = match suggest(&variant.name, &names) {
            Some(s) => diag.with_hint(Text::new(
                format!("did you mean `{enum_name}.{s}`?"),
                format!("هل تقصد `{enum_name}.{s}`؟"),
            )),
            None => diag.with_hint(Text::new(
                format!("variants: {}", names.join(", ")),
                format!("الحالات: {}", names.join("، ")),
            )),
        };
        self.push(diag);
    }

    /// Checks `E.V` / `E.V(args)` and rewrites it to `ExprKind::Variant`.
    pub(crate) fn check_variant_expr(&mut self, e: &mut Expr, enum_name: String) -> Ty {
        let span = e.span;
        let (variant, mut args, called) =
            match std::mem::replace(&mut e.kind, ExprKind::Bool(false)) {
                ExprKind::Field { name, .. } => (name, Vec::new(), false),
                ExprKind::Call { callee, args, .. } => match callee.kind {
                    ExprKind::Field { name, .. } => (name, args, true),
                    _ => unreachable!("checked by enum_target"),
                },
                _ => unreachable!("checked by enum_target"),
            };
        let fields = self.enums[&enum_name]
            .variants
            .iter()
            .find(|(n, _)| n == &variant.name)
            .map(|(_, f)| f.clone());
        let Some(fields) = fields else {
            self.unknown_variant(&enum_name, &variant);
            self.check_args_loosely(&mut args);
            e.kind = ExprKind::Variant {
                enum_name,
                variant: variant.name,
                args: args.into_iter().map(|a| a.value).collect(),
            };
            return Ty::Error;
        };
        self.reject_named_args(&args);
        let full = format!("{enum_name}.{}", variant.name);
        if !called && !fields.is_empty() {
            self.error(
                "E2002",
                span,
                Text::new(
                    format!("`{full}` needs {} value(s): `{full}(...)`", fields.len()),
                    format!("`{full}` تحتاج {} من القيم: `{full}(...)`", fields.len()),
                ),
            );
        } else if called && fields.is_empty() {
            self.error(
                "E2002",
                span,
                Text::new(
                    format!("`{full}` takes no values; write `{full}`"),
                    format!("`{full}` لا تأخذ قيمًا؛ اكتب `{full}`"),
                ),
            );
            self.check_args_loosely(&mut args);
        } else if args.len() != fields.len() {
            self.wrong_arg_count(&full, &fields.len().to_string(), args.len(), span);
            self.check_args_loosely(&mut args);
        } else {
            for (arg, ty) in args.iter_mut().zip(&fields) {
                self.check_expr_against(&mut arg.value, ty);
            }
        }
        e.kind = ExprKind::Variant {
            enum_name: enum_name.clone(),
            variant: variant.name,
            args: args.into_iter().map(|a| a.value).collect(),
        };
        Ty::Enum(enum_name)
    }

    pub(crate) fn check_variant_pattern(
        &mut self,
        p: &Pattern,
        enum_name: &Ident,
        variant: &Ident,
        args: &[Pattern],
        ty: &Ty,
    ) {
        if !self.enums.contains_key(&enum_name.name) {
            self.push(Diagnostic::error(
                "E1006",
                enum_name.span,
                Text::new(
                    format!("unknown enum `{}`", enum_name.name),
                    format!("نوع معدود غير معروف `{}`", enum_name.name),
                ),
            ));
            return;
        }
        let fields = self.enums[&enum_name.name]
            .variants
            .iter()
            .find(|(n, _)| n == &variant.name)
            .map(|(_, f)| f.clone());
        let Some(fields) = fields else {
            self.unknown_variant(&enum_name.name, variant);
            return;
        };
        let enum_ty = Ty::Enum(enum_name.name.clone());
        if !self.subst.unify(ty, &enum_ty) {
            self.mismatch(ty, &enum_ty, p.span);
            return;
        }
        if args.len() != fields.len() {
            let full = format!("{}.{}", enum_name.name, variant.name);
            self.wrong_arg_count(&full, &fields.len().to_string(), args.len(), p.span);
            return;
        }
        for (arg, ty) in args.iter().zip(&fields) {
            self.check_pattern(arg, ty);
        }
    }

    /// Variants of `enum_name` that no arm covers, as `E.V` or `E.V(_)`.
    pub(crate) fn missing_variants(&self, enum_name: &str, arms: &[MatchArm]) -> Vec<String> {
        let Some(info) = self.enums.get(enum_name) else {
            return Vec::new();
        };
        info.variants
            .iter()
            .filter(|(vname, _)| {
                !arms.iter().any(|a| match &a.pattern.kind {
                    PatternKind::Variant {
                        enum_name: e,
                        variant,
                        args,
                    } => {
                        e.name == enum_name
                            && &variant.name == vname
                            && args.iter().all(crate::expr::irrefutable)
                    }
                    _ => false,
                })
            })
            .map(|(vname, fields)| {
                if fields.is_empty() {
                    format!("{enum_name}.{vname}")
                } else {
                    format!("{enum_name}.{vname}(_)")
                }
            })
            .collect()
    }
}
