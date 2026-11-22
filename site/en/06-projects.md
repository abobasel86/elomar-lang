---
layout: default
title: Projects, modules and tests
lang: en
slug: 06-projects
---

# 6. Projects, modules and tests

## A new project

```text
elomar new school
cd school
elomar run src/main.elo
elomar test
```

This creates:

```text
school/
├── elomar.toml               settings and permissions
├── src/main.elo              the program
├── src/greeting.elo          a module
└── tests/greeting_test.elo   tests
```

## Modules

Each file in `src/` is a module. Only items marked `pub` can be used from
other files:

```elomar
// src/grading.elo
pub fn level(grade: Int) -> String {
    return match grade >= 90 {
        true => "excellent"
        false => "keep going"
    }
}
```

```elomar
// src/main.elo
use app.grading

fn main() {
    print(grading.level(95))
}
```

You can also import names directly: `use app.grading.{level}` and then call
`level(95)`.

## Tests

Every `.elo` file in `tests/` holds tests: functions whose names start with
`test_`:

```elomar
// tests/grading_test.elo
use app.grading

fn test_high_grades_are_excellent() {
    assert_eq(grading.level(95), "excellent")
}

fn test_low_grades() {
    assert(grading.level(10) != "excellent")
}
```

`elomar test` runs them all and shows only the failures. Use
`elomar test --filter high` to run some of them.

## Formatting

`elomar fmt` rewrites your files in one standard style (4 spaces, spaces
around operators, and so on). `elomar fmt --check` only reports files that
need it, which is handy in CI.

## Sharing a program

`elomar build` makes one executable file of your program, with its
modules, packages and `elomar.toml` inside. Send it to anyone: it runs
without Elomar-Lang installed.

```text
elomar build src/main.elo -o grades.exe
grades.exe Lina Omar          arguments go to the program (os.args())
```

The executable is for the system it was built on: build on Windows for
Windows, on Linux for Linux. Paths in the program, and the folders in
`[permissions]`, start at the folder it is run from.

## Packages

Use code that others wrote:

```text
elomar search text            find packages in the public registry
elomar add slugify            add one to elomar.toml and download it
elomar add tools --git https://github.com/user/tools --tag v1.0.0
elomar update                 move to the newest allowed versions
```

`elomar.lock` records the exact versions, so everyone builds the same
thing. To share your own package, see the
[registry](https://github.com/abobasel86/elomar-registry).
