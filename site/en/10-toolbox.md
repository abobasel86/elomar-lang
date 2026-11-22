---
layout: default
title: The toolbox and next steps
lang: en
slug: 10-toolbox
---

# 10. The toolbox and next steps

The standard library has a module for most everyday jobs. Bring one in with
`use std.<name>`.

## Math, random numbers and time

```elomar
use std.math
use std.random
use std.time

fn main() {
    print(math.pi.round_to(4), math.sqrt(2), math.pow(2, 8))
    print(random.int(1, 6), random.choice(["red", "green"]))

    let start = time.make(2026, 1, 1)
    print(time.format(start))             // 2026-01-01T00:00:00Z
    print(time.date(start).weekday)       // 4 (Thursday; Monday is 1)
}
```

Times are `Int` milliseconds since 1970 (UTC). `time.parse` reads ISO 8601
text and returns an `Option`:

```elomar
use std.time

fn main() {
    match time.parse("2026-03-21T10:30:00+03:00") {
        Some(t) => print(time.format(t))   // 2026-03-21T07:30:00Z
        None => print("not a date")
    }
    print(time.today())
}
```

Dates can be written your way and moved by days or months:

```elomar
use std.time

fn main() {
    let t = time.make(2026, 1, 31, 9, 30)
    print(time.format_as(t, "DD/MM/YYYY HH:mm"))           // 31/01/2026 09:30
    print(time.format_as(time.add_months(t, 1), "DD/MM"))  // 28/02: the month's last day
    print(time.days_between(t, time.add_days(t, 10)))      // 10
}
```

## Time limits

`timeout` stops work that takes too long. `await timeout(work, 2s)` is
`Some(result)`, or `None` when the time ran out:

```elomar
async fn count_forever() -> Int {
    var n = 0
    while true {
        n += 1
    }
    return n
}

async fn main() {
    match await timeout(count_forever(), 200ms) {
        Some(n) => print("finished: {n}")
        None => print("stopped after 200ms")
    }
}
```

## Text patterns, encodings and hashing

```elomar
use std.regex
use std.encoding
use std.crypto

fn main() {
    print(regex.find_all("[0-9]+", "rooms 12 and 7"))
    print(regex.replace(r"(\w+)@", "write to lina@example.com", "***@"))
    print(encoding.base64_encode("مرحبا"))
    print(encoding.url_encode("a b&c"))
    print(crypto.sha256("hello"))
    print(crypto.uuid())
}
```

Write patterns as *raw strings*, `r"..."`: inside them a backslash and
`{` mean themselves, so `r"\d{2,4}"` is exactly the pattern. (In a normal
string, `{` starts interpolation and a backslash must be doubled.)

## The command line

```elomar
use std.os

fn main() {
    let args = os.args()
    if args.length == 0 {
        print("usage: elomar run tool.elo <name>")
        os.exit(2)
    }
    print("Hello, {args[0]}")
    print(os.env("HOME"))
}
```

`elomar run tool.elo Lina` passes `["Lina"]`; `os.exit(2)` ends the program
with exit code 2.

## Running other programs

```elomar
use std.process

fn main() {
    match process.run("git", ["log", "--oneline", "-3"]) {
        Ok(out) => print(out.stdout)
        Err(e) => print(e.kind(), e.message())
    }
}
```

Programs must be listed in `elomar.toml` (`commands = ["git"]` under
`[permissions]`). They run without a shell, so arguments are never treated
as commands.

## Python libraries

```text
py.call<Float>("statistics", "mean", [1, 2, 3])
```

`std.py` calls Python functions, with typed results, after you allow it
with `python = true`.

## Where to go next

- The [language reference](https://github.com/abobasel86/elomar-lang/blob/main/docs/grammar.md)
  lists every function, type and error code.
- The [examples](https://github.com/abobasel86/elomar-lang/tree/main/examples)
  are complete programs, each tested on every change.
- Questions, ideas and bugs are welcome in the
  [issue tracker](https://github.com/abobasel86/elomar-lang/issues).
