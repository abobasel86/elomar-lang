//! Tokenizer for Elomar-Lang.
//!
//! Lexing happens in two passes: [`raw_tokens`] turns the text into tokens
//! with a `Newline` marker at every line break, then [`apply_newline_rules`]
//! keeps only the line breaks that terminate a statement (spec §8.5).

use elomar_diagnostics::{Diagnostic, Span, Text};
use unicode_normalization::UnicodeNormalization;

/// A piece of an interpolated string literal (`"Hello {name}"`).
#[derive(Debug, Clone, PartialEq)]
pub enum StrPart {
    /// Literal text, with escapes already applied.
    Lit(String),
    /// The source of an embedded expression. `offset` is its position
    /// relative to the start of the string token, so the parser can lex it
    /// with correct spans while the token itself stays position-independent.
    Code { src: String, offset: usize },
}

#[derive(Debug, Clone, PartialEq)]
pub enum TokenKind {
    Int(i64),
    Float(f64),
    /// `500ms`, `10s`, `2m`, `1h`, in milliseconds.
    Duration(i64),
    Str(String),
    /// A string literal with at least one `{expr}` part.
    Template(Vec<StrPart>),
    Ident(String),

    // Keywords
    Let,
    Var,
    Fn,
    Return,
    If,
    Else,
    While,
    For,
    In,
    Break,
    Continue,
    True,
    False,
    Type,
    Enum,
    Match,
    Use,
    Pub,
    Async,
    Await,
    Trait,
    Impl,
    SelfKw,
    As,
    Div,

    // Operators and punctuation
    Plus,
    Minus,
    Star,
    Slash,
    Percent,
    Eq,
    EqEq,
    Bang,
    BangEq,
    Lt,
    Le,
    Gt,
    Ge,
    AndAnd,
    OrOr,
    PlusEq,
    MinusEq,
    StarEq,
    SlashEq,
    Arrow,
    FatArrow,
    Dot,
    DotDot,
    DotDotEq,
    Comma,
    Colon,
    Semi,
    /// `?`: propagate an `Err` or `None` to the caller.
    Question,
    /// `|`: around closure parameters, `|x| x * 2`.
    Pipe,
    LParen,
    RParen,
    LBracket,
    RBracket,
    LBrace,
    RBrace,

    /// A line break that terminates a statement.
    Newline,
    /// A `//` or `/* */` comment; only produced by [`tokens_with_comments`].
    Comment,
    Eof,
}

impl TokenKind {
    /// How the token is written in source, for diagnostics.
    pub fn describe(&self) -> String {
        use TokenKind::*;
        let s = match self {
            Int(n) => return n.to_string(),
            Duration(ms) => return format!("{ms}ms"),
            Float(f) => return f.to_string(),
            Str(_) | Template(_) => return "string literal".into(),
            Ident(name) => return name.clone(),
            Newline => return "end of line".into(),
            Comment => return "comment".into(),
            Eof => return "end of file".into(),
            Let => "let",
            Var => "var",
            Fn => "fn",
            Return => "return",
            If => "if",
            Else => "else",
            While => "while",
            For => "for",
            In => "in",
            Break => "break",
            Continue => "continue",
            True => "true",
            False => "false",
            Type => "type",
            Enum => "enum",
            Match => "match",
            Use => "use",
            Pub => "pub",
            Async => "async",
            Await => "await",
            Trait => "trait",
            Impl => "impl",
            SelfKw => "self",
            As => "as",
            Div => "div",
            Plus => "+",
            Minus => "-",
            Star => "*",
            Slash => "/",
            Percent => "%",
            Eq => "=",
            EqEq => "==",
            Bang => "!",
            BangEq => "!=",
            Lt => "<",
            Le => "<=",
            Gt => ">",
            Ge => ">=",
            AndAnd => "&&",
            OrOr => "||",
            PlusEq => "+=",
            MinusEq => "-=",
            StarEq => "*=",
            SlashEq => "/=",
            Arrow => "->",
            FatArrow => "=>",
            Dot => ".",
            DotDot => "..",
            DotDotEq => "..=",
            Comma => ",",
            Colon => ":",
            Semi => ";",
            Question => "?",
            Pipe => "|",
            LParen => "(",
            RParen => ")",
            LBracket => "[",
            RBracket => "]",
            LBrace => "{",
            RBrace => "}",
        };
        format!("`{s}`")
    }

