//! Operators, prelude functions and methods on built-in types. The names and
//! signatures mirror `elomar_types::builtins`.

use std::rc::Rc;

use elomar_ast::{BinOp, Span};
use elomar_diagnostics::Text;
use unicode_segmentation::UnicodeSegmentation;

use crate::value::format_float;
use crate::{Host, RuntimeError, Value};

type R<T> = Result<T, RuntimeError>;

pub(crate) fn graphemes(s: &str) -> unicode_segmentation::Graphemes<'_> {
    s.graphemes(true)
}

pub(crate) fn overflow(span: Span) -> RuntimeError {
    RuntimeError::new(
        "R0002",
        span,
        Text::new(
            "integer overflow: the result does not fit in `Int` (64-bit)",
            "تجاوز سعة العدد الصحيح: النتيجة لا تتسع في `Int` (64 بت)",
        ),
    )
}

fn div_by_zero(span: Span) -> RuntimeError {
    RuntimeError::new("R0001", span, Text::new("division by zero", "قسمة على صفر"))
}

fn invalid_arg(span: Span, en: String, ar: String) -> RuntimeError {
    RuntimeError::new("R0006", span, Text::new(en, ar))
}

fn conversion(span: Span, x: f64) -> RuntimeError {
    let shown = format_float(x);
    RuntimeError::new(
        "R0007",
        span,
        Text::new(
            format!("cannot convert {shown} to `Int`"),
            format!("لا يمكن تحويل {shown} إلى `Int`"),
        ),
    )
}

pub(crate) fn missing_key(key: &Value, span: Span) -> RuntimeError {
    let shown = match key {
        Value::Str(s) => format!("{s:?}"),
        other => other.to_string(),
    };
    RuntimeError::new(
        "R0011",
        span,
        Text::new(
            format!("the key {shown} is not in the map; use `.get(key)` when it may be missing"),
            format!("المفتاح {shown} غير موجود في القاموس؛ استخدم `.get(key)` إن كان قد يغيب"),
        ),
    )
}

/// Validates a list index, returning it as `usize`.
pub(crate) fn check_index(i: i64, len: usize, span: Span) -> R<usize> {
    if i >= 0 && (i as usize) < len {
        Ok(i as usize)
    } else {
        Err(RuntimeError::new(
            "R0003",
            span,
            Text::new(
                format!("index {i} is out of bounds for a list of length {len}"),
                format!("الفهرس {i} خارج حدود قائمة طولها {len}"),
            ),
        ))
    }
}

fn float_to_int(x: f64, span: Span) -> R<i64> {
    if x.is_finite() && x >= i64::MIN as f64 && x < i64::MAX as f64 {
        Ok(x as i64)
    } else {
        Err(conversion(span, x))
    }
}

pub(crate) fn binary(op: BinOp, l: Value, r: Value, span: Span) -> R<Value> {
    use Value::*;
    Ok(match op {
        BinOp::Eq => Bool(l.equals(&r)),
        BinOp::Ne => Bool(!l.equals(&r)),
        BinOp::Lt => Bool(l.compare(&r).is_lt()),
        BinOp::Le => Bool(l.compare(&r).is_le()),
        BinOp::Gt => Bool(l.compare(&r).is_gt()),
        BinOp::Ge => Bool(l.compare(&r).is_ge()),
        BinOp::And | BinOp::Or => unreachable!("short-circuited by the caller"),
        _ => match (l, r) {
            (Int(a), Int(b)) => match op {
                BinOp::Add => Int(a.checked_add(b).ok_or_else(|| overflow(span))?),
                BinOp::Sub => Int(a.checked_sub(b).ok_or_else(|| overflow(span))?),
                BinOp::Mul => Int(a.checked_mul(b).ok_or_else(|| overflow(span))?),
                BinOp::Div => {
                    if b == 0 {
                        return Err(div_by_zero(span));
                    }
                    Float(a as f64 / b as f64)
                }
                BinOp::IntDiv | BinOp::Rem => {
                    if b == 0 {
                        return Err(div_by_zero(span));
                    }
                    let v = if op == BinOp::IntDiv {
                        a.checked_div(b)
                    } else {
                        a.checked_rem(b)
                    };
                    Int(v.ok_or_else(|| overflow(span))?)
                }
                _ => unreachable!(),
            },
            (Str(a), Str(b)) => {
                let mut s = String::with_capacity(a.len() + b.len());
                s.push_str(&a);
                s.push_str(&b);
                Value::str(s)
            }
            (Duration(a), Duration(b)) => Duration(
                match op {
                    BinOp::Add => a.checked_add(b),
                    _ => a.checked_sub(b),
                }
                .ok_or_else(|| overflow(span))?,
            ),
            (List(a), List(b)) => {
                let mut items = (*a).clone();
                items.extend(b.iter().cloned());
                Value::list(items)
            }
            (l, r) => {
                let (a, b) = (l.as_f64().unwrap_or(0.0), r.as_f64().unwrap_or(0.0));
                Float(match op {
                    BinOp::Add => a + b,
                    BinOp::Sub => a - b,
                    BinOp::Mul => a * b,
                    BinOp::Div | BinOp::Rem => {
                        if b == 0.0 {
                            return Err(div_by_zero(span));
                        }
                        if op == BinOp::Div { a / b } else { a % b }
                    }
                    _ => unreachable!(),
                })
            }
        },
    })
}

