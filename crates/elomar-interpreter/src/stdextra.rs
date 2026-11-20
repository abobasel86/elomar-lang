//! `std.math`, `std.random`, `std.time` and `std.os` at run time.
//! Times are `Int` milliseconds since 1970-01-01 00:00 UTC.

use std::cell::Cell;
use std::rc::Rc;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use elomar_ast::Span;
use elomar_diagnostics::Text;

use crate::{Host, RuntimeError, Value};

type R<T> = Result<T, RuntimeError>;

fn num(v: &Value) -> f64 {
    v.as_f64().unwrap_or(0.0)
}

fn int(v: &Value) -> i64 {
    match v {
        Value::Int(n) => *n,
        _ => 0,
    }
}

fn invalid(span: Span, en: String, ar: String) -> RuntimeError {
    RuntimeError::new("R0006", span, Text::new(en, ar))
}

/// The value of `math.pi` and the other constants.
pub(crate) fn math_constant(name: &str) -> Value {
    Value::Float(match name {
        "pi" => std::f64::consts::PI,
        "e" => std::f64::consts::E,
        "tau" => std::f64::consts::TAU,
        _ => f64::INFINITY,
    })
}

pub(crate) fn call(
    module: &str,
    func: &str,
    args: &[Value],
    host: &mut dyn Host,
    span: Span,
) -> R<Value> {
    match module {
        "std.math" => Ok(math(func, args)),
        "std.random" => random(func, args, span),
        "std.time" => time(func, args, span),
        _ => os(func, args, host),
    }
}

fn math(func: &str, args: &[Value]) -> Value {
    let x = args.first().map(num).unwrap_or(0.0);
    let y = args.get(1).map(num).unwrap_or(0.0);
    if func == "is_nan" {
        return Value::Bool(x.is_nan());
    }
    Value::Float(match func {
        "sqrt" => x.sqrt(),
        "cbrt" => x.cbrt(),
        "pow" => x.powf(y),
        "exp" => x.exp(),
        "ln" => x.ln(),
        "log10" => x.log10(),
        "log2" => x.log2(),
        "sin" => x.sin(),
        "cos" => x.cos(),
        "tan" => x.tan(),
        "asin" => x.asin(),
        "acos" => x.acos(),
        "atan" => x.atan(),
        "atan2" => x.atan2(y),
        "hypot" => x.hypot(y),
        other => unreachable!("checked math function `{other}`"),
    })
}

// ----- random --------------------------------------------------------------

thread_local! {
    /// xorshift64* state; seeded from the clock on first use.
    static RNG: Cell<u64> = const { Cell::new(0) };
}

fn next_u64() -> u64 {
    RNG.with(|state| {
        let mut x = state.get();
        if x == 0 {
            x = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .map(|d| d.as_nanos() as u64)
                .unwrap_or(0x9E37_79B9_7F4A_7C15)
                | 1;
        }
        x ^= x >> 12;
        x ^= x << 25;
        x ^= x >> 27;
        state.set(x);
        x.wrapping_mul(0x2545_F491_4F6C_DD1D)
    })
}

/// A uniform integer in `0..n` (n > 0), without modulo bias.
fn below(n: u64) -> u64 {
    let zone = u64::MAX - (u64::MAX % n);
    loop {
        let r = next_u64();
        if r < zone {
            return r % n;
        }
    }
}

fn random(func: &str, args: &[Value], span: Span) -> R<Value> {
    Ok(match func {
        "int" => {
            let (lo, hi) = (int(&args[0]), int(&args[1]));
            if lo > hi {
                return Err(invalid(
                    span,
                    format!("`random.int` needs min <= max, got {lo} and {hi}"),
                    format!("`random.int` تحتاج أن يكون الأدنى <= الأعلى، والموجود {lo} و{hi}"),
                ));
            }
            let span_size = (hi as i128 - lo as i128 + 1) as u128;
            let offset = if span_size > u64::MAX as u128 {
                next_u64() as i128
            } else {
                below(span_size as u64) as i128
            };
            Value::Int((lo as i128 + offset) as i64)
        }
        "float" => Value::Float((next_u64() >> 11) as f64 / (1u64 << 53) as f64),
        "bool" => Value::Bool(next_u64() & 1 == 1),
        "seed" => {
            let seed = int(&args[0]) as u64;
            RNG.with(|s| s.set(seed.wrapping_mul(0x9E37_79B9_7F4A_7C15) | 1));
            Value::Unit
        }
        "choice" => match &args[0] {
            Value::List(items) if !items.is_empty() => {
                Value::some(items[below(items.len() as u64) as usize].clone())
            }
            _ => Value::none(),
        },
        "shuffle" => {
            let Value::List(items) = &args[0] else {
                unreachable!("checked list")
            };
            let mut v = (**items).clone();
            for i in (1..v.len()).rev() {
                let j = below(i as u64 + 1) as usize;
                v.swap(i, j);
            }
            Value::list(v)
        }
        other => unreachable!("checked random function `{other}`"),
    })
}

