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
        // Single-file examples have `main.elo`; projects have `src/main.elo`.
        let main = [dir.join("main.elo"), dir.join("src/main.elo")]
            .into_iter()
            .find(|p| p.exists())
            .unwrap_or_else(|| dir.join("main.elo"));
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

// ----- Multi-file modules ----------------------------------------------------

/// Writes a project under the test temp folder and returns its main file.
fn module_project(name: &str, files: &[(&str, &str)]) -> PathBuf {
    let dir = Path::new(env!("CARGO_TARGET_TMPDIR")).join(name);
    let _ = std::fs::remove_dir_all(&dir);
    for (path, text) in files {
        let p = dir.join(path);
        std::fs::create_dir_all(p.parent().unwrap()).unwrap();
        std::fs::write(p, text).unwrap();
    }
    dir.join("src/main.elo")
}

fn check_codes(main: &Path) -> (Vec<String>, String) {
    let out = elomar(&["check", main.to_str().unwrap()]);
    let stderr = String::from_utf8_lossy(&out.stderr).into_owned();
    let codes = stderr
        .lines()
        .filter_map(|l| l.strip_prefix("error["))
        .filter_map(|l| l.split(']').next())
        .map(str::to_string)
        .collect();
    (codes, stderr)
}

const LIB: &str = "pub fn shout(s: String) -> String {\n    return s.upper() + suffix()\n}\n\nfn suffix() -> String {\n    return \"!\"\n}\n\npub enum Mood {\n    Happy\n    Sad\n}\n";

#[test]
fn modules_run_and_respect_pub() {
    let ok = module_project(
        "modules-ok",
        &[
            ("elomar.toml", ""),
            ("src/lib.elo", LIB),
            (
                "src/main.elo",
                "use app.lib\nfn main() {\n    print(lib.shout(\"hi\"), lib.Mood.Sad)\n    match lib.Mood.Happy {\n        lib.Mood.Happy => print(\"happy\")\n        lib.Mood.Sad => print(\"sad\")\n    }\n}\n",
            ),
        ],
    );
    let out = elomar(&["run", ok.to_str().unwrap()]);
    assert_eq!(
        lines(&String::from_utf8_lossy(&out.stdout)),
        "HI! lib.Mood.Sad\nhappy\n",
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );

    let private = module_project(
        "modules-private",
        &[
            ("elomar.toml", ""),
            ("src/lib.elo", LIB),
            (
                "src/main.elo",
                "use app.lib\nfn main() {\n    print(lib.suffix())\n}\n",
            ),
        ],
    );
    let (codes, stderr) = check_codes(&private);
    assert_eq!(codes, vec!["E1012"], "{stderr}");
    assert!(stderr.contains("mark it `pub` in src/lib.elo"), "{stderr}");
}

#[test]
fn module_errors() {
    let missing = module_project(
        "modules-missing",
        &[
            ("elomar.toml", ""),
            ("src/main.elo", "use app.nothing\nfn main() {}\n"),
        ],
    );
    let (codes, stderr) = check_codes(&missing);
    assert_eq!(codes, vec!["E1011"], "{stderr}");
    assert!(stderr.contains("src/nothing.elo"), "{stderr}");

    // Errors inside a module point at the module's file.
    let broken = module_project(
        "modules-broken",
        &[
            ("elomar.toml", ""),
            ("src/lib.elo", "pub fn f() -> Int {\n    return \"x\"\n}\n"),
            (
                "src/main.elo",
                "use app.lib\nfn main() {\n    print(lib.f())\n}\n",
            ),
        ],
    );
    let (codes, stderr) = check_codes(&broken);
    assert_eq!(codes, vec!["E2001"], "{stderr}");
    assert!(stderr.contains("src/lib.elo:2:12"), "{stderr}");

    // Imports are per file: `lib` does not see `std.ai` imported by main.
    let isolated = module_project(
        "modules-isolated",
        &[
            ("elomar.toml", ""),
            (
                "src/lib.elo",
                "pub async fn ask() -> Result<String, AIError> !{ai} {\n    return await ai.infer<String>(\"mock\", \"hi\", \"x\")\n}\n",
            ),
            (
                "src/main.elo",
                "use std.ai\nuse app.lib\nasync fn main() {\n    print(await lib.ask())\n}\n",
            ),
        ],
    );
    let (codes, stderr) = check_codes(&isolated);
    assert_eq!(codes, vec!["E1001"], "{stderr}");

    // Unknown items and private types.
    let unknown = module_project(
        "modules-unknown",
        &[
            ("elomar.toml", ""),
            (
                "src/lib.elo",
                "type Secret = { x: Int }\npub fn shout() {}\n",
            ),
            (
                "src/main.elo",
                "use app.lib\nfn main() {\n    lib.shuot()\n    let s: lib.Secret = { x: 1 }\n}\n",
            ),
        ],
    );
    let (codes, stderr) = check_codes(&unknown);
    assert_eq!(codes, vec!["E1008", "E1012"], "{stderr}");
    assert!(stderr.contains("did you mean `lib.shout`?"), "{stderr}");
}