fn arg_str(v: &Value) -> &str {
    match v {
        Value::Str(s) => s,
        _ => "",
    }
}

fn arg_int(v: &Value) -> i64 {
    match v {
        Value::Int(n) => *n,
        _ => 0,
    }
}

pub(crate) fn call_function(
    host: &mut dyn Host,
    name: &str,
    args: Vec<Value>,
    span: Span,
) -> R<Value> {
    let mut args = args;
    Ok(match name {
        "Some" => Value::some(args.remove(0)),
        "Ok" => Value::Result(Ok(Rc::new(args.remove(0)))),
        "Err" => Value::Result(Err(Rc::new(args.remove(0)))),
        "print" => {
            let mut line = args
                .iter()
                .map(|v| v.to_string())
                .collect::<Vec<_>>()
                .join(" ");
            line.push('\n');
            host.print(&line);
            Value::Unit
        }
        "input" => {
            let prompt = args.first().map(arg_str).unwrap_or("");
            Value::str(host.input(prompt).unwrap_or_default())
        }
        "to_float" => Value::Float(args[0].as_f64().unwrap_or(0.0)),
        "to_int" => match &args[0] {
            Value::Float(x) => Value::Int(float_to_int(*x, span)?),
            other => other.clone(),
        },
        "to_string" => Value::str(args[0].to_string()),
        "parse_int" => parse_int(arg_str(&args[0])),
        "parse_float" => parse_float(arg_str(&args[0])),
        "assert" => {
            if !matches!(args[0], Value::Bool(true)) {
                let (en, ar) = match args.get(1) {
                    Some(msg) => (
                        format!("assertion failed: {}", arg_str(msg)),
                        format!("فشل التحقق: {}", arg_str(msg)),
                    ),
                    None => ("assertion failed".to_string(), "فشل التحقق".to_string()),
                };
                return Err(RuntimeError::new("R0004", span, Text::new(en, ar)));
            }
            Value::Unit
        }
        "assert_eq" => {
            if !args[0].equals(&args[1]) {
                let (actual, expected) = (repr(&args[0]), repr(&args[1]));
                return Err(RuntimeError::new(
                    "R0004",
                    span,
                    Text::new(
                        format!(
                            "assertion failed: values differ\n    actual:   {actual}\n    expected: {expected}"
                        ),
                        format!(
                            "فشل التحقق: القيمتان مختلفتان\n    الفعلية:   {actual}\n    المتوقعة: {expected}"
                        ),
                    ),
                ));
            }
            Value::Unit
        }
        "min" | "max" => {
            let (a, b) = (&args[0], &args[1]);
            let pick_a = if name == "min" {
                a.compare(b).is_le()
            } else {
                a.compare(b).is_ge()
            };
            let v = if pick_a { a.clone() } else { b.clone() };
            // Mixed Int/Float arguments give a Float result (spec §10.3).
            match (a, b, v) {
                (Value::Int(_), Value::Int(_), v) => v,
                (_, _, v) => Value::Float(v.as_f64().unwrap_or(0.0)),
            }
        }
        "abs" => match &args[0] {
            Value::Int(n) => Value::Int(n.checked_abs().ok_or_else(|| overflow(span))?),
            Value::Float(x) => Value::Float(x.abs()),
            other => other.clone(),
        },
        _ => unreachable!("unknown builtin `{name}`"),
    })
}

