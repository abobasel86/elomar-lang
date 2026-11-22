# Elomar-Lang for VS Code

[Elomar-Lang](https://github.com/abobasel86/elomar-lang) is a simple, typed
programming language with AI built in, in Arabic and English.


- **Syntax highlighting** for `.elo` files, including Arabic names.
- **Errors while you type**, in English or Arabic, in the file where they
  occur (also inside imported modules), with hints and related locations.
- **Hover** for the type of any variable, field or expression name and the
  signature of functions and methods (with their effects).
- **Go to Definition** (F12) for variables, functions, methods, types,
  enums and traits, also into imported modules.
- **Outline** of functions, types, enums, traits and `impl` blocks.
- **Completion**: keywords, built-ins, your functions and variables, and
  fields and methods after `.` (also `fs.`, `json.`, `db.`, ...).
- **Run** button in the editor title bar, `Elomar: Run File`, or **F5**.
  There is no debugger yet (no breakpoints), so F5 runs the file.
- **Formatting** with `elomar fmt`: Format Document, and on save by default
  (turn off with `"[elomar]": { "editor.formatOnSave": false }`).
- **Snippets**: `main`, `amain`, `fn`, `pfn`, `type`, `enum`, `trait`,
  `impl`, `matchr`, `matcho`, `for`, `ife`, `infer`, `readjson`.

## Setup

1. Install the `elomar` command: download it for your system from the
   [latest release](https://github.com/abobasel86/elomar-lang/releases/latest)
   and put it on your `PATH` (or set `elomar.path`).
2. Install this extension:
   - in VSCodium, Cursor, Windsurf and other editors that use
     [Open VSX](https://open-vsx.org/extension/abobasel86/elomar-lang):
     search for **Elomar-Lang** in the Extensions view;
   - in Visual Studio Code: download `elomar-lang-<version>.vsix` from the
     [latest release](https://github.com/abobasel86/elomar-lang/releases/latest)
     and use **Extensions → … → Install from VSIX**.
3. Open a folder with `.elo` files. New to the language? Start with the
   [tutorial](https://abobasel86.github.io/elomar-lang/) in English or Arabic.

## Settings

| Setting | Default | Meaning |
|---|---|---|
| `elomar.path` | `elomar` | The `elomar` executable, if it is not on `PATH` |
| `elomar.language` | `auto` | `en`, `ar`, or follow VS Code's display language |
| `elomar.checkOnType` | `true` | Check while typing; otherwise on open and save |

## How it works

The extension starts the language server `elomar lsp` and talks to it over
standard input/output (the Language Server Protocol), sending the editor's
current text as you type. The server needs no `main`, so a module file can
be checked on its own. It restarts automatically if it stops. Formatting
runs `elomar fmt --stdin`.

## بالعربية

إضافة لغة Elomar-Lang: تلوين الكود، وعرض الأخطاء أثناء الكتابة بالعربية أو الإنجليزية،
ومعلومات الأنواع عند مرور المؤشر، والانتقال إلى التعريف، والإكمال التلقائي، والتنسيق، وزر
التشغيل. ثبّت الأمر `elomar` من [صفحة الإصدارات](https://github.com/abobasel86/elomar-lang/releases/latest)،
وابدأ بـ[الدروس](https://abobasel86.github.io/elomar-lang/ar/). لعرض الأخطاء بالعربية اضبط
`"elomar.language": "ar"`.
