# Changelog

The extension's version follows Elomar-Lang's; a patch version (such as
0.3.1) changes only the extension. See the language's
[changelog](https://github.com/abobasel86/elomar-lang/blob/main/CHANGELOG.md)
for what each version adds.

## 1.12.1

Works with Elomar-Lang 1.12.1 (ARM builds for Linux and Windows).

## 1.12.0

Works with Elomar-Lang 1.12.0 (`elomar upgrade`, a wider `elomar doctor`).

## 1.11.0

Works with Elomar-Lang 1.11.0 (`elomar new --ai`, `elomar doctor`).

## 1.10.0

Works with Elomar-Lang 1.10.0 (local models without a key, `[ai.cache]`).

## 1.9.0

Works with Elomar-Lang 1.9.0 (built-in functions as values, `ai.search`).

## 1.8.0

Works with Elomar-Lang 1.8.0 (functions with trait bounds as values, batched `ai.embed`).

## 1.7.0

Works with Elomar-Lang 1.7.0 (`ai.stream`, `ai.embed`, `impl<T: A>`, map and set methods).

## 1.6.0

Works with Elomar-Lang 1.6.0 (five more `par_` list methods, faster record fields).

## 1.5.0

Works with Elomar-Lang 1.5.0 (more `par_` list methods, faster string building and indexing).

## 1.4.0

Works with Elomar-Lang 1.4.0 (`par_filter` and `par_fold` on several processor cores).

## 1.3.0

Works with Elomar-Lang 1.3.0 (`list.par_map` on several processor cores).

## 1.2.0

Works with Elomar-Lang 1.2.0 (`await all` on several processor cores).

## 1.1.1

Works with Elomar-Lang 1.1.1 (two fixes in the formatter and the checker).

## 1.1.0

Works with Elomar-Lang 1.1, whose programs run about twice as fast. No new
features in the extension.

## 1.0.0

Works with Elomar-Lang 1.0, the first stable version. No new features since
0.9.0.

## 0.9.0

**Find All References** (Shift+F12), **Rename** (F2) across the project's
files, and **parameter hints** while typing a call. Needs `elomar` 0.9.0.

## 0.7.0

Works with the 0.7.0 language: `timeout`, `std.csv`, the new file and date
functions, and `elomar build`. `all` and `timeout` are colored as built-in
functions.

## 0.5.1

A **Sponsor** button on the extension's page, linking to
[GitHub Sponsors](https://github.com/sponsors/abobasel86). Nothing else
changes.

## 0.5.0

Debugging: press **F5** to stop at breakpoints, step through the program
and look at its variables (it runs `elomar dap`). **Ctrl+F5** still runs
the file in a terminal. Needs `elomar` 0.5.0.

## 0.4.0

Works with the 0.4.0 language: `elomar repl` and the list methods `zip`,
`windows`, `sum_by`, `min_by` and `max_by`.

## 0.3.2

The setup steps say where to install the extension: Open VSX, or the
`.vsix` file of a release for Visual Studio Code.

## 0.3.1

A new icon: `{EL}` with the name Elomar Lang.

## 0.3.0

Works with the 0.3.0 language: tuples, pattern alternatives and ranges,
default trait methods, generic impls and `await all`.

## 0.2.0

Highlighting for string interpolation (`"Hello {name}"`), raw strings
(`r"..."`) and the new built-in types; a store icon and metadata.

## 0.1.0

First release: highlighting, diagnostics in English or Arabic, hover, go to
definition, outline, completion, formatting, snippets and a Run button.
