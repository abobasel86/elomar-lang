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
  <a href="https://marketplace.visualstudio.com/items?itemName=abobasel86.elomar-lang"><img alt="VS Code Marketplace" src="https://img.shields.io/badge/VS%20Code-Marketplace-0f6e63"></a>
  <a href="https://open-vsx.org/extension/abobasel86/elomar-lang"><img alt="Open VSX" src="https://img.shields.io/open-vsx/v/abobasel86/elomar-lang?label=Open%20VSX&color=0f6e63"></a>
  <a href="https://github.com/abobasel86/elomar-lang/actions/workflows/ci.yml"><img alt="CI" src="https://github.com/abobasel86/elomar-lang/actions/workflows/ci.yml/badge.svg"></a>
  <a href="#license"><img alt="License" src="https://img.shields.io/badge/license-MIT%20%2F%20Apache--2.0-0f6e63"></a>
  <a href="https://github.com/sponsors/abobasel86"><img alt="Sponsor" src="https://img.shields.io/github/sponsors/abobasel86?label=Sponsor&logo=GitHub&color=ea4aaa"></a>
  <a href="https://x.com/ElomarLang"><img alt="Follow @ElomarLang on X" src="https://img.shields.io/badge/follow-%40ElomarLang-0f6e63?logo=x"></a>
</p>

<p align="center">
  <a href="https://abobasel86.github.io/elomar-lang/"><b>Tutorial (English / العربية)</b></a> ·
  <a href="https://abobasel86.github.io/elomar-lang/en/playground.html"><b>Try it in the browser</b></a> ·
  <a href="https://github.com/abobasel86/elomar-lang/releases/latest"><b>Download</b></a> ·
  <a href="https://open-vsx.org/extension/abobasel86/elomar-lang"><b>Editor extension</b></a>
</p>

<p align="center">
  <img src="docs/images/demo.gif" width="780" alt="A terminal: elomar new helpdesk --ai mock creates a project; its Ticket type and ai.infer call; elomar run prints [URGENT] billing; then an Arabic error message for a variable with an Arabic name">
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

> **Project status: v1.14.0, stable.** Programs written for 1.0 keep working in
> every 1.x version: see [`docs/stability.md`](docs/stability.md).
> A working interpreter for the language, typed AI calls (an offline **mock**
> provider, **OpenAI** and **Anthropic Claude**), agents with MCP, a standard
> library (files, JSON, HTTP client and server, SQLite, regex, time, crypto,
> Python, processes), packages with a public registry, editor support, a
> bilingual tutorial and a browser playground.
> See [`docs/grammar.md`](docs/grammar.md) (in Arabic; [English translation](docs/grammar.en.md)) and the [changelog](CHANGELOG.md).

## Vision

Elomar-Lang aims to combine predictable traditional programming with typed,
permission-aware AI inference. Effects such as printing, network access or
model calls are visible in types, can be tested offline, and are rejected
by the compiler when they are not declared.

## What works today

