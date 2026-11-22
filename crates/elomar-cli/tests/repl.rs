//! `elomar repl` driven through standard input.

use std::io::Write;
use std::process::{Command, Stdio};
use std::sync::atomic::{AtomicUsize, Ordering};

/// Runs `elomar repl` in a fresh folder with `lines` as input; returns
/// (stdout, stderr).
fn repl(lang: &str, lines: &[&str]) -> (String, String) {
    static NEXT: AtomicUsize = AtomicUsize::new(0);
    let dir = std::env::temp_dir().join(format!(
        "elomar-repl-{}-{}",
        std::process::id(),
        NEXT.fetch_add(1, Ordering::Relaxed)
    ));
    std::fs::create_dir_all(&dir).unwrap();
    let mut child = Command::new(env!("CARGO_BIN_EXE_elomar"))
        .args(["--no-color", "--lang", lang, "repl"])
        .current_dir(&dir)
        // Keeps the test's typed lines out of the user's history file.
        .env("HOME", &dir)
        .env("USERPROFILE", &dir)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("run elomar repl");
    let mut input = lines.join("\n");
    input.push('\n');
    child
        .stdin
        .take()
        .unwrap()
        .write_all(input.as_bytes())
        .unwrap();
    let out = child.wait_with_output().unwrap();
    let _ = std::fs::remove_dir_all(&dir);
    (
        String::from_utf8_lossy(&out.stdout).replace("\r\n", "\n"),
        String::from_utf8_lossy(&out.stderr).replace("\r\n", "\n"),
    )
}

/// The printed results, without the greeting line.
fn results(stdout: &str) -> Vec<&str> {
    stdout.lines().skip(1).collect()
}

#[test]
fn values_variables_and_functions_carry_over() {
    let (out, err) = repl(
        "en",
        &[
            "1 + 2",
            "let x = 10",
            "x * 2",
            // Typing `let x` again replaces it.
            "let x = \"hi\"",
            "x",
            "var n = 1",
            "n += 5",
            "n",
            "fn sq(a: Int) -> Int {",
            "    return a * a",
            "}",
            "sq(n)",
            // A function typed again replaces the old one.
            "fn sq(a: Int) -> Int { return a * a * a }",
            "sq(2)",
            "let f = |v: Int| v + 100",
            "[1, 2].map(f)",
            "let (a, b) = (1, \"z\")",
            "b",
            "print(\"printed\")",
        ],
    );
    assert_eq!(err, "");
    assert_eq!(
        results(&out),
        [
            "3",
            "20",
            "\"hi\"",
            "6",
            "36",
            "8",
            "[101, 102]",
            "\"z\"",
            "printed"
        ]
    );
}

#[test]
fn errors_do_not_end_the_session() {
    let (out, err) = repl("en", &["nope + 1", "10 / 0", "let ok = 7", "ok"]);
    assert!(err.contains("E1001"), "{err}");
    assert!(err.contains("<input>:1:1"), "{err}");
    assert!(err.contains("R0001"), "{err}");
    assert_eq!(results(&out), ["7"]);
}

#[test]
fn await_and_types_and_commands() {
    let (out, err) = repl(
        "en",
        &[
            "use std.ai",
            "type P = { name: String }",
            "let p: P = { name: \"Lina\" }",
            "p.name",
            "await ai.infer<String>(\"mock\", \"Say hi\", \"x\")",
            ":show",
            ":reset",
            "p",
            ":quit",
            "1 + 1",
        ],
    );
    let r = results(&out);
    assert_eq!(r[0], "\"Lina\"");
    assert_eq!(r[1], "Ok(\"mock\")");
    // `:show` lists declarations, then the inputs that defined variables.
    assert_eq!(
        &r[2..5],
        [
            "use std.ai",
            "type P = { name: String }",
            "let p: P = { name: \"Lina\" }"
        ]
    );
    assert_eq!(r[5], "Cleared.");
    // After `:reset`, `p` is unknown; after `:quit`, nothing runs.
    assert!(err.contains("E1001"), "{err}");
    assert_eq!(r.len(), 6);
}

#[test]
fn arabic_names_and_messages() {
    let (out, err) = repl("ar", &["let العدد = 4", "العدد * العدد", "مجهول", ":help"]);
    assert!(out.starts_with("Elomar-Lang"), "{out}");
    assert!(out.contains("اكتب كودًا"), "{out}");
    assert!(out.contains("\n16\n"), "{out}");
    assert!(out.contains(":reset   امسح كل شيء"), "{out}");
    assert!(err.contains("<المدخل>"), "{err}");
}
