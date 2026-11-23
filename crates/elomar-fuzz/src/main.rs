//! Fuzz testing: feeds the parser, the checker, the formatter, the error
//! renderer and the interpreter thousands of broken and unusual programs,
//! made by mutating the programs in the repository, and reports every one
//! that makes them crash (panic).
//!
//! ```text
//! cargo run --release -p elomar-fuzz -- [--seconds 60] [--seed 7]
//! ```
//!
//! Each program that crashed is saved in `target/fuzz-results/crash-N.elo` with the
//! message. Before each program runs, it is written to
//! `target/fuzz-results/current.elo`: if the process itself dies (a stack overflow
//! or running out of memory, which cannot be caught), that file is the
//! culprit. Programs run with no file, network, Python or process access,
//! the offline AI model `mock`, and a time limit.
//!
//! Each program runs twice, in the bytecode engine and in the tree-walking
//! interpreter, and the two must print the same and fail with the same
//! error: a difference is reported like a crash.

use std::collections::BTreeMap;
use std::panic::{AssertUnwindSafe, catch_unwind};
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{Duration, Instant};

use elomar_ai_core::AiRuntime;
use elomar_ai_provider_mock::MockProvider;
use elomar_diagnostics::{Lang, RenderOptions, SourceFile, render};
use elomar_interpreter::{BufferHost, Engine, FsPolicy};

/// Pieces inserted into programs: syntax that tends to reach unusual paths.
const TOKENS: &[&str] = &[
    "{",
    "}",
    "(",
    ")",
    "[",
    "]",
    "<",
    ">",
    ",",
    ":",
    ".",
    "..",
    "..=",
    "?",
    "|",
    "|x|",
    "=>",
    "->",
    "=",
    "==",
    "+=",
    "-",
    "*",
    "/",
    "%",
    " div ",
    "!",
    "&&",
    "||",
    "\"",
    "\\",
    "\\{",
    "r\"",
    "{x}",
    "\n",
    "\n\n",
    "    ",
    "\t",
    "//",
    "/*",
    "*/",
    "fn ",
    "let ",
    "var ",
    "match ",
    "if ",
    "else ",
    "while ",
    "for ",
    " in ",
    "return ",
    "break",
    "continue",
    "async ",
    "await ",
    "type ",
    "enum ",
    "trait ",
    "impl ",
    "use ",
    "pub ",
    "const ",
    "Some(",
    "None",
    "Ok(",
    "Err(",
    "true",
    "false",
    "0",
    "1",
    "-1",
    "9223372036854775807",
    "-9223372036854775808",
    "99999999999999999999",
    "1.5",
    "1e308",
    "0.0",
    "1ms",
    "10s",
    "x",
    "_",
    "main",
    "self",
    "Self",
    "List<",
    "Map<",
    "Option<",
    "Int",
    "String",
    "Float",
    "Bool",
    "()",
    "(1, \"a\")",
    "[]",
    "[:]",
    "set([])",
    "ع",
    "مرحبا",
    "👍🏽",
    "\u{200f}",
    "a.b.c",
    "x.0",
    "x[0]",
    "x[-1]",
    "f(",
    "f()",
    "f(x: 1)",
    "<T>",
    "<T: A>",
    "!{io}",
    "timeout(",
    "all([",
    "1 div 0",
    "2.pow(64)",
    "\"a\".repeat(3)",
];

/// Calls whose results change from run to run: the engines are not
/// compared on programs that use them.
const NONDETERMINISTIC: &[&str] = &["random", "uuid", "now", "today", "time."];

/// Calls that reach outside the process or wait; programs using them are
/// checked and formatted, but not run.
const NOT_RUN: &[&str] = &[
    "server.", "process.", "py.", "http.", "mcp.", "agent.", "sleep", "input(", "repeat(", "pow(",
    "os.exit",
];

struct Rng(u64);

impl Rng {
    fn next(&mut self) -> u64 {
        self.0 ^= self.0 << 13;
        self.0 ^= self.0 >> 7;
        self.0 ^= self.0 << 17;
        self.0
    }
    fn below(&mut self, n: usize) -> usize {
        (self.next() % n.max(1) as u64) as usize
    }
}

/// The nearest char boundary at or before `i`.
fn floor(s: &str, i: usize) -> usize {
    let mut i = i.min(s.len());
    while !s.is_char_boundary(i) {
        i -= 1;
    }
    i
}

