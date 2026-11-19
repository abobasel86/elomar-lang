//! Runtime values. Lists, records and strings are reference-counted and
//! copied on write (`Rc::make_mut`), which gives value semantics (spec §10.7)
//! without copying on every assignment.

use std::cell::RefCell;
use std::cmp::Ordering;
use std::collections::{BTreeMap, BTreeSet};
use std::fmt;
use std::rc::Rc;

use elomar_ast::Span;

use crate::ai::InferCall;

/// Work deferred until a future is awaited.
#[derive(Debug, Clone)]
pub enum Thunk {
    /// A call to an `async fn`.
    Call {
        name: String,
        args: Vec<Value>,
        span: Span,
    },
    Infer(InferCall),
}

/// A `Map` key. The checker allows only `String` and `Int` keys; maps
/// iterate in key order.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub enum MapKey {
    Int(i64),
    Str(Rc<str>),
}

impl MapKey {
    pub fn from_value(v: &Value) -> MapKey {
        match v {
            Value::Int(n) => MapKey::Int(*n),
            Value::Str(s) => MapKey::Str(s.clone()),
            other => unreachable!("checker allows only String and Int keys, got {other:?}"),
        }
    }

    pub fn to_value(&self) -> Value {
        match self {
            MapKey::Int(n) => Value::Int(*n),
            MapKey::Str(s) => Value::Str(s.clone()),
        }
    }
}

/// A function value.
#[derive(Debug)]
pub enum Function {
    /// A named function used as a value: `xs.map(double)`.
    Named(String),
    /// A closure: `key` identifies its `|...| body` expression, and
    /// `captured` holds copies of the variables visible where it was made.
    Closure {
        key: usize,
        captured: Vec<(String, Value)>,
    },
}

#[derive(Debug, Clone)]
pub enum FutureState {
    Pending(Thunk),
    Running,
    Done(Value),
}

#[derive(Debug, Clone)]
pub enum Value {
    Int(i64),
    Float(f64),
    Bool(bool),
    Str(Rc<str>),
    Unit,
    List(Rc<Vec<Value>>),
    /// Fields in the order they were written.
    Record(Rc<Vec<(Rc<str>, Value)>>),
    Range {
        start: i64,
        end: i64,
        inclusive: bool,
    },
    /// Milliseconds.
    Duration(i64),
    /// `Shape.Circle(1.0)`; `fields` is empty for a plain variant.
    Enum {
        enum_name: Rc<str>,
        variant: Rc<str>,
        fields: Rc<Vec<Value>>,
    },
    Map(Rc<BTreeMap<MapKey, Value>>),
    /// Elements are `String` or `Int` and iterate in order.
    Set(Rc<BTreeSet<MapKey>>),
    /// Shared handle: copies of a future refer to the same computation.
    Future(Rc<RefCell<FutureState>>),
    Option(Option<Rc<Value>>),
    Result(Result<Rc<Value>, Rc<Value>>),
    Function(Rc<Function>),
}

impl Value {
    pub fn str(s: impl Into<Rc<str>>) -> Value {
        Value::Str(s.into())
    }

    pub fn list(items: Vec<Value>) -> Value {
        Value::List(Rc::new(items))
    }

    pub fn some(v: Value) -> Value {
        Value::Option(Some(Rc::new(v)))
    }

    pub fn none() -> Value {
        Value::Option(None)
    }

    pub fn future(thunk: Thunk) -> Value {
        Value::Future(Rc::new(RefCell::new(FutureState::Pending(thunk))))
    }

    pub fn as_f64(&self) -> Option<f64> {
        match self {
            Value::Int(n) => Some(*n as f64),
            Value::Float(f) => Some(*f),
            _ => None,
        }
    }

    /// Structural equality; `Int` and `Float` compare numerically.
    pub fn equals(&self, other: &Value) -> bool {
        use Value::*;
        match (self, other) {
            (Int(a), Int(b)) => a == b,
            (Int(_) | Float(_), Int(_) | Float(_)) => self.as_f64() == other.as_f64(),
            (Bool(a), Bool(b)) => a == b,
            (Str(a), Str(b)) => a == b,
            (Unit, Unit) => true,
            (Duration(a), Duration(b)) => a == b,
            (
                Enum {
                    enum_name: e1,
                    variant: v1,
                    fields: f1,
                },
                Enum {
                    enum_name: e2,
                    variant: v2,
                    fields: f2,
                },
            ) => e1 == e2 && v1 == v2 && f1.iter().zip(f2.iter()).all(|(x, y)| x.equals(y)),
            (Set(a), Set(b)) => a == b,
            (Map(a), Map(b)) => {
                a.len() == b.len() && a.iter().all(|(k, v)| b.get(k).is_some_and(|w| v.equals(w)))
            }
            (Future(a), Future(b)) => Rc::ptr_eq(a, b),
            (Function(a), Function(b)) => Rc::ptr_eq(a, b),
            (List(a), List(b)) => {
                a.len() == b.len() && a.iter().zip(b.iter()).all(|(x, y)| x.equals(y))
            }
            (Record(a), Record(b)) => {
                a.len() == b.len()
                    && a.iter()
                        .all(|(name, v)| b.iter().any(|(n, w)| n == name && v.equals(w)))
            }
            (
                Range {
                    start: s1,
                    end: e1,
                    inclusive: i1,
                },
                Range {
                    start: s2,
                    end: e2,
                    inclusive: i2,
                },
            ) => s1 == s2 && e1 == e2 && i1 == i2,
            (Option(a), Option(b)) => match (a, b) {
                (Some(x), Some(y)) => x.equals(y),
                (None, None) => true,
                _ => false,
            },
            (Result(a), Result(b)) => match (a, b) {
                (Ok(x), Ok(y)) | (Err(x), Err(y)) => x.equals(y),
                _ => false,
            },
            _ => false,
        }
    }

