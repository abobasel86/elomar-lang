---
layout: default
title: Values, variables and text
lang: en
slug: 02-basics
---

# 2. Values, variables and text

## Names for values

`let` gives a name to a value. It cannot change afterwards:

```elomar
fn main() {
    let city = "Amman"
    let year = 2026
    print(city, year)
}
```

When a value must change, use `var`:

```elomar
fn main() {
    var score = 10
    score = score + 5
    score += 1
    print(score)
}
```

Using `let` by default makes programs easier to follow: you know a value
stays the same.

## The basic types

| Type | Examples | Notes |
|---|---|---|
| `Int` | `42`, `-7` | whole numbers |
| `Float` | `3.14`, `2.0` | decimal numbers |
| `Bool` | `true`, `false` | yes or no |
| `String` | `"hello"`, `"مرحبا"` | text |
| `Duration` | `500ms`, `10s`, `2m` | lengths of time |

You rarely write types for variables; Elomar-Lang works them out. You can
still write them when it helps the reader:

```elomar
fn main() {
    let price: Float = 9.5
    let count: Int = 3
    print(price * to_float(count))
}
```

## Numbers

```elomar
fn main() {
    print(7 / 2)        // 3.5: `/` always gives a Float
    print(7 div 2)      // 3: whole-number division
    print(7 % 2)        // 1: remainder
    print(2.pow(10))    // 1024
    print(3.14159.round_to(2), 2.5.round())
}
```

An `Int` that grows too big is an error, never a silent wrong answer.

## Text

```elomar
fn main() {
    let name = "Elomar"
    print(name.length)                  // 6
    print(name.upper(), name.lower())
    print("  spaces  ".trim())
    print("a,b,c".split(","))           // ["a", "b", "c"]
    print(name.contains("lo"), name.starts_with("E"))
    print("ab".repeat(3), name.slice(0, 3))
}
```

`length` counts the letters you see, so `"مرحبا".length` is 5.

### Putting values into text

```elomar
fn main() {
    let item = "tea"
    let cups = 3
    print("{cups} cups of {item} cost {cups * 2} dinars")
}
```

To write a real brace in text, put a backslash before it: `"\{"`. That is
useful for JSON:

```elomar
fn main() {
    let n = 5
    print("\{\"count\": {n}\}")
}
```

## Converting

```elomar
fn main() {
    print(to_string(42) + "!")
    print("12".parse_int())       // Some(12)
    print("abc".parse_int())      // None: not a number
    print(to_int(3.9), to_float(2))
}
```

`parse_int` returns an `Option`: either `Some(number)` or `None`. Lesson 5
shows how to handle both.

## Comments

```elomar
// A comment to the end of the line.
/* A comment
   over several lines. */
fn main() {
    print("comments are ignored")
}
```
