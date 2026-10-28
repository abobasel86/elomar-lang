//! Types and unification.

use std::fmt::Write as _;

#[derive(Debug, Clone, PartialEq)]
pub enum Ty {
    Int,
    Float,
    Bool,
    Str,
    Unit,
    Range,
    Duration,
    List(Box<Ty>),
    /// The pending result of an `async fn` call or `ai.infer`.
    Future(Box<Ty>),
    Option(Box<Ty>),
    Result(Box<Ty>, Box<Ty>),
    /// Structural record; fields are kept sorted by name.
    Record(Vec<(String, Ty)>),
    /// A nominal enum, by name.
    Enum(String),
    /// `Map<K, V>`; keys are `String` or `Int`.
    Map(Box<Ty>, Box<Ty>),
    /// An inference variable, index into [`Subst`].
    Var(u32),
    /// Produced after an error has been reported; unifies with anything so
    /// one mistake does not cascade into many diagnostics.
    Error,
}

impl Ty {
    pub fn list(t: Ty) -> Ty {
        Ty::List(Box::new(t))
    }

    pub fn map(k: Ty, v: Ty) -> Ty {
        Ty::Map(Box::new(k), Box::new(v))
    }

    pub fn future(t: Ty) -> Ty {
        Ty::Future(Box::new(t))
    }

    pub fn option(t: Ty) -> Ty {
        Ty::Option(Box::new(t))
    }

    pub fn result(t: Ty, e: Ty) -> Ty {
        Ty::Result(Box::new(t), Box::new(e))
    }

    pub fn record(mut fields: Vec<(String, Ty)>) -> Ty {
        fields.sort_by(|a, b| a.0.cmp(&b.0));
        Ty::Record(fields)
    }

    pub fn is_numeric(&self) -> bool {
        matches!(self, Ty::Int | Ty::Float)
    }
}

/// Bindings for inference variables.
#[derive(Debug, Default)]
pub struct Subst {
    vars: Vec<Option<Ty>>,
}

impl Subst {
    pub fn fresh(&mut self) -> Ty {
        self.vars.push(None);
        Ty::Var(self.vars.len() as u32 - 1)
    }

    /// Follows variable bindings at the top level only.
    pub fn shallow(&self, ty: &Ty) -> Ty {
        let mut ty = ty.clone();
        while let Ty::Var(v) = ty {
            match &self.vars[v as usize] {
                Some(t) => ty = t.clone(),
                None => break,
            }
        }
        ty
    }

    /// Replaces every bound variable, recursively.
    pub fn resolve(&self, ty: &Ty) -> Ty {
        match self.shallow(ty) {
            Ty::List(t) => Ty::list(self.resolve(&t)),
            Ty::Future(t) => Ty::future(self.resolve(&t)),
            Ty::Option(t) => Ty::option(self.resolve(&t)),
            Ty::Result(t, e) => Ty::result(self.resolve(&t), self.resolve(&e)),
            Ty::Map(k, v) => Ty::map(self.resolve(&k), self.resolve(&v)),
            Ty::Record(fields) => Ty::Record(
                fields
                    .iter()
                    .map(|(n, t)| (n.clone(), self.resolve(t)))
                    .collect(),
            ),
            other => other,
        }
    }

    fn occurs(&self, v: u32, ty: &Ty) -> bool {
        match self.shallow(ty) {
            Ty::Var(w) => v == w,
            Ty::List(t) | Ty::Option(t) | Ty::Future(t) => self.occurs(v, &t),
            Ty::Result(t, e) | Ty::Map(t, e) => self.occurs(v, &t) || self.occurs(v, &e),
            Ty::Record(fields) => fields.iter().any(|(_, t)| self.occurs(v, t)),
            _ => false,
        }
    }

