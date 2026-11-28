//! `std.server`: a small HTTP/1.1 server for web APIs and pages.
//!
//! - It listens only on addresses listed in `[permissions] listen`.
//! - Requests are handled one at a time by calling the program's handler,
//!   so handlers never run concurrently (no shared-state races).
//! - Each connection carries one request (`Connection: close`). Bodies need
//!   `Content-Length` (chunked uploads get 411), and size and time limits
//!   protect the program from slow or huge requests.
//! - An error inside the handler becomes a 500 response; the error is
//!   printed and the server keeps running.

use std::collections::BTreeMap;
use std::io::{BufRead, BufReader, Read, Write};
use std::net::{TcpListener, TcpStream};
use std::rc::Rc;
use std::time::Duration;

use elomar_ast::Span;
use elomar_diagnostics::Text;

use crate::value::Function;
use crate::{Exec, Flow, Interpreter, MapKey, RuntimeError, Value};

const MAX_HEADER_BYTES: usize = 64 * 1024;
const MAX_HEADERS: usize = 100;
const MAX_BODY_BYTES: usize = 10 * 1024 * 1024;
const IO_TIMEOUT: Duration = Duration::from_secs(10);

fn error(kind: &str, message: String) -> Value {
    Value::Result(Err(Rc::new(Value::enum_value(
        "HttpError",
        kind,
        vec![Value::str(message)],
    ))))
}

fn text(v: &Value) -> &str {
    match v {
        Value::Str(s) => s,
        _ => "",
    }
}

fn text_map(pairs: Vec<(String, String)>) -> Value {
    let map: BTreeMap<MapKey, Value> = pairs
        .into_iter()
        .map(|(k, v)| (MapKey::from_value(&Value::str(k)), Value::str(v)))
        .collect();
    Value::Map(Rc::new(map))
}

fn record(fields: Vec<(&str, Value)>) -> Value {
    Value::Record(Rc::new(
        fields.into_iter().map(|(n, v)| (Rc::from(n), v)).collect(),
    ))
}

/// A response record, as `server.text` and the other helpers build it.
pub(crate) fn reply(status: i64, content_type: &str, body: String) -> Value {
    record(vec![
        ("status", Value::Int(status)),
        (
            "headers",
            text_map(vec![("Content-Type".into(), content_type.into())]),
        ),
        ("body", Value::str(body)),
    ])
}

/// A field of a request or response record.
fn field<'a>(v: &'a Value, name: &str) -> Option<&'a Value> {
    match v {
        Value::Record(fields) => fields.iter().find(|(k, _)| &**k == name).map(|(_, v)| v),
        _ => None,
    }
}

/// `name=value` pairs separated by `;`, as a browser sends cookies.
fn parse_cookies(header: &str) -> Vec<(String, String)> {
    header
        .split(';')
        .filter_map(|pair| {
            let (name, value) = pair.split_once('=')?;
            let name = name.trim();
            let value = value.trim().trim_matches('"');
            let value = percent_encoding::percent_decode_str(value)
                .decode_utf8()
                .ok()?;
            (!name.is_empty()).then(|| (name.to_string(), value.into_owned()))
        })
        .collect()
}

/// Characters a cookie value may hold as they are; others are encoded.
const COOKIE_VALUE: &percent_encoding::AsciiSet = &percent_encoding::CONTROLS
    .add(b' ')
    .add(b'"')
    .add(b',')
    .add(b';')
    .add(b'\\')
    .add(b'%');

