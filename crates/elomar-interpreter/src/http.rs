//! `std.http` at run time: `http.get` and `http.post`.
//!
//! Every request is checked against `[permissions] network_hosts` first.
//! Redirects are not followed (a redirect could lead to a host that is not
//! allowed); the program gets the 3xx response instead. Plain `http` is
//! allowed only for loopback hosts. Failures are `Err(HttpError...)`
//! values; an error status such as 404 is a normal response.

use std::rc::Rc;
use std::time::Duration;

use elomar_ai_core::Permissions;
use url::Url;

use crate::Value;

/// How long a request may take in total.
pub const TIMEOUT: Duration = Duration::from_secs(30);

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

/// Parses `url` and checks the scheme and the host allowlist.
fn checked_url(permissions: &Permissions, url: &str) -> Result<Url, Value> {
    let parsed =
        Url::parse(url).map_err(|e| error("InvalidUrl", format!("invalid URL `{url}`: {e}")))?;
    let Some(host) = parsed.host_str().map(str::to_ascii_lowercase) else {
        return Err(error("InvalidUrl", format!("the URL `{url}` has no host")));
    };
    let loopback = matches!(host.as_str(), "localhost" | "127.0.0.1" | "[::1]");
    match parsed.scheme() {
        "https" => {}
        "http" if loopback => {}
        "http" => {
            return Err(error(
                "InvalidUrl",
                format!("`{url}` uses plain http; use https (http is allowed only for localhost)"),
            ));
        }
        other => {
            return Err(error(
                "InvalidUrl",
                format!("unsupported URL scheme `{other}` in `{url}`; use https"),
            ));
        }
    }
    let port = parsed.port_or_known_default().unwrap_or(443);
    if !permissions.allows_host(&host, port) {
        return Err(error(
            "PolicyDenied",
            format!(
                "network access to `{host}` is not permitted; add \"{host}\" to `network_hosts` under `[permissions]` in elomar.toml"
            ),
        ));
    }
    Ok(parsed)
}

pub(crate) fn call(permissions: &Permissions, func: &str, args: &[Value]) -> Value {
    let url = match checked_url(permissions, text(&args[0])) {
        Ok(u) => u,
        Err(e) => return e,
    };
    let agent = elomar_tls::agent_builder()
        .redirects(0)
        .timeout(TIMEOUT)
        .user_agent(concat!("elomar-lang/", env!("CARGO_PKG_VERSION")))
        .build();
    let result = match func {
        "get" => agent.get(url.as_str()).call(),
        "post" => {
            let content_type = args.get(2).map_or("application/json", text);
            agent
                .post(url.as_str())
                .set("Content-Type", content_type)
                .send_string(text(&args[1]))
        }
        other => unreachable!("checked http function `{other}`"),
    };
    let response = match result {
        Ok(r) | Err(ureq::Error::Status(_, r)) => r,
        Err(ureq::Error::Transport(t)) => return transport_error(&t, url.as_str()),
    };
    let status = i64::from(response.status());
    match response.into_string() {
        Ok(body) => Value::Result(Ok(Rc::new(Value::Record(Rc::new(vec![
            ("status".into(), Value::Int(status)),
            ("body".into(), Value::str(body)),
        ]))))),
        Err(e) if e.kind() == std::io::ErrorKind::InvalidData => error(
            "TooLarge",
            format!("the response from `{url}` is larger than 10 MB or is not text"),
        ),
        Err(e) => transport_error(&e, url.as_str()),
    }
}

fn transport_error(e: &(dyn std::error::Error + 'static), url: &str) -> Value {
    let mut source: Option<&(dyn std::error::Error + 'static)> = Some(e);
    while let Some(err) = source {
        if let Some(io) = err.downcast_ref::<std::io::Error>()
            && matches!(
                io.kind(),
                std::io::ErrorKind::TimedOut | std::io::ErrorKind::WouldBlock
            )
        {
            return error(
                "Timeout",
                format!("no answer from `{url}` within {} s", TIMEOUT.as_secs()),
            );
        }
        source = err.source();
    }
    error("Network", format!("could not reach `{url}`: {e}"))
}
