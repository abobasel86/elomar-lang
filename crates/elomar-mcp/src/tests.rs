use std::path::PathBuf;
use std::time::Duration;

use serde_json::json;

use super::*;

fn python() -> String {
    std::env::var("ELOMAR_PYTHON")
        .ok()
        .filter(|p| !p.trim().is_empty())
        .unwrap_or_else(|| if cfg!(windows) { "python" } else { "python3" }.to_string())
}

fn fixture_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures")
}

fn start(mode: &str) -> Result<Client, McpError> {
    let launch = Launch {
        command: python(),
        args: vec!["server.py".into(), mode.into()],
        env: Vec::new(),
        timeout: Duration::from_secs(10),
    };
    Client::start("fixture", &launch, &fixture_dir())
}

fn exercise(mut c: Client) {
    let tools = c.list_tools().unwrap();
    let names: Vec<&str> = tools.iter().map(|t| t.name.as_str()).collect();
    assert_eq!(names, ["add", "fail", "ask"]);
    assert_eq!(tools[0].description, "Adds two numbers");
    assert_eq!(tools[0].input_schema["required"], json!(["a", "b"]));

    let r = c.call_tool("add", json!({"a": 2, "b": 3.5})).unwrap();
    assert_eq!(r.text, "5.5");
    assert!(!r.is_error);
    assert_eq!(r.structured, Some(json!({"sum": 5.5})));

    let r = c.call_tool("fail", json!({})).unwrap();
    assert!(r.is_error);
    assert_eq!(r.text, "it broke");

    let e = c.call_tool("nope", json!({})).unwrap_err();
    assert!(
        matches!(&e, McpError::Protocol(m) if m.contains("Unknown tool: nope")),
        "{e:?}"
    );
}

#[test]
fn modern_server() {
    let c = start("modern").unwrap();
    assert!(c.is_modern());
    exercise(c);
}

#[test]
fn legacy_server() {
    let c = start("legacy").unwrap();
    assert!(!c.is_modern());
    exercise(c);
}

#[test]
fn silent_legacy_server() {
    let c = start("silent-legacy").unwrap();
    assert!(!c.is_modern());
    exercise(c);
}

#[test]
fn input_required_is_an_error() {
    let mut c = start("modern").unwrap();
    let e = c.call_tool("ask", json!({})).unwrap_err();
    assert!(e.message().contains("asked for more input"), "{e:?}");
}

#[test]
fn crashed_server_explains() {
    let e = start("crash").err().unwrap();
    assert!(
        matches!(&e, McpError::Unavailable(m) if m.contains("cannot open the notes database")),
        "{e:?}"
    );
}

#[test]
fn missing_command() {
    let launch = Launch {
        command: "elomar-no-such-mcp-server".into(),
        ..Launch::default()
    };
    let e = Client::start("x", &launch, &fixture_dir()).err().unwrap();
    assert!(e.message().contains("cannot run"), "{e:?}");
}
