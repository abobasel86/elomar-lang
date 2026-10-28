# Elomar-Lang

**A general-purpose programming language designed for AI-native applications.**

> **Project status: Phase 2 — early development. Not production-ready.**
> A working interpreter exists for the language core and for typed AI calls
> against an offline **mock** provider and **OpenAI**, as described in
> [`docs/grammar.md`](docs/grammar.md). No modules, packages, file or process
> permissions, or wider standard library yet.

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
- Effect inference with mandatory declarations on `pub` functions.
- `async fn` / `await` (sequential, lazy futures).
- **Typed AI calls:** `ai.infer<T>(...)` returns `Result<T, AIError>` (`AIError` is an enum, e.g. `AIError.Timeout(msg)`); answers are
  validated against `T` (missing, extra or mistyped fields become
  `InvalidStructuredOutput`), and calling a model is the `ai` effect.
- An offline **mock provider** configured in `elomar.toml`: canned answers,
  simulated errors and latency, call budgets — deterministic and free.
- An **OpenAI provider** (structured outputs) with the key read from an
  environment variable, a default-deny `[permissions] network_hosts`
  allowlist, and the `net` effect required for cloud models. See
  [`examples/ai-openai`](examples/ai-openai).
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

## Getting started

Requires a recent stable Rust toolchain.

```bash
cargo build --release
```

```bash
cargo run -p elomar-cli -- run examples/hello/main.elo
```

```bash
cargo run -p elomar-cli -- --lang ar check tests/fail/unknown_name.elo
```

| Command | Purpose |
|---|---|
| `elomar run FILE` | Check, then run a program |
| `elomar check FILE` | Report errors and warnings without running |
| `--lang en\|ar` | Diagnostic language (or set `ELOMAR_LANG`) |
| `--no-color` | Plain output (also honors `NO_COLOR`) |

## Repository layout

| Path | Contents |
|---|---|
| `crates/elomar-diagnostics` | Spans, bilingual diagnostics, rendering |
| `crates/elomar-lexer` | Tokens and newline rules |
| `crates/elomar-ast` | Syntax tree |
| `crates/elomar-parser` | Recursive-descent + Pratt parser |
| `crates/elomar-types` | Name resolution, type inference, effect checking |
| `crates/elomar-interpreter` | Tree-walking interpreter, futures, `ai.infer` execution |
| `crates/elomar-ai-core` | Schemas (shared with `std.json`), AI errors, `elomar.toml`, answer validation |
| `crates/elomar-policy` | File-system permission checks |
| `crates/elomar-ai-provider-mock` | Deterministic offline model provider |
| `crates/elomar-ai-provider-openai` | OpenAI Chat Completions provider over HTTPS |
| `crates/elomar-cli` | The `elomar` command |
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

## Roadmap

1. ✅ Lexer, parser, type and effect checker, interpreter, CLI.
2. ✅ Typed AI runtime: `async`/`await`, `ai.infer<T>`, validation, mock and OpenAI providers, network permissions, `net` effect.
3. 🟡 v0.2: ✅ enums, `Map`, `?`, `std.fs`, `std.json`, CI; ⏳ `Set`, modules, formatter, package tooling.
4. HTTP/database support, Python bridge, controlled AI tools and agents.
5. Bytecode VM, performance work and stable releases.

## License

To be determined before any public release. Apache-2.0 is the proposed
option, subject to ownership and licensing review. Until a `LICENSE` file is
added, no license is granted.

---

## بالعربية

**Elomar-Lang** لغة برمجة عامة الأغراض، أصلية التكامل مع الذكاء الاصطناعي. المفسّر الحالي
ينفذ نواة اللغة: الأنواع والاستنتاج، والسجلات، و`match`، والتأثيرات، ورسائل الأخطاء
بالعربية والإنجليزية. لتشغيل مثال بالعربية:

```bash
cargo run -p elomar-cli -- --lang ar run examples/arabic/main.elo
```

المرجع النحوي الملزم: [`docs/grammar.md`](docs/grammar.md). الرؤية والخطة:
[`Elomar-Lang_Master_Specification_AR.md`](Elomar-Lang_Master_Specification_AR.md).
