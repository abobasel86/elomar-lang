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
    /// An open database (`std.db`).
    Db,
    List(Box<Ty>),
    /// The pending result of an `async fn` call or `ai.infer`.
    Future(Box<Ty>),
    Option(Box<Ty>),
    Result(Box<Ty>, Box<Ty>),
    /// Structural record; fields are kept sorted by name.
    Record(Vec<(String, Ty)>),
    /// `(Int, String)`: two or more values by position.
    Tuple(Vec<Ty>),
    /// A nominal enum, by name, with its type arguments (`Tree<Int>`).
    Enum(String, Vec<Ty>),
    /// `Map<K, V>`; keys are `String` or `Int`.
    Map(Box<Ty>, Box<Ty>),
    /// `Set<T>`; elements are `String` or `Int`.
    Set(Box<Ty>),
    /// A type parameter inside a generic function (`T` in `fn f<T>`):
    /// equal only to itself.
    Param(String),
    /// A function value: `fn(Int, String) -> Bool`.
    Fn(Vec<Ty>, Box<Ty>),
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

    pub fn set(t: Ty) -> Ty {
        Ty::Set(Box::new(t))
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

    /// Replaces type parameters by the types in `map`.
    pub fn substitute(&self, map: &[(String, Ty)]) -> Ty {
        let sub = |t: &Ty| t.substitute(map);
        match self {
            Ty::Param(name) => map
                .iter()
                .find(|(n, _)| n == name)
                .map_or_else(|| self.clone(), |(_, t)| t.clone()),
            Ty::List(t) => Ty::list(sub(t)),
            Ty::Option(t) => Ty::option(sub(t)),
            Ty::Future(t) => Ty::future(sub(t)),
            Ty::Set(t) => Ty::set(sub(t)),
            Ty::Result(t, e) => Ty::result(sub(t), sub(e)),
            Ty::Map(k, v) => Ty::map(sub(k), sub(v)),
            Ty::Fn(ps, r) => Ty::Fn(ps.iter().map(sub).collect(), Box::new(sub(r))),
            Ty::Enum(n, args) => Ty::Enum(n.clone(), args.iter().map(sub).collect()),
            Ty::Record(fields) => {
                Ty::Record(fields.iter().map(|(n, t)| (n.clone(), sub(t))).collect())
            }
            Ty::Tuple(items) => Ty::Tuple(items.iter().map(sub).collect()),
            other => other.clone(),
        }
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
            Ty::Set(t) => Ty::set(self.resolve(&t)),
            Ty::Fn(ps, r) => Ty::Fn(
                ps.iter().map(|p| self.resolve(p)).collect(),
                Box::new(self.resolve(&r)),
            ),
            Ty::Option(t) => Ty::option(self.resolve(&t)),
            Ty::Enum(n, args) => Ty::Enum(n, args.iter().map(|a| self.resolve(a)).collect()),
            Ty::Result(t, e) => Ty::result(self.resolve(&t), self.resolve(&e)),
            Ty::Map(k, v) => Ty::map(self.resolve(&k), self.resolve(&v)),
            Ty::Record(fields) => Ty::Record(
                fields
                    .iter()
                    .map(|(n, t)| (n.clone(), self.resolve(t)))
                    .collect(),
            ),
            Ty::Tuple(items) => Ty::Tuple(items.iter().map(|t| self.resolve(t)).collect()),
            other => other,
        }
    }

    fn occurs(&self, v: u32, ty: &Ty) -> bool {
        match self.shallow(ty) {
            Ty::Var(w) => v == w,
            Ty::List(t) | Ty::Option(t) | Ty::Future(t) | Ty::Set(t) => self.occurs(v, &t),
            Ty::Result(t, e) | Ty::Map(t, e) => self.occurs(v, &t) || self.occurs(v, &e),
            Ty::Record(fields) => fields.iter().any(|(_, t)| self.occurs(v, t)),
            Ty::Tuple(items) => items.iter().any(|t| self.occurs(v, t)),
            Ty::Fn(ps, r) => ps.iter().any(|p| self.occurs(v, p)) || self.occurs(v, &r),
            Ty::Enum(_, args) => args.iter().any(|a| self.occurs(v, a)),
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
            | (Ty::Future(x), Ty::Future(y))
            | (Ty::Set(x), Ty::Set(y)) => self.unify(x, y),
            (Ty::Result(t1, e1), Ty::Result(t2, e2)) | (Ty::Map(t1, e1), Ty::Map(t2, e2)) => {
                let ok = self.unify(t1, t2);
                self.unify(e1, e2) && ok
            }
            (Ty::Enum(n1, a1), Ty::Enum(n2, a2)) => {
                if n1 != n2 || a1.len() != a2.len() {
                    return false;
                }
                let mut ok = true;
                for (x, y) in a1.iter().zip(a2) {
                    ok &= self.unify(x, y);
                }
                ok
            }
            (Ty::Fn(p1, r1), Ty::Fn(p2, r2)) => {
                if p1.len() != p2.len() {
                    return false;
                }
                let mut ok = true;
                for (a, b) in p1.iter().zip(p2) {
                    ok &= self.unify(a, b);
                }
                self.unify(r1, r2) && ok
            }
            (Ty::Tuple(a), Ty::Tuple(b)) => {
                if a.len() != b.len() {
                    return false;
                }
                let mut ok = true;
                for (x, y) in a.iter().zip(b) {
                    ok &= self.unify(x, y);
                }
                ok
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
            Ty::Db => out.push_str("Db"),
            Ty::Param(name) => out.push_str(name),
            Ty::Enum(name, args) => {
                out.push_str(name);
                if !args.is_empty() {
                    out.push('<');
                    for (i, a) in args.iter().enumerate() {
                        if i > 0 {
                            out.push_str(", ");
                        }
                        self.write(out, a);
                    }
                    out.push('>');
                }
            }
            Ty::Fn(ps, r) => {
                out.push_str("fn(");
                for (i, p) in ps.iter().enumerate() {
                    if i > 0 {
                        out.push_str(", ");
                    }
                    self.write(out, p);
                }
                out.push_str(") -> ");
                self.write(out, r);
            }
            Ty::Set(t) => {
                out.push_str("Set<");
                self.write(out, t);
                out.push('>');
            }
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
            Ty::Tuple(items) => {
                out.push('(');
                for (i, t) in items.iter().enumerate() {
                    if i > 0 {
                        out.push_str(", ");
                    }
                    self.write(out, t);
                }
                out.push(')');
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
