//! End-to-end tests through the `elomar` binary:
//! - every `examples/*/main.elo` must run and print `expected.out`;
//! - every `tests/fail/*.elo` must fail with the codes on its `// expect:` line.

use std::path::{Path, PathBuf};
use std::process::Command;

fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn elomar(args: &[&str]) -> std::process::Output {
    Command::new(env!("CARGO_BIN_EXE_elomar"))
        .args(["--no-color", "--lang", "en"])
        .args(args)
        .output()
        .expect("run elomar")
}

/// Normalizes line endings so expected files work on every platform.
fn lines(s: &str) -> String {
    s.replace("\r\n", "\n")
}

#[test]
fn examples_produce_expected_output() {
    let mut count = 0;
    for entry in std::fs::read_dir(root().join("examples")).unwrap() {
        let dir = entry.unwrap().path();
        let main = dir.join("main.elo");
        let expected = dir.join("expected.out");
        if !main.exists() || !expected.exists() {
            continue;
        }
        let out = elomar(&["run", main.to_str().unwrap()]);
        let stdout = String::from_utf8_lossy(&out.stdout);
        let stderr = String::from_utf8_lossy(&out.stderr);
        assert!(out.status.success(), "{} failed:\n{stderr}", main.display());
        assert_eq!(
            lines(&stdout),
            lines(&std::fs::read_to_string(&expected).unwrap()),
            "output of {}",
            main.display()
        );
        count += 1;
    }
    assert!(count >= 5, "expected at least 5 examples, found {count}");
}

#[test]
fn failing_programs_report_expected_codes() {
    let mut count = 0;
    for entry in std::fs::read_dir(root().join("tests/fail")).unwrap() {
        let path = entry.unwrap().path();
        let src = std::fs::read_to_string(&path).unwrap();
        let expected: Vec<&str> = src
            .lines()
            .next()
            .and_then(|l| l.strip_prefix("// expect:"))
            .expect("first line must be `// expect: CODE...`")
            .split_whitespace()
            .collect();
        let out = elomar(&["run", path.to_str().unwrap()]);
        let stderr = String::from_utf8_lossy(&out.stderr);
        let codes: Vec<&str> = stderr
            .lines()
            .filter_map(|l| l.strip_prefix("error["))
            .filter_map(|l| l.split(']').next())
            .collect();
        assert!(!out.status.success(), "{} should fail", path.display());
        assert_eq!(codes, expected, "codes for {}:\n{stderr}", path.display());
        count += 1;
    }
    assert!(count >= 10);
}

#[test]
fn check_does_not_run_the_program() {
    let main = root().join("examples/hello/main.elo");
    let out = elomar(&["check", main.to_str().unwrap()]);
    assert!(out.status.success());
    assert!(out.stdout.is_empty());
    assert!(String::from_utf8_lossy(&out.stderr).contains("no errors"));
}

#[test]
fn diagnostics_in_arabic() {
    let path = root().join("tests/fail/type_mismatch.elo");
    let out = Command::new(env!("CARGO_BIN_EXE_elomar"))
        .args([
            "--no-color",
            "--lang",
            "ar",
            "check",
            path.to_str().unwrap(),
        ])
        .output()
        .unwrap();
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(stderr.contains("خطأ[E2001]: عدم توافق الأنواع"), "{stderr}");
    assert!(stderr.contains("type_mismatch.elo:3:22"), "{stderr}");
}

#[test]
fn version_and_missing_file() {
    let out = elomar(&["--version"]);
    assert!(String::from_utf8_lossy(&out.stdout).contains("spec 0.1.0-draft.2"));
    let out = elomar(&["run", "does-not-exist.elo"]);
    assert_eq!(out.status.code(), Some(2));
}

#[test]
fn invalid_config_is_reported() {
    let main = root().join("tests/config-bad/main.elo");
    let out = elomar(&["run", main.to_str().unwrap()]);
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert_eq!(out.status.code(), Some(2), "{stderr}");
    assert!(stderr.contains("unknown error kind `Oops`"), "{stderr}");
    assert!(out.stdout.is_empty());
}

// ----- OpenAI provider, end to end against a local fake server -------------

mod fake_openai {
    use std::io::{Read, Write};
    use std::net::TcpListener;
    use std::path::{Path, PathBuf};

    /// Answers every request with `content` as the assistant message, until
    /// the test process exits. Returns the port.
    pub fn start(content: &'static str) -> u16 {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let port = listener.local_addr().unwrap().port();
        std::thread::spawn(move || {
            for stream in listener.incoming() {
                let Ok(mut stream) = stream else { continue };
                let mut raw = Vec::new();
                let mut buf = [0u8; 8192];
                while let Ok(n) = stream.read(&mut buf) {
                    if n == 0 {
                        break;
                    }
                    raw.extend_from_slice(&buf[..n]);
                    let text = String::from_utf8_lossy(&raw);
                    if let Some(end) = text.find("\r\n\r\n") {
                        let len = text[..end]
                            .lines()
                            .find_map(|l| {
                                l.to_ascii_lowercase()
                                    .strip_prefix("content-length:")
                                    .map(|v| v.trim().parse::<usize>().unwrap_or(0))
                            })
                            .unwrap_or(0);
                        if raw.len() >= end + 4 + len {
                            break;
                        }
                    }
                }
                let authorized = String::from_utf8_lossy(&raw).contains("Bearer sk-test-key");
                let (status, body) = if authorized {
                    let body = format!(
                        r#"{{"choices":[{{"message":{{"role":"assistant","content":{}}},"finish_reason":"stop"}}]}}"#,
                        serde_json_string(content)
                    );
                    (200, body)
                } else {
                    (
                        401,
                        r#"{"error":{"message":"bad key","code":"invalid_api_key"}}"#.to_string(),
                    )
                };
                let _ = stream.write_all(
                    format!(
                        "HTTP/1.1 {status} X\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
                        body.len()
                    )
                    .as_bytes(),
                );
            }
        });
        port
    }