    /// Arabic rendering of [`TokenKind::describe`] for the few non-symbol cases.
    pub fn describe_ar(&self) -> String {
        match self {
            TokenKind::Str(_) | TokenKind::Template(_) => "نص حرفي".into(),
            TokenKind::Newline => "نهاية السطر".into(),
            TokenKind::Eof => "نهاية الملف".into(),
            other => other.describe(),
        }
    }

    /// Whether a line break after this token ends the statement (spec §8.5).
    fn can_end_statement(&self) -> bool {
        use TokenKind::*;
        matches!(
            self,
            Int(_)
                | Float(_)
                | Duration(_)
                | Str(_)
                | Template(_)
                | Ident(_)
                | True
                | False
                | SelfKw
                | RParen
                | RBracket
                | RBrace
                | Question
                | Return
                | Break
                | Continue
        )
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct Token {
    pub kind: TokenKind,
    pub span: Span,
}

/// Tokenizes `src`. Always ends with an `Eof` token; lexical errors are
/// reported and the offending characters skipped, so parsing can continue.
pub fn lex(src: &str) -> (Vec<Token>, Vec<Diagnostic>) {
    lex_at(src, 0)
}

/// Like [`lex`], with every span shifted by `base` (see `SourceMap`).
pub fn lex_at(src: &str, base: usize) -> (Vec<Token>, Vec<Diagnostic>) {
    let (raw, diags) = raw_tokens_at(src, base);
    (apply_newline_rules(raw), diags)
}

fn keyword(s: &str) -> Option<TokenKind> {
    use TokenKind::*;
    Some(match s {
        "let" => Let,
        "var" => Var,
        "fn" => Fn,
        "return" => Return,
        "if" => If,
        "else" => Else,
        "while" => While,
        "for" => For,
        "in" => In,
        "break" => Break,
        "continue" => Continue,
        "true" => True,
        "false" => False,
        "type" => Type,
        "enum" => Enum,
        "match" => Match,
        "use" => Use,
        "pub" => Pub,
        "async" => Async,
        "await" => Await,
        "trait" => Trait,
        "impl" => Impl,
        "self" => SelfKw,
        "as" => As,
        "div" => Div,
        _ => return None,
    })
}

fn is_ident_start(c: char) -> bool {
    c == '_' || unicode_ident::is_xid_start(c)
}

fn is_ident_continue(c: char) -> bool {
    unicode_ident::is_xid_continue(c)
}

struct Lexer<'a> {
    src: &'a str,
    pos: usize,
    /// Added to every span, so several files can share one span space.
    base: usize,
    tokens: Vec<Token>,
    diags: Vec<Diagnostic>,
    /// Emit `Comment` tokens (for the formatter) instead of skipping them.
    keep_comments: bool,
}

impl<'a> Lexer<'a> {
    fn peek(&self) -> Option<char> {
        self.src[self.pos..].chars().next()
    }

    fn peek_at(&self, n: usize) -> Option<char> {
        self.src[self.pos..].chars().nth(n)
    }

    fn bump(&mut self) -> Option<char> {
        let c = self.peek()?;
        self.pos += c.len_utf8();
        Some(c)
    }

    fn push(&mut self, kind: TokenKind, start: usize) {
        self.tokens.push(Token {
            kind,
            span: Span::new(self.base + start, self.base + self.pos),
        });
    }

    fn error(&mut self, code: &'static str, start: usize, end: usize, text: Text) {
        self.diags.push(Diagnostic::error(
            code,
            Span::new(self.base + start, self.base + end),
            text,
        ));
    }

