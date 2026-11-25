# Contributing to Elomar-Lang

Thank you for helping. Bug reports, ideas, lessons, translations and code
are all welcome, in English or Arabic.

## Reporting a bug or asking for a feature

Open an [issue](https://github.com/abobasel86/elomar-lang/issues/new/choose)
and pick a form. For a bug, the most useful things are the smallest
program that shows it, what you expected, what happened, and the output of
`elomar --version`. If the problem is with an AI model, run `elomar doctor`
in the project and include its output (it never prints keys).

Security problems go privately, not in an issue: see [SECURITY.md](SECURITY.md).

## Building and testing

You need a recent stable Rust (the version is pinned in
`rust-toolchain.toml`) and, for the `std.py` and MCP tests, Python 3.

```bash
cargo build -p elomar-cli                 # the `elomar` program: target/debug/elomar
cargo test --workspace                    # every test; no network and no API keys needed
cargo clippy --workspace --all-targets -- -D warnings
cargo fmt --all
```

The browser playground is the same interpreter built for WebAssembly:

```bash
cargo clippy -p elomar-playground --target wasm32-unknown-unknown -- -D warnings
```

The fuzzer runs generated programs on both engines and compares them:

```bash
cargo run --profile fuzzing -p elomar-fuzz -- --seconds 60
```

## How the code is organized

| Folder | What is there |
|---|---|
| `crates/elomar-lexer`, `elomar-parser`, `elomar-ast` | Source text to syntax tree |
| `crates/elomar-types` | The checker: types, effects, traits, error codes |
| `crates/elomar-interpreter` | Two engines (a bytecode VM and a tree walker) and the standard library |
| `crates/elomar-ai-core`, `elomar-ai-provider-*` | AI calls: policy, schemas, OpenAI, Anthropic, the mock |
| `crates/elomar-cli` | The `elomar` program, the language server and the debugger |
| `crates/elomar-playground` | The browser playground (WebAssembly) |
| `docs/grammar.md`, `docs/grammar.en.md` | The specification, in Arabic and English |
| `site/en`, `site/ar` | The tutorial website |
| `tests/engines` | Programs both engines must print identically (`.elo` and `.out`) |
| `examples` | Example projects; those with `expected.out` are tested |

## What a change needs

- **Tests.** A language change gets a checker test and usually a program in
  `tests/engines` with its `.out`; both engines must print the same
  (`ELOMAR_ENGINE=tree elomar run file.elo` runs the tree walker).
- **Both languages.** Error messages come in English and Arabic
  (`Text::new(en, ar)`). New features are documented in both
  specifications and on the website in both languages (`site/en`,
  `site/ar`); every function and method must be on the reference page
  (`12-reference.md`), which a test checks.
- **Examples that run.** Code blocks on the website with `fn main` are
  checked by the tests, so write the real output in comments.
- **`CHANGELOG.md`**: add a line under `[Unreleased]`.
- **Compatibility.** Programs written for 1.0 must keep working: see
  [docs/stability.md](docs/stability.md).

## Pull requests

Keep a pull request to one change, describe what it does and why, and make
sure `cargo fmt`, `cargo clippy` and `cargo test` pass. CI runs them on
Linux and Windows, plus the WebAssembly build, a minute of fuzzing and the
installers.

---

## المساهمة بالعربية

شكرًا لمساعدتك. نرحب بتقارير الأخطاء والأفكار والدروس والترجمات والكود، بالعربية أو الإنجليزية.

- **الإبلاغ عن خطأ أو طلب ميزة:** افتح [مسألة](https://github.com/abobasel86/elomar-lang/issues/new/choose) واختر النموذج
  المناسب. أرفق أصغر برنامج يُظهر المشكلة، وما توقعته، وما حدث، وناتج `elomar --version`. وإن كانت المشكلة مع نموذج ذكاء
  اصطناعي فأرفق ناتج `elomar doctor` (لا يطبع أي مفتاح).
- **الثغرات الأمنية** تُبلَّغ سرًا لا في مسألة عامة: انظر [SECURITY.md](SECURITY.md).
- **البناء والاختبار:** `cargo test --workspace` يشغّل كل الاختبارات دون إنترنت ودون مفاتيح، ومعه `cargo clippy` و`cargo fmt`.
- **ما يحتاجه أي تغيير:** اختبارات، ورسائل الأخطاء باللغتين، وتوثيق الميزة في المواصفة والموقع باللغتين، وسطر في `CHANGELOG.md`،
  وألا تتعطل البرامج المكتوبة للإصدار 1.0.