// ----- time ----------------------------------------------------------------

const DAY_MS: i64 = 86_400_000;

/// Days since 1970-01-01 for a date in the proleptic Gregorian calendar.
fn days_from_civil(y: i64, m: i64, d: i64) -> i64 {
    let y = if m <= 2 { y - 1 } else { y };
    let era = if y >= 0 { y } else { y - 399 } / 400;
    let yoe = y - era * 400;
    let mp = if m > 2 { m - 3 } else { m + 9 };
    let doy = (153 * mp + 2) / 5 + d - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    era * 146_097 + doe - 719_468
}

/// The date of a day number from [`days_from_civil`].
fn civil_from_days(z: i64) -> (i64, i64, i64) {
    let z = z + 719_468;
    let era = if z >= 0 { z } else { z - 146_096 } / 146_097;
    let doe = z - era * 146_097;
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    (yoe + era * 400 + i64::from(m <= 2), m, d)
}

fn days_in_month(y: i64, m: i64) -> i64 {
    days_from_civil(
        if m == 12 { y + 1 } else { y },
        if m == 12 { 1 } else { m + 1 },
        1,
    ) - days_from_civil(y, m, 1)
}

struct Parts {
    year: i64,
    month: i64,
    day: i64,
    hour: i64,
    minute: i64,
    second: i64,
    millis: i64,
    weekday: i64,
}

fn parts(ms: i64) -> Parts {
    let days = ms.div_euclid(DAY_MS);
    let rest = ms.rem_euclid(DAY_MS);
    let (year, month, day) = civil_from_days(days);
    Parts {
        year,
        month,
        day,
        hour: rest / 3_600_000,
        minute: rest / 60_000 % 60,
        second: rest / 1000 % 60,
        millis: rest % 1000,
        // ISO: Monday = 1 ... Sunday = 7. 1970-01-01 was a Thursday.
        weekday: (days + 3).rem_euclid(7) + 1,
    }
}

pub(crate) fn format_time(ms: i64) -> String {
    let p = parts(ms);
    let mut s = format!(
        "{:04}-{:02}-{:02}T{:02}:{:02}:{:02}",
        p.year, p.month, p.day, p.hour, p.minute, p.second
    );
    if p.millis != 0 {
        s.push_str(&format!(".{:03}", p.millis));
    }
    s.push('Z');
    s
}

fn digits(s: &str, n: usize) -> Option<i64> {
    if s.len() == n && s.bytes().all(|b| b.is_ascii_digit()) {
        s.parse().ok()
    } else {
        None
    }
}

/// Parses `YYYY-MM-DD`, optionally followed by `THH:MM[:SS[.fff]]` and a
/// zone (`Z` or `+HH:MM` / `-HH:MM`; none means UTC).
pub(crate) fn parse_time(text: &str) -> Option<i64> {
    let text = text.trim();
    let (date, rest) = match text.find(['T', ' ']) {
        Some(i) => (&text[..i], &text[i + 1..]),
        None => (text, ""),
    };
    let mut d = date.split('-');
    let year = digits(d.next()?, 4)?;
    let month = digits(d.next()?, 2)?;
    let day = digits(d.next()?, 2)?;
    if d.next().is_some()
        || !(1..=12).contains(&month)
        || day < 1
        || day > days_in_month(year, month)
    {
        return None;
    }
    let (mut hour, mut minute, mut second, mut millis, mut offset) = (0, 0, 0, 0, 0);
    if !rest.is_empty() {
        let (clock, zone) = match rest.find(['Z', '+', '-']) {
            Some(i) => (&rest[..i], &rest[i..]),
            None => (rest, ""),
        };
        let (clock, frac) = match clock.split_once('.') {
            Some((c, f)) => (c, Some(f)),
            None => (clock, None),
        };
        let mut c = clock.split(':');
        hour = digits(c.next()?, 2)?;
        minute = digits(c.next()?, 2)?;
        if let Some(sec) = c.next() {
            second = digits(sec, 2)?;
        }
        if c.next().is_some() || hour > 23 || minute > 59 || second > 59 {
            return None;
        }
        if let Some(f) = frac {
            if f.is_empty() || f.len() > 9 || !f.bytes().all(|b| b.is_ascii_digit()) {
                return None;
            }
            let padded = format!("{f:0<3}");
            millis = padded[..3].parse().ok()?;
        }
        if !zone.is_empty() && zone != "Z" {
            let sign = if zone.starts_with('-') { -1 } else { 1 };
            let (h, m) = zone[1..].split_once(':')?;
            let (h, m) = (digits(h, 2)?, digits(m, 2)?);
            if h > 23 || m > 59 {
                return None;
            }
            offset = sign * (h * 60 + m) * 60_000;
        }
    }
    let days = days_from_civil(year, month, day);
    Some(days * DAY_MS + hour * 3_600_000 + minute * 60_000 + second * 1000 + millis - offset)
}

