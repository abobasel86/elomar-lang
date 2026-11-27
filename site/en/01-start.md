---
layout: default
title: Getting started
lang: en
slug: 01-start
---

# 1. Getting started

## Install

On **Windows**, paste this into PowerShell:

```powershell
irm https://raw.githubusercontent.com/abobasel86/elomar-lang/main/install.ps1 | iex
```

On **Linux** or **macOS**, paste this into a terminal:

```bash
curl -fsSL https://raw.githubusercontent.com/abobasel86/elomar-lang/main/install.sh | sh
```

The installer downloads the latest release for your system, checks it
against the release's checksums, puts `elomar` in a folder of its own
(`.elomar/bin` in your home folder) and adds that folder to your `PATH`.
Open a new terminal afterwards. Later, `elomar upgrade` moves you to the
newest version.

Or with a package manager:

```powershell
# Windows, with Scoop
scoop bucket add elomar https://github.com/abobasel86/elomar-lang
scoop install elomar
```

```bash
# macOS or Linux, with Homebrew
brew tap abobasel86/elomar https://github.com/abobasel86/elomar-lang
brew install elomar
```

Then `scoop update elomar` or `brew upgrade elomar` moves you to a new
version.

Or install by hand:

1. Open the [latest release](https://github.com/abobasel86/elomar-lang/releases/latest)
   and download the file for your system:

   | System | File |
   |---|---|
   | Windows | `elomar-<version>-x86_64-pc-windows-msvc.zip` |
   | Windows on ARM | `elomar-<version>-aarch64-pc-windows-msvc.zip` |
   | Linux | `elomar-<version>-x86_64-unknown-linux-gnu.tar.gz` |
   | Linux on ARM (such as Raspberry Pi) | `elomar-<version>-aarch64-unknown-linux-gnu.tar.gz` |
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

### With Arabic keywords

Every keyword also has an Arabic spelling: `دالة` is `fn`, `ليكن` is `let`,
`إذا` is `if` and `وإلا` is `else`. A program may start in `رئيسية` instead
of `main`:

```elomar
دالة رئيسية() {
    ليكن الاسم = "لينا"
    إذا الاسم.طول > 3 {
        اطبع("مرحبًا يا {الاسم}")
    } وإلا {
        اطبع("أهلًا")
    }
}
```

`اطبع` is `print` and `.طول` is `.length`: the built-in functions, types
and methods have Arabic names too ([all of them](12-reference.html#arabic-names)). Write in English, in Arabic,
or both: it is the same program. The lessons
use the English keywords; all the Arabic ones are in the
[table of keywords](12-reference.html#keywords). Above every lesson,
**«دالة ليكن إذا»** shows all the examples with Arabic keywords, and the
[playground](playground.html) has a **fn ⇄ دالة** button for your program.

## Try things at the prompt

`elomar repl` opens a prompt: type a line and see its result at once,
without making a file. It is the quickest way to try an idea:

```text
elomar repl
> 2 + 3 * 4
14
> let name = "Lina"
> "Hello, {name}!"
"Hello, Lina!"
> [3, 1, 2].sorted()
[1, 2, 3]
```

Variables and functions you type stay for the next lines. `:help` lists
the commands and `:quit` leaves.

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

In VS Code, VSCodium, Cursor and other editors, search for **Elomar-Lang**
in the Extensions view and install it.
You get colors, errors while you type, hover information, completion and a
**Run** button.

To find a mistake, click left of a line number to set a breakpoint (a red
dot) and press **F5**. The program stops there: you see the value of every
variable, and can go on one line at a time (F10), step into a function
(F11) or continue (F5). If the program fails, it stops on the failing line
so you can see why.

## Commands you will use

| Command | What it does |
|---|---|
| `elomar run file.elo` | Check and run a program |
| `elomar repl` | Type code and see the result at once |
| `elomar check file.elo` | Only check for errors |
| `elomar build file.elo` | Make an executable to share |
| `elomar fmt file.elo` | Format the code in the standard style |
| `elomar new myapp` | Create a project with tests |
| `elomar new myapp --ai ollama` | Create a project that calls a model (`mock`, `ollama`, `openai` or `anthropic`) |
| `elomar doctor` | Check a project's setup: AI keys and models, permissions, Python, MCP servers, packages |
| `elomar upgrade` | Move to the newest version |
| `elomar test` | Run a project's tests |