// ----- Editor support: `check --format json` ----------------------------------

fn check_json(main: &Path, stdin_text: &str, lang: &str) -> serde_json::Value {
    use std::io::Write as _;
    let mut child = Command::new(env!("CARGO_BIN_EXE_elomar"))
        .args(["--lang", lang, "check", "--format", "json", "--stdin"])
        .arg(main)
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .spawn()
        .unwrap();
    child
        .stdin
        .take()
        .unwrap()
        .write_all(stdin_text.as_bytes())
        .unwrap();
    let out = child.wait_with_output().unwrap();
    serde_json::from_slice(&out.stdout).expect("valid JSON")
}

#[test]
fn json_diagnostics_for_editors() {
    let main = module_project(
        "editor-json",
        &[
            ("elomar.toml", ""),
            ("src/lib.elo", "pub fn f() -> Int {\n    return \"x\"\n}\n"),
            ("src/main.elo", "fn main() {}\n"),
        ],
    );
    // The unsaved buffer (stdin) is checked, not the file on disk.
    // The emoji is two UTF-16 units wide, as editors count: `"😀"` spans 17..21.
    let result = check_json(
        &main,
        "use app.lib\nfn main() {\n    let ok = 1\n    let x: Int = \"😀\"\n}\n",
        "en",
    );
    let diags = result["diagnostics"].as_array().unwrap();
    let codes: Vec<&str> = diags.iter().map(|d| d["code"].as_str().unwrap()).collect();
    assert!(codes.contains(&"E2001"), "{result}");
    let in_main = diags
        .iter()
        .find(|d| d["file"].as_str().unwrap().ends_with("main.elo") && d["code"] == "E2001")
        .unwrap();
    assert_eq!(
        in_main["range"]["start"],
        serde_json::json!({ "line": 3, "character": 17 })
    );
    assert_eq!(
        in_main["range"]["end"],
        serde_json::json!({ "line": 3, "character": 21 })
    );
    // Errors inside an imported module carry that module's path.
    assert!(
        diags
            .iter()
            .any(|d| d["file"].as_str().unwrap().ends_with("lib.elo")),
        "{result}"
    );

    // A module file can be checked on its own: no `main` needed.
    let lib = main.parent().unwrap().join("lib.elo");
    let result = check_json(&lib, "pub fn g() -> Int {\n    return 1\n}\n", "ar");
    assert_eq!(result["diagnostics"], serde_json::json!([]), "{result}");

    // Messages follow `--lang`.
    let result = check_json(&main, "fn main() {\n    print(nope)\n}\n", "ar");
    assert_eq!(result["diagnostics"][0]["message"], "اسم غير معروف `nope`");
}

// ----- elomar new / elomar test --------------------------------------------

fn elomar_in(dir: &Path, args: &[&str]) -> std::process::Output {
    Command::new(env!("CARGO_BIN_EXE_elomar"))
        .args(["--no-color", "--lang", "en"])
        .args(args)
        .current_dir(dir)
        .output()
        .unwrap()
}

#[test]
fn new_project_runs_and_passes_its_tests() {
    let base = Path::new(env!("CARGO_TARGET_TMPDIR")).join("new-project");
    let _ = std::fs::remove_dir_all(&base);
    std::fs::create_dir_all(&base).unwrap();

    let out = elomar_in(&base, &["new", "myapp"]);
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    let app = base.join("myapp");
    for f in [
        "elomar.toml",
        "src/main.elo",
        "src/greeting.elo",
        "tests/greeting_test.elo",
    ] {
        assert!(app.join(f).is_file(), "missing {f}");
    }
    let run = elomar_in(&app, &["run", "src/main.elo"]);
    assert_eq!(
        lines(&String::from_utf8_lossy(&run.stdout)),
        "Hello, world!\n"
    );
    let test = elomar_in(&app, &["test"]);
    let stdout = String::from_utf8_lossy(&test.stdout);
    assert!(test.status.success(), "{stdout}");
    assert!(
        stdout.contains("test result: ok. 2 passed; 0 failed"),
        "{stdout}"
    );

    // An existing, non-empty folder is never overwritten.
    let again = elomar_in(&base, &["new", "myapp"]);
    assert_eq!(again.status.code(), Some(2));
    let bad = elomar_in(&base, &["new", "../escape"]);
    assert_eq!(bad.status.code(), Some(2));
}

