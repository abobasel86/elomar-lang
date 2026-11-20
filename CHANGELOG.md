# Changelog

All notable changes to Elomar-Lang. Versions follow
[semantic versioning](https://semver.org); before 1.0, any minor version may
change the language.

## [Unreleased]

### Added

- String interpolation: `"Hello {name}, next year {age + 1}"`. Any
  expression goes between `{` and `}` and is shown as `print` shows it.
- Anthropic Claude provider: `provider = "anthropic"` in `elomar.toml`
  (default model `claude-opus-5-5`, key in `ANTHROPIC_API_KEY`), for
  `ai.infer`, `ai.run` and `agent.run`, with structured outputs, strict
  tools and the server-side fallback on refusals.
- `std.math` (constants such as `math.pi`, roots, powers, logarithms,
  trigonometry), `std.random` (`int`, `float`, `choice`, `shuffle`, `seed`),
  `std.time` (`now`, `format`, `parse`, `date`, `make`, `sleep`, `today`)
  and `std.os` (`args`, `env`, `platform`, `exit`).
- `std.regex` (`is_match`, `find`, `find_all`, `captures`, `replace`,
  `split`; literal patterns are checked when compiling, `E2023`),
  `std.encoding` (Base64, hex, URL) and `std.crypto` (`sha256`,
  `hmac_sha256`, and secure `uuid` and `token`).
- `elomar run main.elo a b` passes `a b` to the program (`os.args()`), and
  `os.exit(code)` sets the exit code.
- New methods: strings `index_of`, `slice`, `lines`, `words`,
  `trim_start`, `trim_end`, `pad_start`, `pad_end`, `reverse`; lists
  `take`, `skip`, `unique`, `min`, `max`, `enumerate`, `chunks`,
  `flat_map`, `group_by`; maps `get_or`, `entries`; numbers `pow`, `clamp`,
  and for `Float` `round_to` and `to_fixed`.

### Changed

- A `{` inside a string now starts an interpolation. Write `\{` for a
  literal brace, for example in JSON text: `"\{\"a\": 1}"`.

## [0.1.0] - 2026-11-20

The first public release: an interpreter, tools and editor support for the
language core, typed AI, and the standard library below. It is early
software. Expect rough edges and changes before 1.0.

### Language

- Static types with local inference, no `null` (`Option<T>`), explicit errors
  (`Result<T, E>`) and the `?` operator.
- Records, enums with payloads, `match` with exhaustiveness checking, lists,
  maps and sets.
- Generic functions, types and enums; traits, `impl` blocks and trait bounds.
- Closures and function values (`xs.map(|x| x * 2)`, `filter`, `fold`, ...).
- Effects (`io`, `fs`, `net`, `ai`, `db`, `py`, `mcp`), inferred for private
  functions and declared on `pub` ones.
- `async fn` and `await`.
- Multi-file programs (`use app.module`) with `pub` visibility.
- Arabic identifiers, and diagnostics in English or Arabic.

### AI

- `ai.infer<T>`: typed answers validated against `T`, with `AIError` values.
- `ai.run<T>`: the model calls only the functions you list as tools.
- `agent.run<T>`: tools of MCP servers, approval decided in code, and a log of
  every step.
- Providers: an offline mock (scripted answers, errors and latency) and OpenAI.
  Network access is denied unless `elomar.toml` allows the host.

### Standard library

- `std.fs` (files, default-deny permissions), `std.json` (typed parsing).
- `std.http` (https client with a host allowlist).
- `std.db` (SQLite with typed rows and named parameters).
- `std.py` (call Python functions; needs `python = true`).
- `std.mcp` (call tools of MCP servers listed in `elomar.toml`; MCP protocol
  2026-07-28, with fallback to older servers).

### Tools

- `elomar run`, `check`, `test`, `fmt`, `new` and `lsp`.
- Packages: `[dependencies]` from folders, git repositories or the registry,
  with semver requirements and `elomar.lock`. Commands: `elomar add`,
  `remove`, `update`, `search` and `publish`.
- The public package registry is
  [elomar-registry](https://github.com/abobasel86/elomar-registry).
- VS Code extension: highlighting, errors while typing, hover, go to
  definition, completion, a Run button and snippets.

### Known limitations

- A tree-walking interpreter: no compiler or native code yet.
- No string interpolation yet; join strings with `+`.
- MCP over stdio only (no Streamable HTTP), and no elicitation.
- One cloud provider (OpenAI).
- The VS Code extension is installed from the `.vsix` file, not from the
  Marketplace.

### بالعربية

أول إصدار عام: مفسّر اللغة، والأدوات، ودعم المحرر، والذكاء الاصطناعي ذو الأنواع
(`ai.infer` و`ai.run` و`agent.run` مع MCP)، والمكتبة القياسية (ملفات وJSON وHTTP
وSQLite وPython وMCP)، والحزم مع السجل العام. ما زال في مرحلة مبكرة وقد يتغير
قبل 1.0.
