# Elomar-Lang — Core Syntax and Semantics (Stages 1, 2 and 3)

This is the English translation of [grammar.md](grammar.md), which is the reference text; when they differ, grammar.md is right.

> **This file is the binding reference** for what the current interpreter implements. When it conflicts with
> `Elomar-Lang_Master_Specification_AR.md`, this file wins. It is updated with every
> change in `crates/`. Every rule here has a test in `crates/*/src/**`,
> `tests/fail/` or `examples/`.

Version: `1.0.0` · Status: **stable**. It is governed by the [stability promise](stability.md): a program that works
with 1.0 keeps working with every 1.x release.

---

## 1. Lexical Structure

| Element | Rule |
|---|---|
| Encoding | UTF-8 |
| Comments | `// ...` to the end of the line, and `/* ... */`, not nested. A comment that spans lines counts as a line end |
| Identifiers | `(XID_Start \| "_") XID_Continue*` per Unicode UAX #31, with **NFC** normalization. Arabic names are allowed |
| Integers | `0`, `42`, `1_000_000` → `Int` (64-bit). A number too large for the range is error `E0005` |
| Floats | `3.14`, `2.5e3`, `1e-3` → `Float`. `1..5` is a range, not `1.` |
| Durations | `500ms`, `10s`, `2m`, `1h` → `Duration`. Any other suffix attached to a number is error `E0006` |
| Strings | `"..."` on one line. Escapes: `\n \t \r \0 \" \\ \{ \} \u{XXXX}` |
| Raw strings | `r"\d+"` and `r#"قال "نعم""#`: no escapes and no interpolation, so every character stays as written. Useful for regular expressions and Windows paths. With `#` the string can contain `"` |
| Interpolation | `"مرحبا {name}، عمرك {age + 1}"`: any expression between `{` and `}` is evaluated and shown as `print` shows it. The result is a `String`. For a literal brace write `\{` (for example JSON: `"\{\"a\": 1}"`). Strings and brackets can be used inside the expression (`"{m["k"]}"`). A `match` pattern cannot be a string with interpolation |
| Reserved words | `let var fn return if else while for in break continue true false type enum match use pub async await trait impl self as div`, and each one's Arabic spelling (§1.0.1) |
| Prelude names | `Some None Ok Err` and the functions in section 7. They are not reserved words, but they cannot be redefined |

### 1.0.1 Arabic Keywords (since 1.13)

Every reserved word also has an Arabic spelling. Both spellings are the same
token, may be mixed in one file, and are reserved: neither can be a name.

| English | Arabic | English | Arabic |
|---|---|---|---|
| `let` | `ليكن` | `type` | `نوع` |
| `var` | `متغير` | `enum` | `تعداد` |
| `fn` | `دالة` | `match` | `طابق` |
| `return` | `أرجع` | `use` | `استخدم` |
| `if` | `إذا` | `pub` | `عام` |
| `else` | `وإلا` | `async` | `غير_متزامن` |
| `while` | `طالما` | `await` | `انتظر` |
| `for` | `لكل` | `trait` | `سمة` |
| `in` | `في` | `impl` | `نفذ` |
| `break` | `توقف` | `self` | `هذا` |
| `continue` | `تابع` | `as` | `بصفة` |
| `true` | `صح` | `div` | `قسمة` |
| `false` | `خطأ` |  | |