#[test]
fn test_command_reports_failures() {
    let main = module_project(
        "test-failures",
        &[
            ("elomar.toml", ""),
            (
                "src/lib.elo",
                "pub fn double(x: Int) -> Int {\n    return x * 2\n}\n",
            ),
            ("src/main.elo", "fn main() {}\n"),
            (
                "tests/lib_test.elo",
                "use app.lib\n\nfn test_ok() {\n    assert_eq(lib.double(2), 4)\n}\n\nfn test_wrong() {\n    print(\"about to fail\")\n    assert_eq(lib.double(2), 5)\n}\n\nfn test_err() -> Result<Int, String> {\n    return Err(\"nope\")\n}\n\nfn not_a_test() {}\n",
            ),
        ],
    );
    let project = main.parent().unwrap().parent().unwrap();
    let out = elomar_in(project, &["test"]);
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert_eq!(out.status.code(), Some(1), "{stdout}");
    assert!(stdout.contains("test test_ok ... ok"), "{stdout}");
    assert!(stdout.contains("test test_wrong ... FAILED"), "{stdout}");
    assert!(
        stdout.contains("about to fail"),
        "output of failed tests is shown: {stdout}"
    );
    assert!(
        stdout.contains("actual:   4") && stdout.contains("expected: 5"),
        "{stdout}"
    );
    assert!(
        stdout.contains("the test returned Err(\"nope\")"),
        "{stdout}"
    );
    assert!(
        stdout.contains("test result: FAILED. 1 passed; 2 failed"),
        "{stdout}"
    );
    assert!(!stdout.contains("not_a_test"), "{stdout}");

    let filtered = elomar_in(project, &["test", "--filter", "ok"]);
    assert!(filtered.status.success());
    assert!(
        String::from_utf8_lossy(&filtered.stdout).contains("1 passed; 0 failed; 2 filtered out")
    );
}

#[test]
fn example_project_tests_pass() {
    let out = elomar(&["test", root().join("examples/school").to_str().unwrap()]);
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(out.status.success(), "{stdout}");
    assert!(stdout.contains("4 passed; 0 failed"), "{stdout}");
}

// ----- elomar fmt -------------------------------------------------------------

#[test]
fn fmt_command() {
    let dir = Path::new(env!("CARGO_TARGET_TMPDIR")).join("fmt-project");
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(dir.join("src")).unwrap();
    let messy = "fn main( ){\nlet x=1+2;   // sum\nprint( x )\n}\n";
    let tidy = "fn main() {\n    let x = 1 + 2 // sum\n    print(x)\n}\n";
    std::fs::write(dir.join("src/main.elo"), messy).unwrap();
    std::fs::write(dir.join("src/ok.elo"), tidy).unwrap();
    std::fs::write(dir.join("src/broken.elo"), "let s = \"open\n").unwrap();

    // --check reports without writing.
    let out = elomar_in(&dir, &["fmt", "--check", "src/main.elo", "src/ok.elo"]);
    assert_eq!(out.status.code(), Some(1));
    assert!(String::from_utf8_lossy(&out.stdout).contains("would reformat src/main.elo"));
    assert_eq!(
        std::fs::read_to_string(dir.join("src/main.elo")).unwrap(),
        messy
    );

    // Formatting a folder rewrites files; broken files are reported, not touched.
    let out = elomar_in(&dir, &["fmt"]);
    assert_eq!(out.status.code(), Some(1), "broken.elo cannot be formatted");
    assert_eq!(
        std::fs::read_to_string(dir.join("src/main.elo")).unwrap(),
        tidy
    );
    assert_eq!(
        std::fs::read_to_string(dir.join("src/broken.elo")).unwrap(),
        "let s = \"open\n"
    );
    assert!(String::from_utf8_lossy(&out.stderr).contains("error[E0002]"));

    // --stdin prints the result (used by the VS Code extension).
    use std::io::Write as _;
    let mut child = Command::new(env!("CARGO_BIN_EXE_elomar"))
        .args(["fmt", "--stdin"])
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .spawn()
        .unwrap();
    child
        .stdin
        .take()
        .unwrap()
        .write_all(messy.as_bytes())
        .unwrap();
    let out = child.wait_with_output().unwrap();
    assert_eq!(String::from_utf8_lossy(&out.stdout), tidy);
}

