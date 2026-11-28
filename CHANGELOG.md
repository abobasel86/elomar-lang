# Changelog

All notable changes to Elomar-Lang. Versions follow
[semantic versioning](https://semver.org); before 1.0, any minor version may
change the language.

## [Unreleased]

### Added

- **Logging in**: `server.form(req)` reads an HTML form, `server.cookies(req)`
  the cookies a browser sent, and `server.with_cookie(reply, name, value)`
  adds a cookie (HttpOnly, SameSite=Lax; an empty value deletes it; several
  cookies per reply). Arabic: `خادم.نموذج`, `خادم.ملفات_ارتباط`,
  `خادم.مع_ملف_ارتباط`.
- **`examples/login`**: a site with accounts, written all in Arabic (sign
  up, log in, a private page, log out), with passwords hashed by
  `crypto.hash_password` and sessions in SQLite, and tests that call it
  without a network.
- `elomar test` also finds tests named `اختبار_...`.

## [1.19.0] - 2026-11-28

### Added

- **Storing passwords**: `crypto.hash_password(password) -> String` and
  `crypto.verify_password(password, stored) -> Bool` (`تشفير.جزئ_كلمة_مرور`,
  `تشفير.طابق_كلمة_مرور`): Argon2id with the parameters OWASP recommends and
  a new random salt each time, in the standard PHC text form. This was the
  last item marked "planned" in the documentation.

## [1.18.0] - 2026-11-27

### Added

- **The last English names get Arabic ones**: `std` is `قياسي`
  (`استخدم قياسي.ملفات`); the types `طلب_ويب`, `رد_ويب`, `قاعدة_بيانات`,
  `مدى`, `مستقبل`, `فراغ` and the error types `خطأ_ملفات`, `خطأ_جيسون`,
  `خطأ_ويب`, `خطأ_بيانات`, `خطأ_بايثون`, `خطأ_أدوات`, `خطأ_عملية`,
  `خطأ_ذكاء`; their variants (`خطأ_ملفات.غير_موجود` is
  `FsError.NotFound`); and the fields of the records modules give
  (`الرد.الحالة` is `response.status`, `التاريخ.السنة` is `date.year`),
  on those records only, also in a record literal (`رد_ويب`). Tables:
  `elomar_ast::arabic::ERROR_VARIANTS`, `FIELD_NAMES`. Conversion and the
  editor know them.

## [1.17.0] - 2026-11-27

### Added

- **Arabic names for the standard modules, their functions and the
  effects**: `use std.ملفات` (`std.fs`), `ملفات.اقرأ_نص` (`fs.read_text`),
  `رياضيات.جذر` (`math.sqrt`), `رياضيات.باي` (`math.pi`), `وقت.الآن`
  (`time.now`), `!{ملفات, شبكة}` (`!{fs, net}`) and every other one (table:
  `elomar_ast::arabic::MODULE_NAMES`, `MODULE_FUNCTIONS`, `EFFECT_NAMES`).
  After a `use` in either language both spellings name the module; a
  program's own name hides it. `fmt --keywords`, the lesson switch and the
  playground convert them, and the editor completes and highlights them.
- The editor highlights the `mcp` and `process` effects.

## [1.16.0] - 2026-11-27

### Added

- **`elomar fmt --keywords ar|en` also converts the built-in names**:
  functions and constructors (`print` ⇄ `اطبع`, `Some` ⇄ `قيمة`), types
  (`List<Int>` ⇄ `قائمة<صحيح>`) and methods (`.length` ⇄ `.طول`,
  `.push` ⇄ `.أضف`), the methods by the type of the value. A name stays as
  written where the other spelling would mean something else (the
  program's own `نص`, a record field `طول`, an `impl` method), and the
  result is checked to name the same built-ins in the same order. Code
  that does not parse gets its keywords converted only.
- The lesson pages' switch (**«دالة ليكن إذا»**) and the playground's
  **fn ⇄ دالة** button use it, so examples turn wholly Arabic.
- `elomar_parser::parse_recording`, `elomar_types::check_recording` and
  `elomar_fmt::convert`.

## [1.15.0] - 2026-11-27

### Added

- **Arabic names for the methods of the built-in types** and for `length`:
  `.طول` (`length`), `.أضف` (`push` on a list, `add` on a set), `.حول`
  (`map`), `.رشح` (`filter`), `.يحتوي` (`contains`), `.اجلب` (`get`),
  `.شذب` (`trim`), `.قسم` (`split`) and every other one; `par_X` is
  `X_بالتوازي`. Not reserved: the type checker resolves them from the type
  of the value, after a record's own field and the program's `impl`
  methods, so no program changes meaning. A misspelt Arabic method gets
  Arabic suggestions, and the editor completes them.
  Table: `elomar_ast::arabic::METHOD_NAMES`.

## [1.14.0] - 2026-11-26

### Added

- **Arabic names for the built-in functions, constructors and core types**:
  `اطبع` (`print`), `اقرأ` (`input`), `إلى_نص`, `إلى_صحيح`, `إلى_عشري`,
  `حلل_صحيح`, `حلل_عشري`, `تأكد`, `تأكد_تساوي`, `مجموعة` (`set` and
  `Set`), `أصغر`, `أكبر`, `مطلق`, `الكل`, `مهلة`; `قيمة`/`لاشيء`
  (`Some`/`None`), `نجاح`/`فشل` (`Ok`/`Err`); and the types `صحيح`,
  `عشري`, `منطقي`, `نص`, `قائمة`, `قاموس`, `اختياري`, `نتيجة`, `مدة`.
  They are not reserved: the parser rewrites them to English
  (`elomar_ast::arabic`) except where the program's own name hides them, so
  no program changes meaning. The editor highlights and completes them.
  See `examples/arabic-names`.

### Fixed

- A string inside a list, record, `Ok(...)` or `Err(...)` is printed with
  its Arabic harakat as written (`"عددًا"`), not as `\u{64b}` escapes.
- `elomar fmt` keeps `قائمة<صحيح>` and other Arabic built-in types with
  type arguments tight.

## [1.13.1] - 2026-11-26

### Added

- **A keyword switch on every lesson page** of the website: «fn let if» or
  «دالة ليكن إذا» shows every example with English or Arabic keywords
  (remembered on the device). The playground has a `fn ⇄ دالة` button
  that rewrites the program in the editor. Both use the new
  `elomar_fmt::convert_keywords`, which changes keywords only; code with
  Arabic keywords is shown right to left.
- `elomar fmt --keywords` now also rewrites keywords inside a string's
  `{...}` (`"{self.name}"` becomes `"{هذا.name}"`).
- A second demo GIF (`docs/images/demo-keywords.gif`): a program with
  Arabic keywords, its output, and `elomar fmt --keywords en`.
  `docs/images/make_demo.py ... keywords` makes it again.

## [1.13.0] - 2026-11-26

**Upgrading:** the Arabic keywords are reserved, so a program that names a
variable, function or type `نوع`, `في`, `عام`, `هذا`, `تابع` or another word
in the table can no longer use that name. Such a program stops with
`E0101`, and the message names the keyword and suggests another name
(`نوع_`). No example or lesson uses these names.

### Added

- **Arabic keywords.** Every keyword also has an Arabic spelling: `دالة`
  (`fn`), `ليكن` (`let`), `متغير` (`var`), `أرجع` (`return`), `إذا` (`if`),
  `وإلا` (`else`), `طالما` (`while`), `لكل` (`for`), `في` (`in`), `توقف`
  (`break`), `تابع` (`continue`), `صح` (`true`), `خطأ` (`false`), `نوع`
  (`type`), `تعداد` (`enum`), `طابق` (`match`), `استخدم` (`use`), `عام`
  (`pub`), `غير_متزامن` (`async`), `انتظر` (`await`), `سمة` (`trait`),
  `نفذ` (`impl`), `هذا` (`self`), `بصفة` (`as`) and `قسمة` (`div`). Both
  spellings are the same token and can be mixed. Arabic keywords match with
  or without hamza, as `ى`/`ه` for `ي`/`ة`, and ignoring harakat, shadda and
  tatweel. Both spellings are reserved, so a name such as `نوع` now needs
  another name (the error says so).
- A program may start in `رئيسية` (`دالة رئيسية() { ... }`) when it has no
  `main`.
- `elomar fmt --keywords ar` (or `en`) writes every keyword in one language;
  without it, each keyword keeps its spelling.
- The editor completes, highlights and has snippets for Arabic keywords; the
  REPL accepts them; Arabic error messages show a keyword in both spellings
  (`` `دالة` (`fn`) ``).
- `examples/arabic-keywords`, a playground gallery program, and a test that
  runs every example rewritten with Arabic keywords in both engines.

- Links to the website show a preview when shared (X, LinkedIn, WhatsApp
  and others): every page has Open Graph and Twitter card tags with its
  title, a description in its language, and the project's image.
- **Scoop and Homebrew**: `scoop bucket add elomar https://github.com/abobasel86/elomar-lang` then
  `scoop install elomar` (Windows); `brew tap abobasel86/elomar https://github.com/abobasel86/elomar-lang`
  then `brew install elomar` (macOS, Linux). `packaging/update.py` writes
  their files (and winget's) from a release's checksums, and the release
  workflow runs it and commits the result; CI installs with both.
- **A gallery of programs** under the playground (`site/_data/gallery.json`,
  which the Examples menu also uses): each card loads and runs its program,
  and `#example=<name>` links open one directly. A test runs every program
  in both languages and checks its output.
- **A demo GIF** of real output (a new AI project, its types, its output,
  an Arabic error message) in the README and on the website;
  `docs/images/make_demo.py` makes it again.

## [1.12.1] - 2026-11-25

### Added

- **ARM builds**: Linux on ARM (`aarch64-unknown-linux-gnu`, such as
  Raspberry Pi and ARM servers) and Windows on ARM
  (`aarch64-pc-windows-msvc`), built and tested on ARM machines. The
  installers pick them on those systems.

### Fixed

- A module of your project or of a package may again be named like a
  standard module, such as `src/csv.elo` with `use app.csv` (or a package's
  `use csvkit.csv`). Since 0.7.0 added `std.csv`, `use app.csv` was refused
  everywhere as "already used by another import"; now only a file that
  imports both `std.csv` and `app.csv` is `E1002`, in either order.

- The installers and `elomar upgrade` find the latest version from where
  `github.com/.../releases/latest` redirects to, instead of the GitHub API,
  whose low limit per address failed on shared networks (HTTP 403). The
  API is still tried if the redirect gives nothing.

## [1.12.0] - 2026-11-25

### Added

- **One-line installers**: `install.ps1` (Windows) and `install.sh` (Linux,
  macOS) download the latest release, check it against `SHA256SUMS.txt`,
  put `elomar` in `~/.elomar/bin` and add it to `PATH`. CI runs them on
  Linux, macOS and Windows.
- **`elomar upgrade`** (`--check`, `--force`): moves to the newest release
  with the official installer.
- **`elomar doctor`** also checks packages, Python, the commands of MCP
  servers and of `[permissions] commands`, and the permitted folders.
- `CONTRIBUTING.md`, `SECURITY.md`, issue forms and a pull request
  template, in English and Arabic.

## [1.11.0] - 2026-11-25

### Added

- **`elomar new NAME --ai PROVIDER`** (`mock`, `ollama`, `openai`,
  `anthropic`): a project that calls a model, with `elomar.toml` set for
  that provider, and tests that pass offline.
- **`elomar doctor`**: checks the AI setup and says what to fix: the
  configuration, each model's key, network permission, whether its server
  answers, and whether the model id exists there (by listing the models,
  which costs nothing). Keys are never printed.
- Exercises for lesson 9: `ai.search`, scores, and handling a failed call.

## [1.10.0] - 2026-11-25

### Added

- **Models on your computer**: an `openai` model whose `base_url` is on
  `localhost` (such as Ollama at `http://localhost:11434/v1`) needs no
  key. `ai.infer`, `ai.stream`, `ai.embed` and `ai.search` were checked
  with Ollama. New example: `examples/ai-local`.
- **Embedding cache**: `[ai.cache] embeddings = "file.json"` keeps the
  vectors of `ai.embed` and `ai.search`, so a text already embedded is not
  sent again and does not count toward `max_calls_per_run`.

## [1.9.0] - 2026-11-25

### Added

- **Built-in functions are values**: `xs.map(to_string)`,
  `xs.fold(0, max)`, `["1", "x"].map(parse_int)` and `let say = print`.
  `all` and `timeout` work on futures and stay calls only. This was the
  last item in the specification's "not implemented yet" section.
- **`ai.search(model, query, texts[, timeout])`**: the texts with their
  similarity to the query, closest first, as
  `List<{ text: String, score: Float }>`. The query and the texts are
  embedded together, so an OpenAI model needs one request for up to 256.
- The fuzzer also builds programs with `impl<T: A>`, functions with bounds
  used as values, and built-in functions used as values, and compares the
  two engines on them.

## [1.8.0] - 2026-11-25

### Added

- **Functions with trait bounds are values**: `xs.map(show)`,
  `let f = show` and `apply(show, 3)` work when `show` is
  `fn show<T: Describe>`. The checker passes the implementations for the
  type once it is known in the same function; if it never is, the error
  is `E2020`, with the type to write.
- **`await all` embeds together**: the `ai.embed` calls in one `all` go
  to each model at once, and an OpenAI model embeds up to 256 texts in one
  request instead of one request per text. Results keep their order, and
  each call keeps its own checks and `Result`.

### Fixed

- **HTTPS behind antivirus programs and proxies**: `std.http` and the
  OpenAI and Anthropic providers now trust the system's certificates as
  well as the bundled ones. Before, a program that inspects HTTPS (such as
  Kaspersky, or a company proxy) made calls fail with `UnknownIssuer`.
- **Unknown model ids**: OpenAI's 404 `model_not_found` is now
  `UnsupportedCapability` instead of `ProviderUnavailable`, and an error
  about the model id ends with a hint to check `model` in `elomar.toml`.

## [1.7.0] - 2026-11-24

### Added

- **`ai.stream(model, instructions, input, on_text)`**: the answer piece
  by piece as the model writes it. `on_text` gets each piece as it
  arrives, while the provider goes on reading on its own thread; the
  result is the whole text. OpenAI and Claude stream for real
  (`"stream": true`); the mock model streams one word at a time. If
  `on_text` fails, the stream stops and its error is the program's.
- **`ai.embed(model, text)`** gives a text's embedding, a
  `List<Float>` whose direction stands for its meaning, and
  **`ai.similarity(a, b)`** compares two (the cosine, from -1 to 1).
  OpenAI models use `/embeddings`; the mock model makes a deterministic
  64-number vector from the words, so semantic search works offline in
  tests; Claude has no embeddings and gives `UnsupportedCapability`.
  `ai.similarity` is plain arithmetic, so it runs inside `par_` methods.
  `examples/ai-stream` shows both.
- **`impl<T: A>`**: a generic impl can require a trait of its type
  parameter, and its methods may call that trait's methods on `T`. Such
  an impl can be passed wherever its trait is needed, nested too
  (`Box<Box<Int>>`): the implementations for `T` go with it. A `T`
  without the trait is error `E2019`.
- **Maps and sets** have `filter`, `any`, `all` and `count` (with
  `f(key, value)` for a map), `map_values` on maps and `map` on sets, in
  key order.

- **The specification in English**: `docs/grammar.en.md` translates
  `docs/grammar.md` section by section; the English pages of the site link
  to it. Stale passages in both were corrected: named functions are
  values, every effect has its sources, and `async fn` runs on several
  cores since 1.2.
- Lesson 11 of the site has three exercises (`par_map`, `par_max_by` and
  `par_sum_by`) and shows how to work on maps and sets on several cores
  through `entries()`, `keys()`, `values()` and `to_list()`.

### Fixed

- `elomar fmt` wrote `impl < T > Stack<T>` with spaces once the impl had
  a bound; the brackets after `impl` are now always kept tight.
- The specification numbered two sections 4.6.1 and put 4.12.1 and 4.13
  after 4.16; the sections are now in order, in both languages.

## [1.6.0] - 2026-11-24

### Added

- **`par_flat_map`, `par_group_by`, `par_sum_by`, `par_min_by` and
  `par_max_by`**: what `flat_map`, `group_by`, `sum_by`, `min_by` and
  `max_by` give, with `f` run on every item on several processor cores
  under the same rule as `par_map`, then the method finished in order.
  When an item fails in `par_sum_by`, the items are gone through again in
  order, so the error is `sum_by`'s (an overflow before that item comes
  first). In `bench/par_more.elo`, over 300,000 numbers, they are 5 to 9
  times faster (`par_sum_by` of Collatz steps: 140 ms instead of 1248 ms).
- **The site has everything.** A lesson, *Speed and parallelism*, shows
  every `par_` method, which functions run on the cores, `await all`,
  writing fast code and `ELOMAR_PARALLEL` / `ELOMAR_ENGINE`. A reference
  page lists every built-in function, every method of every type, every
  function of the standard modules and every command, in English and
  Arabic, and a test fails when one is added without a line there. A
  *What's new* page tells what each version brought.

### Changed

- Reading a record's field remembers where the field was found, so the
  next read with the same instruction looks there first, and a record is
  built with one allocation instead of two: reading the sixth of six
  fields a million times takes 20 ms instead of 29 ms, and building
  300,000 records 18 ms instead of 23 ms (`bench/calls.elo`). Calling a
  compiled closure passes its arguments straight to its registers.

### Testing

- A fifth of the programs the fuzzer makes are written for the `par_`
  methods: random lists, conditions true here and there, failing at one
  item or never ending after another, and functions that print. Each runs
  on every core and on one, and the two must agree: about 9,000 such
  programs ran without a difference.

## [1.5.0] - 2026-11-24

### Added

- **`par_any`, `par_all`, `par_find`, `par_count` and `par_sort_by`**: what
  `any`, `all`, `find`, `count` and `sort_by` give, with `f` run on several
  processor cores under the same rule as `par_map`. `par_any`, `par_all`
  and `par_find` look in all the parts at once and stop the parts after
  the first that finds the item; nothing after that item changes the
  result, so an error or an endless loop there does not show, as `find`
  never reaches it. `par_sort_by` computes the keys on the cores and keeps
  the sort stable. In `bench/par_find.elo`, over 300,000 numbers, they are
  5 to 7 times faster (`par_sort_by` with Collatz keys: 152 ms instead of
  1040 ms).

### Changed

- `s = s + t` and `s += t` append to a string or list in place when no
  other value shares it, instead of copying it each time: building a
  470 KB string in a loop takes 9 ms instead of 571 ms.
- `.length` and `slice` on a long string remember where its visible
  characters start, so a loop over a string by index is no longer
  quadratic: going through 40,000 characters with `s.slice(i, i + 1)`
  takes 5 ms instead of 37 s. `bench/data.elo` times text, maps, JSON and
  SQLite.

## [1.4.0] - 2026-11-24

### Added

- **`xs.par_filter(f)`**: the list `xs.filter(f)` gives, with `f` run on
  several processor cores under the same rule as `par_map`. In
  `bench/par_filter_fold.elo`, finding the primes up to 300,000 takes
  70 ms instead of 434 ms.
- **`xs.par_fold(init, f)`**, where `f` takes and returns the item type:
  the list is split into at most 64 neighbouring parts, which depend on
  its length alone. The first part is folded from `init` and each other
  part from its first item, on several cores, and the parts' results are
  then folded in order with `f`. The result is the same on every machine,
  with or without threads, and is `fold`'s when `f` is associative (such
  as `+`, `max` or joining strings). The longest Collatz path up to
  300,000 takes 533 ms instead of 4669 ms.

## [1.3.0] - 2026-11-24

### Added

- **`xs.par_map(f)`**: the list `xs.map(f)` gives, computed on several
  processor cores. The list is split into parts that run in threads when
  `f` is isolated (the same rule as in `await all`: a named function that
  is not `async`, or a closure whose body is isolated); otherwise it runs
  like `map`. If items fail, the error is the first one's, and the parts
  after it are stopped. In `bench/par_map.elo`, 300,000 Collatz counts
  take 193 ms instead of 1199 ms one after another, and 7453 ms in the
  tree walker of 1.0.

### Changed

- The GitHub workflows use actions that run on Node.js 24.

## [1.2.0] - 2026-11-23

### Faster

- **`await all([...])` uses several processor cores.** Neighbouring calls
  to `async fn`s that are isolated run at the same time, each in its own
  interpreter on its own thread: eight computations in `bench/parallel.elo`
  take 139 ms instead of 337 ms (1.1) and 1787 ms (1.0).
- A call is isolated when it, and every function it may call, prints
  nothing, reads no input, and uses no file, database, process, Python,
  MCP, AI model or random number; it may compute and make HTTP requests.
  Its arguments hold no futures and no functions that are not isolated.
- The results are the ones running the calls one after another gives: in
  the same order; if one fails, the first failure in the list is the
  error, and the calls after it are stopped and left not started; the
  call depth limit and `timeout` around `all` apply in the threads. Other
  calls still run one after another, as before.
- `ELOMAR_PARALLEL=0` runs every call one after another. The comparison of
  the two engines now runs the bytecode engine on several cores and the
  tree walker one call after another; `tests/engines/parallel.elo` covers
  failures, endless calls, `timeout` and closures as results.

## [1.1.1] - 2026-11-23

### Fixed

- `elomar fmt` wrote a function type inside type arguments with spaces, as
  in `List < fn() -> Int >`; it now keeps `List<fn() -> Int>`.
- `group_by` with a `Bool` key passed the checker, then stopped the program
  with an internal error: map keys are `String` or `Int`. It is now error
  `E2005` when the program is checked.

## [1.1.0] - 2026-11-23

### Faster

- **A bytecode engine.** The first time a function is called, it is compiled
  into instructions over numbered registers: a variable is read by its
  number instead of being searched for by name, and loops are jumps.
  Closures are compiled too. On the programs in `bench/` (made ten times
  larger), it is about twice as fast as 1.0 overall: loops about 5 times,
  function calls about 2 times.
- Nothing else changes: programs print the same and fail with the same
  errors, at the same places. The tree-walking interpreter is still there,
  for the debugger, and to check the engine against: `ELOMAR_ENGINE=tree`
  runs a program in it only.

### Testing

- The fuzzer runs every program in both engines and reports any difference
  in output or errors, and `tests/engines/` holds programs for the cases
  where the two could differ (closures, the order things run in, errors and
  their call traces, `break` in nested loops, `timeout`).

## [1.0.0] - 2026-11-22

The first stable version. From now on, a program that works with 1.0 keeps
working with every 1.x version; [`docs/stability.md`](docs/stability.md)
says exactly what is promised. The specification
([`docs/grammar.md`](docs/grammar.md)) is version `1.0.0`.

1.0 is 0.9.0 with this promise: no change to the language or the library.
What came together on the way here:

- **The language:** typed, with records, enums, generics, traits, closures,
  `Option`/`Result` and `?`, pattern matching, `async`/`await` with `all`
  and `timeout`, effects (`!{fs, net, ai}`), modules and packages, and names
  in Arabic or English, with every error explained in both languages.
- **AI in the language:** `ai.infer<T>` checked against your types, tools,
  agents and MCP, with an offline model, OpenAI and Claude.
- **The standard library:** files and folders, JSON, CSV, HTTP client and
  server, SQLite, regex, dates, math, crypto, Python and other programs, all
  behind permissions in `elomar.toml`.
- **Tools:** `elomar run`, `check`, `test`, `fmt`, `new`, `repl`, `build`
  (one executable to share), packages with a public registry, a language
  server and a debugger, and the VS Code extension.
- **Learning:** a bilingual tutorial with exercises checked in the browser,
  and a playground.
- **Reliability:** about a million generated programs run without a crash,
  and fuzz testing on every change.

## [0.9.0] - 2026-11-22

### Editors

- Find all references (Shift+F12), rename (F2) and parameter hints while
  typing a call, in `elomar lsp` and the VS Code extension. Rename changes
  every file of the project and refuses names that are not valid.

### Reliability

- `elomar-fuzz` feeds the parser, checker, formatter and interpreter
  mutated programs and reports any crash; CI runs it for a minute on every
  change. About a million programs ran without a crash after these fixes:
  - an unknown type in `ai.infer<...>` crashed the checker; it is an error;
  - `pad_start` / `pad_end` to billions of letters ran out of memory; like
    `repeat`, more than 100 million letters is error `R0006`.
- The browser playground stops a program after 8 seconds with error
  `R0013`, keeping what it printed, instead of losing the output.

### Specification

- The specification is frozen at `1.0.0-rc.1` (`elomar --version`), and
  [`docs/stability.md`](docs/stability.md) states what 1.0 promises:
  programs written for 1.0 keep working in every 1.x.

### Performance

- Programs run about 1.25 times faster than 0.7.0 (`python bench/run.py`:
  505 ms to about 405 ms in all), and function calls about 1.5 times
  faster. Values take half the memory (24 bytes), calls reuse the memory
  of finished calls, a call's variables live in one list instead of one
  per block, `x += 1` finds `x` once, record field names are shared, and
  release builds are optimized across crates (LTO), which also makes
  `elomar` smaller (10 MB).

## [0.7.0] - 2026-11-22

### Tools

- `elomar build main.elo -o app.exe`: one executable file with the program,
  its modules, its packages and `elomar.toml` inside. It runs on computers
  without Elomar-Lang, with every argument going to the program. Programs
  with errors are not built. The executable is for the system it was built
  on; in it, relative paths start at the folder it is run from.

### Language

- `timeout(future, limit)`: `await timeout(slow(), 5s)` is `Some(result)`, or
  `None` when the time ran out; the work is then stopped at its next
  statement or loop step, and `time.sleep` ends early.
- `()` is another way to write the type `Unit` (`Result<(), FsError>`).

### Standard library

- `std.fs`: `create_dir`, `remove`, `remove_dir`, `rename`, `copy`, `is_dir`
  and `size`. A folder listed in `[permissions]` itself cannot be deleted.
- `std.csv`: `parse`, `parse_records` (rows as maps by column name) and
  `stringify`, with quoted fields as in RFC 4180.
- `std.time`: `format_as(t, "DD/MM/YYYY HH:mm")`, `add_days`, `add_months`
  and `days_between`.

### Website

- Interactive exercises at the end of lessons 1–5, 7 and 10 (16 in all),
  in English and Arabic. **Check** tests the answer in the browser on
  several cases and shows what was expected and what came back; hints and
  the solution are there when needed. Answers and progress are kept in the
  browser, and finished lessons are ticked in the lesson list.

## [0.5.0] - 2026-11-22

### Tools

- A debugger: `elomar dap` speaks the Debug Adapter Protocol, and the VS
  Code extension uses it for **F5**. Breakpoints, step over / into / out,
  the call stack, variables (lists, records and maps open up), variables
  and fields in the Debug Console and on hover, and a stop at the statement
  where a runtime error happens. Output goes to the Debug Console, where
  `input(...)` also reads. **Ctrl+F5** still runs the file in a terminal.

### Performance

- Programs run about 1.6 times faster overall (1.4 to 2 times on the
  programs in `bench/`): a faster memory allocator, variables that no longer
  copy their names, closures that share their captured variables between
  calls instead of copying them, and short paths for the most common
  statements and integer operations. `python bench/run.py` times them.

## [0.4.0] - 2026-11-22

### Language

- List methods `zip` (pairs, as long as the shorter list), `windows(n)`,
  `sum_by(f)` (`Int` or `Float`), `min_by(f)` and `max_by(f)` (the first
  smallest or largest key).

### Tools

- `elomar repl`: an interactive prompt. Expressions show their value;
  variables, functions and types stay for the next lines, `let x` may be
  typed again, `await` works directly, and errors point into the line you
  typed. Commands: `:help`, `:show`, `:reset`, `:quit`. Lines are kept in
  `~/.elomar_history`.

### Fixes

- The HTTP server test no longer hangs when another test takes its port.

### Website

- The collections lesson shows the new list methods.
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
