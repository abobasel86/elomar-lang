<p align="center">
  <img src="editor/vscode/images/icon.png" width="110" alt="Elomar-Lang icon">
</p>

<h1 align="center">Elomar-Lang</h1>

<p align="center">
  <b>A simple, typed programming language with AI built in. Write code in Arabic or English.</b><br>
  <span dir="rtl">لغة برمجة بسيطة، فيها الذكاء الاصطناعي جزء من اللغة، وتكتب بها بالعربية أو الإنجليزية.</span>
</p>

<p align="center">
  <a href="https://github.com/abobasel86/elomar-lang/releases/latest"><img alt="Latest release" src="https://img.shields.io/github/v/release/abobasel86/elomar-lang?color=0f6e63"></a>
  <a href="https://open-vsx.org/extension/abobasel86/elomar-lang"><img alt="Open VSX" src="https://img.shields.io/open-vsx/v/abobasel86/elomar-lang?label=Open%20VSX&color=0f6e63"></a>
  <a href="https://github.com/abobasel86/elomar-lang/actions/workflows/ci.yml"><img alt="CI" src="https://github.com/abobasel86/elomar-lang/actions/workflows/ci.yml/badge.svg"></a>
  <a href="#license"><img alt="License" src="https://img.shields.io/badge/license-MIT%20%2F%20Apache--2.0-0f6e63"></a>
</p>

<p align="center">
  <a href="https://abobasel86.github.io/elomar-lang/"><b>Tutorial (English / العربية)</b></a> ·
  <a href="https://abobasel86.github.io/elomar-lang/en/playground.html"><b>Try it in the browser</b></a> ·
  <a href="https://github.com/abobasel86/elomar-lang/releases/latest"><b>Download</b></a> ·
  <a href="https://open-vsx.org/extension/abobasel86/elomar-lang"><b>Editor extension</b></a>
</p>

```elomar
use std.ai

enum Priority { Low, Normal, Urgent }
type Ticket = { category: String, priority: Priority }

async fn main() {
    let طلبات = ["I was charged twice!", "خُصم المبلغ مرتين", "How do I reset my password?"]
    // The three requests are sent to the model at the same time.
    let نتائج = await all(طلبات.map(|t| ai.infer<Ticket>("mock", "Classify the support request", t)))
    for نتيجة in نتائج {
        match نتيجة {
            Ok(t) => print("{t.priority}: {t.category}")
            Err(e) => print("failed: {e.kind()}")
        }
    }
}
```

The model's answer is checked against `Ticket` before your code sees it, a
failure is a value you must handle, and `"mock"` runs offline; switch to
OpenAI or Claude in `elomar.toml` without changing the code.

> **Project status: v0.4.0, early development. Not production-ready.**
> A working interpreter for the language, typed AI calls (an offline **mock**
> provider, **OpenAI** and **Anthropic Claude**), agents with MCP, a standard
> library (files, JSON, HTTP client and server, SQLite, regex, time, crypto,
> Python, processes), packages with a public registry, editor support, a
> bilingual tutorial and a browser playground.
> See [`docs/grammar.md`](docs/grammar.md) and the [changelog](CHANGELOG.md).

## Vision

Elomar-Lang aims to combine predictable traditional programming with typed,
permission-aware AI inference. Effects such as printing, network access or
model calls are visible in types, can be tested offline, and are rejected
by the compiler when they are not declared.

## What works today

- Static types with local inference; no `null` (`Option<T>`), explicit errors (`Result<T, E>`).
- No semicolons needed; `/` always returns a `Float`, `div` is integer division.
- Structural records and value semantics (no surprising aliasing).
- `match` with exhaustiveness checking.
- Enums with payloads (`Shape.Circle(1.0)`), maps (`["a": 1]`), and the `?`
  operator for propagating `Err`/`None`.
- Files and JSON: `std.fs` (the `fs` effect, default-deny `filesystem_read` /
  `filesystem_write` in `elomar.toml`) and typed `json.parse<T>` /
  `json.stringify`. See [`examples/files`](examples/files).
- Sets: `set([1, 2])` with `union`, `intersection`, `difference`, `is_subset`.
- HTTP client: `http.get(url)` / `http.post(url, body)` from `std.http`, with
  the `net` effect and the same `network_hosts` allowlist (https only, no
  redirects followed).
- SQLite databases: `db.query<Student>(conn, "SELECT ... WHERE grade > :min", { min: 60 })`
  returns typed rows; named parameters (no SQL injection), the `db` effect,
  and file databases need `filesystem_write`. See [`examples/database`](examples/database).