    fn serde_json_string(s: &str) -> String {
        format!("\"{}\"", s.replace('\\', "\\\\").replace('"', "\\\""))
    }

    /// Writes a project with an OpenAI model pointing at the fake server.
    pub fn project(name: &str, port: u16, allow: bool, effects: &str) -> PathBuf {
        let dir = Path::new(env!("CARGO_TARGET_TMPDIR")).join(name);
        std::fs::create_dir_all(&dir).unwrap();
        let hosts = if allow {
            format!("network_hosts = [\"127.0.0.1:{port}\"]")
        } else {
            "network_hosts = []".to_string()
        };
        std::fs::write(
            dir.join("elomar.toml"),
            format!(
                "[ai.models.assistant]\nprovider = \"openai\"\nmodel = \"test-model\"\nbase_url = \"http://127.0.0.1:{port}/v1\"\napi_key_env = \"ELOMAR_TEST_OPENAI_KEY\"\n\n[permissions]\n{hosts}\n"
            ),
        )
        .unwrap();
        std::fs::write(
            dir.join("main.elo"),
            format!(
                "use std.ai\n\ntype Answer = {{ label: String, confidence: Float }}\n\npub async fn label(text: String) -> Result<Answer, AIError> {effects} {{\n    return await ai.infer<Answer>(\"assistant\", \"Label the text.\", text, timeout: 5s)\n}}\n\nasync fn main() {{\n    match await label(\"hello\") {{\n        Ok(a) => print(\"label:\", a.label, a.confidence)\n        Err(e) => print(\"error:\", e.kind())\n    }}\n}}\n"
            ),
        )
        .unwrap();
        dir.join("main.elo")
    }
}

fn run_with_key(main: &Path, key: Option<&str>) -> std::process::Output {
    let mut cmd = Command::new(env!("CARGO_BIN_EXE_elomar"));
    cmd.args(["--no-color", "run", main.to_str().unwrap()]);
    match key {
        Some(k) => cmd.env("ELOMAR_TEST_OPENAI_KEY", k),
        None => cmd.env_remove("ELOMAR_TEST_OPENAI_KEY"),
    };
    cmd.output().unwrap()
}

#[test]
fn openai_end_to_end() {
    let port = fake_openai::start(r#"{"label": "greeting", "confidence": 0.75}"#);

    let ok = fake_openai::project("openai-ok", port, true, "!{ai, net}");
    let out = run_with_key(&ok, Some("sk-test-key"));
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(out.status.success(), "{stderr}");
    assert_eq!(
        lines(&String::from_utf8_lossy(&out.stdout)),
        "label: greeting 0.75\n"
    );

    let out = run_with_key(&ok, None);
    assert_eq!(
        lines(&String::from_utf8_lossy(&out.stdout)),
        "error: Authentication\n"
    );

    let out = run_with_key(&ok, Some("sk-wrong"));
    assert_eq!(
        lines(&String::from_utf8_lossy(&out.stdout)),
        "error: Authentication\n"
    );

    let denied = fake_openai::project("openai-denied", port, false, "!{ai, net}");
    let out = run_with_key(&denied, Some("sk-test-key"));
    assert_eq!(
        lines(&String::from_utf8_lossy(&out.stdout)),
        "error: PolicyDenied\n"
    );
}

#[test]
fn cloud_models_require_the_net_effect() {
    let main = fake_openai::project("openai-no-net", 9, true, "!{ai}");
    let out = run_with_key(&main, Some("sk-test-key"));
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert_eq!(out.status.code(), Some(1), "{stderr}");
    assert!(
        stderr.contains("error[E3001]") && stderr.contains("effect `net`"),
        "{stderr}"
    );
}

#[test]
fn openai_config_errors() {
    let dir = Path::new(env!("CARGO_TARGET_TMPDIR")).join("openai-bad-config");
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::write(
        dir.join("elomar.toml"),
        "[ai.models.a]\nprovider = \"openai\"\nmodel = \"x\"\nbase_url = \"http://api.example.com/v1\"\n",
    )
    .unwrap();
    std::fs::write(dir.join("main.elo"), "fn main() {}\n").unwrap();
    let out = run_with_key(&dir.join("main.elo"), None);
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert_eq!(out.status.code(), Some(2), "{stderr}");
    assert!(stderr.contains("must use https"), "{stderr}");
}
