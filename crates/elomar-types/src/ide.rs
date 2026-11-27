//! Information for editors (`elomar lsp`): what each name means, where it
//! is defined, and which members a value has.
//!
//! Recording is off unless [`crate::analyze`] is used. Types are rendered
//! only after the whole program is checked, when inference is complete.

use std::collections::{BTreeSet, HashMap};

use elomar_ast::Span;

use crate::{Checker, Ty, modules};

/// The result of [`crate::analyze`] besides diagnostics.
#[derive(Debug, Clone, Default)]
pub struct IdeInfo {
    pub hovers: Vec<Hover>,
    /// (use, definition): where each name used in the program is defined.
    pub definitions: Vec<(Span, Span)>,
}

#[derive(Debug, Clone)]
pub struct Hover {
    pub span: Span,
    /// Elomar code, e.g. `let total: Int` or `fn average(xs: List<Int>) -> Float`.
    pub text: String,
    /// For a value: its fields and methods, for completion after `.`.
    pub members: Vec<String>,
}

pub(crate) enum Pending {
    /// `text` followed by the type, e.g. `let x: ` + `Int`.
    Typed(String, Ty),
    /// A function's signature, by full name.
    Function(String),
    Text(String),
}

#[derive(Default)]
pub(crate) struct Recorder {
    hovers: Vec<(Span, Pending)>,
    definitions: Vec<(Span, Span)>,
}

impl Checker {
    pub(crate) fn note_typed(&mut self, span: Span, text: impl Into<String>, ty: &Ty) {
        if let Some(r) = &mut self.ide {
            r.hovers
                .push((span, Pending::Typed(text.into(), ty.clone())));
        }
    }

    pub(crate) fn note_fn(&mut self, span: Span, fn_name: &str) {
        if let Some(r) = &mut self.ide {
            r.hovers
                .push((span, Pending::Function(fn_name.to_string())));
        }
    }

    pub(crate) fn note_text(&mut self, span: Span, text: String) {
        if let Some(r) = &mut self.ide {
            r.hovers.push((span, Pending::Text(text)));
        }
    }

    pub(crate) fn note_def(&mut self, use_span: Span, def_span: Span) {
        if let Some(r) = &mut self.ide
            && use_span != def_span
        {
            r.definitions.push((use_span, def_span));
        }
    }

    /// A function's signature as Elomar code, with its effects (declared,
    /// or inferred for private functions).
    fn signature(&self, fn_name: &str) -> String {
        let Some(sig) = self.fns.get(fn_name) else {
            return format!("fn {fn_name}");
        };
        let show = |t: &Ty| self.subst.show(&self.subst.resolve(t));
        let name = modules::base_name(fn_name);
        let mut out = String::new();
        if sig.is_pub {
            out.push_str("pub ");
        }
        if sig.is_async {
            out.push_str("async ");
        }
        out.push_str("fn ");
        out.push_str(name);
        if !sig.type_params.is_empty() {
            let params: Vec<String> = sig
                .type_params
                .iter()
                .map(|p| {
                    let bounds: Vec<&str> = sig
                        .bounds
                        .iter()
                        .filter(|(bp, _)| bp == p)
                        .map(|(_, t)| modules::base_name(t))
                        .collect();
                    if bounds.is_empty() {
                        p.clone()
                    } else {
                        format!("{p}: {}", bounds.join(" + "))
                    }
                })
                .collect();
            out.push_str(&format!("<{}>", params.join(", ")));
        }
        let params: Vec<String> = sig
            .params
            .iter()
            .map(|(n, t)| {
                if n == "self" {
                    "self".to_string()
                } else {
                    format!("{n}: {}", show(t))
                }
            })
            .collect();
        out.push_str(&format!("({})", params.join(", ")));
        if sig.ret != Ty::Unit {
            out.push_str(&format!(" -> {}", show(&sig.ret)));
        }
        let effects: Option<&BTreeSet<String>> = sig
            .declared
            .as_ref()
            .map(|(e, _)| e)
            .or_else(|| self.inferred_effects.get(fn_name));
        if let Some(effects) = effects.filter(|e| !e.is_empty()) {
            let list: Vec<&str> = effects.iter().map(String::as_str).collect();
            out.push_str(&format!(" !{{{}}}", list.join(", ")));
        }
        out
    }

    /// Fields and methods of a value of type `ty`.
    fn members(&self, ty: &Ty) -> Vec<String> {
        let mut out: Vec<String> = Vec::new();
        match ty {
            Ty::Record(fields) => out.extend(fields.iter().map(|(n, _)| n.clone())),
            Ty::List(_) | Ty::Str | Ty::Map(..) | Ty::Set(_) => {
                out.extend(["length".into(), "طول".into()])
            }
            _ => {}
        }
        if !matches!(ty, Ty::Var(_) | Ty::Error | Ty::Param(_)) {
            let methods = crate::builtins::methods_of(ty);
            out.extend(methods.iter().map(|m| m.to_string()));
            // And their Arabic names (`.أضف` for `.push`).
            out.extend(
                methods
                    .iter()
                    .filter_map(|m| elomar_ast::arabic::method_arabic(m))
                    .map(str::to_string),
            );
        }
        for m in &self.methods {
            if self.ty_matches(&m.target, ty) && !matches!(ty, Ty::Var(_)) {
                out.push(m.name.clone());
            }
        }
        if let Ty::Param(p) = ty {
            for (bp, tr) in self.all_bounds.iter().filter(|(bp, _)| bp == p) {
                let _ = bp;
                if let Some(info) = self.traits.get(tr) {
                    out.extend(info.methods.iter().map(|s| s.name.clone()));
                }
            }
        }
        let mut seen = std::collections::HashSet::new();
        out.retain(|m| seen.insert(m.clone()));
        out
    }

    pub(crate) fn ide_info(&mut self) -> IdeInfo {
        let Some(recorder) = self.ide.take() else {
            return IdeInfo::default();
        };
        let mut hovers = Vec::with_capacity(recorder.hovers.len());
        // The last record for a span wins (e.g. a name checked twice).
        let mut by_span: HashMap<(usize, usize), usize> = HashMap::new();
        for (span, pending) in recorder.hovers {
            let (text, members) = match pending {
                Pending::Typed(text, ty) => {
                    let ty = self.subst.resolve(&ty);
                    (format!("{text}{}", self.subst.show(&ty)), self.members(&ty))
                }
                Pending::Function(name) => (self.signature(&name), Vec::new()),
                Pending::Text(text) => (text, Vec::new()),
            };
            let hover = Hover {
                span,
                text,
                members,
            };
            let key = (span.start as usize, span.end as usize);
            match by_span.get(&key) {
                Some(&i) => hovers[i] = hover,
                None => {
                    by_span.insert(key, hovers.len());
                    hovers.push(hover);
                }
            }
        }
        IdeInfo {
            hovers,
            definitions: recorder.definitions,
        }
    }
}
