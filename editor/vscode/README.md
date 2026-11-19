# Elomar-Lang for VS Code

- **Syntax highlighting** for `.elo` files, including Arabic names.
- **Errors while you type**, in English or Arabic, in the file where they
  occur (also inside imported modules), with hints and related locations.
- **Run** button in the editor title bar, `Elomar: Run File`, or **F5**.
  There is no debugger yet (no breakpoints), so F5 runs the file.
- **Formatting** with `elomar fmt`: Format Document, and on save by default
  (turn off with `"[elomar]": { "editor.formatOnSave": false }`).
- **Snippets**: `main`, `amain`, `fn`, `pfn`, `type`, `enum`, `matchr`,
  `matcho`, `for`, `ife`, `infer`, `readjson`.

## Setup

1. Install the `elomar` command (from the repository root):

   ```bash
   cargo install --path crates/elomar-cli
   ```

2. Install this extension from the folder: copy `editor/vscode` to your VS
   Code extensions folder as `abobasel86.elomar-lang-0.0.1`, then reload VS
   Code.

   - Windows: `%USERPROFILE%\.vscode\extensions\`
   - macOS / Linux: `~/.vscode/extensions/`

## Settings

| Setting | Default | Meaning |
|---|---|---|
| `elomar.path` | `elomar` | The `elomar` executable, if it is not on `PATH` |
| `elomar.language` | `auto` | `en`, `ar`, or follow VS Code's display language |
| `elomar.checkOnType` | `true` | Check while typing; otherwise on open and save |

## How it works

The extension runs `elomar check --format json --stdin <file>` with the
editor's current text. That command needs no `main`, so a module file can be
checked on its own. A language server (LSP) with go-to-definition and hover
is planned once the language's structure is stable (spec §20.3).