    fn run(mut self) -> (Vec<Token>, Vec<Diagnostic>) {
        while let Some(c) = self.peek() {
            let start = self.pos;
            match c {
                '\n' => {
                    self.bump();
                    self.push(TokenKind::Newline, start);
                }
                c if c.is_whitespace() => {
                    self.bump();
                }
                '/' if self.peek_at(1) == Some('/') => {
                    while let Some(c) = self.peek() {
                        if c == '\n' {
                            break;
                        }
                        self.bump();
                    }
                    if self.keep_comments {
                        self.push(TokenKind::Comment, start);
                    }
                }
                '/' if self.peek_at(1) == Some('*') => self.block_comment(start),
                '"' => self.string(start),
                c if c.is_ascii_digit() => self.number(start),
                c if is_ident_start(c) => self.ident(start),
                _ => self.punct(start, c),
            }
        }
        let end = self.src.len();
        self.tokens.push(Token {
            kind: TokenKind::Eof,
            span: Span::new(self.base + end, self.base + end),
        });
        (self.tokens, self.diags)
    }

    fn block_comment(&mut self, start: usize) {
        self.pos += 2;
        let mut saw_newline = false;
        loop {
            match self.peek() {
                None => {
                    self.error(
                        "E0004",
                        start,
                        start + 2,
                        Text::new("unterminated block comment", "تعليق متعدد الأسطر غير مغلق"),
                    );
                    return;
                }
                Some('*') if self.peek_at(1) == Some('/') => {
                    self.pos += 2;
                    break;
                }
                Some(c) => {
                    saw_newline |= c == '\n';
                    self.bump();
                }
            }
        }
        if self.keep_comments {
            self.push(TokenKind::Comment, start);
            return;
        }
        // A comment spanning lines behaves like a line break (as in Go).
        if saw_newline {
            self.push(TokenKind::Newline, start);
        }
    }

    fn string(&mut self, start: usize) {
        self.bump();
        let mut value = String::new();
        let mut parts: Vec<StrPart> = Vec::new();
        loop {
            let Some(c) = self.peek() else {
                self.unterminated_string(start);
                break;
            };
            match c {
                '"' => {
                    self.bump();
                    break;
                }
                '\n' => {
                    self.unterminated_string(start);
                    break;
                }
                '\\' => {
                    let esc_start = self.pos;
                    self.bump();
                    match self.bump() {
                        Some('n') => value.push('\n'),
                        Some('t') => value.push('\t'),
                        Some('r') => value.push('\r'),
                        Some('0') => value.push('\0'),
                        Some('"') => value.push('"'),
                        Some('\\') => value.push('\\'),
                        Some('{') => value.push('{'),
                        Some('}') => value.push('}'),
                        Some('u') => {
                            if let Some(ch) = self.unicode_escape() {
                                value.push(ch);
                            } else {
                                self.invalid_escape(esc_start);
                            }
                        }
                        _ => self.invalid_escape(esc_start),
                    }
                }
                '{' => {
                    if !value.is_empty() {
                        parts.push(StrPart::Lit(std::mem::take(&mut value)));
                    }
                    if let Some(part) = self.interpolation(start) {
                        parts.push(part);
                    } else {
                        break;
                    }
                }
                _ => {
                    value.push(c);
                    self.bump();
                }
            }
        }
        if parts.is_empty() {
            self.push(TokenKind::Str(value), start);
        } else {
            if !value.is_empty() {
                parts.push(StrPart::Lit(value));
            }
            self.push(TokenKind::Template(parts), start);
        }
    }

