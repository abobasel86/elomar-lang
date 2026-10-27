//! Source spans and bilingual diagnostics.
//!
//! Every diagnostic carries both an English and an Arabic message under the
//! same stable code (e.g. `E2001`), so the CLI can render either language.

use std::fmt::Write as _;

/// A half-open byte range `[start, end)` into a source file.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct Span {
    pub start: u32,
    pub end: u32,
}

impl Span {
    pub fn new(start: usize, end: usize) -> Self {
        Span {
            start: start as u32,
            end: end as u32,
        }
    }

    /// The smallest span covering both `self` and `other`.
    pub fn to(self, other: Span) -> Span {
        Span {
            start: self.start.min(other.start),
            end: self.end.max(other.end),
        }
    }
}

/// Output language for diagnostics.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Lang {
    #[default]
    En,
    Ar,
}

impl Lang {
    /// Parses `en`/`ar` (also `english`/`arabic`, case-insensitive).
    pub fn parse(s: &str) -> Option<Lang> {
        match s.trim().to_ascii_lowercase().as_str() {
            "en" | "english" => Some(Lang::En),
            "ar" | "arabic" => Some(Lang::Ar),
            _ => None,
        }
    }
}

/// A message available in both supported languages.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Text {
    pub en: String,
    pub ar: String,
}

impl Text {
    pub fn new(en: impl Into<String>, ar: impl Into<String>) -> Self {
        Text {
            en: en.into(),
            ar: ar.into(),
        }
    }

