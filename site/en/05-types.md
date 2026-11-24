---
layout: default
title: Your own types and errors
lang: en
slug: 05-types
---

# 5. Your own types and errors

## Records

A record groups named values. Give its shape a name with `type`:

```elomar
type Student = { name: String, grade: Int }

fn describe(s: Student) -> String {
    return "{s.name} has {s.grade}"
}

fn main() {
    let lina: Student = { name: "Lina", grade: 91 }
    print(describe(lina))
    let better = { name: lina.name, grade: lina.grade + 5 }
    print(better)
}
```

## Enums: one of several kinds

```elomar
enum Shape {
    Circle(Float)
    Rect(Float, Float)
    Dot
}

fn area(s: Shape) -> Float {
    return match s {
        Shape.Circle(r) => 3.14159 * r * r
        Shape.Rect(w, h) => w * h
        Shape.Dot => 0.0
    }
}

fn main() {
    for s in [Shape.Circle(1.0), Shape.Rect(2.0, 3.0), Shape.Dot] {
        print(area(s))
    }
}
```

If you add a new kind of shape, every `match` that forgets it becomes an
error, so nothing is missed.

## No null: Option

A value that may be missing has the type `Option<T>`: `Some(value)` or
`None`. You must handle both:

```elomar
fn find_age(name: String) -> Option<Int> {
    return match name {
        "Lina" => Some(30)
        _ => None
    }
}

fn main() {
    match find_age("Lina") {
        Some(age) => print("age {age}")
        None => print("unknown")
    }
    print(find_age("Omar").unwrap_or(0))
}
```

Instead of a `match`, the methods of `Option` often say it shorter:

```elomar
fn main() {
    let age = "42".parse_int()
    print(age.map(|n| n + 1))                  // Some(43)
    print(age.filter(|n| n >= 18).is_some())   // true
    print("x".parse_int().unwrap_or_else(|| 0))
}
```

## Errors as values: Result

A function that can fail returns `Result<T, E>`: `Ok(value)` or
`Err(error)`. Nothing is thrown behind your back:

```elomar
fn parse_age(text: String) -> Result<Int, String> {
    return match text.parse_int() {
        Some(n) => match n >= 0 {
            true => Ok(n)
            false => Err("age cannot be negative")
        }
        None => Err("'{text}' is not a number")
    }
}

fn main() {
    for t in ["42", "-1", "abc"] {
        match parse_age(t) {
            Ok(age) => print("ok: {age}")
            Err(why) => print("error: {why}")
        }
    }
}
```

## The ? operator

Inside a function that returns `Result`, `?` returns the error early, or
gives you the value:

```elomar
fn parse_age(text: String) -> Result<Int, String> {
    return match text.parse_int() {
        Some(n) => Ok(n)
        None => Err("'{text}' is not a number")
    }
}

fn total_age(a: String, b: String) -> Result<Int, String> {
    let x = parse_age(a)?
    let y = parse_age(b)?
    return Ok(x + y)
}

fn main() {
    print(total_age("20", "22"))     // Ok(42)
    print(total_age("20", "x"))      // Err("'x' is not a number")
}
```

`Result` has the same kind of methods: `map`, `map_err`, `and_then`,
`unwrap_or_else`, `ok` and `err`.

## Leaving early from a match

An arm can `return`:

```elomar
fn double(text: String) -> Int {
    let n = match text.parse_int() {
        Some(n) => n
        None => return 0
    }
    return n * 2
}

fn main() {
    print(double("21"), double("?"))
}
```

## Nested patterns

Patterns can look inside each other:

```elomar
fn show(r: Result<Option<Int>, String>) -> String {
    return match r {
        Ok(Some(n)) => "found {n}"
        Ok(None) => "nothing"
        Err(e) => "failed: {e}"
    }
}

fn main() {
    print(show(Ok(Some(3))), show(Ok(None)), show(Err("disk")))
}
```

## Tuples: several values together

A tuple groups a few values without naming them. It is handy for
returning two results:

```elomar
fn min_max(xs: List<Int>) -> (Int, Int) {
    return (xs.min().unwrap_or(0), xs.max().unwrap_or(0))
}

fn main() {
    let (low, high) = min_max([4, 9, 2])
    print(low, high)
    let pair = ("Lina", 91)
    print(pair.0, pair.1)
    for (name, grade) in ["Lina": 91, "Omar": 55] {
        print("{name}: {grade}")
    }
}
```

When the values have meanings worth naming, a record is clearer.

## Traits: shared behavior

```elomar
trait Describe {
    fn describe(self) -> String
}

type Book = { title: String }

impl Describe for Book {
    fn describe(self) -> String {
        return "the book {self.title}"
    }
}

fn main() {
    let b: Book = { title: "Dune" }
    print(b.describe())
}
```

A trait can give a method a default body. Each `impl` gets it unless it
writes its own:

```elomar
trait Greet {
    fn name(self) -> String
    fn greet(self) -> String {
        return "Hello, {self.name()}!"
    }
}

type Person = { first: String }

impl Greet for Person {
    fn name(self) -> String {
        return self.first
    }
}

fn main() {
    let p: Person = { first: "Lina" }
    print(p.greet())
}
```

Methods can also belong to generic types:

```elomar
type Stack<T> = { items: List<T> }

impl<T> Stack<T> {
    fn push(self, x: T) -> Stack<T> {
        return { items: self.items + [x] }
    }
    fn top(self) -> Option<T> {
        return self.items.last()
    }
}

fn main() {
    let s: Stack<String> = { items: [] }
    print(s.push("a").push("b").top())
}
```

A generic `impl` can ask its type parameter for a trait, with `impl<T:
Describe>`. Its methods may then use the trait's methods on `T`, and the
type works wherever the trait is needed:

```elomar
trait Describe {
    fn describe(self) -> String
}

type Box<T> = { item: T }

impl Describe for Int {
    fn describe(self) -> String {
        return "the number {self}"
    }
}

impl<T: Describe> Describe for Box<T> {
    fn describe(self) -> String {
        return "a box holding {self.item.describe()}"
    }
}

fn show<T: Describe>(x: T) -> String {
    return x.describe()
}

fn main() {
    let b: Box<Int> = { item: 7 }
    print(b.describe())     // a box holding the number 7
    let bb: Box<Box<Int>> = { item: b }
    print(show(bb))         // a box holding a box holding the number 7
}
```

A function with bounds, like `show`, can be passed as a value too:
`[1, 2].map(show)` gives `["the number 1", "the number 2"]`.