/// `server.with_cookie(reply, name, value)`: adds a `Set-Cookie` header,
/// HttpOnly and SameSite=Lax; an empty value deletes the cookie. Several
/// cookies on one reply are kept on separate lines of the one header.
fn with_cookie(args: &[Value], span: Span) -> Result<Value, RuntimeError> {
    let name = text(&args[1]);
    let token = |c: char| c.is_ascii_alphanumeric() || "!#$&'*+-.^_`|~".contains(c);
    if name.is_empty() || !name.chars().all(token) {
        return Err(RuntimeError::new(
            "R0006",
            span,
            Text::new(
                format!("`{name}` is not a valid cookie name (use letters, digits, `-` and `_`)"),
                format!(
                    "`{name}` ليس اسمًا صالحًا لملف ارتباط (استخدم الحروف اللاتينية والأرقام و`-` و`_`)"
                ),
            ),
        ));
    }
    let value = text(&args[2]);
    let mut cookie = format!(
        "{name}={}; Path=/; HttpOnly; SameSite=Lax",
        percent_encoding::utf8_percent_encode(value, COOKIE_VALUE)
    );
    if value.is_empty() {
        cookie.push_str("; Max-Age=0");
    }
    let Value::Record(fields) = &args[0] else {
        unreachable!("checked reply")
    };
    let mut fields = (**fields).clone();
    for (k, v) in fields.iter_mut() {
        if &**k == "headers"
            && let Value::Map(m) = v
        {
            let mut m = (**m).clone();
            let key = MapKey::from_value(&Value::str("Set-Cookie"));
            let line = match m.get(&key) {
                Some(Value::Str(old)) if !old.is_empty() => format!("{old}\n{cookie}"),
                _ => cookie.clone(),
            };
            m.insert(key, Value::str(line));
            *v = Value::Map(Rc::new(m));
        }
    }
    Ok(Value::Record(Rc::new(fields)))
}

/// `server.text`, `html`, `json`, `redirect`, `form`, `cookies` and
/// `with_cookie`.
pub(crate) fn helper(func: &str, args: &[Value], span: Span) -> Result<Value, RuntimeError> {
    let status = match args.first() {
        Some(Value::Int(n)) => *n,
        _ => 200,
    };
    Ok(match func {
        "form" => text_map(
            url::form_urlencoded::parse(field(&args[0], "body").map_or("", text).as_bytes())
                .into_owned()
                .collect(),
        ),
        "cookies" => {
            let header = match field(&args[0], "headers") {
                Some(Value::Map(m)) => m
                    .iter()
                    .find(|(k, _)| text(&k.to_value()).eq_ignore_ascii_case("cookie"))
                    .map(|(_, v)| text(v).to_string())
                    .unwrap_or_default(),
                _ => String::new(),
            };
            text_map(parse_cookies(&header))
        }
        "with_cookie" => return with_cookie(args, span),
        "text" => reply(status, "text/plain; charset=utf-8", text(&args[1]).into()),
        "html" => reply(status, "text/html; charset=utf-8", text(&args[1]).into()),
        "json" => reply(
            status,
            "application/json",
            crate::ai::to_json(&args[1]).to_string(),
        ),
        _ => record(vec![
            ("status", Value::Int(303)),
            (
                "headers",
                text_map(vec![("Location".into(), text(&args[0]).into())]),
            ),
            ("body", Value::str("")),
        ]),
    })
}

struct Request {
    method: String,
    path: String,
    query: Vec<(String, String)>,
    headers: Vec<(String, String)>,
    body: String,
}