/// A random char boundary in `s`.
fn boundary(s: &str, rng: &mut Rng) -> usize {
    floor(s, rng.below(s.len() + 1))
}

fn mutate(src: &str, seeds: &[String], rng: &mut Rng) -> String {
    let mut s = src.to_string();
    for _ in 0..1 + rng.below(4) {
        let a = boundary(&s, rng);
        let b = boundary(&s, rng);
        let (lo, hi) = (a.min(b), floor(&s, a.max(b).min(a.min(b) + 300)));
        match rng.below(6) {
            // Delete a short piece.
            0 => {
                let end = floor(&s, lo + 1 + rng.below(16)).max(lo);
                s.replace_range(lo..end, "");
            }
            // Duplicate a piece.
            1 => {
                let piece = s[lo..hi].to_string();
                s.insert_str(hi, &piece);
            }
            2 | 3 => s.insert_str(lo, TOKENS[rng.below(TOKENS.len())]),
            // A piece of another program.
            4 => {
                let other = &seeds[rng.below(seeds.len())];
                let x = boundary(other, rng);
                let y = floor(other, x + rng.below(300));
                s.insert_str(lo, &other[x..y.max(x)]);
            }
            // Move a piece elsewhere.
            _ => {
                let piece = s[lo..hi].to_string();
                s.replace_range(lo..hi, "");
                let at = boundary(&s, rng);
                s.insert_str(at, &piece);
            }
        }
        if s.len() > 20_000 {
            s.truncate(floor(&s, 20_000));
        }
    }
    s
}

/// Runs one program through every stage. Panics if any stage panics.
fn run_case(src: &str) {
    let file = SourceFile::new("main.elo", src);
    let (mut program, mut diags) = elomar_parser::parse(src);
    let _ = elomar_fmt::format(src);
    if diags.iter().all(|d| !d.is_error()) {
        diags.extend(elomar_types::check(
            &mut program,
            elomar_types::CheckOptions {
                require_main: true,
                ..Default::default()
            },
        ));
    }
    for lang in [Lang::En, Lang::Ar] {
        for d in &diags {
            let _ = render(d, &file, RenderOptions { lang, color: false });
        }
    }
    if diags.iter().any(|d| d.is_error()) || NOT_RUN.iter().any(|n| src.contains(n)) {
        return;
    }
    RAN.fetch_add(1, Ordering::Relaxed);
    let bytecode = run_engine(&program, Engine::Bytecode);
    let tree = run_engine(&program, Engine::Tree);
    // The two engines must agree, unless the time limit or something random
    // decided the result.
    let timed_out = |r: &Outcome| matches!(&r.1, Err(e) if e.code == "R0013");
    if !timed_out(&bytecode)
        && !timed_out(&tree)
        && !NONDETERMINISTIC.iter().any(|n| src.contains(n))
    {
        let show = |r: &Outcome| match &r.1 {
            Ok(()) => format!("output {:?}", r.0),
            Err(e) => format!(
                "output {:?}, error {} at {:?}: {:?}, trace {:?}",
                r.0, e.code, e.span, e.message, e.trace
            ),
        };
        COMPARED.fetch_add(1, Ordering::Relaxed);
        let (a, b) = (show(&bytecode), show(&tree));
        assert!(
            a == b,
            "the engines differ:
  bytecode: {a}
  tree:     {b}"
        );
    }
    if let Err(e) = bytecode.1 {
        let _ = render(
            &e.to_diagnostic(),
            &file,
            RenderOptions {
                lang: Lang::Ar,
                color: false,
            },
        );
    }
}

/// What a program printed, and how it ended.
type Outcome = (String, Result<(), elomar_interpreter::RuntimeError>);

fn run_engine(program: &elomar_ast::Program, engine: Engine) -> Outcome {
    elomar_interpreter::set_engine(engine);
    let outcome = elomar_interpreter::with_large_stack(|| {
        let mut host = BufferHost::default();
        let mut ai = AiRuntime::default();
        ai.register(
            "mock",
            Box::new(MockProvider::new(Vec::new()).expect("no rules")),
        );
        let result = elomar_interpreter::run_limited(
            program,
            &mut host,
            &mut ai,
            FsPolicy::deny_all("."),
            300,
        );
        (host.output, result)
    });
    elomar_interpreter::set_engine(Engine::Bytecode);
    outcome
}