/// A value as it would appear inside a list: strings are quoted.
fn repr(v: &Value) -> String {
    match v {
        Value::Str(s) => format!("{s:?}"),
        other => other.to_string(),
    }
}

fn parse_int(s: &str) -> Value {
    match s.trim().parse::<i64>() {
        Ok(n) => Value::some(Value::Int(n)),
        Err(_) => Value::none(),
    }
}

fn parse_float(s: &str) -> Value {
    match s.trim().parse::<f64>() {
        Ok(x) if x.is_finite() => Value::some(Value::Float(x)),
        _ => Value::none(),
    }
}

pub(crate) fn is_mutating(method: &str) -> bool {
    matches!(method, "push" | "pop" | "insert" | "remove")
}

/// Methods that change their receiver in place.
pub(crate) fn call_mutating(
    slot: &mut Value,
    method: &str,
    mut args: Vec<Value>,
    span: Span,
) -> R<Value> {
    if let Value::Map(entries) = slot {
        // The only mutating map method is `remove`.
        let removed = Rc::make_mut(entries).remove(&crate::MapKey::from_value(&args[0]));
        return Ok(removed.map_or(Value::none(), Value::some));
    }
    let Value::List(items) = slot else {
        unreachable!("checked receiver")
    };
    let items = Rc::make_mut(items);
    Ok(match method {
        "push" => {
            items.push(args.remove(0));
            Value::Unit
        }
        "pop" => match items.pop() {
            Some(v) => Value::some(v),
            None => Value::none(),
        },
        "insert" => {
            let i = arg_int(&args[0]);
            if i < 0 || i as usize > items.len() {
                check_index(i, items.len(), span)?;
            }
            items.insert(i as usize, args.remove(1));
            Value::Unit
        }
        "remove" => {
            let i = check_index(arg_int(&args[0]), items.len(), span)?;
            items.remove(i)
        }
        _ => unreachable!("not mutating: {method}"),
    })
}

pub(crate) fn call_method(recv: Value, method: &str, args: Vec<Value>, span: Span) -> R<Value> {
    if method == "to_string" {
        return Ok(Value::str(recv.to_string()));
    }
    Ok(match (recv, method) {
        (Value::List(items), _) => list_method(&items, method, &args, span)?,
        (Value::Map(entries), _) => {
            let key = || crate::MapKey::from_value(&args[0]);
            match method {
                "get" => entries
                    .get(&key())
                    .cloned()
                    .map_or(Value::none(), Value::some),
                "contains_key" => Value::Bool(entries.contains_key(&key())),
                "keys" => Value::list(entries.keys().map(|k| k.to_value()).collect()),
                "values" => Value::list(entries.values().cloned().collect()),
                "is_empty" => Value::Bool(entries.is_empty()),
                m => unreachable!("checked map method `{m}`"),
            }
        }
        (Value::Enum { variant, .. }, "kind") => Value::Str(variant),
        (Value::Enum { fields, .. }, "message") => fields.first().cloned().unwrap_or(Value::Unit),
        (Value::Str(s), _) => str_method(&s, method, &args, span)?,
        (Value::Int(n), "to_float") => Value::Float(n as f64),
        (Value::Int(n), "abs") => Value::Int(n.checked_abs().ok_or_else(|| overflow(span))?),
        (Value::Float(x), "to_int") => Value::Int(float_to_int(x, span)?),
        (Value::Float(x), "round") => Value::Int(float_to_int(x.round(), span)?),
        (Value::Float(x), "floor") => Value::Int(float_to_int(x.floor(), span)?),
        (Value::Float(x), "ceil") => Value::Int(float_to_int(x.ceil(), span)?),
        (Value::Float(x), "abs") => Value::Float(x.abs()),
        (Value::Float(x), "sqrt") => {
            if x < 0.0 {
                let shown = format_float(x);
                return Err(invalid_arg(
                    span,
                    format!("cannot take the square root of a negative number ({shown})"),
                    format!("لا يمكن حساب الجذر التربيعي لعدد سالب ({shown})"),
                ));
            }
            Value::Float(x.sqrt())
        }
        (Value::Duration(ms), "millis") => Value::Int(ms),
        (Value::Duration(ms), "seconds") => Value::Float(ms as f64 / 1000.0),
        (Value::Option(o), "is_some") => Value::Bool(o.is_some()),
        (Value::Option(o), "is_none") => Value::Bool(o.is_none()),
        (Value::Option(o), "unwrap_or") => match o {
            Some(v) => (*v).clone(),
            None => args.into_iter().next().unwrap_or(Value::Unit),
        },
        (Value::Result(r), "is_ok") => Value::Bool(r.is_ok()),
        (Value::Result(r), "is_err") => Value::Bool(r.is_err()),
        (Value::Result(r), "unwrap_or") => match r {
            Ok(v) => (*v).clone(),
            Err(_) => args.into_iter().next().unwrap_or(Value::Unit),
        },
        (_, m) => unreachable!("checked method `{m}`"),
    })
}

