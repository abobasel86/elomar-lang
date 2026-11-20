//! The Elomar-Lang formatter (`elomar fmt`).
//!
//! It works on tokens, not on the syntax tree, so comments stay exactly
//! where they are and the author's line breaks are kept. It rewrites only
//! whitespace and statement-ending semicolons:
//!
//! - indentation is 4 spaces per open `{`, `(` or `[`, plus one level for a
//!   line that continues an expression (after `=`, `+`, `&&`, `=>`, …, or
//!   starting with `.`);
//! - spacing follows one style: `a + b`, `f(x, y)`, `{ name: "Lina" }`,
//!   `0..10`, `!{ai, net}`, `xs[0]`, `-x`;
//! - `;` at the end of a line is removed (it is optional there);
//! - runs of blank lines become one, trailing spaces are removed, and the
//!   file ends with exactly one newline.
//!
//! Every result is checked: if the formatted code would not produce the same
//! tokens as the input, nothing is changed and [`FormatError::Unsafe`] is
//! returned.

use elomar_diagnostics::Diagnostic;
use elomar_lexer::{Token, TokenKind};

#[derive(Debug)]
pub enum FormatError {
    /// The code has lexical errors (e.g. an unterminated string).
    Lex(Vec<Diagnostic>),
    /// Formatting would change the program; nothing was changed.
    Unsafe,
}

const INDENT: &str = "    ";

/// Formats a whole file.
pub fn format(src: &str) -> Result<String, FormatError> {
    let (tokens, diags) = elomar_lexer::tokens_with_comments(src);
    if !diags.is_empty() {
        return Err(FormatError::Lex(diags));
    }
    let out = Printer::new(src).print(&tokens);
    if same_program(src, &out) {
        Ok(out)
    } else {
        Err(FormatError::Unsafe)
    }
}

/// Splits tokens into source lines (a multi-line block comment stays one
/// token on the line where it starts).
fn lines(tokens: &[Token]) -> Vec<Vec<&Token>> {
    let mut lines: Vec<Vec<&Token>> = vec![Vec::new()];
    for tok in tokens {
        match tok.kind {
            TokenKind::Newline => lines.push(Vec::new()),
            TokenKind::Eof => {}
            _ => lines.last_mut().expect("at least one line").push(tok),
        }
    }
    lines
}

fn is_opener(k: &TokenKind) -> bool {
    matches!(
        k,
        TokenKind::LBrace | TokenKind::LParen | TokenKind::LBracket
    )
}

fn is_closer(k: &TokenKind) -> bool {
    matches!(
        k,
        TokenKind::RBrace | TokenKind::RParen | TokenKind::RBracket
    )
}

/// Binary operators: spaced on both sides, and a line ending with one
/// continues on the next line.
fn is_binary(k: &TokenKind) -> bool {
    use TokenKind::*;
    matches!(
        k,
        Plus | Minus
            | Star
            | Slash
            | Percent
            | Div
            | EqEq
            | BangEq
            | Le
            | Ge
            | AndAnd
            | OrOr
            | Eq
            | PlusEq
            | MinusEq
            | StarEq
            | SlashEq
            | Arrow
            | FatArrow
    )
}

/// Operators that, at the start of a line, continue the previous line
/// (the lexer's rule): such a line is indented one more level.
fn starts_continuation(k: &TokenKind) -> bool {
    use TokenKind::*;
    matches!(
        k,
        OrOr | AndAnd
            | Plus
            | Star
            | Slash
            | Percent
            | Div
            | EqEq
            | BangEq
            | Lt
            | Le
            | Gt
            | Ge
            | DotDot
            | DotDotEq
    )
}

/// Whether a `-` or `!` after `prev` is a prefix operator.
fn starts_operand(prev: Option<&TokenKind>) -> bool {
    use TokenKind::*;
    match prev {
        None => true,
        Some(k) => {
            is_binary(k)
                || is_opener(k)
                || matches!(
                    k,
                    Comma
                        | Pipe
                        | Colon
                        | Semi
                        | Lt
                        | Gt
                        | Bang
                        | Return
                        | If
                        | While
                        | In
                        | Match
                        | Await
                        | Else
                        | DotDot
                        | DotDotEq
                )
        }
    }
}

struct Printer<'a> {
    src: &'a str,
}

impl<'a> Printer<'a> {
    fn new(src: &'a str) -> Self {
        Printer { src }
    }

