---
layout: default
title: What's new
lang: en
other: ar/news.html
---

# What's new

## Every core, the same answers

*Elomar-Lang 1.1 to 1.6: how the language became many times faster without
changing what a single program prints.*

A year's worth of speed came in a few weeks of versions. The rule behind
all of it is simple: **a faster program must print exactly what the slower
one printed, and fail with exactly the same error at the same place.**
Nothing you write has to change to get the speed.

### 1.1: a bytecode engine

Each function is now compiled into instructions over numbered registers
the first time it is called. A variable is read by its number instead of
being searched for by name, and a loop is a jump. Programs run about twice
as fast as in 1.0, loops about five times.

### 1.2: `await all` on every core

`await all([...])` runs neighbouring `async` calls at the same time, each
on its own core, when they are **isolated**: they print nothing, read no
input, and touch no file, database, process or AI model. Eight heavy
computations take 139 ms instead of 1787 ms in 1.0.

### 1.3 to 1.6: the `par_` methods

A list method whose name starts with `par_` gives what the same method
without it gives, with the work split over the cores:

| Over 300,000 numbers | One core | Every core |
|---|---|---|
| `par_map` (Collatz steps) | 1199 ms | 193 ms |
| `par_filter` (primes) | 434 ms | 70 ms |
| `par_fold` (longest path) | 4669 ms | 533 ms |
| `par_count` (primes) | 298 ms | 55 ms |
| `par_sort_by` (Collatz keys) | 1040 ms | 152 ms |
| `par_sum_by` (Collatz steps) | 1248 ms | 140 ms |
| `par_max_by` (Collatz steps) | 1252 ms | 149 ms |

There are thirteen of them now: `par_map`, `par_filter`, `par_fold`,
`par_any`, `par_all`, `par_find`, `par_count`, `par_sort_by`,
`par_min_by`, `par_max_by`, `par_sum_by`, `par_flat_map` and
`par_group_by`. [Speed and parallelism](11-speed.html) shows them all.

### How the answers stay the same

- Only isolated functions run on the cores; anything else runs one item
  after another, so what it prints keeps its order.
- When several items fail, the error is the first one's, and the parts
  after it are stopped. `par_find` stops at the first part that finds its
  item, so an error or an endless loop after it never shows, just as
  `find` would never reach it.
- `par_fold` splits a list into parts that depend on its length alone, so
  it gives the same result on every computer.
- A fuzzer writes thousands of programs aimed at the `par_` methods, with
  conditions that fail at one item or never end after another, and runs
  each on every core and on one: in more than 9,000 such programs, the two
  never differed.

### Text and records, too

- 1.5: `s += t` appends in place, so building a large text in a loop takes
  9 ms instead of 571 ms; going through a long text by index with `slice`
  takes 5 ms instead of 37 seconds, Arabic text with marks included.
- 1.6: reading a field remembers where the field was, and records are
  built with one allocation: reading the sixth field of a record a million
  times takes 20 ms instead of 29 ms.

## Versions

| Version | What it brought |
|---|---|
| **1.6** (next) | `par_flat_map`, `par_group_by`, `par_sum_by`, `par_min_by`, `par_max_by`; faster fields and records; the [reference](12-reference.html) and [speed](11-speed.html) pages |
| 1.5 | `par_any`, `par_all`, `par_find`, `par_count`, `par_sort_by`; appending in place; fast indexing into text |
| 1.4 | `par_filter` and `par_fold` |
| 1.3 | `par_map` |
| 1.2 | `await all` on every core |
| 1.1.1 | Fixes in `elomar fmt` and `group_by` |
| 1.1 | The bytecode engine |
| 1.0 | The first stable version: programs that work with 1.0 keep working with every 1.x |

Every change, in detail, is in the
[changelog](https://github.com/abobasel86/elomar-lang/blob/main/CHANGELOG.md).