    pub fn get(&self, lang: Lang) -> &str {
        match lang {
            Lang::En => &self.en,
            Lang::Ar => &self.ar,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Severity {
    Error,
    Warning,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Label {
    pub span: Span,
    pub text: Option<Text>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Diagnostic {
    pub severity: Severity,
    pub code: &'static str,
    pub message: Text,
    pub label: Label,
    pub notes: Vec<Label>,
    pub hint: Option<Text>,
}

impl Diagnostic {
    pub fn error(code: &'static str, span: Span, message: Text) -> Self {
        Diagnostic {
            severity: Severity::Error,
            code,
            message,
            label: Label { span, text: None },
            notes: Vec::new(),
            hint: None,
        }
    }

    pub fn warning(code: &'static str, span: Span, message: Text) -> Self {
        Diagnostic {
            severity: Severity::Warning,
            ..Diagnostic::error(code, span, message)
        }
    }

    /// Text shown under the caret of the primary span.
    pub fn with_label(mut self, text: Text) -> Self {
        self.label.text = Some(text);
        self
    }

    /// A secondary location, e.g. where something was first declared.
    pub fn with_note(mut self, span: Span, text: Text) -> Self {
        self.notes.push(Label {
            span,
            text: Some(text),
        });
        self
    }

    pub fn with_hint(mut self, text: Text) -> Self {
        self.hint = Some(text);
        self
    }

    pub fn is_error(&self) -> bool {
        self.severity == Severity::Error
    }
}

/// A named source text with precomputed line offsets.
#[derive(Debug, Clone)]
pub struct SourceFile {
    pub name: String,
    pub text: String,
    line_starts: Vec<usize>,
}

impl SourceFile {
    pub fn new(name: impl Into<String>, text: impl Into<String>) -> Self {
        let text = text.into();
        let mut line_starts = vec![0];
        for (i, b) in text.bytes().enumerate() {
            if b == b'\n' {
                line_starts.push(i + 1);
            }
        }
        SourceFile {
            name: name.into(),
            text,
            line_starts,
        }
    }

    /// 1-based line and column (in characters) of a byte offset.
    pub fn line_col(&self, offset: usize) -> (usize, usize) {
        let offset = offset.min(self.text.len());
        let line = match self.line_starts.binary_search(&offset) {
            Ok(i) => i,
            Err(i) => i - 1,
        };
        let start = self.line_starts[line];
        let col = self.text[start..offset].chars().count() + 1;
        (line + 1, col)
    }

    /// The text of a 1-based line, without its line terminator.
    pub fn line_text(&self, line: usize) -> &str {
        let start = self.line_starts[line - 1];
        let end = self
            .line_starts
            .get(line)
            .copied()
            .unwrap_or(self.text.len());
        self.text[start..end].trim_end_matches(['\n', '\r'])
    }
}

/// Controls how [`render`] formats a diagnostic.
#[derive(Debug, Clone, Copy, Default)]
pub struct RenderOptions {
    pub lang: Lang,
    pub color: bool,
}

const RED: &str = "\x1b[1;31m";
const YELLOW: &str = "\x1b[1;33m";
const BLUE: &str = "\x1b[1;34m";
const BOLD: &str = "\x1b[1m";
const RESET: &str = "\x1b[0m";

/// Renders a diagnostic in a rustc-like layout:
///
/// ```text
/// error[E2001]: type mismatch: expected `Int`, found `String`
///   --> main.elo:3:18
///    |
///  3 |     let x: Int = "hi"
///    |                  ^^^^ expected `Int`
///    = hint: ...
/// ```
pub fn render(diag: &Diagnostic, file: &SourceFile, opts: RenderOptions) -> String {
    let paint = |code: &'static str, s: &str| -> String {
        if opts.color {
            format!("{code}{s}{RESET}")
        } else {
            s.to_string()
        }
    };
    let lang = opts.lang;
    let (kind, kind_color) = match (diag.severity, lang) {
        (Severity::Error, Lang::En) => ("error", RED),
        (Severity::Error, Lang::Ar) => ("خطأ", RED),
        (Severity::Warning, Lang::En) => ("warning", YELLOW),
        (Severity::Warning, Lang::Ar) => ("تحذير", YELLOW),
    };

    let mut out = String::new();
    let _ = writeln!(
        out,
        "{}{}",
        paint(kind_color, &format!("{kind}[{}]", diag.code)),
        paint(BOLD, &format!(": {}", diag.message.get(lang)))
    );

    let mut labels = vec![&diag.label];
    labels.extend(diag.notes.iter());
    let max_line = labels
        .iter()
        .map(|l| file.line_col(l.span.start as usize).0)
        .max()
        .unwrap_or(1);
    let gutter = max_line.to_string().len();
    let pad = " ".repeat(gutter);

    let (line, col) = file.line_col(diag.label.span.start as usize);
    let _ = writeln!(
        out,
        "{pad}{} {}:{line}:{col}",
        paint(BLUE, "-->"),
        file.name
    );

    for (i, label) in labels.iter().enumerate() {
        let (line, col) = file.line_col(label.span.start as usize);
        let text = file.line_text(line);
        let width = caret_width(file, label.span, text, col);
        let marker = if i == 0 { "^" } else { "-" };
        let color = if i == 0 { kind_color } else { BLUE };
        let _ = writeln!(out, "{pad} {}", paint(BLUE, "|"));
        let _ = writeln!(
            out,
            "{} {} {text}",
            paint(BLUE, &format!("{line:>gutter$}")),
            paint(BLUE, "|")
        );
        let caret = format!("{}{}", " ".repeat(col - 1), marker.repeat(width));
        let msg = label
            .text
            .as_ref()
            .map(|t| format!(" {}", t.get(lang)))
            .unwrap_or_default();
        let _ = writeln!(
            out,
            "{pad} {} {}",
            paint(BLUE, "|"),
            paint(color, &format!("{caret}{msg}"))
        );
    }

    if let Some(hint) = &diag.hint {
        let word = match lang {
            Lang::En => "hint",
            Lang::Ar => "تلميح",
        };
        let _ = writeln!(out, "{pad} {} {word}: {}", paint(BLUE, "="), hint.get(lang));
    }
    out
}

/// Number of caret characters to draw under a span on its first line.
fn caret_width(file: &SourceFile, span: Span, line_text: &str, col: usize) -> usize {
    let start = span.start as usize;
    let end = (span.end as usize).max(start).min(file.text.len());
    let in_span = file.text[start..end]
        .split('\n')
        .next()
        .unwrap_or("")
        .chars()
        .count();
    let remaining = line_text.chars().count().saturating_sub(col - 1);
    in_span.min(remaining.max(1)).max(1)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn line_col_counts_characters_not_bytes() {
        let f = SourceFile::new("t.elo", "let اسم = 1\nprint(اسم)");
        let off = f.text.find('=').unwrap();
        assert_eq!(f.line_col(off), (1, 9));
        let off2 = f.text.rfind('(').unwrap();
        assert_eq!(f.line_col(off2), (2, 6));
    }

    #[test]
    fn renders_both_languages() {
        let f = SourceFile::new("t.elo", "let x: Int = \"hi\"\n");
        let start = f.text.find('"').unwrap();
        let d = Diagnostic::error(
            "E2001",
            Span::new(start, start + 4),
            Text::new("type mismatch", "عدم توافق الأنواع"),
        )
        .with_label(Text::new("expected `Int`", "المتوقع `Int`"));
        let en = render(&d, &f, RenderOptions::default());
        assert!(en.starts_with("error[E2001]: type mismatch"));
        assert!(en.contains("--> t.elo:1:14"));
        assert!(en.contains("^^^^ expected `Int`"));
        let ar = render(
            &d,
            &f,
            RenderOptions {
                lang: Lang::Ar,
                color: false,
            },
        );
        assert!(ar.starts_with("خطأ[E2001]: عدم توافق الأنواع"));
    }
}
