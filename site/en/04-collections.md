---
layout: default
title: Lists, maps and sets
lang: en
slug: 04-collections
---

# 4. Lists, maps and sets

## Lists

A list keeps values of one type, in order:

```elomar
fn main() {
    let grades = [90, 75, 60]
    print(grades[0], grades.length)
    print(grades.first(), grades.last())     // Some(90) Some(60)
    print(grades.contains(75), grades.sorted())

    var names: List<String> = []
    names.push("Lina")
    names.push("Omar")
    print(names, names.join(" & "))
}
```

Reading past the end, like `grades[9]`, stops the program with a clear
error. `first()` and `last()` return an `Option` instead, which is safe for
empty lists.

## Doing something with every item

Functions written inline with `|...|` are *closures*:

```elomar
fn main() {
    let prices = [12.5, 3.0, 8.25]
    print(prices.map(|p| p * 2.0))
    print(prices.filter(|p| p > 5.0))
    print(prices.sum(), prices.max())
    print(prices.fold(0.0, |total, p| total + p))
    print(prices.any(|p| p > 10.0), prices.all(|p| p > 1.0))
    print(prices.find(|p| p < 5.0))
    print(prices.sort_by(|p| -p))
}
```

More useful methods: `take(n)`, `skip(n)`, `unique()`, `reverse()`,
`enumerate()`, `chunks(n)`, `flat_map(f)` and `group_by(f)`:

```elomar
fn main() {
    let words = ["apple", "avocado", "banana", "cherry"]
    for item in words.enumerate() {
        print(item.index, item.value)
    }
    print(words.group_by(|w| w.slice(0, 1)))
    print([1, 2, 3, 4, 5].chunks(2))
}
```

Pair two lists with `zip`, look at neighbors with `windows(n)`, and add up
or pick by a key with `sum_by`, `max_by` and `min_by`:

```elomar
type Student = { name: String, grade: Int }

fn main() {
    let names = ["Lina", "Omar"]
    let grades = [91, 55]
    for (name, grade) in names.zip(grades) {
        print("{name}: {grade}")
    }
    print([3, 5, 9].windows(2))              // [[3, 5], [5, 9]]

    let class: List<Student> = [{ name: "Lina", grade: 91 }, { name: "Omar", grade: 55 }]
    print(class.sum_by(|s| s.grade))         // 146
    print(class.max_by(|s| s.grade).map(|s| s.name))   // Some("Lina")
}
```

For heavy work on a long list, `par_map(f)` gives the same list as
`map(f)`, but splits the work over the processor's cores. It does so only
when `f` just computes: it prints nothing, reads no input, and uses no
files, databases or random numbers. Otherwise it runs like `map`, so the
result never changes:

```elomar
fn steps(n: Int) -> Int {
    var x = n
    var count = 0
    while x != 1 {
        if x % 2 == 0 {
            x = x div 2
        } else {
            x = 3 * x + 1
        }
        count += 1
    }
    return count
}

fn main() {
    var numbers: List<Int> = []
    for i in 1..=100000 {
        numbers.push(i)
    }
    print(numbers.par_map(steps).sum())   // like map, several times faster
}
```

In the same way, `par_filter(f)` gives what `filter(f)` gives.
`par_fold(init, f)` folds a list in parts on the cores, then folds the
parts' results together, so `f` takes two items and returns one of the
same type. When `f` gives the same result however the items are grouped,
as adding, `max` or joining strings do, it is the result of `fold`:

```elomar
fn main() {
    var numbers: List<Int> = []
    for i in 1..=100000 {
        numbers.push(i)
    }
    let sevens = numbers.par_filter(|n| n % 7 == 0)
    print(sevens.par_fold(0, |a, b| a + b))   // 714264285
}
```

`par_any`, `par_all`, `par_find`, `par_count`, `par_sort_by`,
`par_flat_map`, `par_group_by`, `par_sum_by`, `par_min_by` and
`par_max_by` likewise give what the same methods without `par_` give, with
the work split over the cores. [Speed and parallelism](11-speed.html) says
which functions can run on the cores and how to make programs fast.

## Maps

A map connects keys to values:

```elomar
fn main() {
    var stock = ["apples": 10, "pears": 4]
    stock["figs"] = 7
    stock["apples"] = stock["apples"] - 1
    print(stock["apples"], stock.length)
    print(stock.get("kiwis"))              // None: not there
    print(stock.get_or("kiwis", 0))        // 0
    for key in stock.keys() {
        print("{key}: {stock[key]}")
    }
}
```

`stock["kiwis"]` on a missing key stops the program; use `get` or `get_or`
when a key may be missing. An empty map is written `[:]`.

Maps have `filter`, `any`, `all` and `count` with a function of the key
and the value, and `map_values` changes every value:

```elomar
fn main() {
    let prices = ["tea": 4, "coffee": 7, "juice": 5]
    print(prices.filter(|name, price| price > 4))   // ["coffee": 7, "juice": 5]
    print(prices.map_values(|price| price * 2))     // ["coffee": 14, "juice": 10, "tea": 8]
    print(prices.any(|name, price| price > 6), prices.count(|name, price| name.length > 3))   // true 2
}
```

## Sets

A set holds each value once:

```elomar
fn main() {
    var seen = set(["a", "b"])
    seen.add("a")
    seen.add("c")
    print(seen, seen.contains("b"))
    let other = set(["c", "d"])
    print(seen.union(other), seen.intersection(other))
}
```

Sets have `filter`, `map`, `any`, `all` and `count` too; `map` gives a
set, so equal results are kept once:

```elomar
fn main() {
    let numbers = set([12, 7, 30, 9])
    print(numbers.filter(|n| n % 2 == 0))   // set([12, 30])
    print(numbers.map(|n| n % 3))           // set([0, 1])
}
```
