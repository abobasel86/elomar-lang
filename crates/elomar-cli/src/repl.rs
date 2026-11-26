//! `elomar repl`: type code and see the result at once.
//!
//! Each input is checked as part of one program made of the declarations
//! typed so far and a function holding the statements typed so far, each
//! in its own nested block (so `let x` may be typed again). Only the new
//! input runs; [`elomar_interpreter::Session`] keeps the variables.

use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::ExitCode;

use elomar_ast::{Item, Program, Stmt, StmtKind};
use elomar_diagnostics::{Diagnostic, Lang, SourceFile, SourceMap, Span, render_in};
use elomar_interpreter::{Session, Value};
use rustyline::DefaultEditor;
use rustyline::error::ReadlineError;

use crate::{Reporter, StdHost, load_ai_runtime, project_root};

/// The function that holds the statements typed so far.
const FN_NAME: &str = "__elomar_repl";
/// The name of the (never saved) file the session is checked as.
const FILE_NAME: &str = "repl.elo";

/// A declaration typed at the prompt.
struct Decl {
    /// The name it defines, for replacing it when typed again.
    name: Option<String>,
    text: String,
}

/// Where the new input sits in the program text built for checking.
struct Built {
    text: String,
    input_start: usize,
}

struct Repl<'r> {
    decls: Vec<Decl>,
    /// Inputs that defined variables (`let`, `var`), oldest first.
    body: Vec<String>,
    session: Session,
    cloud_models: Vec<String>,
    root: PathBuf,
    reporter: &'r Reporter,
}

pub(crate) fn run(reporter: &Reporter) -> ExitCode {
    let entry = Path::new(FILE_NAME);
    let ai = match load_ai_runtime(entry) {
        Ok(ai) => ai,
        Err((config_path, message)) => {
            let shown = config_path.display();
            reporter.say(
                &format!("error: invalid `{shown}`: {message}"),
                &format!("خطأ: الملف `{shown}` غير صالح: {message}"),
            );
            return ExitCode::from(2);
        }
    };
    elomar_interpreter::with_large_stack(move || {
        let mut repl = Repl {
            decls: Vec::new(),
            body: Vec::new(),
            session: Session::new(Box::new(StdHost { args: Vec::new() }), ai.runtime, ai.fs),
            cloud_models: ai.cloud_models,
            root: project_root(entry),
            reporter,
        };
        repl.main_loop()
    })
}