- Generics: `fn first<T>(xs: List<T>) -> Option<T>`, `type Pair<A, B>`,
  `enum Tree<T>`. See [`examples/generics`](examples/generics).
- Traits and methods: `trait Describe`, `impl Describe for Student`,
  `impl Shape { fn area(self) ... }`, bounds `fn report<T: Describe>`.
  See [`examples/traits`](examples/traits).
- Closures and function values: `xs.map(|x| x * 2)`, `filter`, `fold`,
  `sort_by`, and `fn(Int) -> Int` types. See [`examples/closures`](examples/closures).
- Multi-file programs: `use app.students` loads `src/students.elo` (or
  `use app.students.{Student, average}` to import names directly); only `pub`
  items are visible to other files. See [`examples/school`](examples/school).
- **VS Code support** in [`editor/vscode`](editor/vscode): highlighting
  (Arabic names included), errors while typing in English or Arabic, hover
  types, go to definition, outline, completion, a Run button, snippets, and
  a **debugger** (F5: breakpoints, stepping, the call stack and variables)
  through `elomar dap`, the Debug Adapter Protocol.
- **Packages:** `[dependencies]` from a local folder or a git repository
  (`elomar add geometry --path vendor/geometry`, `elomar add kit --git URL --tag v1`),
  locked in `elomar.lock`, used as `use geometry.shapes`. See
  [`examples/packages`](examples/packages).