#[test]
fn repository_elo_files_are_formatted() {
    let out = elomar(&[
        "fmt",
        "--check",
        root().join("examples").to_str().unwrap(),
        root().join("tests").to_str().unwrap(),
    ]);
    assert!(
        out.status.success(),
        "run `elomar fmt examples tests`:\n{}",
        String::from_utf8_lossy(&out.stdout)
    );
}

#[test]
fn named_imports() {
    let main = module_project(
        "named-imports",
        &[
            ("elomar.toml", ""),
            (
                "src/lib.elo",
                "pub type Point = { x: Int, y: Int }\npub enum Dir {\n    Up\n    Down\n}\npub let ORIGIN: Point = { x: 0, y: 0 }\npub fn moved(p: Point, d: Dir) -> Point {\n    return match d {\n        Dir.Up => ({ x: p.x, y: p.y + 1 })\n        Dir.Down => ({ x: p.x, y: p.y - 1 })\n    }\n}\nfn secret() {}\n",
            ),
            (
                "src/main.elo",
                "use app.lib.{Point, Dir, ORIGIN, moved}\nfn main() {\n    let p: Point = moved(ORIGIN, Dir.Up)\n    match Dir.Down {\n        Dir.Up => print(\"up\")\n        Dir.Down => print(p)\n    }\n}\n",
            ),
        ],
    );
    let out = elomar(&["run", main.to_str().unwrap()]);
    assert_eq!(
        lines(&String::from_utf8_lossy(&out.stdout)),
        "{ x: 0, y: 1 }\n",
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );

    let bad = module_project(
        "named-imports-bad",
        &[
            ("elomar.toml", ""),
            ("src/lib.elo", "pub fn shout() {}\nfn secret() {}\n"),
            (
                "src/main.elo",
                "use app.lib.{shuot, secret, shout}\nfn shout() {}\nfn main() {}\n",
            ),
        ],
    );
    let (codes, stderr) = check_codes(&bad);
    assert_eq!(codes, vec!["E1008", "E1012", "E1002"], "{stderr}");
    assert!(stderr.contains("did you mean `shout`?"), "{stderr}");
}

#[test]
fn packages_add_use_and_remove() {
    let main = module_project(
        "packages",
        &[
            ("app/elomar.toml", "[package]\nname = \"app\"\n"),
            (
                "app/src/main.elo",
                "use geo.shapes\n\nfn main() {\n    print(shapes.area(2.0, 3.0), shapes.label())\n}\n",
            ),
            (
                "geo/elomar.toml",
                "[package]\nname = \"geo\"\n\n[dependencies]\nunits = { path = \"../units\" }\n",
            ),
            (
                "geo/src/shapes.elo",
                "use app.helpers\nuse units.names\n\npub fn area(w: Float, h: Float) -> Float {\n    return helpers.times(w, h)\n}\n\npub fn label() -> String {\n    return names.unit()\n}\n",
            ),
            (
                "geo/src/helpers.elo",
                "pub fn times(a: Float, b: Float) -> Float {\n    return a * b\n}\n",
            ),
            ("units/elomar.toml", ""),
            (
                "units/src/names.elo",
                "pub fn unit() -> String {\n    return \"m2\"\n}\n",
            ),
        ],
    );
    let root = main.parent().unwrap().parent().unwrap();
    let app = root.join("app");
    let run_in = |dir: &Path, args: &[&str]| {
        Command::new(env!("CARGO_BIN_EXE_elomar"))
            .args(["--no-color", "--lang", "en"])
            .args(args)
            .current_dir(dir)
            .output()
            .unwrap()
    };
    // Not a dependency yet: an unknown module.
    let out = run_in(&app, &["check", "src/main.elo"]);
    assert!(String::from_utf8_lossy(&out.stderr).contains("E1009"));

    let out = run_in(&app, &["add", "geo", "--path", "../geo"]);
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    let toml = std::fs::read_to_string(app.join("elomar.toml")).unwrap();
    assert!(
        toml.contains("[dependencies]\ngeo = { path = \"../geo\" }"),
        "{toml}"
    );
    let out = run_in(&app, &["run", "src/main.elo"]);
    assert_eq!(
        lines(&String::from_utf8_lossy(&out.stdout)),
        "6.0 m2\n",
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );

    let out = run_in(&app, &["remove", "geo"]);
    assert!(out.status.success());
    assert!(
        !std::fs::read_to_string(app.join("elomar.toml"))
            .unwrap()
            .contains("geo =")
    );
    assert!(!run_in(&app, &["remove", "geo"]).status.success());
    assert!(
        !run_in(&app, &["add", "App", "--path", "../geo"])
            .status
            .success()
    );
}

