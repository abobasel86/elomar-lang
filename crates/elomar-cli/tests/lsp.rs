//! A Language Server Protocol session with `elomar lsp`, over pipes.

use std::io::{BufRead, BufReader, Read, Write};
use std::process::{ChildStdin, ChildStdout, Command, Stdio};

use serde_json::{Value, json};

struct Session {
    stdin: ChildStdin,
    stdout: BufReader<ChildStdout>,
    next_id: i64,
    notifications: Vec<Value>,
}

impl Session {
    fn send(&mut self, message: Value) {
        let body = message.to_string();
        write!(self.stdin, "Content-Length: {}\r\n\r\n{body}", body.len()).unwrap();
        self.stdin.flush().unwrap();
    }

    fn receive(&mut self) -> Value {
        let mut length = 0;
        loop {
            let mut line = String::new();
            self.stdout.read_line(&mut line).unwrap();
            let line = line.trim_end();
            if line.is_empty() {
                break;
            }
            if let Some(v) = line.strip_prefix("Content-Length:") {
                length = v.trim().parse().unwrap();
            }
        }
        let mut body = vec![0; length];
        self.stdout.read_exact(&mut body).unwrap();
        serde_json::from_slice(&body).unwrap()
    }

    fn request(&mut self, method: &str, params: Value) -> Value {
        self.next_id += 1;
        let id = self.next_id;
        self.send(json!({ "jsonrpc": "2.0", "id": id, "method": method, "params": params }));
        loop {
            let message = self.receive();
            if message["id"] == id {
                return message["result"].clone();
            }
            self.notifications.push(message);
        }
    }

    fn notify(&mut self, method: &str, params: Value) {
        self.send(json!({ "jsonrpc": "2.0", "method": method, "params": params }));
    }
}

const SOURCE: &str = "type Student = { name: String, grades: List<Int> }

fn average(xs: List<Int>) -> Float {
    let total = xs.sum()
    return total / xs.length
}

fn main() {
    let students: List<Student> = [{ name: \"Lina\", grades: [90, 80] }]
    for s in students {
        print(s.name, average(s.grades))
    }
    let bad: Int = \"x\"
}
";

/// The LSP position of the `nth` occurrence of `needle`, plus `delta`.
fn pos(needle: &str, nth: usize, delta: usize) -> Value {
    let idx = SOURCE.match_indices(needle).nth(nth).unwrap().0;
    let line = SOURCE[..idx].matches('\n').count();
    let col = idx - SOURCE[..idx].rfind('\n').map_or(0, |i| i + 1) + delta;
    json!({ "line": line, "character": col })
}

#[test]
fn language_server_session() {
    let dir = std::env::temp_dir().join(format!("elomar-lsp-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let file = dir.join("main.elo");
    std::fs::write(&file, SOURCE).unwrap();
    let uri = format!(
        "file:///{}",
        file.to_string_lossy()
            .replace('\\', "/")
            .trim_start_matches('/')
    );

    let mut child = Command::new(env!("CARGO_BIN_EXE_elomar"))
        .args(["--lang", "en", "lsp"])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()
        .unwrap();
    let mut s = Session {
        stdin: child.stdin.take().unwrap(),
        stdout: BufReader::new(child.stdout.take().unwrap()),
        next_id: 0,
        notifications: Vec::new(),
    };

    let init = s.request("initialize", json!({ "capabilities": {} }));
    assert_eq!(init["capabilities"]["hoverProvider"], true);
    s.notify("initialized", json!({}));
    s.notify(
        "textDocument/didOpen",
        json!({ "textDocument": { "uri": uri, "languageId": "elomar", "version": 1, "text": SOURCE } }),
    );
    let doc = json!({ "uri": uri });

    let hover = |s: &mut Session, needle: &str, nth: usize| {
        let r = s.request(
            "textDocument/hover",
            json!({ "textDocument": doc, "position": pos(needle, nth, 1) }),
        );
        r["contents"]["value"]
            .as_str()
            .unwrap_or_default()
            .to_string()
    };
    assert!(hover(&mut s, "average(", 1).contains("fn average(xs: List<Int>) -> Float"));
    assert!(hover(&mut s, "total", 1).contains("let total: Int"));
    assert!(
        hover(&mut s, "Student", 1).contains("type Student = { grades: List<Int>, name: String }")
    );

    // The diagnostic for `bad` was published when the document opened.
    let published = s
        .notifications
        .iter()
        .find(|n| n["method"] == "textDocument/publishDiagnostics")
        .expect("diagnostics");
    assert_eq!(published["params"]["diagnostics"][0]["code"], "E2001");
    assert_eq!(
        published["params"]["diagnostics"][0]["range"]["start"]["line"],
        12
    );

    let def = s.request(
        "textDocument/definition",
        json!({ "textDocument": doc, "position": pos("average(", 1, 1) }),
    );
    assert_eq!(def["range"]["start"], json!({ "line": 2, "character": 3 }));

    let completion = s.request(
        "textDocument/completion",
        json!({ "textDocument": doc, "position": pos("s.name", 0, 2) }),
    );
    let labels: Vec<&str> = completion
        .as_array()
        .unwrap()
        .iter()
        .map(|c| c["label"].as_str().unwrap())
        .collect();
    assert!(
        labels.contains(&"name") && labels.contains(&"grades"),
        "{labels:?}"
    );

    let symbols = s.request(
        "textDocument/documentSymbol",
        json!({ "textDocument": doc }),
    );
    let names: Vec<&str> = symbols
        .as_array()
        .unwrap()
        .iter()
        .map(|c| c["name"].as_str().unwrap())
        .collect();
    assert_eq!(names, ["Student", "average", "main"]);

    // Editing the text re-checks it; fixing the error clears the diagnostic.
    let fixed = SOURCE.replace("let bad: Int = \"x\"", "let ok: Int = 1");
    s.notify(
        "textDocument/didChange",
        json!({ "textDocument": { "uri": uri, "version": 2 }, "contentChanges": [{ "text": fixed }] }),
    );
    let messy = fixed.replace("let ok: Int = 1", "let ok:Int=1");
    s.notify(
        "textDocument/didChange",
        json!({ "textDocument": { "uri": uri, "version": 3 }, "contentChanges": [{ "text": messy }] }),
    );
    let edits = s.request(
        "textDocument/formatting",
        json!({ "textDocument": doc, "options": {} }),
    );
    assert!(
        edits[0]["newText"]
            .as_str()
            .unwrap()
            .contains("let ok: Int = 1")
    );
    let cleared = s
        .notifications
        .iter()
        .rev()
        .find(|n| n["method"] == "textDocument/publishDiagnostics")
        .unwrap();
    assert_eq!(cleared["params"]["diagnostics"], json!([]));

    s.request("shutdown", Value::Null);
    s.notify("exit", Value::Null);
    assert!(child.wait().unwrap().success());
}
