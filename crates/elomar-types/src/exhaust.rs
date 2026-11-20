//! Exhaustiveness of `match`, including nested patterns: `Ok(Some(x))`,
//! `Ok(None)` and `Err(e)` together cover `Result<Option<T>, E>`.

use elomar_ast::*;

use crate::expr::irrefutable;
use crate::{Checker, Ty};

/// One constructor of a type: how it is written and its payload types.
struct Ctor {
    shown: String,
    fields: Vec<Ty>,
}

impl Checker {
    /// The cases `pats` leave uncovered at type `ty`, written as patterns
    /// (`Ok(None)`, `Shape.Circle(_)`, `_`). Empty when every value matches.
    pub(crate) fn missing_cases(&mut self, pats: &[&Pattern], ty: &Ty) -> Vec<String> {
        if pats.iter().any(|p| irrefutable(p)) {
            return Vec::new();
        }
        let ctors = match self.subst.resolve(ty) {
            Ty::Option(t) => vec![
                Ctor {
                    shown: "Some".into(),
                    fields: vec![*t],
                },
                Ctor {
                    shown: "None".into(),
                    fields: Vec::new(),
                },
            ],
            Ty::Result(t, e) => vec![
                Ctor {
                    shown: "Ok".into(),
                    fields: vec![*t],
                },
                Ctor {
                    shown: "Err".into(),
                    fields: vec![*e],
                },
            ],
            Ty::Bool => vec![
                Ctor {
                    shown: "true".into(),
                    fields: Vec::new(),
                },
                Ctor {
                    shown: "false".into(),
                    fields: Vec::new(),
                },
            ],
            Ty::Enum(name, args) => {
                let Some(info) = self.enums.get(&name) else {
                    return Vec::new();
                };
                let map: Vec<(String, Ty)> = info.params.iter().cloned().zip(args).collect();
                info.variants
                    .iter()
                    .map(|(v, fields)| Ctor {
                        shown: format!("{name}.{v}"),
                        fields: fields.iter().map(|f| f.substitute(&map)).collect(),
                    })
                    .collect()
            }
            Ty::Error | Ty::Var(_) => return Vec::new(),
            _ => return vec!["_".to_string()],
        };
        // Patterns name enums as written; compare by full name.
        let resolved: Vec<Option<String>> = pats
            .iter()
            .map(|p| match &p.kind {
                PatternKind::Variant {
                    enum_name, variant, ..
                } => self
                    .resolve_type_name(enum_name, false)
                    .map(|e| format!("{e}.{}", variant.name)),
                _ => None,
            })
            .collect();
        let mut missing = Vec::new();
        for ctor in ctors {
            let rows: Vec<&[Pattern]> = pats
                .iter()
                .zip(&resolved)
                .filter_map(|(p, full)| match &p.kind {
                    PatternKind::Ctor { name, args } if name.name == ctor.shown => {
                        Some(args.as_slice())
                    }
                    PatternKind::Name(n) if *n == ctor.shown => Some(&[][..]),
                    PatternKind::Bool(b) if b.to_string() == ctor.shown => Some(&[][..]),
                    PatternKind::Variant { args, .. } if full.as_deref() == Some(&ctor.shown) => {
                        Some(args.as_slice())
                    }
                    _ => None,
                })
                .collect();
            let blank = || {
                if ctor.fields.is_empty() {
                    ctor.shown.clone()
                } else {
                    let holes = vec!["_"; ctor.fields.len()].join(", ");
                    format!("{}({holes})", ctor.shown)
                }
            };
            if rows.is_empty() {
                missing.push(blank());
                continue;
            }
            if ctor.fields.is_empty()
                || rows
                    .iter()
                    .any(|r| r.len() == ctor.fields.len() && r.iter().all(irrefutable))
            {
                continue;
            }
            if ctor.fields.len() == 1 {
                let column: Vec<&Pattern> = rows.iter().filter_map(|r| r.first()).collect();
                for m in self.missing_cases(&column, &ctor.fields[0]) {
                    missing.push(format!("{}({m})", ctor.shown));
                }
                continue;
            }
            // Several fields: covered if, for some field, the rows that
            // leave every other field open cover that field.
            let mut covered = false;
            for (i, field) in ctor.fields.iter().enumerate() {
                let column: Vec<&Pattern> = rows
                    .iter()
                    .filter(|r| {
                        r.len() == ctor.fields.len()
                            && r.iter().enumerate().all(|(j, p)| j == i || irrefutable(p))
                    })
                    .map(|r| &r[i])
                    .collect();
                if !column.is_empty() && self.missing_cases(&column, field).is_empty() {
                    covered = true;
                    break;
                }
            }
            if !covered {
                missing.push(blank());
            }
        }
        missing
    }
}