impl Repl<'_> {
    fn ar(&self) -> bool {
        self.reporter.opts.lang == Lang::Ar
    }

    fn main_loop(&mut self) -> ExitCode {
        let version = env!("CARGO_PKG_VERSION");
        self.reporter.out(
            &format!("Elomar-Lang {version}. Type code to run it, :help for help, :quit to leave."),
            &format!("Elomar-Lang {version}. اكتب كودًا لتنفيذه، و:help للمساعدة، و:quit للخروج."),
        );
        let Ok(mut editor) = DefaultEditor::new() else {
            self.reporter.say(
                "error: cannot read from the terminal",
                "خطأ: تعذّرت القراءة من الطرفية",
            );
            return ExitCode::from(2);
        };
        let history = history_file();
        if let Some(h) = &history {
            let _ = editor.load_history(h);
        }
        let mut pending = String::new();
        let code = loop {
            let prompt = if pending.is_empty() { "> " } else { ". " };
            match editor.readline(prompt) {
                Ok(line) => {
                    if !pending.is_empty() {
                        pending.push('\n');
                    }
                    pending.push_str(&line);
                    if needs_more(&pending) {
                        continue;
                    }
                    let input = std::mem::take(&mut pending);
                    let trimmed = input.trim();
                    if trimmed.is_empty() {
                        continue;
                    }
                    let _ = editor.add_history_entry(trimmed);
                    if let Some(command) = trimmed.strip_prefix(':') {
                        match self.command(command.trim()) {
                            Some(code) => break code,
                            None => continue,
                        }
                    }
                    if let Some(code) = self.submit(&input) {
                        break code;
                    }
                }
                // Ctrl+C drops what was typed so far.
                Err(ReadlineError::Interrupted) => {
                    pending.clear();
                }
                Err(_) => break ExitCode::SUCCESS,
            }
        };
        if let Some(h) = &history {
            let _ = editor.save_history(h);
        }
        code
    }

    /// Runs a `:command`. Returns an exit code to leave the REPL.
    fn command(&mut self, command: &str) -> Option<ExitCode> {
        match command {
            "q" | "quit" | "exit" => return Some(ExitCode::SUCCESS),
            "reset" => {
                self.decls.clear();
                self.body.clear();
                self.session.clear();
                self.reporter.out("Cleared.", "مُسح كل شيء.");
            }
            "show" => {
                for d in &self.decls {
                    println!("{}", d.text);
                }
                for s in &self.body {
                    println!("{s}");
                }
            }
            "h" | "help" => self.reporter.out(HELP_EN, HELP_AR),
            other => self.reporter.say(
                &format!("unknown command `:{other}`; type :help"),
                &format!("أمر غير معروف `:{other}`؛ اكتب :help"),
            ),
        }
        None
    }

    /// Checks and runs one input. Returns an exit code if the program
    /// asked to stop (`os.exit`).
    fn submit(&mut self, input: &str) -> Option<ExitCode> {
        let decl = declaration_name(input);
        let is_decl = decl.is_some();
        let decl_name = decl.flatten();
        let built = self.build(input, is_decl, decl_name.as_deref());

        let loaded = match elomar_loader::load_with(
            Path::new(FILE_NAME),
            &self.root,
            Some(built.text.clone()),
        ) {
            Ok(l) => l,
            Err(e) => {
                self.reporter
                    .say(&format!("error: {e}"), &format!("خطأ: {e}"));
                return None;
            }
        };
        let mut program = loaded.program;
        let mut diags = loaded.diagnostics;
        if diags.iter().all(|d| !d.is_error()) {
            diags.extend(elomar_types::check(
                &mut program,
                elomar_types::CheckOptions {
                    require_main: false,
                    cloud_models: self.cloud_models.clone(),
                },
            ));
        }
        let region = built.input_start..built.input_start + input.len();
        let shown: Vec<&Diagnostic> = diags
            .iter()
            .filter(|d| {
                let inside = region.contains(&(d.label.span.start as usize));
                // Shadowing and unused values are normal at a prompt, and
                // warnings about earlier input were shown already.
                d.is_error() || (inside && !matches!(d.code, "W1001" | "W1002"))
            })
            .collect();
        for d in &shown {
            self.emit(d, input, built.input_start, &loaded.sources);
        }
        if shown.iter().any(|d| d.is_error()) {
            return None;
        }

        // Earlier inputs' closures point into earlier programs, so every
        // program stays for the whole session.
        let program: &'static Program = Box::leak(Box::new(program));
        let stmts = if is_decl {
            Vec::new()
        } else {
            input_statements(program, self.body.len())?
        };
        let mut result = Ok(None);
        if stmts.is_empty() {
            result = self.session.step(program, None);
        }
        for stmt in &stmts {
            result = self.session.step(program, Some(stmt));
            if result.is_err() {
                break;
            }
        }
        let _ = std::io::stdout().flush();
        match result {
            Ok(value) => {
                if is_decl {
                    self.decls
                        .retain(|d| decl_name.is_none() || d.name != decl_name);
                    self.decls.push(Decl {
                        name: decl_name,
                        text: input.trim_end().to_string(),
                    });
                } else if defines_names(&stmts) {
                    self.body.push(input.trim_end().to_string());
                }
                if let Some(v) = value {
                    show_value(&v);
                }
                None
            }
            Err(err) if err.exit.is_some() => Some(ExitCode::from(err.exit.unwrap_or(0) as u8)),
            Err(err) => {
                self.emit(
                    &err.to_diagnostic(),
                    input,
                    built.input_start,
                    &loaded.sources,
                );
                // Statements before the failing one ran; keep them so their
                // variables stay known to the checker.
                if !is_decl && stmts.len() > 1 && defines_names(&stmts) {
                    self.body.push(input.trim_end().to_string());
                }
                None
            }
        }
    }

    /// The program text for checking `input`: the declarations (without the
    /// one `input` replaces), then the statements, each opening a block for
    /// the ones after it.
    fn build(&self, input: &str, is_decl: bool, replaces: Option<&str>) -> Built {
        let mut text = String::new();
        let mut input_start = 0;
        for d in &self.decls {
            if is_decl && replaces.is_some() && d.name.as_deref() == replaces {
                continue;
            }
            text.push_str(&d.text);
            text.push_str("\n\n");
        }
        if is_decl {
            input_start = text.len();
            text.push_str(input);
            text.push_str("\n\n");
        }
        text.push_str("async fn ");
        text.push_str(FN_NAME);
        text.push_str("() {\n");
        for s in &self.body {
            text.push_str(s);
            text.push_str("\n{\n");
        }
        if !is_decl {
            input_start = text.len();
            text.push_str(input);
            text.push('\n');
        }
        for _ in &self.body {
            text.push_str("}\n");
        }
        text.push_str("}\n");
        Built { text, input_start }
    }

    /// Shows a diagnostic: against the input alone when it points into the
    /// input, else against the whole session text.
    fn emit(&self, diag: &Diagnostic, input: &str, input_start: usize, sources: &SourceMap) {
        let region = input_start..=input_start + input.len();
        let inside =
            |s: Span| region.contains(&(s.start as usize)) && region.contains(&(s.end as usize));
        let opts = self.reporter.opts;
        let text = if inside(diag.label.span) {
            let mut d = diag.clone();
            let shift =
                |s: Span| Span::new(s.start as usize - input_start, s.end as usize - input_start);
            d.label.span = shift(d.label.span);
            d.notes.retain(|n| inside(n.span));
            for n in &mut d.notes {
                n.span = shift(n.span);
            }
            let file = SourceFile::new(
                if self.ar() {
                    "<المدخل>"
                } else {
                    "<input>"
                },
                input,
            );
            render_in(&d, &SourceMap::single(file), opts)
        } else {
            render_in(diag, sources, opts)
        };
        eprintln!("{text}");
    }
}