- Static types with local inference; no `null` (`Option<T>`), explicit errors (`Result<T, E>`).
- No semicolons needed; `/` always returns a `Float`, `div` is integer division.
- **Arabic keywords:** every keyword also has an Arabic spelling (`دالة` is
  `fn`, `ليكن` is `let`, `إذا` is `if`, `لكل … في` is `for … in`), and a
  program may start in `رئيسية`. Mix them freely; `elomar fmt --keywords ar`
  (or `en`) rewrites a file in one language. See
  [`examples/arabic-keywords`](examples/arabic-keywords) and the
  [table](https://abobasel86.github.io/elomar-lang/en/12-reference.html#keywords).
- **Arabic names for built-ins:** `اطبع` is `print`, `نص` is `String`,
  `قائمة<صحيح>` is `List<Int>`, `قيمة(x)` is `Some(x)`. Not reserved: a
  program's own name hides them. See [`examples/arabic-names`](examples/arabic-names).
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

The **[tutorial](https://abobasel86.github.io/elomar-lang/)** has twelve short
lessons in English and Arabic, from a first program to web servers, AI and
using every processor core, and a reference page with every function and
method.
Most lessons end with exercises you solve in the browser: press **Check**
and your answer is tested on several cases, with hints and the solution
when you need them. Every program in the tutorial is checked by the test
suite.

Or try it without installing anything in the
**[playground](https://abobasel86.github.io/elomar-lang/en/playground.html)**,
which runs Elomar-Lang in your browser. Its gallery has short programs
(Arabic text, JSON into your own types, dates, every processor core, typed
AI answers) that open and run with one click, each with a link you can
share.

## Getting started

Install with one line. On Windows (PowerShell):

```powershell
irm https://raw.githubusercontent.com/abobasel86/elomar-lang/main/install.ps1 | iex
```

On Linux or macOS:

```bash
curl -fsSL https://raw.githubusercontent.com/abobasel86/elomar-lang/main/install.sh | sh
```

The installer checks the download against the release's `SHA256SUMS.txt`,
puts `elomar` in `~/.elomar/bin` and adds it to your `PATH`; `elomar upgrade`
moves to a newer release later. Contributions are welcome: see
[CONTRIBUTING.md](CONTRIBUTING.md).

Or with a package manager:

```powershell
# Windows, with Scoop
scoop bucket add elomar https://github.com/abobasel86/elomar-lang
scoop install elomar
```

```bash
# macOS or Linux, with Homebrew
brew tap abobasel86/elomar https://github.com/abobasel86/elomar-lang
brew install elomar
```

Then `scoop update elomar` or `brew upgrade elomar` moves you to a new
version.

Or download `elomar` for your system from the
[latest release](https://github.com/abobasel86/elomar-lang/releases/latest):

| System | File |
|---|---|
| Windows (x64) | `elomar-<version>-x86_64-pc-windows-msvc.zip` |
| Windows (ARM) | `elomar-<version>-aarch64-pc-windows-msvc.zip` |
| Linux (x64) | `elomar-<version>-x86_64-unknown-linux-gnu.tar.gz` |
| Linux (ARM, such as Raspberry Pi) | `elomar-<version>-aarch64-unknown-linux-gnu.tar.gz` |
| macOS (Apple silicon) | `elomar-<version>-aarch64-apple-darwin.tar.gz` |
| macOS (Intel) | `elomar-<version>-x86_64-apple-darwin.tar.gz` |

Unpack it and put `elomar` (or `elomar.exe`) in a folder on your `PATH`.
`SHA256SUMS.txt` in the release lists each file's checksum. On macOS, the
binary is not signed, so the first run may need
`xattr -d com.apple.quarantine elomar`.

For the editor, search for **Elomar-Lang** in the Extensions view of VS Code
([Marketplace](https://marketplace.visualstudio.com/items?itemName=abobasel86.elomar-lang)) or of VSCodium, Cursor and other editors that use
[Open VSX](https://open-vsx.org/extension/abobasel86/elomar-lang).

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
| `elomar build FILE [-o OUT]` | Make one executable of the program, to run where Elomar-Lang is not installed |
| `--lang en\|ar` | Diagnostic language (or set `ELOMAR_LANG`) |
| `--no-color` | Plain output (also honors `NO_COLOR`) |
| `elomar check --format json [--stdin] FILE` | Diagnostics as JSON for editors (`main` not required) |
| `elomar new NAME` | Create a project with a program, a module and a passing test |
| `elomar new NAME --ai PROVIDER` | The same, calling a model, with `elomar.toml` set for `mock`, `ollama`, `openai` or `anthropic` |
| `elomar doctor` | Check the project's setup: the configuration, each model's key, network permission, server and model id, and Python, MCP servers, packages and folders |
| `elomar upgrade [--check] [--force]` | Move to the newest release with the official installer |
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
| `crates/elomar-fuzz` | Fuzz testing: mutated programs through every stage, to find crashes |
| `crates/elomar-types` | Name resolution, type inference, effect checking |
| `crates/elomar-interpreter` | Bytecode engine and tree-walking interpreter, futures, `ai.infer` execution |
| `crates/elomar-ai-core` | Schemas (shared with `std.json`), AI errors, `elomar.toml`, answer validation |
| `crates/elomar-policy` | File-system permission checks |
| `crates/elomar-ai-provider-mock` | Deterministic offline model provider |
| `crates/elomar-ai-provider-openai` | OpenAI Chat Completions provider over HTTPS |
| `crates/elomar-ai-provider-anthropic` | Anthropic Claude (Messages API) provider over HTTPS |
| `crates/elomar-cli` | The `elomar` command |
| `editor/vscode` | VS Code extension (no build step) |
| `examples/` | Programs with expected output (run by the test suite) |
| `bucket/`, `Formula/`, `packaging/` | Scoop, Homebrew and winget files, written for each release by `packaging/update.py` |
| `site/` | The website: tutorial, playground and its gallery (`site/_data/gallery.json`) |
| `tests/fail/` | Programs that must fail with specific error codes |
| `docs/grammar.md` | Authoritative grammar, semantics and error codes (version 1.0.0), in Arabic |
| `docs/grammar.en.md` | Its English translation |
| `docs/stability.md` | What 1.0 promises: programs written for 1.0 keep working in every 1.x |
| `Elomar-Lang_Master_Specification_AR.md` | Vision, architecture and roadmap (Arabic) |

## Testing

```bash
cargo test --workspace
```

Every example in `examples/` is executed through the real `elomar` binary
and compared with its `expected.out`.

Programs run in a bytecode engine: each function is compiled to register
instructions the first time it is called. The tree-walking interpreter it
replaced is still there, and the two must agree exactly: the programs in
`tests/engines/` and the examples run in both, and so does every program the
fuzzer makes. To run a program in the tree walker only:

```bash
ELOMAR_ENGINE=tree elomar run main.elo
```

`await all([...])` and the `par_` list methods (`par_map`, `par_filter`,
`par_fold`, `par_any`, ...) run isolated calls on several cores; the
comparison runs them one after another in the tree walker.
`ELOMAR_PARALLEL=0` runs them one after another in any engine.

To time the interpreter, build it in release mode and run the programs in
`bench/`:

```bash
cargo build --release -p elomar-cli && python bench/run.py
```

To look for crashes, feed the parser, checker, formatter and interpreter
mutated programs for a minute, each run in both engines (CI does this on
every change):

```bash
cargo run --profile fuzzing -p elomar-fuzz -- --seconds 60
```

## Roadmap

1. ✅ Lexer, parser, type and effect checker, interpreter, CLI.
2. ✅ Typed AI runtime: `async`/`await`, `ai.infer<T>`, validation, mock and OpenAI providers, network permissions, `net` effect.
3. ✅ v0.2: enums, `Map`, `Set`, `?`, `std.fs`, `std.json`, modules and named imports, VS Code support, `elomar new`, `elomar test`, `elomar fmt`, CI.
4. ✅ v0.3: generic functions, types and enums, closures and `map`/`filter`/`fold`, `std.http` client, traits and methods, SQLite (`std.db`), AI tools (`ai.run`), Python bridge (`std.py`), language server (`elomar lsp`), packages (`elomar add`), agents and MCP (`agent.run`, `std.mcp`), a package registry (`elomar publish`, `elomar search`).
5. ✅ v0.4–v0.5: `elomar repl`, a debugger (`elomar dap`, F5 in VS Code), new list methods, a faster interpreter.
6. ✅ v0.6–v0.7: `timeout`, folder and file functions, `std.csv`, date arithmetic, `elomar build` (one executable file).
7. ✅ v0.8: a faster interpreter (smaller values, cheaper calls, LTO builds).
8. ✅ v0.9: fuzz testing of the parser and interpreter, rename and find references in editors, the specification frozen.
9. ✅ v1.0: a stable release; programs written for 1.0 keep working in every 1.x.
10. ✅ v1.1: a bytecode engine, about twice as fast overall and up to five times on loops.
11. ✅ v1.1.1: fixes in the formatter and the checker.
12. ✅ v1.2: `await all([...])` runs isolated `async fn` calls on several processor cores, with the same results.
13. ✅ v1.3: `list.par_map(f)` computes `map` on several processor cores when `f` is isolated.
14. ✅ v1.4: `par_filter` and `par_fold` on several processor cores.
15. ✅ v1.5: `par_any`, `par_all`, `par_find`, `par_count` and `par_sort_by`; appending in place and fast string indexing.
16. ✅ v1.6: `par_flat_map`, `par_group_by`, `par_sum_by`, `par_min_by` and `par_max_by`; faster fields and records; a site with every function and a lesson on speed.
17. ✅ v1.7: `ai.stream` and `ai.embed`; `impl<T: A>`; `filter`, `map` and friends on maps and sets; the specification in English.
18. ✅ v1.8: functions with trait bounds as values; `await all` embeds many texts in one request; HTTPS behind antivirus programs and proxies.
19. ✅ v1.9: built-in functions as values, so every function is a value; `ai.search` ranks texts by meaning.
20. ✅ v1.10: models on your own computer (Ollama) with no key; an embedding cache.
21. ✅ v1.11: `elomar new --ai` and `elomar doctor`; exercises for the AI lesson.
22. ✅ v1.12: one-line installers and `elomar upgrade`; `elomar doctor` checks the whole project; contributor guides.

## Editors

`elomar lsp` speaks the Language Server Protocol on standard input/output:
diagnostics (English or Arabic with `--lang ar`), hover, go to definition,
find references, rename, parameter hints, document symbols, completion and
formatting.

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

## Sponsor

Elomar-Lang is free and open source. If it is useful to you or your company,
you can support its development through
[GitHub Sponsors](https://github.com/sponsors/abobasel86).

### Sponsors

Thank you to everyone who supports Elomar-Lang.
[Become the first sponsor!](https://github.com/sponsors/abobasel86)

<!-- sponsors: add each $25+ monthly sponsor's name or logo below this line -->

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

**التثبيت بسطر واحد:** على Windows في PowerShell:
`irm https://raw.githubusercontent.com/abobasel86/elomar-lang/main/install.ps1 | iex`،
وعلى Linux أو macOS:
`curl -fsSL https://raw.githubusercontent.com/abobasel86/elomar-lang/main/install.sh | sh`.
أو بمدير حزم: `scoop bucket add elomar https://github.com/abobasel86/elomar-lang` ثم `scoop install elomar` على Windows،
و`brew tap abobasel86/elomar https://github.com/abobasel86/elomar-lang` ثم `brew install elomar` على macOS وLinux.
ثم ابحث عن **Elomar-Lang** في قائمة الإضافات في VS Code أو VSCodium أو Cursor وثبّتها.

جرّبها دون تثبيت في [صفحة التجربة](https://abobasel86.github.io/elomar-lang/ar/playground.html)،
وفيها معرض برامج قصيرة تعمل بنقرة واحدة.

**اكتب بكلمات عربية:** لكل كلمة محجوزة كتابة عربية (`دالة` هي `fn`، و`ليكن` هي `let`،
و`إذا` هي `if`)، وللدوال والأنواع المدمجة أسماء عربية (`اطبع` هي `print`، و`نص` هي `String`)، ويجوز المزج بين اللغتين، و`elomar fmt --keywords ar` يحوّل الملف كله إلى العربية:

```elomar
دالة رئيسية() {
    لكل ع في 1..=3 {
        إذا ع == 2 { تابع }
        اطبع("العدد", ع)
    }
}
```

الجدول الكامل في [المرجع](https://abobasel86.github.io/elomar-lang/ar/12-reference.html#keywords)، وفوق كل درس
في الموقع زر يعرض الأمثلة بالكلمات العربية.

<p align="center"><img src="docs/images/demo-keywords.gif" width="780" alt="برنامج بكلمات عربية، ومخرجاته، ثم تحويله بالأمر elomar fmt --keywords en"></p>

لتشغيل مثال بالعربية من نسخة المصدر:

```bash
cargo run -p elomar-cli -- --lang ar run examples/arabic/main.elo
cargo run -p elomar-cli -- --lang ar run examples/arabic-keywords/main.elo
```

**ادعم المشروع:** Elomar-Lang مجانية ومفتوحة المصدر. إن كانت مفيدة لك أو لمؤسستك،
يمكنك دعم تطويرها عبر [GitHub Sponsors](https://github.com/sponsors/abobasel86).

المرجع النحوي الملزم: [`docs/grammar.md`](docs/grammar.md). الرؤية والخطة:
[`Elomar-Lang_Master_Specification_AR.md`](Elomar-Lang_Master_Specification_AR.md).