    /// Makes `a` and `b` equal, binding variables as needed.
    pub fn unify(&mut self, a: &Ty, b: &Ty) -> bool {
        let a = self.shallow(a);
        let b = self.shallow(b);
        match (&a, &b) {
            (Ty::Error, _) | (_, Ty::Error) => true,
            (Ty::Var(x), Ty::Var(y)) if x == y => true,
            (Ty::Var(x), t) | (t, Ty::Var(x)) => {
                if self.occurs(*x, t) {
                    return false;
                }
                self.vars[*x as usize] = Some(t.clone());
                true
            }
            (Ty::List(x), Ty::List(y))
            | (Ty::Option(x), Ty::Option(y))
            | (Ty::Future(x), Ty::Future(y)) => self.unify(x, y),
            (Ty::Result(t1, e1), Ty::Result(t2, e2)) | (Ty::Map(t1, e1), Ty::Map(t2, e2)) => {
                let ok = self.unify(t1, t2);
                self.unify(e1, e2) && ok
            }
            (Ty::Record(f1), Ty::Record(f2)) => {
                if f1.len() != f2.len() || f1.iter().zip(f2).any(|(a, b)| a.0 != b.0) {
                    return false;
                }
                let mut ok = true;
                for ((_, t1), (_, t2)) in f1.iter().zip(f2) {
                    ok &= self.unify(t1, t2);
                }
                ok
            }
            (x, y) => x == y,
        }
    }

    /// Renders a type for diagnostics; unknown parts print as `_`.
    pub fn show(&self, ty: &Ty) -> String {
        let mut out = String::new();
        self.write(&mut out, &self.resolve(ty));
        out
    }

    fn write(&self, out: &mut String, ty: &Ty) {
        match ty {
            Ty::Int => out.push_str("Int"),
            Ty::Float => out.push_str("Float"),
            Ty::Bool => out.push_str("Bool"),
            Ty::Str => out.push_str("String"),
            Ty::Unit => out.push_str("Unit"),
            Ty::Range => out.push_str("Range"),
            Ty::Duration => out.push_str("Duration"),
            Ty::Enum(name) => out.push_str(name),
            Ty::Map(k, v) => {
                out.push_str("Map<");
                self.write(out, k);
                out.push_str(", ");
                self.write(out, v);
                out.push('>');
            }
            Ty::Future(t) => {
                out.push_str("Future<");
                self.write(out, t);
                out.push('>');
            }
            Ty::List(t) => {
                out.push_str("List<");
                self.write(out, t);
                out.push('>');
            }
            Ty::Option(t) => {
                out.push_str("Option<");
                self.write(out, t);
                out.push('>');
            }
            Ty::Result(t, e) => {
                out.push_str("Result<");
                self.write(out, t);
                out.push_str(", ");
                self.write(out, e);
                out.push('>');
            }
            Ty::Record(fields) => {
                out.push_str("{ ");
                for (i, (name, t)) in fields.iter().enumerate() {
                    if i > 0 {
                        out.push_str(", ");
                    }
                    let _ = write!(out, "{name}: ");
                    self.write(out, t);
                }
                out.push_str(if fields.is_empty() { "}" } else { " }" });
            }
            Ty::Var(_) | Ty::Error => out.push('_'),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unifies_through_variables() {
        let mut s = Subst::default();
        let a = s.fresh();
        assert!(s.unify(&Ty::list(a.clone()), &Ty::list(Ty::Int)));
        assert_eq!(s.resolve(&a), Ty::Int);
        assert!(!s.unify(&a, &Ty::Str));
    }

    #[test]
    fn records_unify_structurally() {
        let mut s = Subst::default();
        let r1 = Ty::record(vec![("b".into(), Ty::Int), ("a".into(), Ty::Str)]);
        let r2 = Ty::record(vec![("a".into(), Ty::Str), ("b".into(), Ty::Int)]);
        assert!(s.unify(&r1, &r2));
        assert_eq!(s.show(&r1), "{ a: String, b: Int }");
    }

    #[test]
    fn occurs_check() {
        let mut s = Subst::default();
        let a = s.fresh();
        assert!(!s.unify(&a, &Ty::list(a.clone())));
    }
}