    /// Reads `{expr}` inside a string, at the `{`. Braces inside the
    /// expression nest, and string literals inside it are skipped whole, so
    /// `"{m["key"]}"` and `"{ {a: 1}.a }"` work. Reports an error and
    /// returns `None` when the part is not closed on this line.
    fn interpolation(&mut self, string_start: usize) -> Option<StrPart> {
        let open = self.pos;
        self.bump();
        let code_start = self.pos;
        let mut depth = 0usize;
        loop {
            match self.peek() {
                None | Some('\n') => {
                    self.error(
                        "E0007",
                        open,
                        open + 1,
                        Text::new(
                            "`{` in a string is not closed; write `\\{` for a literal brace",
                            "القوس `{` داخل النص غير مغلق؛ اكتب `\\{` لقوس حرفي",
                        ),
                    );
                    return None;
                }
                // `"{\"a\": 1}"`: an escape can only mean text, so the `{`
                // was meant as a literal brace.
                Some('\\') => {
                    self.error(
                        "E0007",
                        open,
                        open + 1,
                        Text::new(
                            "`{` in a string starts an expression; write `\\{` for a literal brace",
                            "القوس `{` داخل النص يبدأ تعبيرًا؛ اكتب `\\{` لقوس حرفي",
                        ),
                    );
                    // Skip the rest of the string, so later lines lex normally.
                    while let Some(c) = self.peek() {
                        if c == '\n' {
                            break;
                        }
                        self.bump();
                        if c == '\\' {
                            self.bump();
                        } else if c == '"' {
                            break;
                        }
                    }
                    return None;
                }
                Some('{') => depth += 1,
                Some('}') if depth == 0 => break,
                Some('}') => depth -= 1,
                Some('"') => {
                    // A string inside the expression: skip to its end.
                    self.bump();
                    while let Some(c) = self.peek() {
                        if c == '"' || c == '\n' {
                            break;
                        }
                        if c == '\\' {
                            self.bump();
                        }
                        self.bump();
                    }
                    if self.peek() != Some('"') {
                        continue;
                    }
                }
                _ => {}
            }
            self.bump();
        }
        let src = self.src[code_start..self.pos].to_string();
        self.bump();
        if src.trim().is_empty() {
            self.error(
                "E0007",
                open,
                self.pos,
                Text::new(
                    "empty `{}` in a string; put an expression inside, or write `\\{` for a literal brace",
                    "`{}` فارغ داخل النص؛ ضع تعبيرًا داخله، أو اكتب `\\{` لقوس حرفي",
                ),
            );
            return Some(StrPart::Lit(String::new()));
        }
        Some(StrPart::Code {
            src,
            offset: code_start - string_start,
        })
    }

    /// Parses the `{XXXX}` part of a `\u{XXXX}` escape.
    fn unicode_escape(&mut self) -> Option<char> {
        if self.peek() != Some('{') {
            return None;
        }
        self.bump();
        let digits_start = self.pos;
        while matches!(self.peek(), Some(c) if c.is_ascii_hexdigit()) {
            self.bump();
        }
        let digits = &self.src[digits_start..self.pos];
        if self.peek() != Some('}') || digits.is_empty() || digits.len() > 6 {
            return None;
        }
        self.bump();
        u32::from_str_radix(digits, 16)
            .ok()
            .and_then(char::from_u32)
    }

    fn unterminated_string(&mut self, start: usize) {
        self.error(
            "E0002",
            start,
            start + 1,
            Text::new("unterminated string literal", "نص حرفي غير مغلق"),
        );
    }

    fn invalid_escape(&mut self, start: usize) {
        let end = self.pos;
        self.error(
            "E0003",
            start,
            end,
            Text::new(
                "invalid escape sequence in string",
                "تسلسل هروب غير صالح داخل النص",
            ),
        );
    }

    fn number(&mut self, start: usize) {
        let mut is_float = false;
        self.digits();
        // `1..5` is a range, not the float `1.`
        if self.peek() == Some('.') && matches!(self.peek_at(1), Some(c) if c.is_ascii_digit()) {
            is_float = true;
            self.bump();
            self.digits();
        }
        if matches!(self.peek(), Some('e' | 'E')) {
            let sign = matches!(self.peek_at(1), Some('+' | '-'));
            let digit_at = if sign { 2 } else { 1 };
            if matches!(self.peek_at(digit_at), Some(c) if c.is_ascii_digit()) {
                is_float = true;
                self.pos += digit_at;
                self.digits();
            }
        }
        let text: String = self.src[start..self.pos]
            .chars()
            .filter(|&c| c != '_')
            .collect();
        let suffix_start = self.pos;
        while matches!(self.peek(), Some(c) if is_ident_continue(c)) {
            self.bump();
        }
        let suffix = &self.src[suffix_start..self.pos];
        if !suffix.is_empty() {
            self.duration(start, &text, suffix, is_float);
            return;
        }
        if is_float {
            let value = text.parse::<f64>().unwrap_or(f64::INFINITY);
            self.push(TokenKind::Float(value), start);
        } else {
            match text.parse::<i64>() {
                Ok(n) => self.push(TokenKind::Int(n), start),
                Err(_) => {
                    let end = self.pos;
                    self.error(
                        "E0005",
                        start,
                        end,
                        Text::new(
                            "integer literal is too large for `Int` (64-bit)",
                            "العدد الصحيح أكبر من سعة `Int` (64 بت)",
                        ),
                    );
                    self.push(TokenKind::Int(0), start);
                }
            }
        }
    }

