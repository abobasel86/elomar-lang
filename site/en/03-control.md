---
layout: default
title: Decisions, loops and functions
lang: en
slug: 03-control
---

# 3. Decisions, loops and functions

## if and else

```elomar
fn main() {
    let temperature = 31
    if temperature > 30 {
        print("hot")
    } else if temperature > 15 {
        print("nice")
    } else {
        print("cold")
    }
}
```

Conditions must be `Bool`. Combine them with `&&` (and), `||` (or) and `!`
(not).

## match: choosing between many cases

```elomar
fn main() {
    let day = "Fri"
    let kind = match day {
        "Fri" => "weekend"
        "Sat" => "weekend"
        _ => "workday"
    }
    print(kind)
}
```

`_` matches anything else. A `match` must cover every case; the checker
tells you when one is missing. Use `match` on a `Bool` to choose between two
values:

```elomar
fn main() {
    let age = 20
    print(match age >= 18 {
        true => "adult"
        false => "minor"
    })
}
```

Several cases can share an arm with `|`, and whole numbers can match a
range:

```elomar
fn main() {
    let day = "Sat"
    print(match day {
        "Fri" | "Sat" => "weekend"
        _ => "workday"
    })
    let score = 84
    print(match score {
        90..=100 => "excellent"
        70..90 => "good"
        _ => "keep going"
    })
}
```

`70..90` includes 70 but not 90; `90..=100` includes both ends.

## Loops

```elomar
fn main() {
    for i in 1..4 {          // 1, 2, 3
        print(i)
    }
    for i in 1..=3 {         // 1, 2, 3 (includes the end)
        print(i * i)
    }
    for fruit in ["apple", "fig"] {
        print("I like {fruit}")
    }
    var n = 3
    while n > 0 {
        print(n)
        n -= 1
    }
}
```

`break` leaves a loop and `continue` skips to the next round.

## Functions

```elomar
fn area(width: Float, height: Float) -> Float {
    return width * height
}

fn greet(name: String) {
    print("Hello, {name}")
}

fn main() {
    print(area(2.0, 3.5))
    greet("Omar")
}
```

Parameters always have types. `-> Float` is the type of the result; a
function without `->` returns nothing.

### Calling with names

Arguments can be named, which helps when there are several:

```elomar
fn order(item: String, count: Int) -> String {
    return "{count} x {item}"
}

fn main() {
    print(order(count: 2, item: "coffee"))
}
```

### Arabic names

Your own names can be Arabic:

```elomar
fn المساحة(الطول: Float, العرض: Float) -> Float {
    return الطول * العرض
}

fn main() {
    print(المساحة(3.0, 4.0))
}
```

## Exercise

Write `fn grade(score: Int) -> String` that returns `"excellent"` from 90,
`"good"` from 70, `"pass"` from 50 and `"fail"` below. Print the grade for
95, 72 and 40.