fn collect_seeds(root: &Path) -> Vec<String> {
    let mut out = Vec::new();
    let mut stack: Vec<PathBuf> = [
        "examples",
        "tests",
        "bench",
        "site",
        "crates/elomar-cli/tests",
    ]
    .iter()
    .map(|d| root.join(d))
    .collect();
    while let Some(dir) = stack.pop() {
        let Ok(entries) = std::fs::read_dir(&dir) else {
            continue;
        };
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                stack.push(path);
                continue;
            }
            let Ok(text) = std::fs::read_to_string(&path) else {
                continue;
            };
            match path.extension().and_then(|e| e.to_str()) {
                Some("elo") => out.push(text),
                Some("md") => {
                    // Programs in the tutorial.
                    for block in text.split("```elomar\n").skip(1) {
                        if let Some(end) = block.find("```") {
                            out.push(block[..end].to_string());
                        }
                    }
                }
                _ => {}
            }
        }
    }
    out
}

static LAST_PANIC: Mutex<String> = Mutex::new(String::new());

/// Programs that ran, and those whose results were compared.
static RAN: AtomicU64 = AtomicU64::new(0);
static COMPARED: AtomicU64 = AtomicU64::new(0);

fn main() {
    let mut seconds = 30u64;
    let mut seed = 0x9e37_79b9_7f4a_7c15u64;
    let args: Vec<String> = std::env::args().skip(1).collect();
    let mut i = 0;
    while i < args.len() {
        match args[i].as_str() {
            "--seconds" => seconds = args.get(i + 1).and_then(|v| v.parse().ok()).unwrap_or(30),
            "--seed" => seed = args.get(i + 1).and_then(|v| v.parse().ok()).unwrap_or(seed) | 1,
            other => {
                eprintln!("unknown argument `{other}`; use --seconds N and --seed N");
                std::process::exit(2);
            }
        }
        i += 2;
    }
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let out_dir = root.join("target/fuzz-results");
    std::fs::create_dir_all(&out_dir).expect("create target/fuzz-results");
    let seeds = collect_seeds(&root);
    assert!(!seeds.is_empty(), "no programs found to start from");

    std::panic::set_hook(Box::new(|info| {
        let place = info
            .location()
            .map(|l| format!("{}:{}", l.file(), l.line()))
            .unwrap_or_default();
        let message = info
            .payload()
            .downcast_ref::<&str>()
            .map(|s| s.to_string())
            .or_else(|| info.payload().downcast_ref::<String>().cloned())
            .unwrap_or_default();
        if let Ok(mut last) = LAST_PANIC.lock() {
            *last = format!("{place}: {message}");
        }
    }));

    let mut rng = Rng(seed);
    let start = Instant::now();
    let mut cases = 0u64;
    // One example per distinct crash place.
    let mut crashes: BTreeMap<String, (String, u64)> = BTreeMap::new();
    let current = out_dir.join("current.elo");
    while start.elapsed() < Duration::from_secs(seconds) {
        let base = &seeds[rng.below(seeds.len())];
        let src = if rng.below(10) == 0 {
            base.clone()
        } else {
            mutate(base, &seeds, &mut rng)
        };
        let _ = std::fs::write(&current, &src);
        cases += 1;
        if catch_unwind(AssertUnwindSafe(|| run_case(&src))).is_err() {
            let message = LAST_PANIC.lock().map(|m| m.clone()).unwrap_or_default();
            let place = message.split(": ").next().unwrap_or("").to_string();
            let entry = crashes.entry(place).or_insert((String::new(), 0));
            entry.1 += 1;
            // Keep the shortest program for each crash.
            if entry.0.is_empty() || src.len() < entry.0.len() {
                entry.0 = src.clone();
                let n = crashes
                    .keys()
                    .position(|k| message.starts_with(k))
                    .unwrap_or(0);
                let _ = std::fs::write(
                    out_dir.join(format!("crash-{n}.elo")),
                    format!("// seed {seed}\n// {message}\n{src}"),
                );
            }
        }
    }
    let _ = std::fs::remove_file(&current);
    println!(
        "{cases} programs in {}s from {} starting programs: {} kind(s) of crash",
        start.elapsed().as_secs(),
        seeds.len(),
        crashes.len()
    );
    println!(
        "  {} ran, {} compared in both engines",
        RAN.load(Ordering::Relaxed),
        COMPARED.load(Ordering::Relaxed)
    );
    for (n, (place, (_, count))) in crashes.iter().enumerate() {
        println!("  crash-{n}.elo  ({count}×)  {place}");
    }
    if !crashes.is_empty() {
        std::process::exit(1);
    }
}