    /// Ordering for `<`, `sorted()` etc. Numbers compare numerically
    /// (`NaN` sorts last), strings by Unicode code point.
    pub fn compare(&self, other: &Value) -> Ordering {
        match (self, other) {
            (Value::Int(a), Value::Int(b)) => a.cmp(b),
            (Value::Str(a), Value::Str(b)) => a.cmp(b),
            (Value::Duration(a), Value::Duration(b)) => a.cmp(b),
            _ => match (self.as_f64(), other.as_f64()) {
                (Some(a), Some(b)) => a.total_cmp(&b),
                _ => Ordering::Equal,
            },
        }
    }

    /// How the value appears inside a list or record: strings are quoted.
    fn fmt_nested(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Value::Str(s) => write!(f, "{s:?}"),
            other => write!(f, "{other}"),
        }
    }
}

/// `1h`, `2m`, `10s` or `250ms`: the largest unit that divides exactly.
pub fn format_duration(ms: i64) -> String {
    for (unit, size) in [("h", 3_600_000), ("m", 60_000), ("s", 1_000)] {
        if ms != 0 && ms % size == 0 {
            return format!("{}{unit}", ms / size);
        }
    }
    format!("{ms}ms")
}

pub fn format_float(x: f64) -> String {
    if x.is_finite() && x.fract() == 0.0 && x.abs() < 1e16 {
        format!("{x:.1}")
    } else if x.is_nan() {
        "NaN".into()
    } else if x.is_infinite() {
        if x > 0.0 { "inf".into() } else { "-inf".into() }
    } else {
        format!("{x}")
    }
}

/// How `print` shows a value: strings appear without quotes.
impl fmt::Display for Value {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Value::Int(n) => write!(f, "{n}"),
            Value::Float(x) => f.write_str(&format_float(*x)),
            Value::Bool(b) => write!(f, "{b}"),
            Value::Str(s) => f.write_str(s),
            Value::Unit => f.write_str("()"),
            Value::List(items) => {
                f.write_str("[")?;
                for (i, item) in items.iter().enumerate() {
                    if i > 0 {
                        f.write_str(", ")?;
                    }
                    item.fmt_nested(f)?;
                }
                f.write_str("]")
            }
            Value::Record(fields) => {
                if fields.is_empty() {
                    return f.write_str("{}");
                }
                f.write_str("{ ")?;
                for (i, (name, v)) in fields.iter().enumerate() {
                    if i > 0 {
                        f.write_str(", ")?;
                    }
                    write!(f, "{name}: ")?;
                    v.fmt_nested(f)?;
                }
                f.write_str(" }")
            }
            Value::Range {
                start,
                end,
                inclusive,
            } => write!(f, "{start}{}{end}", if *inclusive { "..=" } else { ".." }),
            Value::Duration(ms) => f.write_str(&format_duration(*ms)),
            Value::Enum {
                enum_name,
                variant,
                fields,
            } => {
                write!(f, "{enum_name}.{variant}")?;
                if !fields.is_empty() {
                    f.write_str("(")?;
                    for (i, v) in fields.iter().enumerate() {
                        if i > 0 {
                            f.write_str(", ")?;
                        }
                        v.fmt_nested(f)?;
                    }
                    f.write_str(")")?;
                }
                Ok(())
            }
            // Written the way a set is created: `set([1, 2])`.
            Value::Set(items) => {
                f.write_str("set([")?;
                for (i, k) in items.iter().enumerate() {
                    if i > 0 {
                        f.write_str(", ")?;
                    }
                    k.to_value().fmt_nested(f)?;
                }
                f.write_str("])")
            }
            Value::Map(entries) => {
                if entries.is_empty() {
                    return f.write_str("[:]");
                }
                f.write_str("[")?;
                for (i, (k, v)) in entries.iter().enumerate() {
                    if i > 0 {
                        f.write_str(", ")?;
                    }
                    k.to_value().fmt_nested(f)?;
                    f.write_str(": ")?;
                    v.fmt_nested(f)?;
                }
                f.write_str("]")
            }
            Value::Future(_) => f.write_str("<future>"),
            Value::Function(_) => f.write_str("<function>"),
            Value::Option(None) => f.write_str("None"),
            Value::Option(Some(v)) => {
                f.write_str("Some(")?;
                v.fmt_nested(f)?;
                f.write_str(")")
            }
            Value::Result(r) => {
                let (tag, v) = match r {
                    Ok(v) => ("Ok", v),
                    Err(v) => ("Err", v),
                };
                write!(f, "{tag}(")?;
                v.fmt_nested(f)?;
                f.write_str(")")
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn display() {
        assert_eq!(Value::Float(85.0).to_string(), "85.0");
        assert_eq!(Value::Float(0.1 + 0.2).to_string(), "0.30000000000000004");
        let rec = Value::Record(Rc::new(vec![
            ("name".into(), Value::str("Lina")),
            ("tags".into(), Value::list(vec![Value::str("a")])),
        ]));
        assert_eq!(rec.to_string(), "{ name: \"Lina\", tags: [\"a\"] }");
        assert_eq!(Value::some(Value::str("x")).to_string(), "Some(\"x\")");
        assert_eq!(Value::str("مرحبا").to_string(), "مرحبا");
    }

    #[test]
    fn equality_is_structural() {
        let a = Value::Record(Rc::new(vec![
            ("x".into(), Value::Int(1)),
            ("y".into(), Value::Int(2)),
        ]));
        let b = Value::Record(Rc::new(vec![
            ("y".into(), Value::Int(2)),
            ("x".into(), Value::Float(1.0)),
        ]));
        assert!(a.equals(&b));
    }
}