#[test]
fn registry_publish_search_add_and_run() {
    let has_git = Command::new("git").arg("--version").output().is_ok();
    if !has_git {
        return;
    }
    let main = module_project(
        "registry",
        &[
            (
                "strings/elomar.toml",
                "[package]\nname = \"strings\"\nversion = \"1.2.0\"\ndescription = \"Text helpers\"\n",
            ),
            (
                "strings/src/text.elo",
                "pub fn shout(s: String) -> String {\n    return s.upper() + \"!\"\n}\n",
            ),
            ("index/README.md", "An Elomar registry index.\n"),
            (
                "app/elomar.toml",
                "[package]\nname = \"app\"\n\n[registry]\nindex = \"../index\"\n",
            ),
            (
                "app/src/main.elo",
                "use strings.text\n\nfn main() {\n    print(text.shout(\"hi\"))\n}\n",
            ),
        ],
    );
    let root = main.parent().unwrap().parent().unwrap().to_path_buf();
    let home = root.join("home");
    let run_in = |dir: &Path, args: &[&str]| {
        let out = Command::new(env!("CARGO_BIN_EXE_elomar"))
            .args(["--no-color", "--lang", "en"])
            .args(args)
            .current_dir(dir)
            .env("ELOMAR_HOME", &home)
            .env_remove("ELOMAR_REGISTRY")
            .output()
            .unwrap();
        (
            out.status.success(),
            lines(&String::from_utf8_lossy(&out.stdout)),
            lines(&String::from_utf8_lossy(&out.stderr)),
        )
    };
    let git = |args: &[&str]| {
        let out = Command::new("git")
            .args(["-c", "user.name=t", "-c", "user.email=t@t"])
            .args(args)
            .current_dir(root.join("strings"))
            .output()
            .unwrap();
        assert!(
            out.status.success(),
            "{}",
            String::from_utf8_lossy(&out.stderr)
        );
    };
    let pkg = root.join("strings");
    let app = root.join("app");
    let url = format!(
        "file:///{}",
        pkg.to_string_lossy()
            .replace('\\', "/")
            .trim_start_matches('/')
    );

    // Uncommitted work cannot be published.
    git(&["init", "--quiet"]);
    let index = root.join("index");
    let index_arg = index.to_str().unwrap();
    let (ok, _, err) = run_in(&pkg, &["publish", "--index", index_arg, "--git", &url]);
    assert!(!ok && err.contains("uncommitted"), "{err}");
    git(&["add", "."]);
    git(&["commit", "--quiet", "-m", "1.2.0"]);
    let (ok, out, err) = run_in(&pkg, &["publish", "--index", index_arg, "--git", &url]);
    assert!(ok, "{err}");
    assert!(out.contains("added strings 1.2.0"), "{out}");
    assert!(err.contains("not on any remote branch"), "{err}");
    let (ok, _, err) = run_in(&pkg, &["publish", "--index", index_arg, "--git", &url]);
    assert!(!ok && err.contains("already published"), "{err}");

    let (ok, out, _) = run_in(&app, &["search", "text"]);
    assert!(ok);
    assert_eq!(out, "strings 1.2.0 - Text helpers\n");

    let (ok, out, err) = run_in(&app, &["add", "strings@2"]);
    assert!(
        !ok && err.contains("no version of `strings` matches `2`"),
        "{out}{err}"
    );
    let (ok, out, err) = run_in(&app, &["add", "strings"]);
    assert!(ok, "{err}");
    assert!(out.contains("added `strings` 1.2.0"), "{out}");
    let toml = std::fs::read_to_string(app.join("elomar.toml")).unwrap();
    assert!(toml.contains("strings = \"^1.2.0\""), "{toml}");
    let (ok, out, err) = run_in(&app, &["run", "src/main.elo"]);
    assert!(ok, "{err}");
    assert_eq!(out, "HI!\n");
    let (_, _, err) = run_in(&app, &["add", "nothing"]);
    assert!(err.contains("not in the registry"), "{err}");
}