/// Why a request could not be read: the status to answer with.
struct Reject(u16, &'static str);

fn read_request(stream: &TcpStream) -> Result<Request, Reject> {
    let mut reader = BufReader::new(stream.take((MAX_HEADER_BYTES + MAX_BODY_BYTES) as u64 + 1));
    let mut head_bytes = 0;
    let mut line = String::new();
    let mut read_line = |reader: &mut BufReader<_>, line: &mut String| -> Result<(), Reject> {
        line.clear();
        let n = reader
            .read_line(line)
            .map_err(|_| Reject(400, "could not read the request"))?;
        head_bytes += n;
        if head_bytes > MAX_HEADER_BYTES {
            return Err(Reject(431, "request headers are too large"));
        }
        if n == 0 {
            return Err(Reject(400, "incomplete request"));
        }
        Ok(())
    };
    read_line(&mut reader, &mut line)?;
    let mut parts = line.split_whitespace();
    let (Some(method), Some(target), Some(version)) = (parts.next(), parts.next(), parts.next())
    else {
        return Err(Reject(400, "malformed request line"));
    };
    if !version.starts_with("HTTP/1.") {
        return Err(Reject(505, "only HTTP/1.x is supported"));
    }
    let method = method.to_ascii_uppercase();
    let target = target.to_string();
    let mut headers: Vec<(String, String)> = Vec::new();
    loop {
        read_line(&mut reader, &mut line)?;
        let trimmed = line.trim_end_matches(['\r', '\n']);
        if trimmed.is_empty() {
            break;
        }
        if headers.len() >= MAX_HEADERS {
            return Err(Reject(431, "too many request headers"));
        }
        let Some((name, value)) = trimmed.split_once(':') else {
            return Err(Reject(400, "malformed header"));
        };
        headers.push((name.trim().to_ascii_lowercase(), value.trim().to_string()));
    }
    let header = |name: &str| {
        headers
            .iter()
            .find(|(n, _)| n == name)
            .map(|(_, v)| v.as_str())
    };
    if header("transfer-encoding").is_some() {
        return Err(Reject(411, "send the body with a Content-Length header"));
    }
    let length = match header("content-length") {
        Some(v) => v
            .parse::<usize>()
            .map_err(|_| Reject(400, "invalid Content-Length"))?,
        None => 0,
    };
    if length > MAX_BODY_BYTES {
        return Err(Reject(413, "the request body is too large"));
    }
    let mut body = vec![0u8; length];
    reader
        .read_exact(&mut body)
        .map_err(|_| Reject(400, "incomplete request body"))?;
    let body = String::from_utf8(body).map_err(|_| Reject(415, "the body must be UTF-8 text"))?;

    let (raw_path, raw_query) = match target.split_once('?') {
        Some((p, q)) => (p, q),
        None => (target.as_str(), ""),
    };
    let path = percent_encoding::percent_decode_str(raw_path)
        .decode_utf8()
        .map_err(|_| Reject(400, "the path is not UTF-8"))?
        .into_owned();
    let query = url::form_urlencoded::parse(raw_query.as_bytes())
        .into_owned()
        .collect();
    Ok(Request {
        method,
        path,
        query,
        headers,
        body,
    })
}

fn reason(status: u16) -> &'static str {
    match status {
        200 => "OK",
        201 => "Created",
        204 => "No Content",
        301 => "Moved Permanently",
        302 => "Found",
        303 => "See Other",
        304 => "Not Modified",
        307 => "Temporary Redirect",
        308 => "Permanent Redirect",
        400 => "Bad Request",
        401 => "Unauthorized",
        403 => "Forbidden",
        404 => "Not Found",
        405 => "Method Not Allowed",
        409 => "Conflict",
        411 => "Length Required",
        413 => "Content Too Large",
        415 => "Unsupported Media Type",
        422 => "Unprocessable Content",
        429 => "Too Many Requests",
        431 => "Request Header Fields Too Large",
        500 => "Internal Server Error",
        501 => "Not Implemented",
        503 => "Service Unavailable",
        505 => "HTTP Version Not Supported",
        _ => "",
    }
}

fn write_response(
    mut stream: &TcpStream,
    status: u16,
    headers: &[(String, String)],
    body: &str,
    head_only: bool,
) {
    let mut out = format!("HTTP/1.1 {status} {}\r\n", reason(status));
    let mut has_type = false;
    for (name, value) in headers {
        let lower = name.to_ascii_lowercase();
        // Several cookies: one `Set-Cookie` line each.
        if lower == "set-cookie" && value.contains('\n') {
            for line in value
                .split('\n')
                .filter(|l| !l.is_empty() && !l.contains('\r'))
            {
                out.push_str(&format!("{name}: {line}\r\n"));
            }
            continue;
        }
        // Header injection: a value may not start a new line.
        if name.is_empty() || name.contains([':', '\r', '\n']) || value.contains(['\r', '\n']) {
            continue;
        }
        if lower == "content-length" || lower == "connection" {
            continue;
        }
        has_type |= lower == "content-type";
        out.push_str(&format!("{name}: {value}\r\n"));
    }
    if !has_type {
        out.push_str("Content-Type: text/plain; charset=utf-8\r\n");
    }
    out.push_str(&format!(
        "Content-Length: {}\r\nConnection: close\r\n\r\n",
        body.len()
    ));
    let _ = stream.write_all(out.as_bytes());
    if !head_only {
        let _ = stream.write_all(body.as_bytes());
    }
    let _ = stream.flush();
}