fn now_ms() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as i64)
        .unwrap_or(0)
}

fn record(fields: Vec<(&str, Value)>) -> Value {
    Value::Record(Rc::new(
        fields.into_iter().map(|(n, v)| (Rc::from(n), v)).collect(),
    ))
}

fn time(func: &str, args: &[Value], span: Span) -> R<Value> {
    Ok(match func {
        "now" => Value::Int(now_ms()),
        "today" => Value::str(format_time(now_ms())[..10].to_string()),
        "sleep" => {
            if let Value::Duration(ms) = &args[0] {
                std::thread::sleep(Duration::from_millis((*ms).max(0) as u64));
            }
            Value::Unit
        }
        "format" => Value::str(format_time(int(&args[0]))),
        "parse" => match parse_time(text(&args[0])) {
            Some(ms) => Value::some(Value::Int(ms)),
            None => Value::none(),
        },
        "date" => {
            let p = parts(int(&args[0]));
            record(vec![
                ("day", Value::Int(p.day)),
                ("hour", Value::Int(p.hour)),
                ("minute", Value::Int(p.minute)),
                ("month", Value::Int(p.month)),
                ("second", Value::Int(p.second)),
                ("weekday", Value::Int(p.weekday)),
                ("year", Value::Int(p.year)),
            ])
        }
        "make" => {
            let v: Vec<i64> = args.iter().map(int).collect();
            let get = |i: usize| v.get(i).copied().unwrap_or(0);
            let (y, mo, d, h, mi, s) = (get(0), get(1), get(2), get(3), get(4), get(5));
            let valid = (1..=12).contains(&mo)
                && (1..=days_in_month(y, mo)).contains(&d)
                && (0..24).contains(&h)
                && (0..60).contains(&mi)
                && (0..60).contains(&s)
                && (-262_000..=262_000).contains(&y);
            if !valid {
                return Err(invalid(
                    span,
                    format!(
                        "`time.make`: {y}-{mo:02}-{d:02} {h:02}:{mi:02}:{s:02} is not a valid date and time"
                    ),
                    format!(
                        "`time.make`: {y}-{mo:02}-{d:02} {h:02}:{mi:02}:{s:02} ليس تاريخًا ووقتًا صالحين"
                    ),
                ));
            }
            Value::Int(days_from_civil(y, mo, d) * DAY_MS + h * 3_600_000 + mi * 60_000 + s * 1000)
        }
        other => unreachable!("checked time function `{other}`"),
    })
}

fn text(v: &Value) -> &str {
    match v {
        Value::Str(s) => s,
        _ => "",
    }
}

// ----- os ------------------------------------------------------------------

fn os(func: &str, args: &[Value], host: &mut dyn Host) -> R<Value> {
    Ok(match func {
        "args" => Value::list(host.args().into_iter().map(Value::str).collect()),
        "env" => match std::env::var(text(&args[0])) {
            Ok(v) => Value::some(Value::str(v)),
            Err(_) => Value::none(),
        },
        "platform" => Value::str(match std::env::consts::OS {
            "macos" => "macos",
            "windows" => "windows",
            "linux" => "linux",
            other => other,
        }),
        "exit" => {
            let code = int(&args[0]).clamp(0, 255) as i32;
            return Err(RuntimeError::exit(code));
        }
        other => unreachable!("checked os function `{other}`"),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn calendar_round_trips() {
        for days in [-1_000_000, -719_468, -1, 0, 1, 10_957, 20_000, 2_932_896] {
            let (y, m, d) = civil_from_days(days);
            assert_eq!(days_from_civil(y, m, d), days, "{y}-{m}-{d}");
        }
        assert_eq!(format_time(0), "1970-01-01T00:00:00Z");
        assert_eq!(format_time(951_782_400_000), "2000-02-29T00:00:00Z");
        assert_eq!(format_time(-1), "1969-12-31T23:59:59.999Z");
        assert_eq!(parse_time("2000-02-29"), Some(951_782_400_000));
        assert_eq!(parse_time("2001-02-29"), None);
        assert_eq!(
            parse_time("2026-11-20T12:30:05.5+03:00"),
            parse_time("2026-11-20T09:30:05.500Z")
        );
        assert_eq!(
            parse_time("2026-11-20 09:30"),
            parse_time("2026-11-20T09:30:00Z")
        );
        assert_eq!(parse_time("20-11-2026"), None);
        assert_eq!(parts(0).weekday, 4);
    }
}