/// The statements of the newest input: inside the REPL function, one block
/// deeper per earlier statement.
fn input_statements(program: &'static Program, depth: usize) -> Option<Vec<&'static Stmt>> {
    let f = program.items.iter().find_map(|i| match i {
        Item::Fn(f) if f.name.name == FN_NAME => Some(f),
        _ => None,
    })?;
    let mut stmts = &f.body.stmts;
    for _ in 0..depth {
        match stmts.last().map(|s| &s.kind) {
            Some(StmtKind::Block(b)) => stmts = &b.stmts,
            _ => return None,
        }
    }
    Some(stmts.iter().collect())
}

/// Whether the statements define variables, which later inputs may use.
/// Other statements need not be checked again.
fn defines_names(stmts: &[&Stmt]) -> bool {
    stmts
        .iter()
        .any(|s| matches!(s.kind, StmtKind::Let { .. } | StmtKind::LetTuple { .. }))
}

/// `Some(name)` if `input` is a declaration (`fn`, `type`, `enum`, `trait`,
/// `impl`, `use`), with the name it defines when it defines one.
fn declaration_name(input: &str) -> Option<Option<String>> {
    use elomar_lexer::TokenKind as K;
    // Arabic keywords (`دالة`, `نوع`, …) count as their English spelling.
    let kind = |w: &str| elomar_lexer::keyword(w);
    let mut words = input.split_whitespace().peekable();
    while words
        .peek()
        .is_some_and(|w| matches!(kind(w), Some(K::Pub | K::Async)))
    {
        words.next();
    }
    let word = words.next()?;
    let keyword = match kind(word) {
        Some(k) => elomar_lexer::keyword_spellings(&k).map_or(word, |(en, _)| en),
        None => word,
    };
    match keyword {
        "fn" | "type" | "enum" | "trait" => {
            let name: String = words
                .next()
                .unwrap_or("")
                .chars()
                .take_while(|c| c.is_alphanumeric() || *c == '_')
                .collect();
            Some((!name.is_empty()).then(|| format!("{keyword} {name}")))
        }
        "impl" => Some(None),
        "use" => Some(Some(format!("use {}", words.collect::<Vec<_>>().join(" ")))),
        _ => None,
    }
}

/// Whether `text` is unfinished: an open bracket or string, or a last line
/// ending in an operator or comma.
fn needs_more(text: &str) -> bool {
    let mut depth = 0i32;
    let mut chars = text.chars().peekable();
    let mut in_string = false;
    while let Some(c) = chars.next() {
        if in_string {
            match c {
                '\\' => {
                    chars.next();
                }
                '"' => in_string = false,
                _ => {}
            }
            continue;
        }
        match c {
            '"' => in_string = true,
            '/' if chars.peek() == Some(&'/') => {
                // A comment runs to the end of the line.
                for c in chars.by_ref() {
                    if c == '\n' {
                        break;
                    }
                }
            }
            '(' | '[' | '{' => depth += 1,
            ')' | ']' | '}' => depth -= 1,
            _ => {}
        }
    }
    if in_string || depth > 0 {
        return true;
    }
    let last = text.trim_end();
    ["=", ",", "+", "-", "*", "/", "&&", "||", "->", "=>", "|"]
        .iter()
        .any(|op| last.ends_with(op) && !last.ends_with("//"))
}

/// Prints the value of an expression typed at the prompt. Strings get
/// quotes, so `"5"` and `5` look different.
fn show_value(v: &Value) {
    match v {
        Value::Unit => {}
        Value::Str(s) => println!("\"{}\"", s.replace('\\', "\\\\").replace('"', "\\\"")),
        v => println!("{v}"),
    }
}

/// `~/.elomar_history`, where typed lines are kept between sessions.
fn history_file() -> Option<PathBuf> {
    std::env::var_os("USERPROFILE")
        .or_else(|| std::env::var_os("HOME"))
        .map(|home| PathBuf::from(home).join(".elomar_history"))
}

const HELP_EN: &str = "\
Type code and press Enter. An expression shows its value; `let`, `var`,
`fn`, `type` and the rest are remembered for the next lines. A line with an
open bracket continues on the next one.

  :help    this help
  :show    what you have typed so far
  :reset   forget everything
  :quit    leave (also Ctrl+D)

Ctrl+C clears an unfinished input. `await` works at the prompt, and
functions see their parameters, not the variables typed at the prompt.";

const HELP_AR: &str = "\
اكتب كودًا واضغط Enter. التعبير يُظهر قيمته، و`let` و`var` و`fn` و`type`
وغيرها تُحفظ للأسطر التالية. والسطر الذي فيه قوس مفتوح يكمل في السطر التالي.

  :help    هذه المساعدة
  :show    ما كتبته حتى الآن
  :reset   امسح كل شيء
  :quit    اخرج (أو Ctrl+D)

Ctrl+C يمسح مدخلًا لم يكتمل. و`await` تعمل مباشرة، والدوال ترى معاملاتها لا
المتغيرات المكتوبة في السطر التفاعلي.";