    /// A number with a unit suffix: `500ms`, `10s`, `2m`, `1h`.
    fn duration(&mut self, start: usize, digits: &str, suffix: &str, is_float: bool) {
        let unit_ms: Option<i64> = match suffix {
            "ms" => Some(1),
            "s" => Some(1_000),
            "m" => Some(60_000),
            "h" => Some(3_600_000),
            _ => None,
        };
        let end = self.pos;
        let Some(unit) = unit_ms.filter(|_| !is_float) else {
            self.error(
                "E0006",
                start,
                end,
                Text::new(
                    format!("invalid number suffix `{suffix}`"),
                    format!("لاحقة عدد غير صالحة `{suffix}`"),
                ),
            );
            self.diags.last_mut().expect("just pushed").hint = Some(Text::new(
                "durations are whole numbers with `ms`, `s`, `m` or `h`, e.g. `10s`; put a space between a number and a name",
                "المدد الزمنية أعداد صحيحة مع `ms` أو `s` أو `m` أو `h`، مثل `10s`؛ وضع مسافة بين العدد والاسم",
            ));
            self.push(TokenKind::Int(0), start);
            return;
        };
        match digits.parse::<i64>().ok().and_then(|n| n.checked_mul(unit)) {
            Some(ms) => self.push(TokenKind::Duration(ms), start),
            None => {
                self.error(
                    "E0005",
                    start,
                    end,
                    Text::new(
                        "duration literal is too large",
                        "المدة الزمنية أكبر من المسموح",
                    ),
                );
                self.push(TokenKind::Duration(0), start);
            }
        }
    }

    fn digits(&mut self) {
        while matches!(self.peek(), Some(c) if c.is_ascii_digit() || c == '_') {
            self.bump();
        }
    }

    fn ident(&mut self, start: usize) {
        self.bump();
        while matches!(self.peek(), Some(c) if is_ident_continue(c)) {
            self.bump();
        }
        let raw = &self.src[start..self.pos];
        let kind = keyword(raw).unwrap_or_else(|| TokenKind::Ident(raw.nfc().collect()));
        self.push(kind, start);
    }