An Arabic keyword matches with or without hamza on alef (`أرجع` or `ارجع`,
`إذا` or `اذا`), with `ى` for final `ي` (`فى`), with `ه` for `ة` (`داله`),
and ignoring harakat, shadda and tatweel (`نفّذ`, `دالـــة`). The program may
start in `رئيسية` instead of `main` when it has no `main`.
`elomar fmt --keywords ar` writes every keyword in Arabic, and
`--keywords en` in English; without the option each keeps its spelling.
Since 1.16 it also writes the built-in names of §1.0.2 and §1.0.3 in that
language, except where the other spelling would mean something else (the
program's own name hides it there).

### 1.0.2 Arabic Names for Built-ins (since 1.14)

The built-in functions, the constructors and the core types also have
Arabic names. Unlike keywords they are **not reserved**: the parser rewrites
each to its English name, except where the program's own function, type,
constant, parameter, variable, closure parameter or pattern binding of that
name is visible, which hides the built-in there (as it would hide `print`).
They match like the Arabic keywords (hamza, `ى`, `ة`, harakat and tatweel do
not matter). Messages show the English names.

| English | Arabic | English | Arabic |
|---|---|---|---|
| `print` | `اطبع` | `min` | `أصغر` |
| `input` | `اقرأ` | `max` | `أكبر` |
| `to_string` | `إلى_نص` | `abs` | `مطلق` |
| `to_int` | `إلى_صحيح` | `all` | `الكل` |
| `to_float` | `إلى_عشري` | `timeout` | `مهلة` |
| `parse_int` | `حلل_صحيح` | `Some` | `قيمة` |
| `parse_float` | `حلل_عشري` | `None` | `لاشيء` |
| `assert` | `تأكد` | `Ok` | `نجاح` |
| `assert_eq` | `تأكد_تساوي` | `Err` | `فشل` |
| `set` | `مجموعة` |  | |

| English | Arabic | English | Arabic |
|---|---|---|---|
| `Int` | `صحيح` | `Set` | `مجموعة` |
| `Float` | `عشري` | `Map` | `قاموس` |
| `Bool` | `منطقي` | `Option` | `اختياري` |
| `String` | `نص` | `Result` | `نتيجة` |
| `List` | `قائمة` | `Duration` | `مدة` |
| `HttpRequest` | `طلب_ويب` | `HttpResponse` | `رد_ويب` |
| `Db` | `قاعدة_بيانات` | `Range` | `مدى` |
| `Future` | `مستقبل` | `Unit` | `فراغ` |
| `AIError` | `خطأ_ذكاء` | `FsError` | `خطأ_ملفات` |
| `JsonError` | `خطأ_جيسون` | `HttpError` | `خطأ_ويب` |
| `DbError` | `خطأ_بيانات` | `PyError` | `خطأ_بايثون` |
| `McpError` | `خطأ_أدوات` | `ProcessError` | `خطأ_عملية` |

### 1.0.3 Arabic Names for Methods (since 1.15)

The methods of the built-in types and the `length` property have Arabic
names too (`.طول` is `.length`, `.أضف` is `.push` on a list and `.add` on a
set); the full table is in the
[reference](https://abobasel86.github.io/elomar-lang/en/12-reference.html#arabic-methods).
They are not reserved. The type checker resolves them from the type of the
value: a record's own field and a method from an `impl` with that name come
first; otherwise the Arabic name becomes the English method of that type.
They match like the Arabic keywords. Messages show the English names.

### 1.0.4 Arabic Names for Standard Modules (since 1.17)

The standard modules, their functions and constants, and the effects have
Arabic names (`use std.ملفات` is `use std.fs`, `ملفات.اقرأ_نص` is
`fs.read_text`, `!{ملفات}` is `!{fs}`); the full table is in the
[reference](https://abobasel86.github.io/elomar-lang/en/12-reference.html#arabic-modules).
The parser resolves them: a `use std.X` with `X` in either language imports
the module, after which either spelling names it (unless the program's own
name hides it), and a function after an imported module's name is looked up
in that module's table. They match like the Arabic keywords.

### 1.0.5 `std`, Errors and Module Records (since 1.18)

`std` may be written `قياسي`. The remaining built-in types have Arabic names
(listed in §1.0.2), and so do the variants of the built-in error enums
(`خطأ_ملفات.غير_موجود` is `FsError.NotFound`), resolved by the parser after
an error type's name. The fields of the records the standard modules give
have Arabic names, resolved by the checker on those records only (by their
fields), in field access and in a record literal of such a type. Tables:
[reference](https://abobasel86.github.io/elomar-lang/en/12-reference.html#arabic-errors).

### 1.1 Line Ends

The lexer keeps a line end as a **statement separator** only if all three conditions hold:

1. The last token before it is an identifier, a literal, `true`/`false`/`self`, `)` `]` `}`, or `return`/`break`/`continue`.
2. We are not inside an open `( )` or `[ ]`.
3. The first token after it is not `.`, not `else`, not the end of the file, and not a binary operator that cannot start an expression: `||` `&&` `+` `*` `/` `%` `div` `==` `!=` `<` `<=` `>` `>=` `..` `..=`. (`-` and `!` can start an expression, so they start a new statement.)

Consecutive blank lines are merged. The semicolon `;` is an optional explicit separator.

```elomar
let total = price *
    quantity              // continues: the previous line ended with *
let names = list
    .sorted()             // continues: the line starts with .
let ok = a > 0
    || b > 0              // continues: the line starts with ||
if ok {
} else {                  // else on the same line or the next — both are valid
}
let a = 1; let b = 2      // two statements on one line need ;
```

The block brace `{` must be on the header line (`if`/`while`/`for`/`fn`/`match`).

---

## 2. Syntax (EBNF)

`SEP = NEWLINE | ";"`. Braces `{ }` in EBNF mean repetition.

```ebnf
program     = { SEP } , { item , ( SEP | EOF ) , { SEP } } ;
item        = use_decl | [ "pub" ] , ( type_decl | enum_decl | const_decl | trait_decl ) | fn_decl | impl_decl ;
trait_decl  = "trait" , IDENT , "{" , { fn_sig , SEP } , "}" ;     (* signatures without bodies *)
impl_decl   = "impl" , [ path , "for" ] , type , "{" , { [ "pub" ] , fn_decl , SEP } , "}" ;
fn_sig      = [ "async" ] , "fn" , IDENT , "(" , "self" , { "," , param } , ")" , [ "->" , type ] , [ effects ] ;
bound_list  = "<" , IDENT , [ ":" , path , { "+" , path } ] , { "," , ... } , ">" ;   (* `fn f<T: A + B>` *)
path        = IDENT , { "." , IDENT } ;

use_decl    = "use" , IDENT , { "." , IDENT } ;
type_decl   = "type" , IDENT , [ "<" , IDENT , { "," , IDENT } , ">" ] , "=" , type ;
enum_decl   = "enum" , IDENT , [ "<" , IDENT , { "," , IDENT } , ">" ] , "{" , [ variant , { ( "," | NEWLINE ) , variant } ] , "}" ;
variant     = IDENT , [ "(" , type , { "," , type } , ")" ] ;
const_decl  = "let" , IDENT , [ ":" , type ] , "=" , expr ;
fn_decl     = [ "pub" ] , [ "async" ] , "fn" , IDENT , [ "<" , IDENT , { "," , IDENT } , ">" ] ,
              "(" , [ param , { "," , param } , [ "," ] ] , ")" ,
              [ "->" , type ] , [ effects ] , block ;
param       = IDENT , ":" , type ;
effects     = "!" , "{" , [ IDENT , { "," , IDENT } ] , "}" ;

type        = path , [ "<" , type , { "," , type } , ">" ]   (* `students.Student` *)
            | "{" , [ field_ty , { ( "," | NEWLINE ) , field_ty } ] , "}"
            | "fn" , "(" , [ type , { "," , type } ] , ")" , [ "->" , type ] ;   (* `fn(Int) -> Bool` *)
field_ty    = IDENT , ":" , type ;

block       = "{" , { SEP } , { stmt , ( SEP | before "}" ) , { SEP } } , "}" ;
stmt        = ( "let" | "var" ) , IDENT , [ ":" , type ] , "=" , expr
            | place , ( "=" | "+=" | "-=" | "*=" | "/=" ) , expr
            | "if" , expr , block , [ "else" , ( block | if_stmt ) ]
            | "while" , expr , block
            | "for" , IDENT , "in" , expr , block
            | "return" , [ expr ] | "break" | "continue"
            | block
            | expr ;
place       = IDENT , { "." , IDENT | "[" , expr , "]" } ;

expr        = binary ;                       (* precedence in section 3 *)
unary       = ( "-" | "!" | "await" ) , unary | postfix ;
postfix     = primary , { "." , IDENT
                        | "[" , expr , "]"
                        | "?"
                        | [ type_args ] , "(" , [ arg , { "," , arg } , [ "," ] ] , ")" } ;
type_args   = "<" , type , { "," , type } , ">" ;   (* with disambiguation rule 3.1 *)
arg         = [ IDENT , ":" ] , expr ;
primary     = INT | FLOAT | DURATION | STRING | "true" | "false" | IDENT
            | "(" , expr , ")"
            | "[" , [ expr , { "," , expr } , [ "," ] ] , "]"
            | "[" , ( ":" | expr , ":" , expr , { "," , expr , ":" , expr } , [ "," ] ) , "]"
            | "{" , [ field , { ( "," | NEWLINE ) , field } ] , "}"
            | "match" , expr , "{" , { arm , ( "," | SEP ) } , "}"
            | lambda ;
lambda      = ( "||" | "|" , [ lparam , { "," , lparam } ] , "|" ) , ( block | expr ) ;
lparam      = IDENT , [ ":" , type ] ;
field       = IDENT , [ ":" , expr ] ;           (* { name } is shorthand for { name: name } *)
arm         = pattern , "=>" , ( block | expr ) ;
pattern     = "_" | IDENT | IDENT , "(" , pattern , ")"
            | path , "." , IDENT , [ "(" , pattern , { "," , pattern } , ")" ]   (* `lib.Level.High` *)
            | [ "-" ] , ( INT | FLOAT ) | STRING | "true" | "false" ;
```

- A statement that ends with `}` needs no separator after it.
- In **statement position**, `{` starts a block. In **expression position**, it starts a record. A `match` arm or a lambda body that starts with `{` is a block. To return a record, write `( { ... } )`.
- `if` is a statement, not an expression. To choose between two values, use `match` on a `Bool`.

---

## 3. Precedence

| From highest | Operators | Associativity |
|---|---|---|
| 1 | `.f` `f()` `a[i]` `x?` | left |
| 2 | `-x` `!x` `await x` | — |
| 3 | `*` `/` `div` `%` | left |
| 4 | `+` `-` | left |
| 5 | `..` `..=` | non-associative |
| 6 | `<` `<=` `>` `>=` | left |
| 7 | `==` `!=` | left |
| 8 | `&&` | left, short-circuit |
| 9 | `\|\|` | left, short-circuit |

### 3.1 Disambiguating `<` and Type Arguments

After a name or `a.b`, the parser tries to read `<T, ...>`. If that succeeds and `(` follows directly, it is
a call with type arguments (`ai.infer<Ticket>(x)` or `first<Int>(xs)`). Otherwise it is the comparison `<`.
Type arguments are accepted by `ai.infer`, `json.parse` and generic functions. Anywhere else they are rejected with `E0103`.

### 3.2 The Bar `|`

`|` appears only around lambda parameters (`|x| x * 2`). `||` in value position
is a lambda with no parameters (`|| 1`). Between two values it is logical "or".

---

## 4. Type System

| Type | Values | Notes |
|---|---|---|
| `Int` | 64-bit integers | Overflow is runtime error `R0002` |
| `Float` | IEEE-754 64-bit | Prints `85.0`, not `85` |
| `Bool` | `true` `false` | Conditions accept only `Bool` |
| `String` | Unicode text | `+` concatenates, `"{x}"` interpolates. No indexing by number |
| `Unit` | no value | The type of a function without `->` |
| `List<T>` | `[1, 2]` | Elements of one type. `+` concatenates |
| `(A, B, ...)` | `(1, "a")` | A tuple of two or more values. `t.0` and `t.1` for access, `let (a, b) = t` to destructure, and `(x, _)` as a pattern. Compared and sorted element by element |
| `Option<T>` | `Some(x)` `None` | |
| `Result<T, E>` | `Ok(x)` `Err(e)` | |
| `Range` | `a..b` `a..=b` | For loops |
| `Duration` | `500ms` `10s` `2m` `1h` | `+`, `-` and comparison. `.millis()` and `.seconds()` |
| `Future<T>` | Result of calling an `async fn` or `ai.infer` | `T` is extracted with `await` |
| `Map<K, V>` | `["a": 1]` and `[:]` | Keys are `String` or `Int`. Ordered by key |
| `Set<T>` | `set([1, 2])` | Elements are `String` or `Int`, no duplicates, sorted. Prints as `set([1, 2])` |
| Enums | `Shape.Circle(1.0)` `Priority.High` | Nominal: an `enum` with the same variants is a different type |
| `AIError` | `AIError.Timeout("...")` | Built-in enum. Each variant carries a message |
| `FsError` | `NotFound` · `PermissionDenied` · `PolicyDenied` · `InvalidData` · `Other` | For `std.fs`. Each variant carries a message |
| `JsonError` | `InvalidJson` · `TypeMismatch` | For `std.json`. Each variant carries a message |
| `{ f: T, ... }` | `{ f: v }` | **Structural**: the same fields with the same types = the same type |

- `type X = ...` is an alias. Circular aliases are error `E1006`.
- **Local inference** by unification: `let xs = []` followed by `xs.push(1)` makes it `List<Int>`.
  Functions must declare their parameter types. The default return type is `Unit`.
- **No implicit conversion**, with two specific exceptions:
  1. **Numeric promotion:** in `+ - * / %` and comparisons, if one side is `Float`, the `Int` is promoted.
  2. **Literals:** an integer literal where a `Float` is expected becomes a float (`var t: Float = 0`).
- `/` **always** gives a `Float`. `div` is integer division that truncates toward zero, and `%` is its remainder (`Int` only for `div`).
- Division by zero (`/` `div` `%`, integer or float) is error `R0001`.

### 4.1 Value Semantics

Assignment and passing copy the value logically (implementation: reference counting with copy on write).
Modification through `x = ...`, `x[i] = ...`, `x.f = ...`, `x.push(...)`... is allowed only if
the root of the path is a local `var` variable. Parameters, `let` bindings and constants cannot be modified (`E1003`).

### 4.2 Names and Scopes

- Every block has a scope. Defining a name twice in the same scope is `E1002`. Shadowing a variable from an outer scope is warning `W1001`.
- Lookup order: local variables → file constants → functions → the Prelude.
- File constants (`let` at file level) are evaluated in order before `main`, and see only what comes before them.
- Functions see all functions and all constants regardless of order.
- Named functions are values that can be passed around (§4.15), and so are built-in functions such as `print` and `to_string` (since 1.9).

### 4.3 `match`

- An arm that starts with `return`, `break` or `continue` is shorthand for a block: `None => return 0` means `None => { return 0 }`.
- The coverage check includes nested patterns: `Ok(Some(x))`, `Ok(None)` and `Err(e)` together cover `Result<Option<T>, E>`, and the `E2009` message names the exact missing case (such as `Ok(None)`).
- Patterns: `_`, a binding name, `Some(p)`/`Ok(p)`/`Err(p)`/`None`, tuples `(p, q)`, numbers, strings and `true`/`false`, alternatives with `|` (`"Fri" | "Sat" =>`; they cannot bind names: `E2024`), and integer ranges `90..=100` and `0..10` (an empty range is `E2025`).
- **Exhaustiveness is required** (`E2009`): `Option` needs `Some(_)` and `None`, `Result` needs `Ok(_)` and `Err(_)`,
  `Bool` needs `true` and `false`, and other types need `_` or a binding name.
- As an expression: all arms have one type. As a statement: arm types do not have to match.

### 4.3.1 Tuples and Destructuring

```elomar
fn min_max(xs: List<Int>) -> (Int, Int) {
    return (xs.min().unwrap_or(0), xs.max().unwrap_or(0))
}

fn main() {
    let (lo, hi) = min_max([4, 9, 2])          // destructuring; `_` to ignore an element
    let pair = (1, "one")
    print(pair.0, pair.1)
    for (name, score) in ["Lina": 91] {        // over a map: the key and the value
        print(name, score)
    }
    for (i, w) in [(1, "a"), (2, "b")] {       // over a list of tuples: the elements
        print(i, w)
    }
}
```

`(x)` with a single value is not a tuple but grouping. Tuples do not pass to JSON or to AI answers. Use a record there.

### 4.4 Return

In a function whose return type is not `Unit`, every path must end with `return` (`E2007`).
A path counts as ending if it is a `return`, an `if/else` whose branches all end, a `match` whose
arms are all ending blocks, or a `while true` without `break`.

---

## 4.5 `async` and `await`

- `async fn f() -> T`, when called, returns a `Future<T>` **without running its body**.
- `await e` runs the Future at the first await and returns `T`. Awaiting the same value a second time returns the saved result without running it again.
- `await` is allowed only inside an `async fn` (`E2013`), and only on a `Future` value (`E2014`).
- A `Future` that is computed but not awaited in a standalone statement is warning `W1003`, because it will not run.
- `async fn main()` is allowed. The environment awaits its result automatically.
- `all(List<Future<T>>) -> Future<List<T>>`: `await all([...])` awaits every operation and returns their results **in the same order**.
  `ai.infer` calls passed directly (such as `await all(texts.map(|t| ai.infer<Ticket>(...)))`) are sent **together at the same time**
  for the OpenAI and Anthropic providers, after the permission check and the call limit are applied to each one in order.
  Adjacent **isolated** `async fn` calls in the list run **together on the CPU cores** (since 1.2). The result is the same
  as if they ran one after another. A call is isolated if it does not print or read input, and does not use files, a database, processes,
  Python, MCP, an AI model or random numbers — this applies to the call and to every function it may call — and its arguments do not carry non-isolated functions
  or `Future` operations. It may compute and make `std.http` requests. If one fails, the error of the first one in the list is returned and the ones after it are stopped,
  and a `timeout` around `all` applies to them. Anything else runs one after another, as does everything in the browser playground and under the debugger.
  `ELOMAR_PARALLEL=0` turns off parallel execution.
- `xs.par_map(f)` (since 1.3) always gives what `xs.map(f)` gives, but it splits the list into consecutive parts and runs them **on the CPU cores**
  if `f` is an isolated function under the same conditions (a named non-`async` function, or a lambda whose body meets the conditions and whose captured variables do not carry
  non-isolated functions). Otherwise it runs like `map`. If an element fails, the error of the first failing element is returned and the parts after it are stopped.
- `xs.par_filter(f)` (since 1.4) always gives what `xs.filter(f)` gives, and runs `f` on the cores under the same conditions.
- `xs.par_fold(init, f)` (since 1.4), where `f: fn(T, T) -> T` and the type of `init` is the element type: splits the list into at most 64 consecutive parts
  (or one part per element for shorter lists). It folds the first part starting from `init`, and each later part starting from its first element,
  then folds the part results in order with the same `f`. The split depends only on the list length, so the result is the same on every machine, with
  and without parallelism, and equals `xs.fold(init, f)` if `f` is associative (`f(f(a, b), c) == f(a, f(b, c))`), such as addition,
  `max` and string concatenation. The parts are folded on the cores under the `par_map` conditions. The error is the error of the first failing part.
- `par_count(f)` and `par_sort_by(f)` (since 1.5) always give what `count(f)` and `sort_by(f)` give (the sort is stable). `f` is computed
  on the cores under the `par_map` conditions.
- `par_any(f)`, `par_all(f)` and `par_find(f)` (since 1.5) always give what `any`, `all` and `find` give: the parts search together for
  the first element for which `f` gives `true` (or `false` for `par_all`), and the parts after the first part that finds it are stopped. What comes after that element
  does not change the result: its error is not returned, and an endless loop in it is not waited on, as if `find` had stopped there.
- `par_flat_map(f)`, `par_group_by(f)`, `par_sum_by(f)`, `par_min_by(f)` and `par_max_by(f)` (since 1.6) always give what
  their counterparts without `par_` give: `f` is computed for all elements on the cores under the `par_map` conditions, then the work is finished in order. If an element fails in
  `par_sum_by`, the elements are walked again in order, so the error is the same error `sum_by` gives (an addition overflow before that element, if one occurs).
- `timeout(Future<T>, Duration) -> Future<Option<T>>`: `await timeout(f, 2s)` gives `Some(result)`, or `None` if the operation does not finish
  in the given time. In that case it is **stopped** at its next statement (or the next loop iteration), and `time.sleep` ends when the time runs out.
  A wait on the network or on an AI model is not cut off in the middle, but the operation stops as soon as it returns (both have their own timeouts).
  With nested `timeout`s, the shortest time wins. Awaiting a stopped operation again is runtime error `R0012`.

```elomar
async fn main() {
    match await timeout(slow_report(), 5s) {
        Some(report) => print(report)
        None => print("took too long; try again later")
    }
}
```
- The effects of an `async` function are counted at its call site.

## 4.6 The `std.ai` Module

```elomar
use std.ai

type Ticket = { category: String, priority: String, summary: String }

pub async fn classify(message: String) -> Result<Ticket, AIError> !{ai} {
    return await ai.infer<Ticket>(
        model: "support.classifier",          // logical name from elomar.toml
        instructions: "Classify the request.",
        input: message,                        // untrusted data, not instructions
        timeout: 10s                           // optional
    )
}
```

| Rule | Code |
|---|---|
| Using `ai` requires `use std.ai` | `E1001` |
| `ai.infer<T>(model: String, instructions: String, input, timeout: Duration)` → `Future<Result<T, AIError>>`. Arguments are positional or named, and `timeout` is optional | `E2002` / `E2010` |
| `T` is required and must have a JSON representation: `String`, `Int`, `Float`, `Bool`, `List<T>`, `Option<T>` and records of these. The same rule applies to the type of `input` | `E2008` / `E2015` |
| The call produces the `ai` effect | `E3001` / `E3002` |

**Execution path:** policy check (`max_calls_per_run`) → model lookup → provider call → JSON parsing →
**check that it matches `T`** (required fields, no extra fields, and types) → `Ok(T)` or `Err(AIError)`.
A structural match does not mean the information is factually correct. When `T` is `String`, free text is accepted.

**`AIError` variants** (each carries the message text): `Authentication` · `ProviderUnavailable` · `RateLimited` · `Timeout` · `Network` ·
`UnsupportedCapability` · `InvalidStructuredOutput` · `PolicyDenied` · `BudgetExceeded` · `ContextLimitExceeded` · `Cancelled`.
Every call failure is an `Err` value that the program handles, not a runtime error that stops it:

```elomar
match await classify(message) {
    Ok(t) => print(t.summary)
    Err(AIError.Timeout(_)) => print("try again later")
    Err(e) => print(e.kind(), e.message())   // e.kind() is the variant name as text
}
```

An enum whose variants are names only (no values) can be part of `T` or of `input`. It is sent to the model as a string value
from its variants (`{"type": "string", "enum": [...]}`). An answer with a value outside the variants → `InvalidStructuredOutput`.

### 4.6.1 `elomar.toml` and the Mock Provider

The nearest `elomar.toml` in the file's folder or a folder above it is read. The `mock` model is always available without setup. It returns a placeholder value
of the requested type: `"mock"`, `0`, `0.0`, `false`, `[]` and `None`.

```toml
[ai.models."support.classifier"]
provider = "mock"          # cloud providers are not implemented yet: they return ProviderUnavailable

[[ai.models."support.classifier".mock]]   # rules are tried in order
input_contains = "password"               # optional; without it the rule applies to every input
response = '{"category": "account", "priority": "high", "summary": "..."}'

[[ai.models."support.classifier".mock]]
input_contains = "crash"
delay_ms = 15000           # simulated response time; if it exceeds timeout → Timeout (no real wait)
response = '{}'

[[ai.models."support.classifier".mock]]
error = "RateLimited"      # any variant of AIError.kind
message = "retry later"

[ai.policy]
max_calls_per_run = 10     # after this → BudgetExceeded
default_timeout_ms = 30000
```

If no rule applies, the provider returns a placeholder value. Errors in the file (such as an unknown error type or a misspelled key
in a rule) stop the run before it starts, with exit code 2.

### 4.6.2 The OpenAI Provider and Permissions

```toml
[ai.models."support.classifier"]
provider = "openai"
model = "..."                      # OpenAI model ID (required, and not written in the code)
api_key_env = "OPENAI_API_KEY"     # name of the environment variable; default OPENAI_API_KEY
# base_url = "https://gateway.example.com/v1"   # optional; https required except for localhost

[permissions]
network_hosts = ["api.openai.com"] # denied by default: any host not listed is rejected with PolicyDenied
```

| Rule | Result |
|---|---|
| Calling a cloud model (any provider other than `mock`) produces `net` in addition to `ai`. If the model name is not a string literal, it is assumed to be a cloud model whenever a cloud model exists in the configuration | `E3001` / `E3002` at compile time |
| The host (or `host:port`) is not in `network_hosts` | `Err` of kind `PolicyDenied` without any connection. It does not count toward `max_calls_per_run` |
| The environment variable is not defined or is empty | `Authentication` without any connection, except for a `base_url` on this computer (`localhost`, `127.0.0.1`), which is sent without a key (since 1.10) |
| HTTP 401/403 → `Authentication` · 429 → `RateLimited` (or `BudgetExceeded` when the credit runs out) · 400 for context length → `ContextLimitExceeded` · other 400/422 and 404 for an unknown model → `UnsupportedCapability` (an error about the model id ends with a hint to check `model` in elomar.toml) · 5xx and other 404 → `ProviderUnavailable` · timeout → `Timeout` · connection failure → `Network` · model refusal → `PolicyDenied` · truncated answer → `ContextLimitExceeded` | An `Err` value |
| `base_url` with `http` for a non-local host, or a model without `model` | Configuration error before the run (exit code 2) |

**Implementation details:** the Chat Completions API with Structured Outputs in `strict` mode. Non-record types are wrapped in `{ "value": ... }`
and then unwrapped, and `String` is requested as free text. `instructions` goes in the system message, with a note that the user message is data, not instructions. `input`
goes in the user message. Redirects are not followed. The key is read on every call and is masked (`sk-***`) in error messages.

**Local models (since 1.10):** a server on the same computer that speaks the OpenAI API, such as Ollama, works with no key:

```toml
[ai.models.local]
provider = "openai"
model = "qwen2.5:7b"                      # a model pulled with `ollama pull`
base_url = "http://localhost:11434/v1"

[permissions]
network_hosts = ["localhost:11434"]
```

`ai.infer`, `ai.stream`, `ai.embed` and `ai.search` all work with it. A local model is still `net` (it is reached over HTTP).

**Embedding cache (since 1.10):**

```toml
[ai.cache]
embeddings = "embeddings.cache.json"     # a path inside the project folder
```

The vectors of `ai.embed` and `ai.search` from models that use the network are kept in this JSON file, by the provider, vendor model
id and `base_url`, and by the exact text. A text already in the file is not sent again and does not count toward `max_calls_per_run`;
changing `model` in elomar.toml makes the old vectors unused. An absolute path or one with `..` is a configuration error. A file that
cannot be read or written only means nothing is saved.

### 4.6.2.1 The Anthropic Provider (Claude)

```toml
[ai.models."support.classifier"]
provider = "anthropic"
model = "claude-opus-5-5"             # optional; this is the default
api_key_env = "ANTHROPIC_API_KEY"     # default ANTHROPIC_API_KEY

[permissions]
network_hosts = ["api.anthropic.com"]
```

The same rules as for OpenAI apply (permissions, `net`, key masking, error mapping). The differences:

- The Messages API (`POST /v1/messages`) with version `2026-10-21`. Types are requested with `output_config.format` (JSON Schema), and tools with `strict: true` (MCP tools use the server's schema without `strict`).
- In the tool loop, the assistant content is sent back exactly as it arrived (including thinking blocks), and the results of one step are collected in one message.
- When the model refuses (`stop_reason: "refusal"`), `PolicyDenied` is returned. For the models `claude-opus-5-5`, `claude-opus-5`, `claude-sonnet-5-5`, `claude-fable-5-1` and `claude-fable-5-2`, automatic switching to a fallback model on refusal is turned on (`fallbacks: "default"`).
- HTTP 402 → `BudgetExceeded` · 529 (overloaded) → `ProviderUnavailable` · 400 with "prompt is too long" → `ContextLimitExceeded`.

### 4.6.3 Tools: `ai.run`

```elomar
fn find_student(name: String) -> Option<Student> { ... }
fn class_average() -> Float { ... }

async fn ask(q: String) -> Result<Answer, AIError> !{ai} {
    return await ai.run<Answer>(
        model: "assistant",
        instructions: "Answer questions about the class. Use the tools for facts.",
        input: q,
        tools: [find_student, class_average],   // only what the model is allowed to call
        max_steps: 5,                           // optional, default 5
        timeout: 20s                            // optional, per step
    )
}
```

- The engine asks the model. If the model requests a tool call, the engine runs the function and sends its result back, and this repeats until the model answers. The final answer is checked against `T` as in `ai.infer`.
- Tools are names of ordinary functions listed explicitly in `tools` (no lambdas, no generic functions, no `async`). Their parameters and result are JSON types. The model sees the function name and its parameter names and types. Tool names may contain only Latin letters, digits and `_` (a limit of the model APIs).
- **Effects:** the effects of the tools are counted on the function that calls `ai.run` (a tool that prints means `io`), in addition to `ai` (and `net` for cloud models).
- **Limits:** every call step counts toward `max_calls_per_run`. Exceeding `max_steps` gives `AIError.BudgetExceeded`. Wrong arguments from the model or a tool that does not exist are not run. Instead an error is sent back to the model so it can correct itself.
- The `mock` model follows a written script: rules that match `input` are used in order, one step per rule. A rule with `tool = "find_student"` and `arguments = { name = "Lina" }` requests a tool, a rule with `response` answers, and `{{last_tool_result}}` in it is replaced with the last result. The OpenAI provider uses Tool Calling in strict mode.
- Tool errors are `E2022`: not a list of names, a lambda, a generic or `async` function, a non-Latin name, or a duplicate tool. A parameter type without JSON is `E2015`.

### 4.6.4 Agents and MCP: `agent.run` and `std.mcp`

```toml
# elomar.toml: MCP servers allowed to run; listing a server here is the permission
[mcp.servers.notes]
command = "python"                 # or npx or any program
args = ["mcp/notes_server.py"]
env = { LEVEL = "1" }              # optional
timeout_ms = 30000                 # optional, per request
```

```elomar
use std.agent
use std.mcp

// direct call to the tools of an MCP server (the mcp effect)
let tools = mcp.tools("notes")                          // Result<List<{ name, description }>, McpError>
let r = mcp.call("notes", "add_note", { text: "hi" })   // Result<String, McpError>

// agent: a model that uses your functions and MCP server tools, with approval before sensitive actions
async fn assist(q: String) -> Result<Answer, AIError> !{ai, io, mcp} {
    return await agent.run<Answer>(
        model: "assistant",
        instructions: "Help the user with their notes.",
        input: q,
        tools: [owner_name],                                    // optional
        mcp: ["notes"],                                         // optional: MCP servers
        require_approval: ["notes.add_note", "notes.delete_all"], // optional
        approve: |call| !call.tool.contains("delete"),          // fn({ tool, arguments }) -> Bool
        on_step: |step| print(step.tool, step.result),          // fn({ tool, arguments, result, approved })
        max_steps: 8                                            // optional, default 10
    )
}
```

- `agent.run` is like `ai.run` (4.6.3), plus: the tools of the MCP servers listed in `mcp`, approval, and a step log. The model sees an MCP tool as `server__tool`, and the program sees it as `server.tool`.
- **Approval:** before running a tool listed in `require_approval` (or every tool if the list is not given), `approve` is called with `{ tool, arguments }` (`arguments` is JSON text). If it returns `false`, the tool is not run and the model is told that the user refused it. The decision is in the code, not in the instruction text (specification §15.2). Literal names in `require_approval` must be tools of the call (`E2022`).
- **Log:** `on_step` is called after every tool call with `{ tool, arguments, result, approved }`, for auditing.
- **Permissions:** only a server listed under `[mcp.servers]` is started. The server program runs with the user's full permissions (like `std.py`). The server starts on first use in the project folder and stays until the end of the run.
- **Effects:** `mcp.*` and a non-empty `mcp` list mean `mcp`. The effects of the tools and of the `approve`/`on_step` functions are counted on the caller, in addition to `ai` (and `net` for cloud models).
- **Protocol:** a stdio client for MCP version 2026-07-28 (no handshake, and `_meta` in every request). It falls back automatically to `initialize` with older servers (2025-11-25 and earlier). It supports `tools/list` and `tools/call`. The result is the content text. Tools that ask the user for extra input (elicitation) are not supported yet.
- `McpError`: `NotConfigured` (server not listed), `Unavailable` (could not start, the server stopped, or the timeout ran out), `Protocol` (a JSON-RPC error such as a tool that does not exist) and `ToolFailed` (the tool reported `isError`). In `agent.run`, an unlisted server becomes `AIError.PolicyDenied` and a failure to start it becomes `AIError.ProviderUnavailable`. Tool errors are sent back to the model to act on.

### 4.6.5 Streaming and Embeddings: `ai.stream` and `ai.embed` (since 1.7)

| Function | Type |
|---|---|
| `ai.stream(model, instructions, input, on_text[, timeout])` | `Future<Result<String, AIError>>`; `on_text: fn(String)` is called with each piece of the answer as soon as it arrives |
| `ai.embed(model, text[, timeout])` | `Future<Result<List<Float>, AIError>>`: a vector whose direction stands for the text's meaning |
| `ai.similarity(a, b)` | `List<Float>`, `List<Float>` → `Float`: the cosine of the angle between the vectors, from -1 to 1, and 0 if either is all zeros |
| `ai.search(model, query, texts[, timeout])` (since 1.9) | `Future<Result<List<{ text: String, score: Float }>, AIError>>`: the texts with their similarity to `query`, most similar first |

- `ai.stream` and `ai.embed` go through the same checks as `ai.infer` (budget, model, network permission); their effect is `ai` (and `net`
  for cloud models).
- `ai.stream`: the OpenAI and Anthropic providers really stream the answer (`"stream": true`); the mock provider streams its answer one word at
  a time. `on_text` runs in the program, in order, while the rest arrives; if it fails, the stream is stopped and its error is the program's.
  In the browser playground the pieces are collected first, then given to `on_text`.
- `ai.embed`: the OpenAI provider uses `/embeddings` with the model named in `model`; the mock provider gives a vector of 64 numbers from the
  words of the text (the same words give the same direction), and an `error` rule matching the text gives its error; the Anthropic provider
  makes no embeddings (`UnsupportedCapability`).
- `ai.similarity` is pure arithmetic with no effects, so it counts as isolated and runs inside `par_` methods.
- `await all([...])` (since 1.8) sends the `ai.embed` calls to each model together: the OpenAI provider embeds up to 256 texts in one
  request. The results are in the order of the list, and each call has its own checks and its own `Result`, as if they ran one after
  another.
- `ai.search` embeds the query and the texts together, as `await all` does (each text counts toward `max_calls_per_run`), and sorts
  by `ai.similarity`; equal scores keep the order of `texts`. An empty `texts` gives `Ok([])` without any call. If any embedding
  fails, the result is the first error.

## 4.7 Enums (`enum`)

```elomar
enum Shape {
    Circle(Float)
    Rect(Float, Float)
    Empty
}

fn area(s: Shape) -> Float {
    return match s {
        Shape.Circle(r) => 3.14 * r * r
        Shape.Rect(w, h) => w * h
        Shape.Empty => 0
    }
}
```

- Variants are separated by a comma or a new line. Each variant has optional positional values.
- Construction and patterns use the full name `Shape.Circle(r)`. A variant without values is written without parentheses (`E2002` otherwise).
- A variant that does not exist is `E1010`, with a suggestion of the closest name.
- A `match` on an enum must cover all its variants or contain `_` (`E2009` with the list of missing ones).
- Enums are **nominal**, and can be **recursive** (`enum Tree { Leaf, Node(Tree, Int, Tree) }`).
- Equality `==` is structural. There is no ordering (`<`) between their values yet.
- A value prints as `Shape.Circle(1.0)` or `Shape.Empty`.

## 4.8 The Map `Map<K, V>`

```elomar
var stock = ["apple": 3, "pear": 0]
stock["kiwi"] = 5            // adds the key or updates it
stock["apple"] += 1          // the key must exist (R0011)
print(stock.get("plum"))     // None — safe read
for entry in stock {         // { key, value } records in key order
    print(entry.key, entry.value)
}
var empty: Map<String, Int> = [:]
```

- Keys are `String` or `Int` only (`E2018`). All values have one type.
- `m[k]` returns the value, or runtime error `R0011` if the key is missing. `m.get(k)` returns `Option<V>`.
- Value semantics as for lists: copies are independent, and modification requires `var`.
- A map is not one of the JSON types accepted by `ai.infer` yet.

## 4.8.1 The Set `Set<T>`

```elomar
var tags = set(["ai", "web", "ai"])    // set(["ai", "web"])
tags.add("rust")
print(tags.contains("web"), tags.length, tags.remove("x"))   // true 3 false
let both = tags.intersection(set(["web", "mobile"]))          // set(["web"])
```

- Created with the `set(list)` function (there is no special literal syntax). Elements are `String` or `Int` only (`E2018`).
- `contains`, `add`*, `remove`* (they return whether the element was present), `is_empty`, `to_list`, `union`, `intersection`, `difference`, `is_subset` and `.length`.
- `for x in s` goes in order. Value semantics as for lists. Equality does not depend on insertion order.

## 4.9 The `?` Operator

```elomar
fn sum(a: String, b: String) -> Result<Int, String> {
    return Ok(parse(a)? + parse(b)?)
}
```

- On `Result<T, E>`: returns `T`, or ends the function at once with `Err(e)`. The function must return `Result<_, E>` with the same error type (`E2001`).
- On `Option<T>`: returns `T`, or ends the function with `None`. The function must return `Option<_>`.
- Outside a function that returns a suitable type (including `main`) it is `E2016`. On a value that is not a `Result` or an `Option` it is `E2017`.
- In an `async fn` it works on the declared return type (what is inside the `Future`).

## 4.10 Files: `std.fs`

```elomar
use std.fs

match fs.read_text("data/students.json") {
    Ok(text) => print(text.length)
    Err(FsError.NotFound(_)) => print("no data yet")
    Err(e) => print(e.kind(), e.message())
}
```

| Function | Type | Permission |
|---|---|---|
| `fs.read_text(path)` | `Result<String, FsError>` (UTF-8; a BOM is removed) | `filesystem_read` |
| `fs.write_text(path, text)` | `Result<Unit, FsError>` (creates the file or replaces its content) | `filesystem_write` |
| `fs.append_text(path, text)` | `Result<Unit, FsError>` (creates the file if missing) | `filesystem_write` |
| `fs.exists(path)` | `Result<Bool, FsError>` | `filesystem_read` |
| `fs.list_dir(path)` | `Result<List<String>, FsError>` (sorted names) | `filesystem_read` |
| `fs.is_dir(path)` | `Result<Bool, FsError>` | `filesystem_read` |
| `fs.size(path)` | `Result<Int, FsError>` (in bytes; files only) | `filesystem_read` |
| `fs.create_dir(path)` | `Result<Unit, FsError>` (with missing parent folders; no error if it exists) | `filesystem_write` |
| `fs.remove(path)` | `Result<Unit, FsError>` (file only) | `filesystem_write` |
| `fs.remove_dir(path)` | `Result<Unit, FsError>` (the folder **and everything in it**) | `filesystem_write` |
| `fs.rename(from, to)` | `Result<Unit, FsError>` (move or rename) | `filesystem_write` for both paths |
| `fs.copy(from, to)` | `Result<Unit, FsError>` (file only) | read `from` and write `to` |

- All `fs` functions produce the `fs` effect. A `pub` function that uses them must declare `!{fs}`.
- `write_text` does not create folders. Create them first with `fs.create_dir`.
- A folder that is itself listed in `[permissions]` is not removed (`PolicyDenied`); only what is inside it is. `fs.remove` on a folder or `fs.remove_dir` on a file is `Other`.
- `()` is another way to write the type `Unit`, so `Result<(), FsError>` is `Result<Unit, FsError>`.
- Every failure is an `Err(FsError...)` value, not a runtime error.

### 4.10.1 File Permissions

```toml
[permissions]
filesystem_read = ["data", "output"]   # files or folders
filesystem_write = ["output"]          # read does not grant write, and the reverse
```

- **Denied by default:** without these lists, every access is rejected with `FsError.PolicyDenied` without touching the file.
- **Base folder:** relative paths in the program and in the lists start from the folder of `elomar.toml`, or the program's folder if that file does not exist (not from the current working folder).
- **No escape:** the path is converted to its real (canonical) form before the comparison, so `..` or symbolic links cannot leave the allowed folder.
- The `NotFound` and `PermissionDenied` messages have fixed wording on all operating systems.

## 4.11 JSON: `std.json`

```elomar
use std.json

type Student = { name: String, grades: List<Int> }

match json.parse<List<Student>>(text) {
    Ok(students) => print(students.length)
    Err(JsonError.TypeMismatch(m)) => print(m)   // "$[0].grades[1]: expected Int, found \"A\""
    Err(e) => print(e.message())
}
print(json.stringify(["a": 1]))   // {"a":1}
print(json.pretty(students))      // indented
```

- `json.parse<T>(text) -> Result<T, JsonError>`: `InvalidJson` if the text is not valid JSON, and `TypeMismatch` if it does not match `T`
  (with the same rules used for `ai.infer` answers: required fields, rejection of extra fields, and the path of the offending field).
- `json.stringify(value)` and `json.pretty(value)` → `String`.
- Accepted types: the JSON types in 4.6, plus `Map<String, T>` (a JSON object with free keys). Anything else is `E2015`.
- `std.json` has no effects.
- Records produced by `json.parse` print their fields in alphabetical order, because record types are structural and do not keep the definition order.

## 4.11.1 HTTP: `std.http`

```elomar
use std.http

fn main() {
    match http.get("https://api.example.com/todos/1") {
        Ok(resp) => print(resp.status, resp.body)
        Err(HttpError.PolicyDenied(m)) => print("not allowed:", m)
        Err(e) => print(e.kind(), e.message())
    }
    let r = http.post("https://api.example.com/todos", "{\"title\": \"x\"}")
}
```

| Function | Type |
|---|---|
| `http.get(url)` | `Result<{ status: Int, body: String }, HttpError>` |
| `http.post(url, body[, content_type])` | The same type. `content_type` defaults to `application/json` |

- Every call is a `net` effect, and the host must be in `[permissions] network_hosts` (`host` or `host:port`, no wildcards). Otherwise `HttpError.PolicyDenied`.
- `https` is required except for `localhost` and `127.0.0.1`. An invalid address is `InvalidUrl`.
- Redirects (3xx) are not followed, because they may lead to a host that is not allowed. The program gets that response itself.
- HTTPS (here and in the AI providers) trusts a certificate that chains to the roots bundled with Elomar or to the system's own (since 1.8), so it works behind antivirus programs and company proxies that inspect HTTPS with a root of their own.
- An error status such as 404 or 500 is **not** an `Err`: it is a response with its own `status`. `Err` is only for connection failures: `Timeout` (30 seconds), `Network` and `TooLarge` (larger than 10 MB, or not text).
- To convert the body to a type: `json.parse<Todo>(resp.body)`.

## 4.11.2 Databases: `std.db` (SQLite)

```elomar
use std.db

type Student = { id: Int, name: String, grade: Int, email: Option<String> }

fn top(conn: Db) -> Result<List<Student>, DbError> {
    return db.query<Student>(conn, "SELECT * FROM students WHERE grade >= :min", { min: 60 })
}

fn main() {
    match db.open("data/school.db") {
        Ok(conn) => {
            print(db.execute(conn, "CREATE TABLE IF NOT EXISTS students (id INTEGER PRIMARY KEY, name TEXT, grade INTEGER, email TEXT)"))
            print(top(conn))
        }
        Err(e) => print(e.kind(), e.message())
    }
}
```

| Function | Type |
|---|---|
| `db.open(path)` | `Result<Db, DbError>`. `":memory:"` is an in-memory database |
| `db.execute(conn, sql[, params])` | `Result<Int, DbError>`: the number of affected rows. Without parameters, several statements separated by `;` are allowed |
| `db.query<T>(conn, sql[, params])` | `Result<List<T>, DbError>` |
| `db.query_one<T>(conn, sql[, params])` | `Result<Option<T>, DbError>`: the first row, if any |

- Every call is a `db` effect. The database file needs `filesystem_write` permission for its folder in `elomar.toml` (otherwise `DbError.PolicyDenied`). `":memory:"` is always allowed.
- **Parameters** are a record whose fields bind the named items in the SQL: `{ min: 60 }` binds `:min`, so SQL is not built by joining strings (no SQL injection). Fields are `Int`, `Float`, `String`, `Bool` and `Option` of these (`None` becomes `NULL`). A field not named in the SQL is `DbError.Sql`.
- **A row** is a record whose fields match the columns by name, or a single value for a single column (`db.query<Int>(conn, "SELECT COUNT(*) FROM t")`). A `NULL` column needs `Option<T>`, `Bool` is read from 0/1, and `Float` accepts integers. Anything else is `DbError.TypeMismatch`.
- `DbError`: `PolicyDenied`, `Open`, `Sql`, `Constraint` (such as `UNIQUE`) and `TypeMismatch`. An unsuitable row or parameter type is `E2021`, and a missing row type is `E2008`.
- A `Db` value is a handle: copies of it point to the same connection. For transactions: `db.execute(conn, "BEGIN")`, then `"COMMIT"` or `"ROLLBACK"`.

## 4.11.3 Python: `std.py`

```elomar
use std.py

type Stats = { mean: Float, median: Float }

fn main() {
    // python/grades.py: def describe(grades, course): ...
    print(py.call<Stats>("grades", "describe", { grades: [91, 72], course: "Math" }))
    print(py.call<Float>("math", "sqrt", 2.0))          // a single value = a positional argument
    print(py.call<Float>("statistics", "mean", [1, 2, 3], 10s))
}
```

| Function | Type |
|---|---|
| `py.call<T>(module, function, args[, timeout])` | `Result<T, PyError>` |

- Requires `python = true` under `[permissions]` in `elomar.toml` (otherwise `PyError.PolicyDenied`), because Python code runs with the user's full permissions: Elomar's file and network permissions **do not apply** inside it.
- Every call is a `py` effect, and runs in a separate Python process (`python3`, or `python` on Windows, or `ELOMAR_PYTHON`) that ends after it. So there is no shared state between calls.
- Modules are looked up in `<project>/python`, then the project folder, then the installed libraries (`math`, `statistics`, `numpy`...).
- `args`: a record becomes named arguments (`f(grades=..., course=...)`), and any other value is a single positional argument. Values pass as JSON, and the result is checked against `T` as in `json.parse`.
- What the function prints appears after the call. The default timeout is 60 seconds.
- `PyError`: `PolicyDenied`, `Unavailable` (no Python), `Exception` (a Python exception, such as `"ValueError: ..."`), `TypeMismatch` and `Timeout`.

## 4.11.4 Math, Randomness, Time and the System

```elomar
use std.math
use std.random
use std.time
use std.os

fn main() {
    print(math.pi, math.sqrt(2), math.pow(2, 10))       // Int is accepted where Float is expected
    random.seed(7)                                        // repeatable results in tests
    print(random.int(1, 6), random.choice(["a", "b"]))
    let t = time.make(2026, 11, 20, 9, 30)               // milliseconds since 1970-01-01 UTC
    print(time.format(t), time.date(t).weekday)           // 2026-11-20T09:30:00Z 5
    print(os.args(), os.env("HOME"))
}
```

| Module | Functions | Effect |
|---|---|---|
| `std.math` | Constants `math.pi` `math.e` `math.tau` `math.inf` · `sqrt` `cbrt` `exp` `ln` `log10` `log2` `sin` `cos` `tan` `asin` `acos` `atan` (number → `Float`) · `pow(x, y)` `atan2(y, x)` `hypot(x, y)` · `is_nan(x) -> Bool` | — |
| `std.random` | `int(min, max) -> Int` (both ends included) · `float() -> Float` in [0, 1) · `bool()` · `choice(List<T>) -> Option<T>` · `shuffle(List<T>) -> List<T>` · `seed(Int)` | — |
| `std.time` | `now() -> Int` · `today() -> String` (`YYYY-MM-DD` in UTC) · `sleep(Duration)` · `format(Int) -> String` (ISO 8601) · `format_as(Int, pattern) -> String` (`YYYY` `MM` `DD` `HH` `mm` `ss`; anything else stays as it is) · `parse(String) -> Option<Int>` · `date(Int) -> { year, month, day, hour, minute, second, weekday }` (Monday = 1) · `make(year, month, day[, hour[, minute[, second]]]) -> Int` · `add_days(Int, n) -> Int` · `add_months(Int, n) -> Int` (January 31 + one month = the last day of February) · `days_between(a, b) -> Int` (calendar days from `a` to `b`) | — |
| `std.os` | `args() -> List<String>` · `env(name) -> Option<String>` · `platform() -> String` (`windows`/`linux`/`macos`) · `exit(code)` | `io` (except `platform`) |

- `random` is pseudo-random (xorshift). It is fit for games and simulations, **not** for passwords and keys.
- `time.parse` accepts `YYYY-MM-DD`, then optionally `THH:MM[:SS[.fff]]` and a time zone (`Z` or `+03:00`). Without a zone it is taken as UTC. A date that does not exist (such as 2001-02-29) gives `None`, and `time.make` with an invalid date is runtime error `R0006`.
- Program arguments: `elomar run main.elo a b --flag`, then `os.args()` gives `["a", "b", "--flag"]`. `os.exit(code)` ends the program at once with the exit code (0–255).

## 4.11.5 Regular Expressions, Encoding and Cryptography

```elomar
use std.regex
use std.encoding
use std.crypto

fn main() {
    print(regex.find_all("[0-9]+", "order 12 and 7"))          // ["12", "7"]
    print(regex.replace(r"(\d+)", "order 12", "#$1"))            // order #12
    print(encoding.base64_encode("مرحبا"), encoding.url_encode("a b"))
    print(crypto.sha256("abc"), crypto.uuid())
}
```

| Module | Functions |
|---|---|
| `std.regex` | `is_match(p, text) -> Bool` · `find(p, text) -> Option<String>` · `find_all(p, text) -> List<String>` · `captures(p, text) -> Option<List<String>>` (element 0 is the whole match, then the groups) · `replace(p, text, with) -> String` (all matches, and `$1` for a group) · `split(p, text) -> List<String>` |
| `std.encoding` | `base64_encode` · `hex_encode` · `url_encode` (`String -> String`) · `base64_decode` · `hex_decode` · `url_decode` (`String -> Option<String>`; `None` if the input is not valid or the result is not UTF-8) |
| `std.crypto` | `sha256(text) -> String` (hex) · `hmac_sha256(key, message) -> String` (hex) · `uuid() -> String` (version 4) · `token(bytes) -> String` (hex, from 1 to 1024 bytes) |

- The regular expression syntax is that of the Rust `regex` library (no backreferences, and guaranteed linear run time). Write patterns as raw strings: `r"\d{2,4}"`, because `{` in a normal string starts interpolation and the backslash needs doubling.
- A pattern written as a string literal is checked at compile time (`E2023`). A pattern built at run time, if invalid, is runtime error `R0006`.
- `uuid` and `token` come from a secure random source provided by the system, so they are fit for session IDs and keys, unlike `std.random`.
- Do not store passwords with `sha256` alone. Use an algorithm made for passwords (planned).

## 4.11.6 Web Server: `std.server`

```elomar
use std.server

fn handle(req: HttpRequest) -> HttpResponse {
    return match req.path {
        "/" => server.html(200, "<h1>مرحبًا</h1>")
        "/hello" => server.text(200, "Hello, {req.query.get_or("name", "guest")}!")
        "/api/time" => server.json(200, { ok: true })
        _ => server.text(404, "not found")
    }
}

fn main() {
    print(server.serve("127.0.0.1:8080", handle))
}
```

```toml
[permissions]
listen = ["127.0.0.1:8080"]     # denied by default; "0.0.0.0:8080" to accept connections from other machines
```

| Name | Type |
|---|---|
| `HttpRequest` | `{ method: String, path: String, query: Map<String, String>, headers: Map<String, String>, body: String }` |
| `HttpResponse` | `{ status: Int, headers: Map<String, String>, body: String }` |
| `server.serve(address, handler[, max_requests])` | `Result<Unit, HttpError>`; `net` effect. It returns only after `max_requests` requests (if given and greater than 0) |
| `server.text(status, body)` · `server.html(status, body)` | `HttpResponse` with the matching content type |
| `server.json(status, value)` | `HttpResponse`. Any value is converted to JSON as in `json.stringify` |
| `server.redirect(url)` | `HttpResponse` with status 303 and `Location` |

- An address not listed in `[permissions] listen` gives `HttpError.PolicyDenied` without opening any port. A failure to open the port is `HttpError.Network`.
- Requests are handled **one after another**, so the handler never runs twice at the same time. Each connection carries one request (`Connection: close`).
- `path` is URL-decoded, `query` comes from `?a=1&b=2`, and `headers` names are lowercase. The body is UTF-8 text of at most 10 MB and needs `Content-Length` (chunked sending gets a 411 response). Headers are at most 64 KB, and the read timeout is 10 seconds.
- A runtime error inside the handler gets a 500 response, the error is printed, and the server keeps running. Response headers that contain a new line are removed (protection against header injection), and `Content-Length` is computed automatically.
- A `HEAD` request is passed to the handler as is, and the response is sent without a body.
- To keep state between requests, use a database (`std.db`) and pass the connection with a lambda: `server.serve(addr, |req| handle(conn, req))`. Full example: `examples/web-api`.

## 4.11.7 Running Programs: `std.process`

```elomar
use std.process

fn main() {
    match process.run("git", ["log", "--oneline", "-3"]) {
        Ok(out) => print(out.status, out.stdout)
        Err(e) => print(e.kind(), e.message())
    }
}
```

```toml
[permissions]
commands = ["git"]     # denied by default: programs allowed to run, by name
```

| Function | Type |
|---|---|
| `process.run(command, args[, timeout])` | `Result<{ status: Int, stdout: String, stderr: String }, ProcessError>`; `process` effect |

- The program runs **directly, without a shell**, so every element of `args` arrives as is: `"; rm -rf /"` is plain text, not a command.
- The current folder is the project folder, and the default timeout is 60 seconds (then the program is stopped: `Timeout`). Output is at most 16 MB for each stream.
- A non-zero exit code is **not** an error: it is `status` in the result. `ProcessError` is `PolicyDenied` (the name is not in `commands`), `NotFound`, `Failed` and `Timeout`.
- An allowed program runs with the user's full permissions, and Elomar's file and network permissions do not apply inside it.

## 4.11.8 Tables: `std.csv`

```elomar
use std.csv

fn main() {
    match csv.parse_records("name,city\nLina,Amman\nعمر,\"إربد, الأردن\"\n") {
        Ok(rows) => {
            for row in rows {
                print(row["name"], row["city"])
            }
        }
        Err(problem) => print("bad CSV:", problem)
    }
    print(csv.stringify([["name", "note"], ["Lina", "a, b"]]))
}
```

| Function | Type |
|---|---|
| `csv.parse(text)` | `Result<List<List<String>>, String>`: all rows |
| `csv.parse_records(text)` | `Result<List<Map<String, String>>, String>`: the first row holds the column names |
| `csv.stringify(rows)` | `List<List<String>> -> String` (one line per row) |

- The format is RFC 4180: a field in double quotes can hold commas and new lines, and `""` inside it is a quote mark. Lines end with `\n` or `\r\n`, and a BOM is removed.
- Errors are text that names the line number: an unclosed field, text after the closing quote, or (in `parse_records`) a row whose field count differs from the column count.
- It has no effect: to read a file, use `fs.read_text` and then `csv.parse`.

## 4.12 Multi-File Modules

```text
school/
├── elomar.toml
└── src/
    ├── main.elo        ← the main program (contains main)
    ├── students.elo    ← use app.students
    └── grading.elo     ← use app.grading
```

```elomar
// src/students.elo
use app.grading

pub type Student = { name: String, grades: List<Int> }

pub fn level(s: Student) -> grading.Level {
    return grading.level_for(average(s))
}

fn round1(x: Float) -> Float {      // private: not visible outside this file
    return (x * 10).round() / 10
}
```

```elomar
// src/main.elo
use app.students
use app.grading

fn main() {
    let s: students.Student = { name: "Lina", grades: [95] }
    match students.level(s) {
        grading.Level.Excellent => print("excellent")
        _ => print("keep going")
    }
}
```

- `use app.a.b` loads the file `src/a/b.elo` from the project folder (the folder of `elomar.toml`, or the program's folder if there is none). If there is no
  `src/` folder, it looks next to `elomar.toml`. The short name is the last part (`b`).
  It may be the name of a standard module (`use app.csv` for `src/csv.elo`); only a file that imports both `std.csv` and `app.csv` is `E1002`.
- Access by qualified name: `students.level(s)`, `students.Student` and `grading.Level.Excellent` (in expressions, patterns and types).
- Or by importing specific names: `use app.students.{Student, level}`, then `level(s)` and `Student` directly (enums too: `use app.grading.{Level}`, then `Level.Excellent`).
  An imported name that conflicts with an item in the same file or with another import is `E1002`, one that does not exist is `E1008`, and one that is not `pub` is `E1012`.
- **Private by default:** from another file, only what is marked `pub` is visible (`pub fn`, `pub type`, `pub enum` and `pub let`). Otherwise `E1012`,
  with a hint about where to add `pub`. An item that does not exist is `E1008`, with a suggestion of the closest name.
- **Imports are per file:** `use std.ai` or `use app.x` in one file does not apply to another file.
- Inside a module, its items are used by their short names (`average(s)`). Modules can import each other, even in a cycle.
- `pub` functions must declare their effects (`E3002`), so a user of the module is not surprised by a network connection or a file write.
- Error messages point to the correct file, with a `:::` line for locations in other files.
- A module that does not exist is `E1011`, with the expected path.
- `let` constants in modules are evaluated before the constants of the main file.

### 4.12.1 Packages (Dependencies)

```toml
# elomar.toml
[dependencies]
geometry = { path = "vendor/geometry" }                              # local folder
textkit = { git = "https://github.com/user/textkit", tag = "v1.0" }  # or branch / rev
```

```elomar
use geometry.shapes               // src/shapes.elo in the geometry package
use geometry.shapes.{Rect, area}
```

- A package is a folder with an `elomar.toml` and its modules in `src/` (or next to `elomar.toml`). It is used with `use <package>.<module>` under the same `pub` rules. Inside the package, `use app.x` means its own module `x`.
- Dependencies of packages are resolved too, and one name must always point to the same source. A package name is lowercase Latin letters, digits and `_`, and is not `app` or `std`.
- git packages are downloaded with the `git` command into `~/.elomar/git` (or `$ELOMAR_HOME/git`), and `elomar.lock` pins the exact commit of each one until `elomar update`. The check in the editor does not connect to the network. `elomar run` and `elomar test` download what is missing.
- Commands: `elomar add <name> --path <folder>` or `--git <url> [--tag|--branch|--rev]`, `elomar remove <name>`, and `elomar update`.
- An error in the dependencies (a folder that does not exist, conflicting sources, a package that was not downloaded) is `E1015`. A module that does not exist in a package is `E1011`.

#### Package Registry

```toml
# elomar.toml of the application
[registry]                       # optional: the default is the public registry https://github.com/abobasel86/elomar-registry
index = "../my-registry"         # a git repository or a local folder (or the ELOMAR_REGISTRY environment variable)

[dependencies]
strings = "1.2"                 # the newest version that satisfies ^1.2 from the registry
other = { version = ">=1, <3" }
```

- The registry is an **index**, not a server: a git repository (or folder) with one file per package, `packages/<name>.toml`, which lists `description`, `git` and a list of `[[versions]]`. Each version has `version` and a full `commit` (40 characters), so the code of a version cannot change after it is published. `yanked = true` hides a version from new resolutions without breaking existing lock files.
- Version requirements use Cargo/semver syntax: `"1.2"` means `^1.2`, plus `"=1.0.0"` and `">=1, <3"`. The newest matching version that is not yanked is chosen, and the version and commit are recorded in `elomar.lock`. Resolution runs again only if the lock no longer satisfies the requirement, or on `elomar update`.
- Each package has one version in the project: if two packages require incompatible versions, it is error `E1015`, which names who required what.
- Commands: `elomar add strings` (writes `"^<newest version>"`) or `elomar add strings@1.2`, and `elomar search [text]`.
- Publishing: in the package folder (a clean git repository, with `version` under `[package]`), run `elomar publish --index <local copy of the registry repository> [--git <URL>]`. It adds the version to the package file in the index. Then you commit and push the index (or open a pull request). The command sends nothing to the network itself, and it warns if the commit has not been pushed to a remote branch.

## 4.13 Projects and Tests

`elomar new myapp` creates a project that runs and whose tests pass:

```text
myapp/
├── elomar.toml               ← [package] and [permissions] (all closed by default)
├── src/main.elo              ← elomar run src/main.elo
├── src/greeting.elo          ← module: use app.greeting
└── tests/greeting_test.elo   ← elomar test
```

```elomar
// tests/greeting_test.elo
use app.greeting

fn test_greets_by_name() {
    assert_eq(greeting.greet("Lina"), "Hello, Lina!")
}
```

- **Discovery:** every `.elo` file inside `tests/` (and its subfolders) is a test file. Every function in it whose name starts with `test_` and that has no parameters is a test.
- **Pass and fail:** a test fails if a runtime error occurs (such as a failed `assert` or `assert_eq`) or if it returns `Err(...)`. It may be `async`.
- **Isolation:** each test runs in a new interpreter with a new AI budget, and its output is shown only if it fails.
- **Access to the code:** test files use the project items marked `pub` through `use app.<module>`. The permissions and models of `elomar.toml` apply to them.
- `elomar test [PATH] [--filter TEXT]`: the path is the project folder (default: the current folder) or a single test file. The exit code is 1 on any failure.
- `elomar new` does not write over an existing folder that is not empty.
- `elomar new NAME --ai PROVIDER` (since 1.11), with `mock`, `ollama`, `openai` or `anthropic`: a project whose `src/tickets.elo` calls
  `ai.infer<Ticket>` and whose `main` also searches with `ai.search` (except for `anthropic`, which makes no embeddings), with
  `elomar.toml` set for that provider (models, `[ai.cache]`, `max_calls_per_run`, `network_hosts`). Its tests pass offline.
- `elomar doctor` (since 1.11) checks the project's AI setup and says what to fix: `elomar.toml` is valid; for each model, the key's
  environment variable is set (not needed for a server on `localhost`), its host is in `network_hosts`, its server answers, and the
  model id is among those listed by `GET .../models` (which costs nothing). Keys are never printed, and only allowed hosts are asked.
  The exit code is 1 if there is a problem. Since 1.12 it also checks packages (downloaded and locked), Python (when `python = true`),
  the commands of MCP servers and of `[permissions] commands`, and the folders of `filesystem_read` and `filesystem_write`.
- `elomar upgrade` (since 1.12) asks GitHub for the latest release and, when it is newer, runs the official installer (`install.ps1` or
  `install.sh`, which checks the download against `SHA256SUMS.txt`) into the folder of the running `elomar`. `--check` only says whether a
  newer version exists; `--force` reinstalls. An `elomar` installed with `cargo install` is told to upgrade the same way.
- `elomar build FILE [-o OUT]` checks the program, then writes a single executable file: a copy of `elomar` itself with the program files at its end
  (the main file, its modules and the modules of its packages) and `elomar.toml`. When run, it runs the program directly, all arguments reach `os.args()`,
  and it does not need Elomar-Lang to be installed. A program with errors is not built (exit code 1). The file is only for the system it was built on.
  In a built program, relative paths and the `[permissions]` lists start from the **current working folder**, not from the project folder,
  and what the program reads at run time (data files, Python scripts, MCP servers) is not included in it.
- `elomar fmt [PATHS] [--check]` formats files in one style: 4 spaces per level, spaces around binary operators, `f(x, y)`, `{ a: 1 }`, `0..10` and `!{ai, net}`, removal of `;` at the end of a line, and merging of consecutive blank lines. It keeps comments and the line breaks the author wrote, and it checks that the output produces the same tokens before writing.

## 4.14 Generic Functions (Generics)

```elomar
fn first<T>(xs: List<T>) -> Option<T> {
    if xs.is_empty() {
        return None
    }
    return Some(xs[0])
}

let n: Option<Int> = first([1, 2])        // T = Int, inferred from the arguments
let s = first<String>([])                 // or written explicitly
```

- Type parameters are written after the function name. At each call they are inferred from the arguments or from the expected type, or given explicitly as `f<Int>(...)`.
- Inside the function, `T` is an unknown type: it can be passed, stored, compared with `==` and printed, but arithmetic and `<` cannot be applied to it (`E2005`), and it cannot be treated as a specific type (`E2001`).
- A wrong number of explicit type arguments is `E2002`. Type arguments for a non-generic function are `E0103`.

### Generic Types and Enums

```elomar
type Pair<A, B> = { left: A, right: B }

enum Tree<T> {
    Leaf
    Node(Tree<T>, T, Tree<T>)
}

fn swap<A, B>(p: Pair<A, B>) -> Pair<B, A> {
    return { left: p.right, right: p.left }
}

let t: Tree<Int> = Tree.Leaf
let names = Tree.Node(Tree.Leaf, "Lina", Tree.Leaf)   // Tree<String> is inferred
```

- `Pair<Int, String>` means the record after substitution, so records stay structural.
- Generic enums are nominal: `Tree<Int>` and `Tree<String>` are two different types. The type of a variant is inferred from its values or from the context, and the type of the values in patterns follows the type of what is matched.
- At use, all type parameters must be given (`Tree` alone is `E1006`). A duplicate parameter is `E1002`.

## 4.15 Lambdas and Functions as Values

```elomar
fn double(x: Int) -> Int {
    return x * 2
}

fn adder(n: Int) -> fn(Int) -> Int {
    return |x| x + n                       // keeps a copy of n
}

let xs = [3, 1, 2]
xs.map(|x| x * 10)                         // [30, 10, 20]
xs.map(double)                             // a named function is a value too
xs.fold(0, |sum, x| sum + x)               // 6
let add5 = adder(5)
print(add5(1))                             // 6
let f = |a: Int, b: Int| {
    return a * b
}
```

- The type is `fn(Int, String) -> Bool` (and `fn()` means it returns `Unit`). Parameter types are written, or inferred from the context (`xs.map(|x| ...)`) or from the body.
- The body is an expression or a block. Inside a block, `return` returns from the lambda itself, and so does `?`.
- **Capture by value:** a lambda takes a copy of the variables visible when it is created. Changing the variable later does not affect it, and modifying a captured variable inside it is `E1013` (return the new value or use `fold`).
- **Effects:** the effects of a lambda body (or of a named function used as a value) are counted on the function that creates it, as if the body ran there. Calling a function value adds no effect.
- List functions: `map`, `filter`, `any`, `all`, `find`, `count`, `fold(init, f)` and `sort_by(f)` (the key is `Int`, `Float` or `String`, and the sort is stable).
- Built-in functions are values too (since 1.9): `xs.map(to_string)`, `xs.fold(0, max)`, `let say = print`. A built-in becomes the closure
  `|a, ...| print(a, ...)` with as many parameters as the expected function type has, or its usual number (two for `min`, `max` and
  `assert_eq`, one for the rest). `all` and `timeout` work on futures and stay calls only (`E0103`). Function equality `==` is by identity, not by behavior.
- Calling a value that is not a function is `E2004`, and a wrong number of arguments is `E2002`.

## 4.16 Traits (`trait`) and Methods (`impl`)

```elomar
trait Describe {
    fn describe(self) -> String
}

type Student = { name: String, grade: Int }

impl Describe for Student {
    fn describe(self) -> String {
        return self.name + ": " + to_string(self.grade)
    }
}

impl Describe for Int {
    fn describe(self) -> String {
        return "the number " + to_string(self)
    }
}

impl Student {                         // methods without a trait
    fn passed(self) -> Bool {
        return self.grade >= 60
    }
}

fn report<T: Describe>(items: List<T>) {   // any type that implements Describe
    for x in items {
        print(x.describe())
    }
}

let s: Student = { name: "Lina", grade: 91 }
print(s.describe(), s.passed())
report([1, 2])
```

- A trait lists function signatures. The first parameter is `self`, and `Self` means the implementing type (`fn scaled(self, k: Float) -> Self`). A function can have a **default body** that every `impl` that does not define it uses, such as `fn greet(self) -> String { return "Hello, {self.name()}" }`.
- `impl Trait for Type` must implement every trait function that has no default body, with the same signature: a missing one is `E1014`, a different one is `E2001`, and an extra one is `E1008` (put it in a separate `impl Type`).
- Any type can have an implementation: records, enums and basic types such as `Int` and `List<Int>`. Records are structural, so an `impl` on `Student` applies to every record with the same fields.
- The bound `<T: A + B>` allows calling the functions of `A` and `B` on values of `T`. Every call to the generic function must give a type that implements them (`E2019`). If the type cannot be determined, `E2020`.
- Implementing a trait twice on the same type, or two methods with the same name on the same type, is `E1002`.
- Methods are available wherever the type is available, and have effects like any function. The effects of an implementation used through a bound are counted on the caller of the generic function.
- `impl` for a generic type: `impl<T> Stack<T> { fn push(self, x: T) -> Stack<T> { ... } }` and `impl<T> Describe for Box<T>`. `T` takes the type of the value at each call.
- `impl<T: A>` (since 1.7): a bound on the impl's type parameter becomes a bound on each of its methods, which may then call the methods of `A`
  on values of `T`. The impl can be passed where its trait is needed (`fn show<X: Describe>`), with the implementations for `T`, nested too
  (`Box<Box<Int>>`). If `T` does not implement `A`, the error is `E2019`.
- A function with bounds is a value too (since 1.8): `xs.map(show)`, `let f = show` and `apply(show, 3)`. The trait implementations are
  found once the type is known in the same function; if it never is (`let f = show` unused), the error is `E2020`: give the type,
  `let f: fn(Int) -> String = show`.

## 5. Effects

Known effects: `io` `fs` `net` `ai` `db` `py` `mcp` `process`. Their sources: `io` (`print`, `input`
and `std.os`), `ai` (`ai.infer`, `ai.run` and `agent.run`), `net` (`std.http`, `std.server` and cloud models, see 4.6.2), `fs`
(`std.fs`, see 4.10), `db` (`std.db`), `py` (`std.py`), `mcp` (`std.mcp`) and `process` (`std.process`).

| Rule | Code |
|---|---|
| A private function without a declaration: its effects are inferred from its body and from what it calls (a fixed point that includes self-calls) | — |
| A `pub` function that has effects must declare them | `E3002` |
| Any function that declares `!{...}` must include all its actual effects in the declaration | `E3001` |
| Unknown effect name | `E3003` |
| The value of a file constant must have no effects | `E3004` |
| `main` needs no declaration | — |

---

## 6. Execution

- The program starts by evaluating the file constants in order, then calls `main()`, which takes no parameters and returns no value (`E2011`).
- The maximum call depth is 3000. Beyond it, `R0005`.
- `print(a, b, ...)` prints the values separated by a space, then a new line. Strings print without quote marks at the top level, and with them inside lists and records.

---

## 7. The Prelude

### 7.1 Functions

| Function | Type | Effect |
|---|---|---|
| `print(x, ...)` | any number of values → `Unit` | `io` |
| `input()` / `input(prompt)` | `String` → `String` (a line without its line end; empty string at end of input) | `io` |
| `to_float(n)` | `Int\|Float` → `Float` | |
| `to_int(x)` | `Int\|Float` → `Int` (drops the fraction; `R0007` for NaN/∞) | |
| `to_string(x)` | any value → `String` | |
| `parse_int(s)` / `parse_float(s)` | `String` → `Option<Int>` / `Option<Float>` | |
| `assert(cond)` / `assert(cond, msg)` | `Bool`, `String` → `Unit` (`R0004`) | |
| `assert_eq(actual, expected)` | two values of the same type → `Unit` (`R0004`, showing both values) | |
| `min(a, b)` / `max(a, b)` | two numbers → `Int` if both are integers, otherwise `Float` | |
| `set(list)` | `List<T>` → `Set<T>` (elements are `String` or `Int`) | |
| `abs(x)` | `Int\|Float` → the same type | |

### 7.2 Properties and Methods

| Type | Members |
|---|---|
| All types | `.to_string()` |
| `List<T>` | `.length` · `push(T)`* · `pop() -> Option<T>`* · `insert(Int, T)`* · `remove(Int) -> T`* · `contains(T)` · `index_of(T) -> Option<Int>` · `is_empty()` · `first()` / `last() -> Option<T>` · `reverse()` · `sorted()` (numbers or strings) · `slice(start, end)` · `sum()` (numbers) · `join(sep)` (`List<String>`) · `map(fn(T) -> U)` · `par_map(fn(T) -> U)` (like `map` on several cores, §4.5) · `filter` / `any` / `all` / `count(fn(T) -> Bool)` · `find(fn(T) -> Bool) -> Option<T>` · `par_filter(fn(T) -> Bool)` (like `filter` on several cores) · `par_any` / `par_all` / `par_count(fn(T) -> Bool)` · `par_find(fn(T) -> Bool) -> Option<T>` (like their counterparts on several cores, §4.5) · `fold(U, fn(U, T) -> U)` · `par_fold(T, fn(T, T) -> T)` (fold with fixed parts on several cores, §4.5) · `sort_by(fn(T) -> K)` · `par_sort_by(fn(T) -> K)` (keys on several cores) · `take(n)` / `skip(n)` · `unique()` · `min()` / `max() -> Option<T>` (numbers or strings) · `enumerate() -> List<{ index: Int, value: T }>` · `chunks(n) -> List<List<T>>` · `flat_map(fn(T) -> List<U>)` · `group_by(fn(T) -> K) -> Map<K, List<T>>` (`K` is `Int` or `String`) · `zip(List<U>) -> List<(T, U)>` (length of the shorter) · `windows(n) -> List<List<T>>` · `sum_by(fn(T) -> N)` (`N` is `Int` or `Float`) · `min_by` / `max_by(fn(T) -> K) -> Option<T>` (the first smallest or largest) · `par_flat_map` / `par_group_by` / `par_sum_by` / `par_min_by` / `par_max_by` (like their counterparts, with `f` on several cores, §4.5) |
| `String` | `.length` (visible characters) · `upper()` · `lower()` · `trim()` · `contains(s)` · `starts_with(s)` · `ends_with(s)` · `split(sep) -> List<String>` · `chars() -> List<String>` · `replace(a, b)` · `repeat(n)` · `parse_int()` · `parse_float()` · `is_empty()` · `index_of(s) -> Option<Int>` · `slice(start, end)` (by visible characters) · `lines()` · `words()` · `trim_start()` / `trim_end()` · `pad_start(n[, fill])` / `pad_end(n[, fill])` · `reverse()` |
| `Int` | `to_float()` · `abs()` · `pow(n)` (n ≥ 0) · `clamp(min, max)` |
| `Float` | `to_int()` · `round()` · `floor()` · `ceil()` (→ `Int`) · `abs()` · `sqrt()` · `pow(Float)` · `clamp(min, max)` · `round_to(digits) -> Float` · `to_fixed(digits) -> String` |
| `Option<T>` | `is_some()` · `is_none()` · `unwrap_or(T) -> T` · `unwrap_or_else(fn() -> T)` · `map(fn(T) -> U) -> Option<U>` · `and_then(fn(T) -> Option<U>)` · `filter(fn(T) -> Bool)` · `is_some_and(fn(T) -> Bool)` · `or(Option<T>)` · `ok_or(E) -> Result<T, E>` |
| `Result<T,E>` | `is_ok()` · `is_err()` · `unwrap_or(T) -> T` · `unwrap_or_else(fn(E) -> T)` · `map(fn(T) -> U) -> Result<U, E>` · `map_err(fn(E) -> F) -> Result<T, F>` · `and_then(fn(T) -> Result<U, E>)` · `ok() -> Option<T>` · `err() -> Option<E>` |
| `Map<K,V>` | `.length` · `get(K) -> Option<V>` · `contains_key(K)` · `remove(K) -> Option<V>`* · `keys() -> List<K>` · `values() -> List<V>` · `is_empty()` · `get_or(K, V) -> V` · `entries() -> List<{ key: K, value: V }>` · `filter(fn(K, V) -> Bool) -> Map<K, V>` · `any` / `all` / `count(fn(K, V) -> Bool)` · `map_values(fn(V) -> U) -> Map<K, U>` (since 1.7, in key order) |
| `Set<T>` | `.length` · `contains(T)` · `add(T)`* · `remove(T) -> Bool`* · `is_empty()` · `to_list()` · `union(Set<T>)` · `intersection(Set<T>)` · `difference(Set<T>)` · `is_subset(Set<T>) -> Bool` · `filter(fn(T) -> Bool) -> Set<T>` · `any` / `all` / `count(fn(T) -> Bool)` · `map(fn(T) -> U) -> Set<U>` (`U` is `Int` or `String`; since 1.7) |
| `AIError` · `FsError` · `JsonError` | `kind() -> String` (the variant name) · `message() -> String` |

\* Modifies the collection, and requires it to be in a `var` variable.

---

## 8. Diagnostic Codes

Every message is available in Arabic and English (`elomar --lang ar ...` or `ELOMAR_LANG=ar`).

### Lexical and Syntax Errors (`E0xxx`)

| Code | Meaning |
|---|---|
| `E0001` | Unexpected character |
| `E0002` | Unclosed string |
| `E0003` | Invalid escape sequence |
| `E0004` | Unclosed multi-line comment |
| `E0005` | Integer or duration larger than 64 bits |
| `E0006` | Invalid number suffix (allowed: `ms`, `s`, `m` and `h` after an integer) |
| `E0007` | Unclosed `{` inside a string, or an empty `{}` (for a literal brace write `\{`) |
| `E0101` | Unexpected token (expected such-and-such) |
| `E0102` | Expected an expression |
| `E0103` | Feature not supported (type arguments for a non-generic function, `all` or `timeout` as a value) |
| `E0104` | Invalid assignment target |
| `E0105` | `var` at file level |
| `E0107` | Expected a pattern |
| `E0108` | Expected a type |

### Name Errors (`E1xxx`)

| Code | Meaning |
|---|---|
| `E1001` | Unknown name (with a suggestion of the closest name) |
| `E1002` | Duplicate definition, or a reserved/built-in name |
| `E1003` | Modifying a value not defined with `var` |
| `E1004` | `break`/`continue` outside a loop |
| `E1005` | No `main` |
| `E1006` | Unknown type, circular alias, or wrong number of type arguments |
| `E1007` | Field does not exist |
| `E1008` | Method does not exist, or function does not exist in a module |
| `E1009` | Unknown module in `use` |
| `E1010` | Variant does not exist in an enum |
| `E1011` | `use app.*` module does not exist |
| `E1012` | Non-`pub` item used from another file |
| `E1013` | Modifying a captured variable inside a lambda |
| `E1014` | Trait `impl` is missing one of the trait functions |
| `E1015` | Error in the `[dependencies]` |

### Type Errors (`E2xxx`)

| Code | Meaning |
|---|---|
| `E2001` | Type mismatch |
| `E2002` | Wrong number of arguments |
| `E2003` | Condition is not `Bool` |
| `E2004` | Value is not a function (or `length` was called as a function) |
| `E2005` | Operator does not apply to these types |
| `E2006` | Value cannot be indexed with `[]` |
| `E2007` | A path does not return a value |
| `E2008` | Type could not be inferred |
| `E2009` | Non-exhaustive `match` |
| `E2010` | Error in named arguments |
| `E2011` | Invalid `main` signature |
| `E2012` | Value cannot be iterated |
| `E2013` | `await` outside an `async fn` |
| `E2014` | `await` on a value that is not a `Future` |
| `E2015` | Type with no JSON representation in `ai.infer` (as output or input) |
| `E2016` | `?` in a function that does not return the matching `Result`/`Option` |
| `E2017` | `?` on a value that is neither `Result` nor `Option` |
| `E2018` | `Map` key is neither `String` nor `Int` |
| `E2019` | Type does not implement the trait required by a `<T: Trait>` bound |
| `E2020` | Could not determine the type required by a trait bound in a call |
| `E2021` | Unsuitable row or parameter type for `std.db` |
| `E2022` | Invalid tool in `tools` for `ai.run` |
| `E2023` | Invalid regular expression literal in `regex.*` |
| `E2024` | An alternatives pattern `\|` binds names |
| `E2025` | Range pattern matches no number |
| `E2026` | Destructuring a value that is not a tuple of the required length (`let (a, b) = ...`) |
| `E2027` | Element number out of the tuple's bounds (`t.5`) |

### Effect Errors (`E3xxx`)

| Code | Meaning |
|---|---|
| `E3001` | Undeclared effect |
| `E3002` | Public function without an effect declaration |
| `E3003` | Unknown effect |
| `E3004` | File constant with effects |

### Warnings (`Wxxxx`)

| Code | Meaning |
|---|---|
| `W1001` | Shadowing a variable from an outer scope |
| `W1002` | Computed value is not used (with a hint: `==` instead of `=`) |
| `W1003` | `Future` is not awaited, so it will not run |

### Runtime Errors (`Rxxxx`)

| Code | Meaning |
|---|---|
| `R0001` | Division by zero |
| `R0002` | `Int` overflow |
| `R0003` | Index out of bounds |
| `R0004` | `assert` failed |
| `R0005` | Too many nested calls |
| `R0006` | Invalid argument (square root of a negative number, repeating a string a negative number of times or to more than 100 million characters, padding a string with `pad_start`/`pad_end` to more than 100 million characters, `pow` with a negative exponent, `clamp` with reversed bounds, `random.int` with reversed bounds, an invalid date in `time.make`) |
| `R0007` | Float cannot be converted to `Int` |
| `R0008` | Constant used before it is initialized |
| `R0009` | Internal interpreter error (reported as a bug) |
| `R0010` | Awaiting a `Future` while it is running |
| `R0011` | Key does not exist in the `Map` on `m[k]` |
| `R0012` | Awaiting an operation that `timeout` stopped |
| `R0013` | The whole program exceeded its time limit (8 seconds in the browser playground) |

---

## 9. Not Implemented Yet (Planned)

Nothing planned for the language itself is missing now. (Working since 1.9: built-in functions as values (§4.15) and `ai.search`
(§4.6.5); since 1.8: a function with bounds as a value (§4.16); since 1.7: `ai.stream` and `ai.embed` and `impl<T: A>`; since 1.2,
running `async fn` on several cores, §4.5.)