    fn text(&self, tok: &Token) -> &'a str {
        let s = &self.src[tok.span.start as usize..tok.span.end as usize];
        if tok.kind == TokenKind::Comment && s.starts_with("//") {
            s.trim_end()
        } else {
            s
        }
    }

    fn print(&self, tokens: &[Token]) -> String {
        let mut out = String::new();
        // One entry per open bracket: whether it added an indent level.
        // Only the outermost bracket left open by a line adds one, so
        // `f([` followed by items indents them once, not twice.
        let mut open: Vec<bool> = Vec::new();
        let mut blank = false;
        let mut prev_last: Option<TokenKind> = None;
        let mut prev_opened = false;

        for line in lines(tokens) {
            let code = without_trailing_semis(&line);
            if code.is_empty() {
                blank = !out.is_empty();
                continue;
            }
            let leading_closers = code.iter().take_while(|t| is_closer(&t.kind)).count();
            let still_open = open.len().saturating_sub(leading_closers);
            let mut indent = open[..still_open].iter().filter(|added| **added).count() as i32;
            let continues = prev_last.as_ref().is_some_and(is_binary) && !prev_opened
                || code[0].kind == TokenKind::Dot
                || starts_continuation(&code[0].kind);
            if continues && leading_closers == 0 {
                indent += 1;
            }
            if blank {
                out.push('\n');
                blank = false;
            }
            for _ in 0..indent {
                out.push_str(INDENT);
            }
            out.push_str(&self.line(&code));
            out.push('\n');

            let mut lowest = open.len();
            for t in &code {
                if is_opener(&t.kind) {
                    open.push(false);
                } else if is_closer(&t.kind) {
                    open.pop();
                    lowest = lowest.min(open.len());
                }
            }
            prev_opened = open.len() > lowest;
            if let Some(first_new) = open.get_mut(lowest) {
                *first_new = true;
            }
            prev_last = code
                .iter()
                .rev()
                .find(|t| t.kind != TokenKind::Comment)
                .map(|t| t.kind.clone());
        }
        out
    }

    /// One line's tokens joined with canonical spacing.
    fn line(&self, tokens: &[&Token]) -> String {
        let generic = type_brackets(tokens);
        let mut s = String::new();
        let mut in_effects = false;
        // Closure parameter bars alternate: the opening `|` has no space
        // after it, the closing one none before it: `|a, b| a + b`.
        let mut pipe_open = false;
        for (i, tok) in tokens.iter().enumerate() {
            let prev = i.checked_sub(1).map(|j| tokens[j]);
            let before_prev = i.checked_sub(2).map(|j| &tokens[j].kind);
            let around_type_args = (i > 0 && generic[i - 1], generic[i]);
            let closing_pipe = tok.kind == TokenKind::Pipe && pipe_open;
            let after_opening_pipe = prev.is_some_and(|p| p.kind == TokenKind::Pipe) && pipe_open;
            if let Some(p) = prev
                && !closing_pipe
                && !after_opening_pipe
                && self.space_between(p, tok, before_prev, in_effects, around_type_args)
            {
                s.push(' ');
            }
            if tok.kind == TokenKind::Pipe {
                pipe_open = !pipe_open;
            }
            // `!{ai, net}` and `use app.x.{a, b}` keep their braces tight.
            if tok.kind == TokenKind::LBrace
                && prev.is_some_and(|p| matches!(p.kind, TokenKind::Bang | TokenKind::Dot))
            {
                in_effects = true;
            } else if tok.kind == TokenKind::RBrace {
                in_effects = false;
            }
            s.push_str(self.text(tok));
        }
        s
    }

    fn space_between(
        &self,
        prev: &Token,
        cur: &Token,
        before_prev: Option<&TokenKind>,
        in_effects: bool,
        (prev_is_type_bracket, cur_is_type_bracket): (bool, bool),
    ) -> bool {
        use TokenKind::*;
        let (p, c) = (&prev.kind, &cur.kind);

        if *c == Comment || *p == Comment {
            return true;
        }
        // Type arguments are tight: `List<Int>`, `ai.infer<T>(x)`.
        if cur_is_type_bracket {
            return false;
        }
        if prev_is_type_bracket && *p == Lt {
            return false;
        }
        // Prefix `-x` / `!x`: no space after the operator.
        if matches!(p, Minus | Bang) && starts_operand(before_prev) {
            return false;
        }
        match c {
            Comma | Semi | Colon | RParen | RBracket | Dot | Question => return false,
            RBrace => return !(in_effects || *p == LBrace),
            DotDot | DotDotEq => return false,
            _ => {}
        }
        match p {
            LParen | LBracket | Dot | DotDot | DotDotEq => return false,
            LBrace => return !in_effects,
            Bang if *c == LBrace => return false,
            // Function types: `fn(Int) -> Int`.
            Fn if *c == LParen => return false,
            _ => {}
        }
        match c {
            // Calls and indexing: `f(x)`, `xs[0]`, `f(x)(y)`, `r?[0]`.
            LParen | LBracket => {
                let type_args_call = prev_is_type_bracket && *p == Gt;
                !(type_args_call || matches!(p, Ident(_) | RParen | RBracket | Question | SelfKw))
            }
            _ => true,
        }
    }
}