    fn punct(&mut self, start: usize, c: char) {
        use TokenKind::*;
        let next = self.peek_at(1);
        let next2 = self.peek_at(2);
        let (kind, len) = match (c, next) {
            ('.', Some('.')) if next2 == Some('=') => (DotDotEq, 3),
            ('.', Some('.')) => (DotDot, 2),
            ('=', Some('=')) => (EqEq, 2),
            ('=', Some('>')) => (FatArrow, 2),
            ('!', Some('=')) => (BangEq, 2),
            ('<', Some('=')) => (Le, 2),
            ('>', Some('=')) => (Ge, 2),
            ('&', Some('&')) => (AndAnd, 2),
            ('|', Some('|')) => (OrOr, 2),
            ('+', Some('=')) => (PlusEq, 2),
            ('-', Some('=')) => (MinusEq, 2),
            ('*', Some('=')) => (StarEq, 2),
            ('/', Some('=')) => (SlashEq, 2),
            ('-', Some('>')) => (Arrow, 2),
            ('+', _) => (Plus, 1),
            ('-', _) => (Minus, 1),
            ('*', _) => (Star, 1),
            ('/', _) => (Slash, 1),
            ('%', _) => (Percent, 1),
            ('=', _) => (Eq, 1),
            ('!', _) => (Bang, 1),
            ('<', _) => (Lt, 1),
            ('>', _) => (Gt, 1),
            ('.', _) => (Dot, 1),
            (',', _) => (Comma, 1),
            (':', _) => (Colon, 1),
            (';', _) => (Semi, 1),
            ('?', _) => (Question, 1),
            ('|', _) => (Pipe, 1),
            ('(', _) => (LParen, 1),
            (')', _) => (RParen, 1),
            ('[', _) => (LBracket, 1),
            (']', _) => (RBracket, 1),
            ('{', _) => (LBrace, 1),
            ('}', _) => (RBrace, 1),
            _ => {
                self.bump();
                let end = self.pos;
                self.error(
                    "E0001",
                    start,
                    end,
                    Text::new(
                        format!("unexpected character `{c}`"),
                        format!("حرف غير متوقع `{c}`"),
                    ),
                );
                return;
            }
        };
        for _ in 0..len {
            self.bump();
        }
        self.push(kind, start);
    }
}

/// First pass: every token, with a `Newline` at each line break.
pub fn raw_tokens(src: &str) -> (Vec<Token>, Vec<Diagnostic>) {
    raw_tokens_at(src, 0)
}

/// Every token including comments, with a `Newline` at each line break
/// and no newline rules applied: the input of the formatter.
pub fn tokens_with_comments(src: &str) -> (Vec<Token>, Vec<Diagnostic>) {
    Lexer {
        src,
        pos: 0,
        base: 0,
        tokens: Vec::new(),
        diags: Vec::new(),
        keep_comments: true,
    }
    .run()
}

fn raw_tokens_at(src: &str, base: usize) -> (Vec<Token>, Vec<Diagnostic>) {
    Lexer {
        src,
        pos: 0,
        base,
        keep_comments: false,
        tokens: Vec::new(),
        diags: Vec::new(),
    }
    .run()
}

/// Second pass: keeps a line break only if it terminates a statement.
///
/// A line break survives when (1) the previous token can end a statement,
/// (2) we are not inside `(...)` or `[...]`, and (3) the next token is not
/// `.` or `else`. Consecutive line breaks collapse into one.
pub fn apply_newline_rules(raw: Vec<Token>) -> Vec<Token> {
    #[derive(PartialEq)]
    enum Delim {
        Paren,
        Brace,
    }
    let mut out: Vec<Token> = Vec::with_capacity(raw.len());
    let mut stack: Vec<Delim> = Vec::new();
    let mut i = 0;
    while i < raw.len() {
        let tok = &raw[i];
        match tok.kind {
            TokenKind::LParen | TokenKind::LBracket => stack.push(Delim::Paren),
            TokenKind::LBrace => stack.push(Delim::Brace),
            TokenKind::RParen | TokenKind::RBracket | TokenKind::RBrace => {
                stack.pop();
            }
            _ => {}
        }
        if tok.kind != TokenKind::Newline {
            out.push(tok.clone());
            i += 1;
            continue;
        }
        let first = i;
        while i < raw.len() && raw[i].kind == TokenKind::Newline {
            i += 1;
        }
        let inside_parens = stack.last() == Some(&Delim::Paren);
        let prev_ends = out.last().is_some_and(|t| t.kind.can_end_statement());
        let next = raw.get(i).map(|t| &t.kind);
        let continues = matches!(
            next,
            Some(TokenKind::Dot | TokenKind::Else | TokenKind::Eof) | None
        );
        if !inside_parens && prev_ends && !continues {
            out.push(raw[first].clone());
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use TokenKind::*;

    fn kinds(src: &str) -> Vec<TokenKind> {
        let (toks, diags) = lex(src);
        assert!(diags.is_empty(), "unexpected diagnostics: {diags:?}");
        toks.into_iter().map(|t| t.kind).collect()
    }

    #[test]
    fn newline_terminates_after_value() {
        assert_eq!(
            kinds("let x = 1\nlet y = 2"),
            vec![
                Let,
                Ident("x".into()),
                Eq,
                Int(1),
                Newline,
                Let,
                Ident("y".into()),
                Eq,
                Int(2),
                Eof
            ]
        );
    }

    #[test]
    fn newline_after_operator_continues() {
        assert_eq!(
            kinds("x = 1 +\n 2"),
            vec![Ident("x".into()), Eq, Int(1), Plus, Int(2), Eof]
        );
    }

    #[test]
    fn newlines_inside_parens_are_ignored() {
        assert_eq!(
            kinds("f(1,\n2\n)"),
            vec![
                Ident("f".into()),
                LParen,
                Int(1),
                Comma,
                Int(2),
                RParen,
                Eof
            ]
        );
    }

    #[test]
    fn else_and_dot_on_next_line_continue() {
        assert_eq!(kinds("}\nelse {"), vec![RBrace, Else, LBrace, Eof]);
        assert_eq!(
            kinds("xs\n  .length"),
            vec![Ident("xs".into()), Dot, Ident("length".into()), Eof]
        );
    }

    #[test]
    fn ranges_are_not_floats() {
        assert_eq!(kinds("0..10"), vec![Int(0), DotDot, Int(10), Eof]);
        assert_eq!(kinds("1..=3"), vec![Int(1), DotDotEq, Int(3), Eof]);
        assert_eq!(kinds("2.5e3"), vec![Float(2500.0), Eof]);
        assert_eq!(kinds("1_000"), vec![Int(1000), Eof]);
    }

    #[test]
    fn duration_literals() {
        assert_eq!(
            kinds("500ms 10s 2m 1h"),
            vec![
                Duration(500),
                Duration(10_000),
                Duration(120_000),
                Duration(3_600_000),
                Eof
            ]
        );
        let (_, diags) = lex("10x 1.5s");
        let codes: Vec<_> = diags.iter().map(|d| d.code).collect();
        assert_eq!(codes, vec!["E0006", "E0006"]);
    }

    #[test]
    fn arabic_identifiers_and_strings() {
        assert_eq!(
            kinds("let اسم = \"مرحبا\\n\""),
            vec![Let, Ident("اسم".into()), Eq, Str("مرحبا\n".into()), Eof]
        );
    }

    #[test]
    fn identifiers_are_nfc_normalized() {
        // "é" written as e + combining acute accent becomes the single code point.
        let decomposed = "caf\u{0065}\u{0301}";
        assert_eq!(kinds(decomposed), vec![Ident("caf\u{00e9}".into()), Eof]);
    }

    #[test]
    fn comments_are_skipped() {
        assert_eq!(
            kinds("1 // one\n/* two\n */ 2"),
            vec![Int(1), Newline, Int(2), Eof]
        );
    }

    #[test]
    fn reports_lexical_errors() {
        let (_, diags) = lex("let s = \"open\nlet t = @");
        let codes: Vec<_> = diags.iter().map(|d| d.code).collect();
        assert_eq!(codes, vec!["E0002", "E0001"]);
        let (_, diags) = lex("99999999999999999999");
        assert_eq!(diags[0].code, "E0005");
    }

    #[test]
    fn interpolated_strings() {
        assert_eq!(
            kinds(r#""Hi {name}!""#),
            vec![
                Template(vec![
                    StrPart::Lit("Hi ".into()),
                    StrPart::Code {
                        src: "name".into(),
                        offset: 5
                    },
                    StrPart::Lit("!".into()),
                ]),
                Eof
            ]
        );
        // Nested braces and strings stay inside the expression.
        assert_eq!(
            kinds(r#""{m["k"]} {{a: 1}.a}""#),
            vec![
                Template(vec![
                    StrPart::Code {
                        src: r#"m["k"]"#.into(),
                        offset: 2
                    },
                    StrPart::Lit(" ".into()),
                    StrPart::Code {
                        src: "{a: 1}.a".into(),
                        offset: 11
                    },
                ]),
                Eof
            ]
        );
        // `\{` and `\}` are literal braces; a lone `}` is literal too.
        assert_eq!(kinds(r#""\{x\} }""#), vec![Str("{x} }".into()), Eof]);
        for bad in [r#""{x""#, r#""{}""#, "\"{x\n}\""] {
            let (_, diags) = lex(bad);
            assert_eq!(diags[0].code, "E0007", "{bad}");
        }
        // An escape inside `{...}` means the brace was meant literally:
        // one clear error, and the next line still lexes.
        let (toks, diags) = lex("let s = \"{\\\"a\\\": 1}\"\nlet t = 2");
        assert_eq!(diags.len(), 1, "{diags:?}");
        assert_eq!(diags[0].code, "E0007");
        assert!(toks.iter().any(|t| t.kind == Ident("t".into())));
    }
}
