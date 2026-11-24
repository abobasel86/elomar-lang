---
layout: default
title: Speed and parallelism
lang: en
slug: 11-speed
---

# 11. Speed and parallelism

Most programs are fast without any effort: `elomar` compiles each function
into bytecode the first time it is called, and runs it in a fast engine.
This lesson shows how to use **all the cores** of your processor, and how
to write code that stays fast as the data grows.

## Many lists, one rule: `par_`

A list method whose name starts with `par_` gives **exactly** what the same
method without `par_` gives, but splits the list into parts and runs the
function on several cores at once:

```elomar
fn collatz(n: Int) -> Int {
    var x = n
    var steps = 0
    while x != 1 {
        if x % 2 == 0 {
            x = x div 2
        } else {
            x = 3 * x + 1
        }
        steps += 1
    }
    return steps
}

fn main() {
    var numbers: List<Int> = []
    for i in 1..=100000 {
        numbers.push(i)
    }
    print(numbers.par_map(collatz).take(5))         // [0, 1, 7, 2, 5]
    print(numbers.par_count(|n| collatz(n) > 200))  // 4001
    print(numbers.par_max_by(collatz))              // Some(77031)
    print(numbers.par_find(|n| collatz(n) > 300))   // Some(26623)
    print(numbers.par_sum_by(collatz))              // 10753840
}
```

On a computer with 8 cores, each of these lines runs 5 to 9 times faster
than with `map`, `count`, `max_by`, `find` and `sum_by`.

| Method | Gives what this gives |
|---|---|
| `par_map(f)` | `map(f)` |
| `par_filter(f)` | `filter(f)` |
| `par_count(f)` | `count(f)` |
| `par_any(f)` / `par_all(f)` | `any(f)` / `all(f)` |
| `par_find(f)` | `find(f)` |
| `par_sort_by(f)` | `sort_by(f)` (the sort is stable) |
| `par_min_by(f)` / `par_max_by(f)` | `min_by(f)` / `max_by(f)` |
| `par_sum_by(f)` | `sum_by(f)` |
| `par_flat_map(f)` | `flat_map(f)` |
| `par_group_by(f)` | `group_by(f)` |
| `par_fold(init, f)` | `fold(init, f)` when `f` is associative (see below) |

The results are the same in every case: the same order, the same items,
and the same error. If the function fails on several items, the error is
the one of the **first** of them, as without `par_`. `par_find`, `par_any`
and `par_all` stop the parts after the first part that finds what it looks
for, so an item after it cannot change the result, just as `find` would
never have reached it.

## What runs on the cores

The work goes to the cores only when it cannot be told apart from running
one item after another. The function must be **isolated**: it, and every
function it calls,

- prints nothing and reads no input;
- uses no file, database, process, Python, MCP tool or AI model;
- uses no random numbers.

It may compute, use text, lists, maps, records and enums, and the modules
`std.math`, `std.json`, `std.regex`, `std.time` and the like. A function
that is not isolated still works: the method then runs one item after
another, in order, so what it prints keeps its order.

```elomar
fn noisy(n: Int) -> Int {
    print("working on {n}")
    return n * n
}

fn main() {
    print([1, 2, 3].par_map(noisy))   // prints in order, then [1, 4, 9]
}
```

Use `par_` methods for **heavy** work on each item. For cheap work, such as
`x * 2`, splitting the list costs more than it saves.

## Folding in parts: `par_fold`

`par_fold(init, f)` splits the list into at most 64 neighbouring parts,
which depend on the length of the list alone, folds each part on a core,
then folds the parts' results together with `f`. So `f` takes two items and
returns an item of the same type. When `f` gives the same result however
the items are grouped, as adding, `max` or joining text do, the result is
the one of `fold`:

```elomar
fn main() {
    let words = ["one", "two", "three", "four"]
    print(words.par_fold("", |a, b| a + b))   // onetwothreefour
    print([4, 9, 2].par_fold(0, |a, b| a + b))  // 15
}
```

The result is the same on every computer, with any number of cores.

## Many calls at once: `await all`

`await all([...])` waits for several `async fn` calls. Neighbouring calls
that are isolated run on several cores at the same time; the results come
back in the order of the list:

```elomar
async fn total(from: Int, to: Int) -> Int {
    var sum = 0
    for i in from..to {
        sum += i % 7
    }
    return sum
}

async fn main() {
    let parts = await all([total(0, 100000), total(100000, 200000), total(200000, 300000)])
    print(parts)   // [299995, 299999, 300003]
}
```

`ai.infer` calls in `all` also run together, as the
[AI lesson](09-ai.html) shows.

## Writing fast code

- **Build text and lists by adding to them.** `text += piece` and
  `list.push(item)` add in place when nothing else holds the same value,
  so a loop that builds a large text stays fast.
- **Index text freely.** `s.slice(i, i + 1)` and `s.length` remember where
  the characters of a long text start, so going through a text by index is
  fast, Arabic text with marks included.
- **Use maps to look things up.** `counts[word]` finds a key at once,
  however large the map is.
- **Measure.** `time.now()` gives the time in milliseconds:

```elomar
use std.time

fn main() {
    let start = time.now()
    var text = ""
    for i in 0..20000 {
        text += "{i},"
    }
    print("{text.length} characters in {time.now() - start} ms")
}
```

## Switches for checking

Two environment variables change how programs run. They never change what
a program prints:

| Variable | What it does |
|---|---|
| `ELOMAR_PARALLEL=0` | Run everything on one core, one item after another |
| `ELOMAR_ENGINE=tree` | Run in the simpler tree-walking interpreter instead of the bytecode engine |

Both are useful to compare times, or to be sure that a result does not
depend on the cores. In PowerShell: `$env:ELOMAR_PARALLEL = "0"`; in a
Unix shell: `ELOMAR_PARALLEL=0 elomar run main.elo`.