/// Marks the `<` and `>` that enclose type arguments. As in the parser, a
/// `<` after a name starts type arguments if a matching `>` closes a list
/// of type names and either the name is capitalized (`List<Int>`) or a `(`
/// follows (`ai.infer<T>(`); any other `<` or `>` is a comparison.
fn type_brackets(tokens: &[&Token]) -> Vec<bool> {
    use TokenKind::*;
    let mut marks = vec![false; tokens.len()];
    for i in 1..tokens.len() {
        let Ident(name) = &tokens[i - 1].kind else {
            continue;
        };
        if tokens[i].kind != Lt || marks[i] {
            continue;
        }
        let mut depth = 0;
        let mut close = None;
        for (j, t) in tokens.iter().enumerate().skip(i) {
            match t.kind {
                Lt => depth += 1,
                Gt => {
                    depth -= 1;
                    if depth == 0 {
                        close = Some(j);
                        break;
                    }
                }
                Ident(_) | Comma | Dot | Colon | Plus => {}
                _ => break,
            }
        }
        let Some(close) = close else { continue };
        let capitalized = name.chars().next().is_some_and(char::is_uppercase);
        let called = tokens.get(close + 1).is_some_and(|t| t.kind == LParen);
        if capitalized || called {
            for (j, t) in tokens.iter().enumerate().take(close + 1).skip(i) {
                if matches!(t.kind, Lt | Gt) {
                    marks[j] = true;
                }
            }
        }
    }
    marks
}

/// The line's tokens without `;` at its end (before a trailing comment).
fn without_trailing_semis<'t>(line: &[&'t Token]) -> Vec<&'t Token> {
    let comment = line.last().filter(|t| t.kind == TokenKind::Comment);
    let mut code: Vec<&Token> = line
        .iter()
        .copied()
        .take(line.len() - usize::from(comment.is_some()))
        .collect();
    while code.last().is_some_and(|t| t.kind == TokenKind::Semi) {
        code.pop();
    }
    code.extend(comment);
    code
}