- **Package registry:** a registry is a git repository (or folder) indexing
  published versions, each pinned to a commit. Depend on `strings = "1.2"`
  (semver ranges), find packages with `elomar search`, add them with
  `elomar add strings`, and publish with `elomar publish --index <clone>`.
  No server is needed. The default is the public
  [elomar-registry](https://github.com/abobasel86/elomar-registry).
- **Language server** `elomar lsp` (LSP over stdio) for any editor; see
  [Editors](#editors).
- Effect inference with mandatory declarations on `pub` functions.
- `async fn` / `await` (sequential, lazy futures).
- **Typed AI calls:** `ai.infer<T>(...)` returns `Result<T, AIError>` (`AIError` is an enum, e.g. `AIError.Timeout(msg)`); answers are
  validated against `T` (missing, extra or mistyped fields become
  `InvalidStructuredOutput`), and calling a model is the `ai` effect.
- **AI with tools:** `ai.run<Answer>(..., tools: [find_student, class_average], max_steps: 5)`
  lets the model call only the functions you list; results go back to the
  model and the final answer is validated against `Answer`. Tools' effects
  count for the caller. See [`examples/ai-tools`](examples/ai-tools).
- **Agents and MCP:** `agent.run<Answer>(..., tools: [...], mcp: ["notes"],
  require_approval: ["notes.delete_all"], approve: |call| ..., on_step: |step| ...)`
  lets a model use your functions and the tools of MCP servers listed in
  `elomar.toml`, with approval in code and a log of every step; `mcp.call`
  calls MCP tools directly. See [`examples/agents`](examples/agents).
- **Python bridge:** `py.call<Stats>("grades", "describe", { grades: xs })` runs a
  function from `python/grades.py` (or any installed package) in a
  subprocess, with typed results, the `py` effect and an explicit
  `python = true` permission. See [`examples/python`](examples/python).
- **Web server:** `server.serve("127.0.0.1:8080", handle)` with
  `fn handle(req: HttpRequest) -> HttpResponse`, JSON and HTML helpers and a
  `listen` permission. See [`examples/web-api`](examples/web-api).
- **Everyday modules:** `std.math` (`math.pi`, `math.sqrt`, ...),
  `std.random`, `std.time` (dates in ISO 8601) and `std.os` (`os.args()`,
  `os.env`, `os.exit`), and string interpolation: `"Hello {name}"`.
- An offline **mock provider** configured in `elomar.toml`: canned answers,
  simulated errors and latency, call budgets — deterministic and free.
- An **OpenAI provider** (structured outputs) with the key read from an
  environment variable, a default-deny `[permissions] network_hosts`
  allowlist, and the `net` effect required for cloud models. See
  [`examples/ai-openai`](examples/ai-openai).
- An **Anthropic Claude provider** (`provider = "anthropic"`, default model
  `claude-opus-5-5`) with structured outputs and tools. See
  [`examples/ai-claude`](examples/ai-claude).
- Arabic identifiers and correct Unicode text handling.
- **Bilingual diagnostics** (English/Arabic) with source locations and hints.

```elomar
type Student = { name: String, grades: List<Int> }

fn average(s: Student) -> Result<Float, String> {
    if s.grades.is_empty() {
        return Err("no grades for " + s.name)
    }
    return Ok(s.grades.sum() / s.grades.length)
}

fn main() {
    let lina = { name: "Lina", grades: [90, 85] }
    match average(lina) {
        Ok(avg) => print(lina.name, "→", avg)
        Err(message) => print("Error:", message)
    }
}
```

### Typed AI, tested offline

```elomar
use std.ai

type Ticket = { category: String, priority: String, summary: String }

pub async fn classify(message: String) -> Result<Ticket, AIError> !{ai} {
    return await ai.infer<Ticket>(
        model: "support.classifier",
        instructions: "Classify the support request.",
        input: message,
        timeout: 10s
    )
}
```

```toml
# elomar.toml
[ai.models."support.classifier"]
provider = "mock"

[[ai.models."support.classifier".mock]]
input_contains = "password"
response = '{"category": "account", "priority": "high", "summary": "Cannot sign in"}'
```

See [`examples/ai-classification`](examples/ai-classification) for timeouts,
invalid answers and rate limits simulated by the mock.

## Learn

The **[tutorial](https://abobasel86.github.io/elomar-lang/)** has ten short
lessons in English and Arabic, from a first program to web servers and AI.
Every program in it is checked by the test suite.

Or try it without installing anything in the
**[playground](https://abobasel86.github.io/elomar-lang/en/playground.html)**,
which runs Elomar-Lang in your browser.

## Getting started

Download `elomar` for your system from the
[latest release](https://github.com/abobasel86/elomar-lang/releases/latest):

| System | File |
|---|---|
| Windows (x64) | `elomar-<version>-x86_64-pc-windows-msvc.zip` |
| Linux (x64) | `elomar-<version>-x86_64-unknown-linux-gnu.tar.gz` |
| macOS (Apple silicon) | `elomar-<version>-aarch64-apple-darwin.tar.gz` |
| macOS (Intel) | `elomar-<version>-x86_64-apple-darwin.tar.gz` |

Unpack it and put `elomar` (or `elomar.exe`) in a folder on your `PATH`.
`SHA256SUMS.txt` in the release lists each file's checksum. On macOS, the
binary is not signed, so the first run may need
`xattr -d com.apple.quarantine elomar`.

The release also has the VS Code extension: install
`elomar-lang-<version>.vsix` with **Extensions → … → Install from VSIX**.

Or build from source with a recent stable Rust toolchain:

```bash
cargo install --git https://github.com/abobasel86/elomar-lang elomar-cli --locked
```

Create a project, run it and test it:

```bash
elomar new myapp
```

```bash
cd myapp && elomar run src/main.elo && elomar test
```

Or try the examples from a checkout:

```bash
cargo run -p elomar-cli -- run examples/hello/main.elo
```

```bash
cargo run -p elomar-cli -- --lang ar check tests/fail/unknown_name.elo
```

| Command | Purpose |
|---|---|
| `elomar run FILE` | Check, then run a program |
| `elomar repl` | An interactive prompt: type code and see the result at once |
| `elomar dap` | The debugger, for editors (Debug Adapter Protocol on stdin/stdout) |
| `elomar check FILE` | Report errors and warnings without running |
| `--lang en\|ar` | Diagnostic language (or set `ELOMAR_LANG`) |
| `--no-color` | Plain output (also honors `NO_COLOR`) |
| `elomar check --format json [--stdin] FILE` | Diagnostics as JSON for editors (`main` not required) |
| `elomar new NAME` | Create a project with a program, a module and a passing test |
| `elomar test [PATH] [--filter TEXT]` | Run the `test_*` functions in `tests/` |
| `elomar fmt [PATHS] [--check] [--stdin]` | Format `.elo` files (comments kept; never changes meaning) |
| `elomar add NAME[@VERSION]` | Add a package from the registry |
| `elomar add NAME --path DIR` / `--git URL [--tag T]` | Add a dependency from a folder or git |
| `elomar remove NAME` / `elomar update` | Remove a dependency / move to the newest allowed versions |
| `elomar search [TEXT]` / `elomar publish --index DIR` | Find packages / add this package's version to a registry index |
| `elomar lsp` | Language server for editors (LSP on stdio) |

## Repository layout

| Path | Contents |
|---|---|
| `crates/elomar-diagnostics` | Spans, bilingual diagnostics, rendering |
| `crates/elomar-lexer` | Tokens and newline rules |
| `crates/elomar-ast` | Syntax tree |
| `crates/elomar-parser` | Recursive-descent + Pratt parser |
| `crates/elomar-loader` | Loads the main file and the modules and packages it imports |
| `crates/elomar-pkg` | Dependencies: path and git packages, cache, `elomar.lock` |
| `crates/elomar-fmt` | The formatter behind `elomar fmt` |
| `crates/elomar-types` | Name resolution, type inference, effect checking |
| `crates/elomar-interpreter` | Tree-walking interpreter, futures, `ai.infer` execution |
| `crates/elomar-ai-core` | Schemas (shared with `std.json`), AI errors, `elomar.toml`, answer validation |
| `crates/elomar-policy` | File-system permission checks |
| `crates/elomar-ai-provider-mock` | Deterministic offline model provider |
| `crates/elomar-ai-provider-openai` | OpenAI Chat Completions provider over HTTPS |
| `crates/elomar-ai-provider-anthropic` | Anthropic Claude (Messages API) provider over HTTPS |
| `crates/elomar-cli` | The `elomar` command |
| `editor/vscode` | VS Code extension (no build step) |
| `examples/` | Programs with expected output (run by the test suite) |
| `tests/fail/` | Programs that must fail with specific error codes |
| `docs/grammar.md` | Authoritative grammar, semantics and error codes |
| `Elomar-Lang_Master_Specification_AR.md` | Vision, architecture and roadmap (Arabic) |

## Testing

```bash
cargo test --workspace
```

Every example in `examples/` is executed through the real `elomar` binary
and compared with its `expected.out`.

To time the interpreter, build it in release mode and run the programs in
`bench/`:

```bash
cargo build --release -p elomar-cli && python bench/run.py
```

## Roadmap

1. ✅ Lexer, parser, type and effect checker, interpreter, CLI.
2. ✅ Typed AI runtime: `async`/`await`, `ai.infer<T>`, validation, mock and OpenAI providers, network permissions, `net` effect.
3. ✅ v0.2: enums, `Map`, `Set`, `?`, `std.fs`, `std.json`, modules and named imports, VS Code support, `elomar new`, `elomar test`, `elomar fmt`, CI.
4. 🟡 v0.3: ✅ generic functions, types and enums, closures and `map`/`filter`/`fold`, `std.http` client, traits and methods, SQLite (`std.db`), AI tools (`ai.run`), Python bridge (`std.py`), language server (`elomar lsp`), packages (`elomar add`), agents and MCP (`agent.run`, `std.mcp`), a package registry (`elomar publish`, `elomar search`).
5. Bytecode VM, performance work and stable releases.

## Editors

`elomar lsp` speaks the Language Server Protocol on standard input/output:
diagnostics (English or Arabic with `--lang ar`), hover, go to definition,
document symbols, completion and formatting.

- **VS Code:** the extension in [`editor/vscode`](editor/vscode) starts it.
- **Neovim** (0.11+):

  ```lua
  vim.filetype.add({ extension = { elo = "elomar" } })
  vim.lsp.config("elomar", { cmd = { "elomar", "lsp" }, filetypes = { "elomar" } })
  vim.lsp.enable("elomar")
  ```

- **Helix** (`languages.toml`):

  ```toml
  [language-server.elomar]
  command = "elomar"
  args = ["lsp"]

  [[language]]
  name = "elomar"
  scope = "source.elomar"
  file-types = ["elo"]
  language-servers = ["elomar"]
  ```

## License

Licensed under either of

- Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE))
- MIT license ([LICENSE-MIT](LICENSE-MIT))

at your option.

Unless you explicitly state otherwise, any contribution you intentionally
submit for inclusion in the work, as defined in the Apache-2.0 license, is
dual licensed as above, without any additional terms or conditions.

---

## بالعربية

**Elomar-Lang** لغة برمجة عامة الأغراض، أصلية التكامل مع الذكاء الاصطناعي. المفسّر الحالي
ينفذ نواة اللغة: الأنواع والاستنتاج، والسجلات، و`match`، والتأثيرات، ورسائل الأخطاء
بالعربية والإنجليزية.

**التثبيت:** نزّل ملف `elomar` المناسب لنظامك من
[آخر إصدار](https://github.com/abobasel86/elomar-lang/releases/latest)، وفك ضغطه،
وضع `elomar` في مجلد ضمن `PATH`. ومع الإصدار ملف إضافة VS Code (`.vsix`) تثبّته
من **Extensions → … → Install from VSIX**.

لتشغيل مثال بالعربية من نسخة المصدر:

```bash
cargo run -p elomar-cli -- --lang ar run examples/arabic/main.elo
```

المرجع النحوي الملزم: [`docs/grammar.md`](docs/grammar.md). الرؤية والخطة:
[`Elomar-Lang_Master_Specification_AR.md`](Elomar-Lang_Master_Specification_AR.md).
