---
layout: default
title: Files, JSON and databases
lang: en
slug: 07-data
---

# 7. Files, JSON and databases

## Permissions first

A program may only use the files you allow in `elomar.toml`, next to your
program:

```toml
[permissions]
filesystem_read = ["data"]
filesystem_write = ["output"]
```

Anything else is refused with an error value. This protects you from a
program (or a package, or an AI model) touching files it should not.

## Files

```elomar
use std.fs

fn main() {
    match fs.write_text("output/note.txt", "first line\n") {
        Ok(_) => print("saved")
        Err(e) => print("cannot save: {e.message()}")
    }
    match fs.read_text("data/students.txt") {
        Ok(text) => print(text.lines())
        Err(e) => print(e.kind())    // NotFound, PolicyDenied, ...
    }
}
```

Other functions: `fs.append_text`, `fs.exists` and `fs.list_dir`. Every
function that touches files has the `fs` *effect*: a public function that
reads files must say so with `!{fs}` in its signature, so readers know.

## JSON

`json.parse<T>` reads JSON and checks that it has the shape of `T`:

```elomar
use std.json

type City = { name: String, people: Int, tags: List<String> }

fn main() {
    let text = "\{\"name\": \"Amman\", \"people\": 4000000, \"tags\": [\"hills\"]\}"
    match json.parse<City>(text) {
        Ok(city) => print(city.name, city.people)
        Err(e) => print("bad JSON: {e.message()}")
    }
    let back = json.stringify({ ok: true, items: [1, 2] })
    print(back)
}
```

## Databases (SQLite)

```elomar
use std.db

type Book = { id: Int, title: String, year: Int }

fn main() {
    let conn = match db.open(":memory:") {
        Ok(c) => c
        Err(e) => {
            print("cannot open: {e.message()}")
            return
        }
    }
    db.execute(conn, "CREATE TABLE books (id INTEGER PRIMARY KEY, title TEXT, year INT)")
    db.execute(conn, "INSERT INTO books (title, year) VALUES (:t, :y)", { t: "Dune", y: 1965 })
    match db.query<Book>(conn, "SELECT * FROM books WHERE year < :before", { before: 2000 }) {
        Ok(books) => print(books)
        Err(e) => print(e.message())
    }
}
```

Values go in through named parameters like `:t`, never by joining text into
SQL, so SQL injection is not possible. A file database (`db.open("app.db")`)
needs that file in the read and write permissions.