impl Interpreter<'_, '_> {
    /// `server.serve(address, handler[, max_requests])`. Returns when
    /// `max_requests` (if given and above 0) have been answered.
    pub(crate) fn serve(&mut self, args: Vec<Value>, span: Span) -> Exec<Value> {
        let address = text(&args[0]).to_string();
        let Some(Value::Function(handler)) = args.get(1) else {
            unreachable!("checked handler")
        };
        let handler: Rc<Function> = handler.clone();
        let limit = match args.get(2) {
            Some(Value::Int(n)) if *n > 0 => Some(*n as u64),
            _ => None,
        };
        if !self.ai.permissions().allows_listen(&address) {
            return Ok(error(
                "PolicyDenied",
                format!(
                    "listening on `{address}` is not permitted; add \"{address}\" to `listen` under `[permissions]` in elomar.toml"
                ),
            ));
        }
        let listener = match TcpListener::bind(&address) {
            Ok(l) => l,
            Err(e) => {
                return Ok(error(
                    "Network",
                    format!("cannot listen on `{address}`: {e}"),
                ));
            }
        };
        let mut served = 0u64;
        for incoming in listener.incoming() {
            let Ok(stream) = incoming else { continue };
            let _ = stream.set_read_timeout(Some(IO_TIMEOUT));
            let _ = stream.set_write_timeout(Some(IO_TIMEOUT));
            match read_request(&stream) {
                Err(Reject(status, why)) => {
                    write_response(&stream, status, &[], why, false);
                }
                Ok(req) => {
                    let head_only = req.method == "HEAD";
                    let label = format!("{} {}", req.method, req.path);
                    let value = record(vec![
                        ("method", Value::str(req.method)),
                        ("path", Value::str(req.path)),
                        ("query", text_map(req.query)),
                        ("headers", text_map(req.headers)),
                        ("body", Value::str(req.body)),
                    ]);
                    match self.call_value(&handler, vec![value], span) {
                        Ok(Value::Record(fields)) => {
                            let field = |n: &str| {
                                fields
                                    .iter()
                                    .find(|(k, _)| &**k == n)
                                    .map(|(_, v)| v.clone())
                            };
                            let status = match field("status") {
                                Some(Value::Int(s)) if (100..=599).contains(&s) => s as u16,
                                _ => 500,
                            };
                            let headers: Vec<(String, String)> = match field("headers") {
                                Some(Value::Map(m)) => m
                                    .iter()
                                    .map(|(k, v)| (text(&k.to_value()).to_string(), v.to_string()))
                                    .collect(),
                                _ => Vec::new(),
                            };
                            let body = field("body").map(|b| b.to_string()).unwrap_or_default();
                            write_response(&stream, status, &headers, &body, head_only);
                        }
                        Ok(_) => write_response(&stream, 500, &[], "Internal Server Error", false),
                        Err(Flow::Error(e)) if e.exit.is_some() => {
                            write_response(&stream, 503, &[], "Service Unavailable", false);
                            return Err(Flow::Error(e));
                        }
                        Err(Flow::Error(e)) => {
                            self.host.print(&format!(
                                "error while handling {label}: {}\n",
                                e.message.en
                            ));
                            write_response(&stream, 500, &[], "Internal Server Error", false);
                        }
                        Err(other) => return Err(other),
                    }
                }
            }
            served += 1;
            if limit.is_some_and(|max| served >= max) {
                break;
            }
        }
        Ok(Value::Result(Ok(Rc::new(Value::Unit))))
    }
}
