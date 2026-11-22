# Changelog

All notable changes to Elomar-Lang. Versions follow
[semantic versioning](https://semver.org); before 1.0, any minor version may
change the language.

## [Unreleased]

### Tools

- `elomar repl`: an interactive prompt. Expressions show their value;
  variables, functions and types stay for the next lines, `let x` may be
  typed again, `await` works directly, and errors point into the line you
  typed. Commands: `:help`, `:show`, `:reset`, `:quit`. Lines are kept in
  `~/.elomar_history`.

### Website

- The first lesson shows `elomar repl` and how to install the extension
  from Open VSX.

## [0.3.0] - 2026-11-21

### Language

- `Option` methods `map`, `and_then`, `filter`, `is_some_and`,
  `unwrap_or_else`, `or` and `ok_or`, and `Result` methods `map`,
  `map_err`, `and_then`, `unwrap_or_else`, `ok` and `err`:
  `"12".parse_int().map(|n| n * 2).unwrap_or(0)`.
- Tuples: `(1, "a")`, the type `(Int, String)`, `t.0`, patterns `(x, _)`,
  `let (a, b) = pair` and `for (k, v) in map` (also over lists of tuples).
  Tuples compare and sort element by element, so `sort_by(|p| (p.age,
  p.name))` works.
- Default method bodies in traits, used by every `impl` that does not
  define the method.
- Generic impls: `impl<T> Stack<T> { ... }` and `impl<T> Describe for
  Box<T>`.
- `await all([...])` waits for a list of futures and returns their results
  in order. `ai.infer` calls passed directly are sent at the same time
  (OpenAI and Anthropic), after the permission and budget checks; other
  futures run one after another.
- Pattern alternatives and ranges in `match`: `"Fri" | "Sat" => ...` and
  `90..=100 => ...`. Alternatives bind no names (`E2024`); an empty range
  is `E2025`.
- `elomar fmt` lines a closing bracket up with the line that opened it,
  as in `print(match x {` ... `}, y)`.

### Website

- The tutorial covers pattern alternatives and ranges, `Option` and
  `Result` methods, tuples, default trait methods, generic impls and
  `await all`, in English and Arabic.

## [0.2.0] - 2026-11-21

A larger language and standard library: string interpolation, a web
server, Claude, and modules for math, time, text patterns, encoding,
hashing and running programs; plus a tutorial website, a browser
playground and the first registry packages.

### Language

- String interpolation: `"Hello {name}, next year {age + 1}"`. Any
  expression goes between `{` and `}` and is shown as `print` shows it.
- Raw strings: `r"\d{2,4}"` and `r#"say "hi""#` keep every character as
  written (no escapes, no interpolation), for regular expressions and
  Windows paths.
- A `match` arm may be `return x`, `break` or `continue`:
  `None => return 0`.
- `match` coverage understands nested patterns (`Ok(Some(x))`, `Ok(None)`,
  `Err(e)`) and names the exact missing case.
- A line that starts with a binary operator that cannot begin an
  expression (`||`, `&&`, `+`, `*`, `==`, `<`, ...) continues the previous
  line. Before, the line `|| b` after `let ok = a` was silently ignored.
- New methods: strings `index_of`, `slice`, `lines`, `words`,
  `trim_start`, `trim_end`, `pad_start`, `pad_end`, `reverse`; lists
  `take`, `skip`, `unique`, `min`, `max`, `enumerate`, `chunks`,
  `flat_map`, `group_by`; maps `get_or`, `entries`; numbers `pow`, `clamp`,
  and for `Float` `round_to` and `to_fixed`.

### AI

- Anthropic Claude provider: `provider = "anthropic"` in `elomar.toml`
  (default model `claude-opus-5-5`, key in `ANTHROPIC_API_KEY`), for
  `ai.infer`, `ai.run` and `agent.run`, with structured outputs, strict
  tools and the server-side fallback on refusals.

### Standard library

- `std.server`: an HTTP server for web APIs and pages.
  `server.serve(address, handler)` calls `fn(HttpRequest) -> HttpResponse`
  for each request, with `server.text`, `html`, `json` and `redirect`
  helpers. It listens only on addresses in `[permissions] listen`. See
  `examples/web-api`.
- `std.process`: `process.run(command, args[, timeout])` runs a program
  listed in `[permissions] commands`, without a shell, and returns its exit
  status and output. New effect `process`, new error `ProcessError`.
- `std.math` (constants such as `math.pi`, roots, powers, logarithms,
  trigonometry), `std.random` (`int`, `float`, `choice`, `shuffle`, `seed`),
  `std.time` (`now`, `format`, `parse`, `date`, `make`, `sleep`, `today`)
  and `std.os` (`args`, `env`, `platform`, `exit`).
- `std.regex` (`is_match`, `find`, `find_all`, `captures`, `replace`,
  `split`; literal patterns are checked when compiling, `E2023`),
  `std.encoding` (Base64, hex, URL) and `std.crypto` (`sha256`,
  `hmac_sha256`, and secure `uuid` and `token`).

### Tools

- `elomar run main.elo a b` passes `a b` to the program (`os.args()`), and
  `os.exit(code)` sets the exit code.
- `elomar fmt` indents a line that starts with `||`, `&&` or another
  binary operator one level deeper, and indents the contents of several
  brackets opened on one line (`f([`) once instead of once per bracket.
- The first registry packages: `textkit` (text helpers), `csvkit` (CSV)
  and `validator` (input checks). `elomar add textkit` installs one.
- The VS Code extension is ready for the Marketplace and Open VSX: an
  icon, store metadata, and a workflow that checks it with `vsce` and
  publishes it on version tags once the store tokens are set
  (`docs/releasing.md`).

### Website

- A tutorial in English and Arabic (`site/`, published to GitHub Pages):
  ten lessons whose programs are type-checked by the test suite.
- A playground: write and run Elomar-Lang in the browser, with examples and
  share links. The interpreter is compiled to WebAssembly
  (`crates/elomar-playground`); files, databases, HTTP, servers, Python and
  processes are not available there, and AI calls use `mock`. The
  interpreter's SQLite and HTTP support is now behind a `native` feature
  (on for the `elomar` command).

### Changed

- A `{` inside a string now starts an interpolation. Write `\{` for a
  literal brace, for example in JSON text: `"\{\"a\": 1}"`, or use a raw
  string.

### بالعربية

إدراج القيم في النصوص (`"مرحبا {name}"`) والنصوص الخام، وخادم ويب (`std.server`)، ومزود
Claude، ووحدات للرياضيات والوقت والأعداد العشوائية والتعابير النمطية والترميز والتشفير وتشغيل
البرامج ومعاملات سطر الأوامر، ودوال جديدة للنصوص والقوائم والقواميس. ومعها موقع دروس بالعربية
والإنجليزية، وساحة تجربة تشغّل اللغة في المتصفح، وأول ثلاث حزم في السجل العام. تنبيه: `{` داخل
النص صار يبدأ إدراجًا، فاكتب `\{` للقوس الحرفي.

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