fn list_method(items: &Rc<Vec<Value>>, method: &str, args: &[Value], span: Span) -> R<Value> {
    Ok(match method {
        "contains" => Value::Bool(items.iter().any(|v| v.equals(&args[0]))),
        "index_of" => match items.iter().position(|v| v.equals(&args[0])) {
            Some(i) => Value::some(Value::Int(i as i64)),
            None => Value::none(),
        },
        "is_empty" => Value::Bool(items.is_empty()),
        "first" => items.first().cloned().map_or(Value::none(), Value::some),
        "last" => items.last().cloned().map_or(Value::none(), Value::some),
        "reverse" => {
            let mut v = (**items).clone();
            v.reverse();
            Value::list(v)
        }
        "sorted" => {
            let mut v = (**items).clone();
            v.sort_by(|a, b| a.compare(b));
            Value::list(v)
        }
        "slice" => {
            let (start, end) = (arg_int(&args[0]), arg_int(&args[1]));
            let len = items.len() as i64;
            if start < 0 || end > len || start > end {
                return Err(RuntimeError::new(
                    "R0003",
                    span,
                    Text::new(
                        format!("slice {start}..{end} is out of bounds for a list of length {len}"),
                        format!("المقطع {start}..{end} خارج حدود قائمة طولها {len}"),
                    ),
                ));
            }
            Value::list(items[start as usize..end as usize].to_vec())
        }
        "join" => Value::str(
            items
                .iter()
                .map(|v| v.to_string())
                .collect::<Vec<_>>()
                .join(arg_str(&args[0])),
        ),
        "sum" => {
            let mut total: i64 = 0;
            for v in items.iter() {
                total = total
                    .checked_add(arg_int(v))
                    .ok_or_else(|| overflow(span))?;
            }
            Value::Int(total)
        }
        // `Iterator::sum` for floats starts at -0.0; start at 0.0 instead.
        "sum.float" => Value::Float(
            items
                .iter()
                .filter_map(Value::as_f64)
                .fold(0.0, |a, b| a + b),
        ),
        _ => unreachable!("checked list method `{method}`"),
    })
}

fn str_method(s: &str, method: &str, args: &[Value], span: Span) -> R<Value> {
    Ok(match method {
        "upper" => Value::str(s.to_uppercase()),
        "lower" => Value::str(s.to_lowercase()),
        "trim" => Value::str(s.trim()),
        "contains" => Value::Bool(s.contains(arg_str(&args[0]))),
        "starts_with" => Value::Bool(s.starts_with(arg_str(&args[0]))),
        "ends_with" => Value::Bool(s.ends_with(arg_str(&args[0]))),
        "split" => {
            let sep = arg_str(&args[0]);
            let parts: Vec<Value> = if sep.is_empty() {
                graphemes(s).map(Value::str).collect()
            } else {
                s.split(sep).map(Value::str).collect()
            };
            Value::list(parts)
        }
        "chars" => Value::list(graphemes(s).map(Value::str).collect()),
        "replace" => Value::str(s.replace(arg_str(&args[0]), arg_str(&args[1]))),
        "repeat" => {
            let n = arg_int(&args[0]);
            if n < 0 || (n as u128) * (s.len() as u128) > 100_000_000 {
                return Err(invalid_arg(
                    span,
                    format!("cannot repeat a string {n} times"),
                    format!("لا يمكن تكرار النص {n} مرة"),
                ));
            }
            Value::str(s.repeat(n as usize))
        }
        "parse_int" => parse_int(s),
        "parse_float" => parse_float(s),
        "is_empty" => Value::Bool(s.is_empty()),
        _ => unreachable!("checked string method `{method}`"),
    })
}
