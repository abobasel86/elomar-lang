//! `elomar dap` driven as an editor would drive it.

use std::io::{BufRead, BufReader, Read, Write};
use std::process::{Child, ChildStdin, Command, Stdio};
use std::sync::mpsc::{Receiver, channel};
use std::time::Duration;

use serde_json::{Value, json};

const PROGRAM: &str = "fn square(n: Int) -> Int {
    let result = n * n
    return result
}

fn main() {
    let names = [\"Lina\", \"Omar\"]
    var total = 0
    for i in 1..=3 {
        total += square(i)
    }
    print(\"total\", total, names.length)
    let bad = 10 div (total - 14)
    print(bad)
}
";

struct Session {
    child: Child,
    stdin: ChildStdin,
    messages: Receiver<Value>,
    seq: i64,
}

impl Session {
    fn start(dir: &std::path::Path) -> Session {
        let mut child = Command::new(env!("CARGO_BIN_EXE_elomar"))
            .args(["--lang", "en", "dap"])
            .current_dir(dir)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .spawn()
            .expect("run elomar dap");
        let stdin = child.stdin.take().unwrap();
        let stdout = child.stdout.take().unwrap();
        let (tx, messages) = channel();
        std::thread::spawn(move || {
            let mut reader = BufReader::new(stdout);
            loop {
                let mut length = 0;
                loop {
                    let mut line = String::new();
                    if reader.read_line(&mut line).unwrap_or(0) == 0 {
                        return;
                    }
                    let line = line.trim_end();
                    if line.is_empty() {
                        break;
                    }
                    if let Some(v) = line.strip_prefix("Content-Length:") {
                        length = v.trim().parse().unwrap();
                    }
                }
                let mut body = vec![0; length];
                reader.read_exact(&mut body).unwrap();
                if tx.send(serde_json::from_slice(&body).unwrap()).is_err() {
                    return;
                }
            }
        });
        Session {
            child,
            stdin,
            messages,
            seq: 0,
        }
    }

    fn send(&mut self, command: &str, arguments: Value) -> i64 {
        self.seq += 1;
        let body = json!({ "seq": self.seq, "type": "request", "command": command, "arguments": arguments })
            .to_string();
        write!(self.stdin, "Content-Length: {}\r\n\r\n{}", body.len(), body).unwrap();
        self.stdin.flush().unwrap();
        self.seq
    }

    /// Waits for the first message `matches` accepts; collects `output`
    /// events seen meanwhile into `output`.
    fn wait(&self, output: &mut String, matches: impl Fn(&Value) -> bool) -> Value {
        loop {
            let m = self
                .messages
                .recv_timeout(Duration::from_secs(20))
                .expect("a message from the debugger");
            if m["event"] == "output" {
                output.push_str(m["body"]["output"].as_str().unwrap_or(""));
            }
            if matches(&m) {
                return m;
            }
        }
    }

    /// Sends a request and returns its response body.
    fn request(&mut self, output: &mut String, command: &str, arguments: Value) -> Value {
        let seq = self.send(command, arguments);
        let r = self.wait(output, |m| {
            m["type"] == "response" && m["request_seq"] == seq
        });
        assert_eq!(r["success"], true, "{r}");
        r["body"].clone()
    }

    fn stopped(&self, output: &mut String) -> Value {
        self.wait(output, |m| m["event"] == "stopped")["body"].clone()
    }

    /// (function name, line) of each frame, innermost first.
    fn stack(&mut self, output: &mut String) -> Vec<(String, u64)> {
        self.request(output, "stackTrace", json!({ "threadId": 1 }))["stackFrames"]
            .as_array()
            .unwrap()
            .iter()
            .map(|f| {
                (
                    f["name"].as_str().unwrap().to_string(),
                    f["line"].as_u64().unwrap(),
                )
            })
            .collect()
    }

    /// name → shown value of the innermost frame's variables.
    fn locals(&mut self, output: &mut String) -> Vec<(String, String, u64)> {
        let scopes = self.request(output, "scopes", json!({ "frameId": 1 }));
        let reference = scopes["scopes"][0]["variablesReference"].clone();
        self.children(output, reference)
    }

    fn children(&mut self, output: &mut String, reference: Value) -> Vec<(String, String, u64)> {
        self.request(
            output,
            "variables",
            json!({ "variablesReference": reference }),
        )["variables"]
            .as_array()
            .unwrap()
            .iter()
            .map(|v| {
                (
                    v["name"].as_str().unwrap().to_string(),
                    v["value"].as_str().unwrap().to_string(),
                    v["variablesReference"].as_u64().unwrap(),
                )
            })
            .collect()
    }
}

#[test]
fn breakpoints_steps_variables_and_errors() {
    let dir = std::env::temp_dir().join(format!("elomar-dap-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let main = dir.join("main.elo");
    std::fs::write(&main, PROGRAM).unwrap();
    let main_path = main.display().to_string();

    let mut s = Session::start(&dir);
    let mut out = String::new();
    s.request(&mut out, "initialize", json!({ "adapterID": "elomar" }));
    s.wait(&mut out, |m| m["event"] == "initialized");
    s.request(&mut out, "launch", json!({ "program": main_path }));
    let bps = s.request(
        &mut out,
        "setBreakpoints",
        json!({ "source": { "path": main_path }, "breakpoints": [{ "line": 10 }] }),
    );
    assert_eq!(bps["breakpoints"][0]["verified"], true);
    s.request(&mut out, "configurationDone", json!({}));

    // The breakpoint in the loop.
    assert_eq!(s.stopped(&mut out)["reason"], "breakpoint");
    assert_eq!(s.stack(&mut out), [("main".to_string(), 10)]);
    let locals = s.locals(&mut out);
    let names: Vec<&str> = locals.iter().map(|(n, _, _)| n.as_str()).collect();
    assert_eq!(names, ["names", "total", "i"]);
    assert_eq!(locals[1].1, "0");
    assert_eq!(locals[2].1, "1");
    // A list can be opened.
    assert!(locals[0].2 > 0);
    let items = s.children(&mut out, json!(locals[0].2));
    assert_eq!(items[0], ("[0]".to_string(), "\"Lina\"".to_string(), 0));
    // The Debug Console shows variables and fields.
    let shown = s.request(
        &mut out,
        "evaluate",
        json!({ "expression": "names.1", "frameId": 1 }),
    );
    assert_eq!(shown["result"], "\"Omar\"");

    // Into `square`, then back out.
    s.request(&mut out, "stepIn", json!({ "threadId": 1 }));
    assert_eq!(s.stopped(&mut out)["reason"], "step");
    assert_eq!(
        s.stack(&mut out),
        [("square".to_string(), 2), ("main".to_string(), 10)]
    );
    assert_eq!(s.locals(&mut out)[0], ("n".to_string(), "1".to_string(), 0));
    s.request(&mut out, "next", json!({ "threadId": 1 }));
    s.stopped(&mut out);
    assert_eq!(s.stack(&mut out)[0], ("square".to_string(), 3));
    s.request(&mut out, "stepOut", json!({ "threadId": 1 }));
    s.stopped(&mut out);
    assert_eq!(s.stack(&mut out), [("main".to_string(), 10)]);
    assert_eq!(s.locals(&mut out)[1].1, "1");

    // The breakpoint again, in the next round.
    s.request(&mut out, "continue", json!({ "threadId": 1 }));
    assert_eq!(s.stopped(&mut out)["reason"], "breakpoint");
    assert_eq!(s.locals(&mut out)[2].1, "3");

    // Without breakpoints, the program runs to the division by zero.
    s.request(
        &mut out,
        "setBreakpoints",
        json!({ "source": { "path": main_path }, "breakpoints": [] }),
    );
    s.request(&mut out, "continue", json!({ "threadId": 1 }));
    let stop = s.stopped(&mut out);
    assert_eq!(stop["reason"], "exception");
    assert!(stop["text"].as_str().unwrap().contains("R0001"), "{stop}");
    assert_eq!(s.stack(&mut out), [("main".to_string(), 13)]);
    assert!(out.contains("total 14 2\n"), "{out}");

    s.request(&mut out, "continue", json!({ "threadId": 1 }));
    let exited = s.wait(&mut out, |m| m["event"] == "exited");
    assert_eq!(exited["body"]["exitCode"], 1);
    s.wait(&mut out, |m| m["event"] == "terminated");
    assert!(out.contains("division by zero"), "{out}");

    s.send("disconnect", json!({}));
    let _ = s.child.wait();
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn stop_on_entry_and_program_errors() {
    let dir = std::env::temp_dir().join(format!("elomar-dap2-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let good = dir.join("good.elo");
    std::fs::write(&good, "fn main() {\n    print(1)\n}\n").unwrap();
    let bad = dir.join("bad.elo");
    std::fs::write(&bad, "fn main() {\n    print(nope)\n}\n").unwrap();

    let mut s = Session::start(&dir);
    let mut out = String::new();
    s.request(&mut out, "initialize", json!({}));
    s.request(
        &mut out,
        "launch",
        json!({ "program": good.display().to_string(), "stopOnEntry": true }),
    );
    s.request(&mut out, "configurationDone", json!({}));
    assert_eq!(s.stopped(&mut out)["reason"], "entry");
    assert_eq!(s.stack(&mut out), [("main".to_string(), 2)]);
    s.request(&mut out, "continue", json!({ "threadId": 1 }));
    let exited = s.wait(&mut out, |m| m["event"] == "exited");
    assert_eq!(exited["body"]["exitCode"], 0);
    assert_eq!(out, "1\n");
    s.send("disconnect", json!({}));
    let _ = s.child.wait();

    // A program with errors does not start; the errors are shown.
    let mut s = Session::start(&dir);
    let mut out = String::new();
    s.request(&mut out, "initialize", json!({}));
    let seq = s.send("launch", json!({ "program": bad.display().to_string() }));
    let r = s.wait(&mut out, |m| {
        m["type"] == "response" && m["request_seq"] == seq
    });
    assert_eq!(r["success"], false);
    assert!(out.contains("E1001"), "{out}");
    s.send("disconnect", json!({}));
    let _ = s.child.wait();
    let _ = std::fs::remove_dir_all(&dir);
}
