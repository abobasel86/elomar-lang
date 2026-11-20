---
layout: default
title: Getting started
lang: en
slug: 01-start
---

# 1. Getting started

## Install

1. Open the [latest release](https://github.com/abobasel86/elomar-lang/releases/latest)
   and download the file for your system:

   | System | File |
   |---|---|
   | Windows | `elomar-<version>-x86_64-pc-windows-msvc.zip` |
   | Linux | `elomar-<version>-x86_64-unknown-linux-gnu.tar.gz` |
   | macOS (Apple chips) | `elomar-<version>-aarch64-apple-darwin.tar.gz` |
   | macOS (Intel) | `elomar-<version>-x86_64-apple-darwin.tar.gz` |

2. Unpack it and put `elomar` (`elomar.exe` on Windows) in a folder on your
   `PATH`. On macOS, the first run may need `xattr -d com.apple.quarantine elomar`.
3. Check it works:

   ```text
   elomar --version
   ```

## Your first program

Create a file named `hello.elo`:

```elomar
fn main() {
    print("Hello, world!")
    print("مرحبًا بالعالم!")
}
```

Run it:

```text
elomar run hello.elo
```

Every program starts at `fn main()`. `print` writes its values and a line
break.

## Talking to the user

`input` shows a prompt and returns what the user typed:

```elomar
fn main() {
    let name = input("What is your name? ")
    print("Nice to meet you, {name}!")
}
```

The `{name}` inside the text is replaced by the value of `name`. This is
called *string interpolation*; you can put any expression between the
braces.

## When something is wrong

Elomar-Lang checks your whole program before running it. Try this:

```text
fn main() {
    let age = 30
    print(age + "years")
}
```

```text
error[E2005]: operator `+` cannot be applied to `Int` and `String`
```

Nothing ran: the mistake was found first. Add `--lang ar` to see messages
in Arabic: `elomar --lang ar run hello.elo`.

## The editor

Install the VS Code extension from the same release page
(`elomar-lang-<version>.vsix`, then **Extensions → … → Install from VSIX**).
You get colors, errors while you type, hover information, completion and a
**Run** button.

## Commands you will use

| Command | What it does |
|---|---|
| `elomar run file.elo` | Check and run a program |
| `elomar check file.elo` | Only check for errors |
| `elomar fmt file.elo` | Format the code in the standard style |
| `elomar new myapp` | Create a project with tests |
| `elomar test` | Run a project's tests |