/// Whether `a` and `b` lex to the same program: the same tokens, where a
/// `;` and a statement-ending line break count as the same separator.
fn same_program(a: &str, b: &str) -> bool {
    fn shape(src: &str) -> Option<Vec<TokenKind>> {
        let (tokens, diags) = elomar_lexer::lex(src);
        if !diags.is_empty() {
            return None;
        }
        let mut out: Vec<TokenKind> = Vec::new();
        for t in tokens {
            let k = match t.kind {
                TokenKind::Semi => TokenKind::Newline,
                k => k,
            };
            let sep = k == TokenKind::Newline;
            let after_sep_or_open = out
                .last()
                .is_none_or(|l| *l == TokenKind::Newline || *l == TokenKind::LBrace);
            if sep && after_sep_or_open {
                continue;
            }
            if matches!(k, TokenKind::RBrace | TokenKind::Eof)
                && out.last() == Some(&TokenKind::Newline)
            {
                out.pop();
            }
            out.push(k);
        }
        Some(out)
    }
    shape(a).is_some() && shape(a) == shape(b)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fmt(src: &str) -> String {
        let out = format(src).unwrap();
        assert_eq!(format(&out).unwrap(), out, "formatting must be idempotent");
        out
    }

    #[test]
    fn indentation_and_spacing() {
        let src = "fn  main( ){\nlet x=1+2*3;\n  if x>2{print( \"big\" ,x )}\nelse{\n        print(-x)\n}\n}\n";
        assert_eq!(
            fmt(src),
            "fn main() {\n    let x = 1 + 2 * 3\n    if x > 2 { print(\"big\", x) }\n    else {\n        print(-x)\n    }\n}\n"
        );
    }

    #[test]
    fn records_lists_maps_and_ranges() {
        assert_eq!(
            fmt(
                "let r={name:\"Lina\",tags:[1,2]}\nlet m=[\"a\":1]\nlet e=[:]\nfor i in 0 .. 10 {}\nlet t: Map<String,Int> = [:]\n"
            ),
            "let r = { name: \"Lina\", tags: [1, 2] }\nlet m = [\"a\": 1]\nlet e = [:]\nfor i in 0..10 {}\nlet t: Map<String, Int> = [:]\n"
        );
    }

    #[test]
    fn effects_generics_and_try() {
        assert_eq!(
            fmt(
                "pub async fn f(q:String)->Result<T,AIError>!{ ai , net }{\nreturn await ai.infer<T>(\"m\",\"i\",q,timeout:10s)\n}\nfn g() -> Option<Int> {\nlet v=xs.first()?\nreturn Some(v)\n}\n"
            ),
            "pub async fn f(q: String) -> Result<T, AIError> !{ai, net} {\n    return await ai.infer<T>(\"m\", \"i\", q, timeout: 10s)\n}\nfn g() -> Option<Int> {\n    let v = xs.first()?\n    return Some(v)\n}\n"
        );
    }

    #[test]
    fn named_imports_are_tight() {
        assert_eq!(
            fmt("use app.students.{ Student ,average }\n"),
            "use app.students.{Student, average}\n"
        );
    }

    #[test]
    fn comparisons_versus_type_arguments() {
        assert_eq!(fmt("let ok = a<b && c>d\n"), "let ok = a < b && c > d\n");
        assert_eq!(
            fmt("let xs : List < List<Int> > = []\nlet r = json.parse < Map<String,Int> > (t)\n"),
            "let xs: List<List<Int>> = []\nlet r = json.parse<Map<String, Int>>(t)\n"
        );
        // Closing type brackets before `)`, `,`, `{` and `!{`.
        let sig = "fn f(m: Map<String, Item>, xs: List<Int>) -> Result<List<Int>, String> {\n}\nfn g() -> List<Int> !{io} {\n}\n";
        assert_eq!(fmt(sig), sig);
    }

    #[test]
    fn comments_and_blank_lines() {
        let src = "// header\n\n\n\nfn main() {   // trailing\n    /* block\n       comment */\n    print(1);   \n\n\n    print(2) // two\n}\n\n\n";
        assert_eq!(
            fmt(src),
            "// header\n\nfn main() { // trailing\n    /* block\n       comment */\n    print(1)\n\n    print(2) // two\n}\n"
        );
    }

    #[test]
    fn continuation_lines() {
        assert_eq!(
            fmt(
                "fn f() -> Int {\nlet total = a +\nb\nlet s = xs\n.sorted()\nreturn match x {\nOk(v) =>\nv\n_ => 0\n}\n}\n"
            ),
            "fn f() -> Int {\n    let total = a +\n        b\n    let s = xs\n        .sorted()\n    return match x {\n        Ok(v) =>\n            v\n        _ => 0\n    }\n}\n"
        );
    }

    #[test]
    fn multi_line_calls_and_literals() {
        assert_eq!(
            fmt("let xs = [\n1,\n2\n]\nprint(\nf(1),\n  g(2)\n)\n"),
            "let xs = [\n    1,\n    2\n]\nprint(\n    f(1),\n    g(2)\n)\n"
        );
    }

    #[test]
    fn closures_and_function_types() {
        assert_eq!(
            fmt(
                "fn apply(f:fn ( Int )->Int)->Int{\nreturn f(1)\n}\nlet g = xs.map( | x | x*2 )\nlet n = xs.sort_by(|x|- x)\nlet h = |a,b|a+b\nlet k = || 1\n"
            ),
            "fn apply(f: fn(Int) -> Int) -> Int {\n    return f(1)\n}\nlet g = xs.map(|x| x * 2)\nlet n = xs.sort_by(|x| -x)\nlet h = |a, b| a + b\nlet k = || 1\n"
        );
    }

    #[test]
    fn semicolons_between_statements_stay() {
        assert_eq!(fmt("let a = 1;let b = 2;\n"), "let a = 1; let b = 2\n");
    }

    #[test]
    fn arabic_code() {
        assert_eq!(
            fmt(
                "fn التقدير(الدرجة:Int)->String{\nreturn match الدرجة>=90{\ntrue=>\"ممتاز\"\nfalse=>\"جيد\"\n}\n}\n"
            ),
            "fn التقدير(الدرجة: Int) -> String {\n    return match الدرجة >= 90 {\n        true => \"ممتاز\"\n        false => \"جيد\"\n    }\n}\n"
        );
    }

    #[test]
    fn lexical_errors_are_reported() {
        assert!(matches!(
            format("let s = \"open\n"),
            Err(FormatError::Lex(_))
        ));
    }

    #[test]
    fn continuation_lines_and_nested_brackets() {
        assert_eq!(
            fmt("fn f(s: String) -> Bool {
let ok = s.contains(\"a\")
|| s.contains(\"b\")
&& s.length > 2
return ok
}
"),
            "fn f(s: String) -> Bool {
    let ok = s.contains(\"a\")
        || s.contains(\"b\")
        && s.length > 2
    return ok
}
"
        );
        // Two brackets opened on one line indent their contents once.
        assert_eq!(
            fmt("fn main() {
let xs = f([
1,
2
])
print(xs)
}
"),
            "fn main() {
    let xs = f([
        1,
        2
    ])
    print(xs)
}
"
        );
    }
}
