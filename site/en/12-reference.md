---
layout: default
title: Reference
lang: en
slug: 12-reference
---

# 12. Reference

Every built-in function, every method of every type, and every function of
the standard modules, in one page. The lessons show them at work; the
[language reference](https://github.com/abobasel86/elomar-lang/blob/main/docs/grammar.en.md)
gives every detail and error code. A method marked * changes the value, so
the value must be in a `var`.

## Built-in functions

| Function | What it does |
|---|---|
| `print(x, ...)` | Prints the values on one line, separated by spaces |
| `input()` / `input(prompt)` | Reads a line; `""` at the end of the input |
| `to_string(x)` | Any value as text |
| `to_int(x)` / `to_float(x)` | A number as `Int` (drops the fraction) or `Float` |
| `parse_int(s)` / `parse_float(s)` | Text as a number: `Option<Int>` / `Option<Float>` |
| `min(a, b)` / `max(a, b)` / `abs(x)` | The smaller, the larger, the absolute value |
| `set(list)` | A `Set` of the list's items |
| `assert(cond[, msg])` / `assert_eq(actual, expected)` | Stops the program when a check fails (for tests) |
| `all([futures])` | Waits for several `async` calls: `await all([...])` |
| `timeout(future, limit)` | `await timeout(work, 2s)`: `Some(result)`, or `None` when time ran out |

## Every type

| Method | What it gives |
|---|---|
| `to_string()` | The value as text, as `print` shows it |

## `List<T>`

| Method | What it does |
|---|---|
| `.length` · `is_empty()` | The number of items; whether there are none |
| `push(x)`* · `pop()`* | Adds at the end; removes the last item: `Option<T>` |
| `insert(i, x)`* · `remove(i)`* | Adds at place `i`; removes the item at place `i` and gives it |
| `contains(x)` · `index_of(x)` | Whether it holds `x`; where: `Option<Int>` |
| `first()` · `last()` | `Option<T>` |
| `take(n)` · `skip(n)` · `slice(start, end)` | The first `n`, all but the first `n`, a part |
| `reverse()` · `sorted()` · `unique()` | Reversed; sorted (numbers or text); each item once |
| `sum()` · `min()` · `max()` | Of numbers (`min` and `max` also of text): `max()` is `Option<T>` |
| `join(sep)` | A `List<String>` as one text |
| `map(f)` · `filter(f)` | Each item through `f`; the items for which `f` is `true` |
| `any(f)` · `all(f)` · `count(f)` · `find(f)` | Whether some or all items pass; how many; the first: `Option<T>` |
| `fold(init, f)` | Combines the items from `init`: `f(acc, item)` |
| `sort_by(f)` · `min_by(f)` · `max_by(f)` | Sorted by the key `f(item)` (stable); the item with the smallest or largest key |
| `sum_by(f)` | The sum of `f(item)` (`Int` or `Float`) |
| `flat_map(f)` | The lists `f` gives, joined into one |
| `group_by(f)` | A `Map` from each key (`Int` or `String`) to the items with it |
| `enumerate()` | `{ index, value }` records |
| `zip(other)` | Pairs `(a, b)`, as long as the shorter list |
| `chunks(n)` · `windows(n)` | Parts of `n` items; every run of `n` neighbours |
| `par_map` · `par_filter` · `par_fold` · `par_any` · `par_all` · `par_find` · `par_count` · `par_sort_by` · `par_min_by` · `par_max_by` · `par_sum_by` · `par_flat_map` · `par_group_by` | The same as without `par_`, on several cores ([lesson 11](11-speed.html)) |

## `String`

| Method | What it does |
|---|---|
| `.length` · `is_empty()` | The number of visible characters; whether it is `""` |
| `upper()` · `lower()` | In capitals; in small letters |
| `trim()` · `trim_start()` · `trim_end()` | Without spaces around it, at the start, at the end |
| `contains(s)` · `starts_with(s)` · `ends_with(s)` · `index_of(s)` | Searching; `index_of` gives `Option<Int>` |
| `slice(start, end)` · `chars()` · `reverse()` | A part, the characters, reversed (by visible characters) |
| `split(sep)` · `lines()` · `words()` | Into a `List<String>` by `sep`, by lines, by spaces |
| `replace(a, b)` · `repeat(n)` | Every `a` as `b`; `n` times |
| `pad_start(n[, fill])` · `pad_end(n[, fill])` | Filled to `n` characters, with spaces or `fill` |
| `parse_int()` · `parse_float()` | As a number: `Option` |

## `Int` and `Float`

| Method | What it does |
|---|---|
| `abs()` · `pow(n)` · `clamp(min, max)` | Absolute value; power; kept between `min` and `max` (both types) |
| `to_float()` | An `Int` as `Float` |
| `to_int()` · `round()` · `floor()` · `ceil()` | A `Float` as `Int`: dropping the fraction, nearest, down, up |
| `sqrt()` · `round_to(digits)` · `to_fixed(digits)` | Square root; rounded to `digits` places; as text with exactly `digits` places |

## `Duration`

Written `500ms`, `10s`, `2m`, `1h`.

| Method | What it gives |
|---|---|
| `millis()` · `seconds()` | The duration in milliseconds (`Int`), in seconds (`Float`) |

## `Option<T>` and `Result<T, E>`

| Method | What it does |
|---|---|
| `is_some()` · `is_none()` · `is_some_and(f)` | Whether there is a value (that passes `f`) |
| `is_ok()` · `is_err()` | Whether it is `Ok` or `Err` |
| `unwrap_or(x)` · `unwrap_or_else(f)` | The value, or `x`, or what `f` gives |
| `map(f)` · `and_then(f)` | The value changed by `f`; `f` giving another `Option` or `Result` |
| `filter(f)` · `or(other)` | (`Option`) `None` unless `f` passes; this or `other` |
| `ok_or(e)` | (`Option`) As a `Result`, with `e` for `None` |
| `map_err(f)` · `ok()` · `err()` | (`Result`) The error changed by `f`; the value or the error as `Option` |

## `Map<K, V>` and `Set<T>`

| Method | What it does |
|---|---|
| `m[k]` · `get(k)` · `get_or(k, default)` | The value of a key: stops if missing; `Option<V>`; or `default` |
| `contains_key(k)` · `remove(k)`* | Whether the key is there; removes it: `Option<V>` |
| `keys()` · `values()` · `entries()` | Lists of keys, values, and `{ key, value }` records |
| `.length` · `is_empty()` | (Both) The number of entries; whether there are none |
| `contains(x)` · `add(x)`* · `remove(x)`* | (`Set`) Whether it holds `x`; adds; removes: `Bool` |
| `union(s)` · `intersection(s)` · `difference(s)` · `is_subset(s)` | (`Set`) Set operations |
| `to_list()` | (`Set`) The items as a sorted `List` |
| `filter(f)` · `any(f)` · `all(f)` · `count(f)` | (Both) With `f(key, value)` for a map, `f(item)` for a set; `filter` keeps the type |
| `map_values(f)` | (`Map`) Every value changed by `f` |
| `map(f)` | (`Set`) Every item through `f`, as a set (`Int` or `String` items) |

## Errors

`AIError`, `FsError`, `JsonError`, `HttpError`, `DbError`, `PyError`,
`McpError` and `ProcessError` are enums; each case carries a message.

| Method | What it gives |
|---|---|
| `kind()` · `message()` | The name of the case, as `"NotFound"`; the message |

## Standard modules

Bring a module in with `use std.<name>`. Modules that touch the world need
permission in `elomar.toml` ([lesson 7](07-data.html)).

| Module | Functions |
|---|---|
| `std.math` | `math.sqrt` · `math.cbrt` · `math.pow` · `math.exp` · `math.ln` · `math.log10` · `math.log2` · `math.sin` · `math.cos` · `math.tan` · `math.asin` · `math.acos` · `math.atan` · `math.atan2` · `math.hypot` · `math.is_nan`; the values `math.pi`, `math.e`, `math.tau`, `math.inf` |
| `std.random` | `random.int(min, max)` · `random.float()` · `random.bool()` · `random.choice(list)` · `random.shuffle(list)` · `random.seed(n)` |
| `std.time` | `time.now()` (ms since 1970) · `time.today()` · `time.sleep(d)` · `time.format(t)` · `time.format_as(t, pattern)` · `time.parse(text)` · `time.date(t)` · `time.make(y, m, d, ...)` · `time.add_days(t, n)` · `time.add_months(t, n)` · `time.days_between(a, b)` |
| `std.os` | `os.args()` · `os.env(name)` · `os.platform()` · `os.exit(code)` |
| `std.fs` | `fs.read_text` · `fs.write_text` · `fs.append_text` · `fs.exists` · `fs.list_dir` · `fs.is_dir` · `fs.size` · `fs.create_dir` · `fs.remove` · `fs.remove_dir` · `fs.rename` · `fs.copy` (each gives `Result<_, FsError>`) |
| `std.json` | `json.parse<T>(text)` · `json.stringify(value)` · `json.pretty(value)` |
| `std.csv` | `csv.parse(text)` · `csv.parse_records(text)` · `csv.stringify(rows)` |
| `std.db` | `db.open(path)` · `db.execute(conn, sql[, params])` · `db.query<T>(conn, sql[, params])` · `db.query_one<T>(conn, sql[, params])` (SQLite) |
| `std.http` | `http.get(url)` · `http.post(url, body[, content_type])` |
| `std.server` | `server.serve(address, handler[, max_requests])` · `server.text(status, body)` · `server.html(status, body)` · `server.json(status, value)` · `server.redirect(url)` |
| `std.regex` | `regex.is_match` · `regex.find` · `regex.find_all` · `regex.captures` · `regex.replace` · `regex.split` (pattern first, then the text) |
| `std.encoding` | `encoding.base64_encode` · `encoding.base64_decode` · `encoding.hex_encode` · `encoding.hex_decode` · `encoding.url_encode` · `encoding.url_decode` |
| `std.crypto` | `crypto.sha256(text)` · `crypto.hmac_sha256(key, message)` · `crypto.uuid()` · `crypto.token(bytes)` |
| `std.process` | `process.run(command, args[, timeout])` |
| `std.py` | `py.call<T>(module, function, args[, timeout])` |
| `std.ai` | `ai.infer<T>(model, instructions, input[, timeout])` · `ai.run<T>(model, instructions, input, tools, ...)` · `ai.stream(model, instructions, input, on_text[, timeout])` · `ai.embed(model, text[, timeout])` · `ai.similarity(a, b)` · `ai.search(model, query, texts[, timeout])` |
| `std.agent` | `agent.run<T>(model, instructions, input, tools, mcp, approve, ...)` |
| `std.mcp` | `mcp.tools(server)` · `mcp.call(server, tool, args)` |

## Commands

| Command | What it does |
|---|---|
| `elomar run file.elo` | Check and run a program |
| `elomar check file.elo` | Only check |
| `elomar repl` | Type code and see the result at once |
| `elomar fmt` | Format in the standard style |
| `elomar test` | Run a project's tests |
| `elomar new name` | Create a project |
| `elomar new name --ai PROVIDER` | Create a project that calls a model: `mock`, `ollama`, `openai` or `anthropic` |
| `elomar doctor` | Check the setup: AI keys, permissions, servers and model ids; Python, MCP servers, packages, folders |
| `elomar upgrade [--check]` | Move to the newest release (or only say whether there is one) |
| `elomar build file.elo` | Make an executable |
| `elomar add` · `elomar remove` · `elomar update` · `elomar search` · `elomar publish` | Packages ([lesson 6](06-projects.html)) |
| `elomar lsp` · `elomar dap` | The language server and the debugger, for editors |
| `elomar --lang ar ...` | Messages in Arabic (or set `ELOMAR_LANG=ar`) |
