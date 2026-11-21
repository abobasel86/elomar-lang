//! Parser for Elomar-Lang: recursive descent for items and statements,
//! Pratt parsing for expressions. The authoritative grammar is
//! `docs/grammar.md`.

// Syntax errors are the cold path and are returned once per statement, so
// carrying the full `Diagnostic` by value is simpler than boxing it.
#![allow(clippy::result_large_err)]

use elomar_ast::*;
use elomar_diagnostics::{Diagnostic, Span, Text};
use elomar_lexer::{StrPart, Token, TokenKind};

/// Lexes and parses a source file. Lexical and syntax errors are both
/// returned; the program contains every item that parsed successfully.
pub fn parse(src: &str) -> (Program, Vec<Diagnostic>) {
    parse_at(src, 0)
}

/// Like [`parse`], with every span shifted by `base`, so several files can
/// share one span space (see `elomar_diagnostics::SourceMap`).
pub fn parse_at(src: &str, base: usize) -> (Program, Vec<Diagnostic>) {
    let (tokens, mut diags) = elomar_lexer::lex_at(src, base);
    let (program, parse_diags) = parse_tokens(tokens);
    diags.extend(parse_diags);
    (program, diags)
}

pub fn parse_tokens(tokens: Vec<Token>) -> (Program, Vec<Diagnostic>) {
    let mut p = Parser {
        tokens,
        pos: 0,
        diags: Vec::new(),
        self_ty: None,
    };
    let program = p.program();
    (program, p.diags)
}

type PResult<T> = Result<T, Diagnostic>;

/// Stop reporting after this many syntax errors; later ones are usually noise.
const MAX_ERRORS: usize = 50;

struct Parser {
    tokens: Vec<Token>,
    pos: usize,
    diags: Vec<Diagnostic>,
    /// The type of `self` while parsing an `impl` or `trait` body.
    self_ty: Option<TypeExpr>,
}

fn not_yet(span: Span, what_en: &str, what_ar: &str, phase: &str) -> Diagnostic {
    Diagnostic::error(
        "E0103",
        span,
        Text::new(
            format!("{what_en} is not supported yet"),
            format!("{what_ar} غير مدعوم بعد"),
        ),
    )
    .with_hint(Text::new(
        format!("planned for {phase}; see the roadmap in the specification"),
        format!("مخطط له في {phase}؛ راجع خطة الإصدارات في المواصفة"),
    ))
}

impl Parser {
    // ----- token helpers -------------------------------------------------

    fn peek(&self) -> &TokenKind {
        &self.tokens[self.pos].kind
    }

    fn peek_n(&self, n: usize) -> &TokenKind {
        let i = (self.pos + n).min(self.tokens.len() - 1);
        &self.tokens[i].kind
    }

    fn span(&self) -> Span {
        self.tokens[self.pos].span
    }

    fn prev_span(&self) -> Span {
        self.tokens[self.pos.saturating_sub(1)].span
    }

    fn bump(&mut self) -> Token {
        let tok = self.tokens[self.pos].clone();
        if self.pos < self.tokens.len() - 1 {
            self.pos += 1;
        }
        tok
    }

    fn at(&self, kind: &TokenKind) -> bool {
        self.peek() == kind
    }

    fn eat(&mut self, kind: &TokenKind) -> bool {
        if self.at(kind) {
            self.bump();
            true
        } else {
            false
        }
    }

    fn expect(&mut self, kind: TokenKind) -> PResult<Span> {
        if self.at(&kind) {
            Ok(self.bump().span)
        } else {
            Err(self.unexpected(&kind.describe(), &kind.describe_ar()))
        }
    }

    fn unexpected(&self, expected_en: &str, expected_ar: &str) -> Diagnostic {
        let found = self.peek();
        Diagnostic::error(
            "E0101",
            self.span(),
            Text::new(
                format!("expected {expected_en}, found {}", found.describe()),
                format!("المتوقع {expected_ar}، لكن وُجد {}", found.describe_ar()),
            ),
        )
    }

    fn ident(&mut self) -> PResult<Ident> {
        if let TokenKind::Ident(name) = self.peek() {
            let name = name.clone();
            let span = self.bump().span;
            Ok(Ident { name, span })
        } else {
            Err(self.unexpected("a name", "اسم"))
        }
    }

    /// Skips statement separators (line breaks and `;`).
    fn skip_seps(&mut self) {
        while matches!(self.peek(), TokenKind::Newline | TokenKind::Semi) {
            self.bump();
        }
    }

    fn skip_newlines(&mut self) {
        while self.at(&TokenKind::Newline) {
            self.bump();
        }
    }

    fn report(&mut self, diag: Diagnostic) {
        if self.diags.len() < MAX_ERRORS {
            self.diags.push(diag);
        }
    }

    /// A statement or item must be followed by a separator, `}` or EOF.
    /// Constructs that end in `}` need no separator.
    fn stmt_end(&mut self) -> PResult<()> {
        match self.peek() {
            TokenKind::Newline | TokenKind::Semi => {
                self.bump();
                Ok(())
            }
            TokenKind::RBrace | TokenKind::Eof => Ok(()),
            _ if self.pos > 0 && self.tokens[self.pos - 1].kind == TokenKind::RBrace => Ok(()),
            _ => Err(self.unexpected("end of line or `;`", "نهاية السطر أو `;`")),
        }
    }

    // ----- items ---------------------------------------------------------

    fn program(&mut self) -> Program {
        let mut items = Vec::new();
        loop {
            self.skip_seps();
            if self.at(&TokenKind::Eof) {
                break;
            }
            match self.item().and_then(|item| self.stmt_end().map(|_| item)) {
                Ok(item) => items.push(item),
                Err(diag) => {
                    self.report(diag);
                    self.recover_item();
                }
            }
        }
        Program {
            packages: Vec::new(),
            items,
            modules: Vec::new(),
        }
    }

    /// Skips to the next token that can start an item at the start of a line.
    fn recover_item(&mut self) {
        let mut depth = 0i32;
        let start = self.pos;
        loop {
            let at_line_start = self.pos > start
                && matches!(
                    self.tokens[self.pos - 1].kind,
                    TokenKind::Newline | TokenKind::Semi | TokenKind::RBrace
                );
            match self.peek() {
                TokenKind::Eof => return,
                TokenKind::Fn
                | TokenKind::Async
                | TokenKind::Type
                | TokenKind::Use
                | TokenKind::Pub
                | TokenKind::Let
                | TokenKind::Var
                | TokenKind::Enum
                | TokenKind::Trait
                | TokenKind::Impl
                    if depth <= 0 && at_line_start =>
                {
                    return;
                }
                TokenKind::LBrace => depth += 1,
                TokenKind::RBrace => depth -= 1,
                _ => {}
            }
            self.bump();
        }
    }

    fn item(&mut self) -> PResult<Item> {
        match self.peek() {
            TokenKind::Pub => {
                let start = self.bump().span;
                match self.peek() {
                    TokenKind::Type => self.type_decl(true).map(Item::Type),
                    TokenKind::Enum => self.enum_decl(true).map(Item::Enum),
                    TokenKind::Let => self.const_decl(true).map(Item::Const),
                    TokenKind::Trait => self.trait_decl(true, start).map(Item::Trait),
                    _ => self.fn_decl(true, start).map(Item::Fn),
                }
            }
            TokenKind::Fn | TokenKind::Async => {
                let start = self.span();
                self.fn_decl(false, start).map(Item::Fn)
            }
            TokenKind::Type => self.type_decl(false).map(Item::Type),
            TokenKind::Let => self.const_decl(false).map(Item::Const),
            TokenKind::Use => self.use_decl().map(Item::Use),
            TokenKind::Var => Err(Diagnostic::error(
                "E0105",
                self.span(),
                Text::new(
                    "`var` is not allowed at file level",
                    "لا يُسمح بـ `var` على مستوى الملف",
                ),
            )
            .with_hint(Text::new(
                "use `let` for a file-level constant, or move the variable into a function",
                "استخدم `let` لثابت على مستوى الملف، أو انقل المتغير إلى داخل دالة",
            ))),
            TokenKind::Enum => self.enum_decl(false).map(Item::Enum),
            TokenKind::Trait => {
                let start = self.span();
                self.trait_decl(false, start).map(Item::Trait)
            }
            TokenKind::Impl => self.impl_decl().map(Item::Impl),
            _ => Err(self.unexpected(
                "`fn`, `type`, `let` or `use`",
                "`fn` أو `type` أو `let` أو `use`",
            )),
        }
    }

    /// `<T, U>` after a type or enum name, or nothing.
    fn type_params(&mut self) -> PResult<Vec<Ident>> {
        let (params, bounds) = self.type_params_with_bounds()?;
        if let Some((p, _)) = bounds.first() {
            return Err(Diagnostic::error(
                "E0101",
                p.span,
                Text::new(
                    "trait bounds are allowed only on functions: `fn f<T: Describe>(...)`",
                    "قيود الواجهات مسموحة على الدوال فقط: `fn f<T: Describe>(...)`",
                ),
            ));
        }
        Ok(params)
    }

    /// `<T, U: Describe + Compare>`: type parameters and their trait bounds.
    #[allow(clippy::type_complexity)]
    fn type_params_with_bounds(&mut self) -> PResult<(Vec<Ident>, Vec<(Ident, Ident)>)> {
        let mut params = Vec::new();
        let mut bounds = Vec::new();
        if self.eat(&TokenKind::Lt) {
            while !self.at(&TokenKind::Gt) {
                let p = self.ident()?;
                if self.eat(&TokenKind::Colon) {
                    loop {
                        bounds.push((p.clone(), self.dotted_name()?));
                        if !self.eat(&TokenKind::Plus) {
                            break;
                        }
                    }
                }
                params.push(p);
                if !self.eat(&TokenKind::Comma) {
                    break;
                }
            }
            self.expect(TokenKind::Gt)?;
        }
        Ok((params, bounds))
    }

    /// `trait Name { fn m(self, ...) -> T ... }`: signatures only.
    fn trait_decl(&mut self, is_pub: bool, start: Span) -> PResult<TraitDecl> {
        self.expect(TokenKind::Trait)?;
        let name = self.ident()?;
        let self_ty = TypeExpr {
            kind: TypeExprKind::Named {
                name: Ident {
                    name: "Self".into(),
                    span: name.span,
                },
                args: Vec::new(),
            },
            span: name.span,
        };
        self.expect(TokenKind::LBrace)?;
        let saved = self.self_ty.replace(self_ty);
        let mut methods = Vec::new();
        let result = (|| {
            self.skip_seps();
            while !self.at(&TokenKind::RBrace) {
                let mstart = self.span();
                let (is_async, mname, type_params, _, params, ret, effects) = self.fn_header()?;
                if let Some(tp) = type_params.first() {
                    return Err(not_yet(
                        tp.span,
                        "generic trait methods",
                        "دوال الواجهة العامة",
                        "a later version",
                    ));
                }
                self.require_self(&params, &mname)?;
                let body = if self.at(&TokenKind::LBrace) {
                    Some(self.block()?)
                } else {
                    None
                };
                methods.push(TraitMethod {
                    is_async,
                    name: mname,
                    params,
                    ret,
                    effects,
                    body,
                    span: mstart.to(self.prev_span()),
                });
                self.skip_seps();
            }
            Ok(())
        })();
        self.self_ty = saved;
        result?;
        let end = self.expect(TokenKind::RBrace)?;
        Ok(TraitDecl {
            is_pub,
            name,
            methods,
            span: start.to(end),
        })
    }

    /// `impl Type { ... }` or `impl Trait for Type { ... }`.
    fn impl_decl(&mut self) -> PResult<ImplDecl> {
        let start = self.expect(TokenKind::Impl)?;
        let first = self.type_expr()?;
        let (trait_name, target) = if self.eat(&TokenKind::For) {
            let trait_name = match first.kind {
                TypeExprKind::Named { name, args } if args.is_empty() => name,
                _ => {
                    return Err(Diagnostic::error(
                        "E0101",
                        first.span,
                        Text::new(
                            "expected a trait name before `for`",
                            "يُتوقع اسم واجهة قبل `for`",
                        ),
                    ));
                }
            };
            (Some(trait_name), self.type_expr()?)
        } else {
            (None, first)
        };
        self.expect(TokenKind::LBrace)?;
        let saved = self.self_ty.replace(target.clone());
        let mut methods = Vec::new();
        let result = (|| {
            self.skip_seps();
            while !self.at(&TokenKind::RBrace) {
                let mstart = self.span();
                let is_pub = self.eat(&TokenKind::Pub);
                let f = self.fn_decl(is_pub, mstart)?;
                self.require_self(&f.params, &f.name)?;
                methods.push(f);
                self.skip_seps();
            }
            Ok(())
        })();
        self.self_ty = saved;
        result?;
        let end = self.expect(TokenKind::RBrace)?;
        Ok(ImplDecl {
            trait_name,
            target,
            methods,
            module: String::new(),
            span: start.to(end),
        })
    }

    fn require_self(&self, params: &[Param], name: &Ident) -> PResult<()> {
        if params.first().is_some_and(|p| p.name.name == "self") {
            return Ok(());
        }
        Err(Diagnostic::error(
            "E0101",
            name.span,
            Text::new(
                format!(
                    "method `{}` must take `self` as its first parameter",
                    name.name
                ),
                format!("يجب أن يكون `self` أول معاملات الدالة `{}`", name.name),
            ),
        )
        .with_hint(Text::new(
            format!("write `fn {}(self, ...)`", name.name),
            format!("اكتب `fn {}(self, ...)`", name.name),
        )))
    }

    fn enum_decl(&mut self, is_pub: bool) -> PResult<EnumDecl> {
        let start = self.expect(TokenKind::Enum)?;
        let name = self.ident()?;
        let type_params = self.type_params()?;
        self.expect(TokenKind::LBrace)?;
        let mut variants = Vec::new();
        self.skip_newlines();
        while !self.at(&TokenKind::RBrace) {
            let vname = self.ident()?;
            let mut fields = Vec::new();
            if self.eat(&TokenKind::LParen) {
                while !self.at(&TokenKind::RParen) {
                    fields.push(self.type_expr()?);
                    if !self.eat(&TokenKind::Comma) {
                        break;
                    }
                }
                self.expect(TokenKind::RParen)?;
            }
            variants.push(VariantDecl {
                name: vname,
                fields,
            });
            let sep = self.eat(&TokenKind::Comma) | self.at(&TokenKind::Newline);
            self.skip_newlines();
            if !sep {
                break;
            }
        }
        let end = self.expect(TokenKind::RBrace)?;
        Ok(EnumDecl {
            is_pub,
            name,
            type_params,
            variants,
            span: start.to(end),
        })
    }

    fn use_decl(&mut self) -> PResult<UseDecl> {
        let start = self.expect(TokenKind::Use)?;
        let mut path = vec![self.ident()?];
        let mut names = Vec::new();
        while self.eat(&TokenKind::Dot) {
            if self.eat(&TokenKind::LBrace) {
                self.skip_newlines();
                while !self.at(&TokenKind::RBrace) {
                    names.push(self.ident()?);
                    self.skip_newlines();
                    if !self.eat(&TokenKind::Comma) {
                        break;
                    }
                    self.skip_newlines();
                }
                self.expect(TokenKind::RBrace)?;
                break;
            }
            path.push(self.ident()?);
        }
        Ok(UseDecl {
            path,
            names,
            module: String::new(),
            span: start.to(self.prev_span()),
        })
    }

    fn type_decl(&mut self, is_pub: bool) -> PResult<TypeDecl> {
        let start = self.expect(TokenKind::Type)?;
        let name = self.ident()?;
        let type_params = self.type_params()?;
        self.expect(TokenKind::Eq)?;
        let ty = self.type_expr()?;
        Ok(TypeDecl {
            is_pub,
            name,
            type_params,
            ty,
            span: start.to(self.prev_span()),
        })
    }

    fn const_decl(&mut self, is_pub: bool) -> PResult<ConstDecl> {
        let start = self.expect(TokenKind::Let)?;
        let name = self.ident()?;
        let ty = if self.eat(&TokenKind::Colon) {
            Some(self.type_expr()?)
        } else {
            None
        };
        self.expect(TokenKind::Eq)?;
        let value = self.expr()?;
        Ok(ConstDecl {
            is_pub,
            name,
            ty,
            value,
            span: start.to(self.prev_span()),
        })
    }

    /// Everything of a function before its body.
    #[allow(clippy::type_complexity)]
    fn fn_header(
        &mut self,
    ) -> PResult<(
        bool,
        Ident,
        Vec<Ident>,
        Vec<(Ident, Ident)>,
        Vec<Param>,
        Option<TypeExpr>,
        Option<EffectSet>,
    )> {
        let is_async = self.eat(&TokenKind::Async);
        self.expect(TokenKind::Fn)?;
        let name = self.ident()?;
        let (type_params, bounds) = self.type_params_with_bounds()?;
        self.expect(TokenKind::LParen)?;
        let mut params = Vec::new();
        while !self.at(&TokenKind::RParen) {
            if self.at(&TokenKind::SelfKw) {
                let span = self.bump().span;
                let Some(ty) = self.self_ty.clone().filter(|_| params.is_empty()) else {
                    return Err(Diagnostic::error(
                        "E0101",
                        span,
                        Text::new(
                            "`self` can only be the first parameter of a method in an `impl` or `trait`",
                            "`self` يكون فقط أول معامل لدالة داخل `impl` أو `trait`",
                        ),
                    ));
                };
                params.push(Param {
                    name: Ident {
                        name: "self".into(),
                        span,
                    },
                    ty,
                });
            } else {
                let pname = self.ident()?;
                self.expect(TokenKind::Colon)?;
                let ty = self.type_expr()?;
                params.push(Param { name: pname, ty });
            }
            if !self.eat(&TokenKind::Comma) {
                break;
            }
        }
        self.expect(TokenKind::RParen)?;
        let ret = if self.eat(&TokenKind::Arrow) {
            Some(self.type_expr()?)
        } else {
            None
        };
        let effects = if self.at(&TokenKind::Bang) {
            Some(self.effect_set()?)
        } else {
            None
        };
        Ok((is_async, name, type_params, bounds, params, ret, effects))
    }

    fn fn_decl(&mut self, is_pub: bool, start: Span) -> PResult<FnDecl> {
        let (is_async, name, type_params, bounds, params, ret, effects) = self.fn_header()?;
        let body = self.block()?;
        Ok(FnDecl {
            is_pub,
            is_async,
            name,
            type_params,
            bounds,
            params,
            ret,
            effects,
            span: start.to(body.span),
            body,
        })
    }

    fn effect_set(&mut self) -> PResult<EffectSet> {
        let start = self.expect(TokenKind::Bang)?;
        self.expect(TokenKind::LBrace)?;
        let mut effects = Vec::new();
        self.skip_newlines();
        while !self.at(&TokenKind::RBrace) {
            effects.push(self.ident()?);
            self.skip_newlines();
            if !self.eat(&TokenKind::Comma) {
                break;
            }
            self.skip_newlines();
        }
        self.skip_newlines();
        let end = self.expect(TokenKind::RBrace)?;
        Ok(EffectSet {
            effects,
            span: start.to(end),
        })
    }

    // ----- types -----------------------------------------------------------

    /// Parses a type. Never reports diagnostics itself, so it can be used
    /// speculatively (see [`Parser::try_type_args`]).
    fn type_expr(&mut self) -> PResult<TypeExpr> {
        match self.peek() {
            TokenKind::Ident(_) => {
                let name = self.dotted_name()?;
                let mut args = Vec::new();
                if self.eat(&TokenKind::Lt) {
                    loop {
                        args.push(self.type_expr()?);
                        if !self.eat(&TokenKind::Comma) {
                            break;
                        }
                    }
                    self.expect(TokenKind::Gt)?;
                }
                let span = name.span.to(self.prev_span());
                Ok(TypeExpr {
                    kind: TypeExprKind::Named { name, args },
                    span,
                })
            }
            TokenKind::LParen => {
                let start = self.bump().span;
                let mut items = Vec::new();
                while !self.at(&TokenKind::RParen) {
                    items.push(self.type_expr()?);
                    if !self.eat(&TokenKind::Comma) {
                        break;
                    }
                }
                let close = self.expect(TokenKind::RParen)?;
                if items.len() == 1 {
                    // `(T)` is just `T`.
                    return Ok(items.pop().expect("one item"));
                }
                Ok(TypeExpr {
                    kind: TypeExprKind::Tuple(items),
                    span: start.to(close),
                })
            }
            TokenKind::Fn => {
                let start = self.bump().span;
                self.expect(TokenKind::LParen)?;
                let mut params = Vec::new();
                while !self.at(&TokenKind::RParen) {
                    params.push(self.type_expr()?);
                    if !self.eat(&TokenKind::Comma) {
                        break;
                    }
                }
                let close = self.expect(TokenKind::RParen)?;
                let ret = if self.eat(&TokenKind::Arrow) {
                    self.type_expr()?
                } else {
                    TypeExpr {
                        kind: TypeExprKind::Named {
                            name: Ident {
                                name: "Unit".into(),
                                span: close,
                            },
                            args: Vec::new(),
                        },
                        span: close,
                    }
                };
                Ok(TypeExpr {
                    span: start.to(self.prev_span()),
                    kind: TypeExprKind::Fn {
                        params,
                        ret: Box::new(ret),
                    },
                })
            }
            TokenKind::LBrace => {
                let start = self.bump().span;
                let mut fields = Vec::new();
                self.skip_newlines();
                while !self.at(&TokenKind::RBrace) {
                    let fname = self.ident()?;
                    self.expect(TokenKind::Colon)?;
                    let fty = self.type_expr()?;
                    fields.push((fname, fty));
                    let sep = self.eat(&TokenKind::Comma) | self.at(&TokenKind::Newline);
                    self.skip_newlines();
                    if !sep {
                        break;
                    }
                }
                let end = self.expect(TokenKind::RBrace)?;
                Ok(TypeExpr {
                    kind: TypeExprKind::Record(fields),
                    span: start.to(end),
                })
            }
            _ => Err(Diagnostic::error(
                "E0108",
                self.span(),
                Text::new(
                    format!("expected a type, found {}", self.peek().describe()),
                    format!("المتوقع نوع، لكن وُجد {}", self.peek().describe_ar()),
                ),
            )),
        }
    }

    /// `Student` or `students.Student`, as one name joined with dots.
    fn dotted_name(&mut self) -> PResult<Ident> {
        let mut name = self.ident()?;
        while self.at(&TokenKind::Dot) && matches!(self.peek_n(1), TokenKind::Ident(_)) {
            self.bump();
            let part = self.ident()?;
            name.name.push('.');
            name.name.push_str(&part.name);
            name.span = name.span.to(part.span);
        }
        Ok(name)
    }

    // ----- statements ------------------------------------------------------

    fn block(&mut self) -> PResult<Block> {
        let start = self.expect(TokenKind::LBrace)?;
        let mut stmts = Vec::new();
        loop {
            self.skip_seps();
            match self.peek() {
                TokenKind::RBrace => break,
                TokenKind::Eof => return Err(self.unexpected("`}`", "`}`")),
                _ => {}
            }
            match self.stmt().and_then(|s| self.stmt_end().map(|_| s)) {
                Ok(stmt) => stmts.push(stmt),
                Err(diag) => {
                    self.report(diag);
                    self.recover_stmt();
                }
            }
        }
        let end = self.bump().span;
        Ok(Block {
            stmts,
            span: start.to(end),
        })
    }

    /// Skips to the end of the current statement, leaving a closing `}`.
    fn recover_stmt(&mut self) {
        let mut depth = 0i32;
        loop {
            match self.peek() {
                TokenKind::Eof => return,
                TokenKind::Newline | TokenKind::Semi if depth == 0 => {
                    self.bump();
                    return;
                }
                TokenKind::RBrace if depth == 0 => return,
                TokenKind::LBrace => depth += 1,
                TokenKind::RBrace => depth -= 1,
                _ => {}
            }
            self.bump();
        }
    }

    /// `(a, b, _)` after `let` or `for`: two or more names.
    fn name_tuple(&mut self) -> PResult<Vec<Ident>> {
        let open = self.expect(TokenKind::LParen)?;
        let mut names = Vec::new();
        while !self.at(&TokenKind::RParen) {
            names.push(self.ident()?);
            if !self.eat(&TokenKind::Comma) {
                break;
            }
        }
        let close = self.expect(TokenKind::RParen)?;
        if names.len() < 2 {
            return Err(Diagnostic::error(
                "E0101",
                open.to(close),
                Text::new(
                    "write two or more names between the parentheses: `(a, b)`",
                    "اكتب اسمين أو أكثر بين القوسين: `(a, b)`",
                ),
            ));
        }
        Ok(names)
    }

    fn stmt(&mut self) -> PResult<Stmt> {
        let start = self.span();
        let kind = match self.peek() {
            TokenKind::Let | TokenKind::Var => {
                let mutable = self.bump().kind == TokenKind::Var;
                if self.at(&TokenKind::LParen) {
                    let names = self.name_tuple()?;
                    self.expect(TokenKind::Eq)?;
                    let value = self.expr()?;
                    return Ok(Stmt {
                        span: start.to(self.prev_span()),
                        kind: StmtKind::LetTuple {
                            mutable,
                            names,
                            value,
                        },
                    });
                }
                let name = self.ident()?;
                let ty = if self.eat(&TokenKind::Colon) {
                    Some(self.type_expr()?)
                } else {
                    None
                };
                self.expect(TokenKind::Eq)?;
                let value = self.expr()?;
                StmtKind::Let {
                    mutable,
                    name,
                    ty,
                    value,
                }
            }
            TokenKind::If => return self.if_stmt(),
            TokenKind::While => {
                self.bump();
                let cond = self.expr()?;
                let body = self.block()?;
                StmtKind::While { cond, body }
            }
            TokenKind::For => {
                self.bump();
                let vars = if self.at(&TokenKind::LParen) {
                    self.name_tuple()?
                } else {
                    vec![self.ident()?]
                };
                self.expect(TokenKind::In)?;
                let iter = self.expr()?;
                let body = self.block()?;
                StmtKind::For { vars, iter, body }
            }
            TokenKind::Return => {
                self.bump();
                let value = if matches!(
                    self.peek(),
                    TokenKind::Newline | TokenKind::Semi | TokenKind::RBrace | TokenKind::Eof
                ) {
                    None
                } else {
                    Some(self.expr()?)
                };
                StmtKind::Return(value)
            }
            TokenKind::Break => {
                self.bump();
                StmtKind::Break
            }
            TokenKind::Continue => {
                self.bump();
                StmtKind::Continue
            }
            TokenKind::LBrace => StmtKind::Block(self.block()?),
            _ => {
                let expr = self.expr()?;
                let op = match self.peek() {
                    TokenKind::Eq => Some(AssignOp::Set),
                    TokenKind::PlusEq => Some(AssignOp::Add),
                    TokenKind::MinusEq => Some(AssignOp::Sub),
                    TokenKind::StarEq => Some(AssignOp::Mul),
                    TokenKind::SlashEq => Some(AssignOp::Div),
                    _ => None,
                };
                match op {
                    None => StmtKind::Expr(expr),
                    Some(op) => {
                        if !is_place(&expr) {
                            return Err(Diagnostic::error(
                                "E0104",
                                expr.span,
                                Text::new(
                                    "invalid assignment target",
                                    "هدف إسناد غير صالح",
                                ),
                            )
                            .with_hint(Text::new(
                                "only variables, fields (`a.b`) and list elements (`a[i]`) can be assigned",
                                "يمكن الإسناد فقط إلى متغير أو حقل (`a.b`) أو عنصر قائمة (`a[i]`)",
                            )));
                        }
                        self.bump();
                        let value = self.expr()?;
                        StmtKind::Assign {
                            target: expr,
                            op,
                            value,
                        }
                    }
                }
            }
        };
        Ok(Stmt {
            kind,
            span: start.to(self.prev_span()),
        })
    }

    fn if_stmt(&mut self) -> PResult<Stmt> {
        let start = self.expect(TokenKind::If)?;
        let cond = self.expr()?;
        let then = self.block()?;
        let otherwise = if self.eat(&TokenKind::Else) {
            if self.at(&TokenKind::If) {
                Some(Else::If(Box::new(self.if_stmt()?)))
            } else {
                Some(Else::Block(self.block()?))
            }
        } else {
            None
        };
        Ok(Stmt {
            kind: StmtKind::If {
                cond,
                then,
                otherwise,
            },
            span: start.to(self.prev_span()),
        })
    }

    // ----- expressions -----------------------------------------------------

    /// Parses the `{expr}` parts of an interpolated string token.
    fn interpolation(&mut self, span: Span, parts: Vec<StrPart>) -> Vec<InterpPart> {
        let mut out = Vec::new();
        for part in parts {
            match part {
                StrPart::Lit(text) => out.push(InterpPart::Lit(text)),
                StrPart::Code { src, offset } => {
                    let (tokens, lex_diags) =
                        elomar_lexer::lex_at(&src, span.start as usize + offset);
                    self.diags.extend(lex_diags);
                    let mut sub = Parser {
                        tokens,
                        pos: 0,
                        diags: Vec::new(),
                        self_ty: self.self_ty.clone(),
                    };
                    sub.skip_newlines();
                    match sub.expr() {
                        Ok(e) => {
                            sub.skip_newlines();
                            if *sub.peek() != TokenKind::Eof {
                                let found = sub.span();
                                sub.diags.push(Diagnostic::error(
                                    "E0101",
                                    found,
                                    Text::new(
                                        format!(
                                            "expected `}}` after the expression in the string, found {}",
                                            sub.peek().describe()
                                        ),
                                        format!(
                                            "المتوقع `}}` بعد التعبير داخل النص، ووُجد {}",
                                            sub.peek().describe_ar()
                                        ),
                                    ),
                                ));
                            }
                            out.push(InterpPart::Expr(e));
                        }
                        Err(d) => sub.diags.push(d),
                    }
                    self.diags.extend(sub.diags);
                }
            }
        }
        out
    }

    fn expr(&mut self) -> PResult<Expr> {
        self.expr_bp(0)
    }

    fn expr_bp(&mut self, min_bp: u8) -> PResult<Expr> {
        let mut lhs = self.unary()?;
        while let Some((infix, l_bp, r_bp)) = infix_op(self.peek()) {
            if l_bp < min_bp {
                break;
            }
            self.bump();
            let rhs = self.expr_bp(r_bp)?;
            let span = lhs.span.to(rhs.span);
            let kind = match infix {
                Infix::Bin(op) => ExprKind::Binary {
                    op,
                    lhs: Box::new(lhs),
                    rhs: Box::new(rhs),
                },
                Infix::Range { inclusive } => ExprKind::Range {
                    start: Box::new(lhs),
                    end: Box::new(rhs),
                    inclusive,
                },
            };
            lhs = Expr { kind, span };
        }
        Ok(lhs)
    }

    fn unary(&mut self) -> PResult<Expr> {
        if self.at(&TokenKind::Await) {
            let start = self.bump().span;
            let expr = self.unary()?;
            return Ok(Expr {
                span: start.to(expr.span),
                kind: ExprKind::Await(Box::new(expr)),
            });
        }
        let op = match self.peek() {
            TokenKind::Minus => UnaryOp::Neg,
            TokenKind::Bang => UnaryOp::Not,
            _ => return self.postfix(),
        };
        let start = self.bump().span;
        let expr = self.unary()?;
        Ok(Expr {
            span: start.to(expr.span),
            kind: ExprKind::Unary {
                op,
                expr: Box::new(expr),
            },
        })
    }

    fn postfix(&mut self) -> PResult<Expr> {
        let mut expr = self.primary()?;
        loop {
            match self.peek() {
                TokenKind::Dot => {
                    self.bump();
                    // `t.0`, and `t.0.1`, which the lexer reads as a float.
                    let names: Vec<Ident> = match self.peek().clone() {
                        TokenKind::Int(n) => {
                            let span = self.bump().span;
                            vec![Ident {
                                name: n.to_string(),
                                span,
                            }]
                        }
                        TokenKind::Float(f) => {
                            let span = self.bump().span;
                            f.to_string()
                                .split('.')
                                .map(|part| Ident {
                                    name: part.to_string(),
                                    span,
                                })
                                .collect()
                        }
                        _ => vec![self.ident()?],
                    };
                    for name in names {
                        expr = Expr {
                            span: expr.span.to(name.span),
                            kind: ExprKind::Field {
                                target: Box::new(expr),
                                name,
                            },
                        };
                    }
                }
                TokenKind::LParen => {
                    expr = self.call(expr, Vec::new())?;
                }
                TokenKind::Question => {
                    let end = self.bump().span;
                    expr = Expr {
                        span: expr.span.to(end),
                        kind: ExprKind::Try(Box::new(expr)),
                    };
                }
                TokenKind::LBracket => {
                    self.bump();
                    let index = self.expr()?;
                    let end = self.expect(TokenKind::RBracket)?;
                    expr = Expr {
                        span: expr.span.to(end),
                        kind: ExprKind::Index {
                            target: Box::new(expr),
                            index: Box::new(index),
                        },
                    };
                }
                TokenKind::Lt
                    if matches!(expr.kind, ExprKind::Name(_) | ExprKind::Field { .. }) =>
                {
                    match self.try_type_args() {
                        Some(type_args) => expr = self.call(expr, type_args)?,
                        None => break,
                    }
                }
                _ => break,
            }
        }
        Ok(expr)
    }

    /// Spec §10.5: after a name, `<` starts type arguments only if a type
    /// list closed by `>` parses and is immediately followed by `(`.
    fn try_type_args(&mut self) -> Option<Vec<TypeExpr>> {
        let save = self.pos;
        self.bump();
        let mut args = Vec::new();
        loop {
            match self.type_expr() {
                Ok(t) => args.push(t),
                Err(_) => {
                    self.pos = save;
                    return None;
                }
            }
            if self.eat(&TokenKind::Comma) {
                continue;
            }
            if self.eat(&TokenKind::Gt) && self.at(&TokenKind::LParen) {
                return Some(args);
            }
            self.pos = save;
            return None;
        }
    }

    fn call(&mut self, callee: Expr, type_args: Vec<TypeExpr>) -> PResult<Expr> {
        self.expect(TokenKind::LParen)?;
        let mut args = Vec::new();
        while !self.at(&TokenKind::RParen) {
            let name = if matches!(self.peek(), TokenKind::Ident(_))
                && self.peek_n(1) == &TokenKind::Colon
            {
                let name = self.ident()?;
                self.bump();
                Some(name)
            } else {
                None
            };
            let value = self.expr()?;
            args.push(Arg { name, value });
            if !self.eat(&TokenKind::Comma) {
                break;
            }
        }
        let end = self.expect(TokenKind::RParen)?;
        Ok(Expr {
            span: callee.span.to(end),
            kind: ExprKind::Call {
                callee: Box::new(callee),
                type_args,
                args,
            },
        })
    }

    fn primary(&mut self) -> PResult<Expr> {
        let span = self.span();
        let kind = match self.peek().clone() {
            TokenKind::Int(n) => {
                self.bump();
                ExprKind::Int(n)
            }
            TokenKind::Float(f) => {
                self.bump();
                ExprKind::Float(f)
            }
            TokenKind::Duration(ms) => {
                self.bump();
                ExprKind::Duration(ms)
            }
            TokenKind::Str(s) => {
                self.bump();
                ExprKind::Str(s)
            }
            TokenKind::Template(parts) => {
                self.bump();
                ExprKind::Interp(self.interpolation(span, parts))
            }
            TokenKind::True => {
                self.bump();
                ExprKind::Bool(true)
            }
            TokenKind::False => {
                self.bump();
                ExprKind::Bool(false)
            }
            TokenKind::Ident(name) => {
                self.bump();
                ExprKind::Name(name)
            }
            TokenKind::LParen => {
                self.bump();
                let inner = self.expr()?;
                if self.eat(&TokenKind::Comma) {
                    let mut items = vec![inner];
                    while !self.at(&TokenKind::RParen) {
                        items.push(self.expr()?);
                        if !self.eat(&TokenKind::Comma) {
                            break;
                        }
                    }
                    let end = self.expect(TokenKind::RParen)?;
                    return Ok(Expr {
                        kind: ExprKind::Tuple(items),
                        span: span.to(end),
                    });
                }
                let end = self.expect(TokenKind::RParen)?;
                return Ok(Expr {
                    kind: inner.kind,
                    span: span.to(end),
                });
            }
            TokenKind::LBracket => return self.list_or_map(),
            TokenKind::LBrace => return self.record_literal(),
            TokenKind::Pipe | TokenKind::OrOr => return self.lambda(),
            TokenKind::Match => return self.match_expr(),
            TokenKind::SelfKw => {
                self.bump();
                return Ok(Expr {
                    kind: ExprKind::Name("self".into()),
                    span,
                });
            }
            TokenKind::If => {
                return Err(Diagnostic::error(
                    "E0102",
                    span,
                    Text::new(
                        "`if` is a statement and cannot be used as a value",
                        "`if` جملة ولا يمكن استخدامها كقيمة",
                    ),
                )
                .with_hint(Text::new(
                    "use `match` on a `Bool` to choose between two values",
                    "استخدم `match` على قيمة `Bool` للاختيار بين قيمتين",
                )));
            }
            _ => {
                return Err(Diagnostic::error(
                    "E0102",
                    span,
                    Text::new(
                        format!("expected an expression, found {}", self.peek().describe()),
                        format!("المتوقع تعبير، لكن وُجد {}", self.peek().describe_ar()),
                    ),
                ));
            }
        };
        Ok(Expr { kind, span })
    }

    /// `[1, 2]`, `["a": 1, "b": 2]` or the empty map `[:]`. A `:` after the
    /// first element makes it a map.
    fn list_or_map(&mut self) -> PResult<Expr> {
        let start = self.expect(TokenKind::LBracket)?;
        if self.at(&TokenKind::Colon) && self.peek_n(1) == &TokenKind::RBracket {
            self.bump();
            let end = self.bump().span;
            return Ok(Expr {
                kind: ExprKind::Map(Vec::new()),
                span: start.to(end),
            });
        }
        let mut items = Vec::new();
        let mut entries = Vec::new();
        let mut is_map = false;
        while !self.at(&TokenKind::RBracket) {
            let first = self.expr()?;
            if items.is_empty() && entries.is_empty() {
                is_map = self.at(&TokenKind::Colon);
            }
            if is_map {
                self.expect(TokenKind::Colon)?;
                let value = self.expr()?;
                entries.push((first, value));
            } else {
                items.push(first);
            }
            if !self.eat(&TokenKind::Comma) {
                break;
            }
        }
        let end = self.expect(TokenKind::RBracket)?;
        let kind = if is_map {
            ExprKind::Map(entries)
        } else {
            ExprKind::List(items)
        };
        Ok(Expr {
            kind,
            span: start.to(end),
        })
    }

    /// `|x, y: Int| body` or `|| body`; the body is an expression or a block.
    fn lambda(&mut self) -> PResult<Expr> {
        let start = self.span();
        let mut params = Vec::new();
        if !self.eat(&TokenKind::OrOr) {
            self.expect(TokenKind::Pipe)?;
            while !self.at(&TokenKind::Pipe) {
                let name = self.ident()?;
                let ty = if self.eat(&TokenKind::Colon) {
                    Some(self.type_expr()?)
                } else {
                    None
                };
                params.push(LambdaParam { name, ty });
                if !self.eat(&TokenKind::Comma) {
                    break;
                }
            }
            self.expect(TokenKind::Pipe)?;
        }
        let body = if self.at(&TokenKind::LBrace) {
            ArmBody::Block(self.block()?)
        } else {
            ArmBody::Expr(self.expr()?)
        };
        Ok(Expr {
            span: start.to(self.prev_span()),
            kind: ExprKind::Lambda {
                params,
                body: Box::new(body),
            },
        })
    }

    /// `{ name: "Lina", age }` — fields separated by `,` or line breaks;
    /// a bare name is shorthand for `name: name`.
    fn record_literal(&mut self) -> PResult<Expr> {
        let start = self.expect(TokenKind::LBrace)?;
        let mut fields = Vec::new();
        self.skip_newlines();
        while !self.at(&TokenKind::RBrace) {
            let name = self.ident()?;
            let value = if self.eat(&TokenKind::Colon) {
                self.expr()?
            } else {
                Expr {
                    kind: ExprKind::Name(name.name.clone()),
                    span: name.span,
                }
            };
            fields.push(FieldInit { name, value });
            let sep = self.eat(&TokenKind::Comma) | self.at(&TokenKind::Newline);
            self.skip_newlines();
            if !sep {
                break;
            }
        }
        let end = self.expect(TokenKind::RBrace)?;
        Ok(Expr {
            kind: ExprKind::Record(fields),
            span: start.to(end),
        })
    }

    fn match_expr(&mut self) -> PResult<Expr> {
        let start = self.expect(TokenKind::Match)?;
        let scrutinee = self.expr()?;
        self.expect(TokenKind::LBrace)?;
        let mut arms = Vec::new();
        self.skip_seps();
        while !self.at(&TokenKind::RBrace) {
            let pattern = self.pattern()?;
            self.expect(TokenKind::FatArrow)?;
            let body = if self.at(&TokenKind::LBrace) {
                ArmBody::Block(self.block()?)
            } else if matches!(
                self.peek(),
                TokenKind::Return | TokenKind::Break | TokenKind::Continue
            ) {
                // `Err(e) => return x` is short for `Err(e) => { return x }`.
                let stmt = self.stmt()?;
                ArmBody::Block(Block {
                    span: stmt.span,
                    stmts: vec![stmt],
                })
            } else {
                ArmBody::Expr(self.expr()?)
            };
            let span = pattern.span.to(self.prev_span());
            arms.push(MatchArm {
                pattern,
                body,
                span,
            });
            let sep = self.eat(&TokenKind::Comma)
                | matches!(self.peek(), TokenKind::Newline | TokenKind::Semi);
            self.skip_seps();
            if !sep && !self.at(&TokenKind::RBrace) {
                return Err(self.unexpected("`,`, a new line or `}`", "`,` أو سطر جديد أو `}`"));
            }
        }
        let end = self.expect(TokenKind::RBrace)?;
        Ok(Expr {
            kind: ExprKind::Match {
                scrutinee: Box::new(scrutinee),
                arms,
            },
            span: start.to(end),
        })
    }

    /// A pattern, with `|` between alternatives.
    fn pattern(&mut self) -> PResult<Pattern> {
        let first = self.pattern_one()?;
        if !self.at(&TokenKind::Pipe) {
            return Ok(first);
        }
        let start = first.span;
        let mut alts = vec![first];
        while self.eat(&TokenKind::Pipe) {
            alts.push(self.pattern_one()?);
        }
        Ok(Pattern {
            span: start.to(self.prev_span()),
            kind: PatternKind::Or(alts),
        })
    }

    /// The end of an `Int` range pattern after `start..` or `start..=`.
    fn range_pattern(&mut self, start: i64, span: Span) -> PResult<Pattern> {
        let inclusive = self.at(&TokenKind::DotDotEq);
        self.bump();
        let negative = self.eat(&TokenKind::Minus);
        let TokenKind::Int(n) = *self.peek() else {
            return Err(self.unexpected("a whole number", "عدد صحيح"));
        };
        self.bump();
        Ok(Pattern {
            span: span.to(self.prev_span()),
            kind: PatternKind::Range {
                start,
                end: if negative { -n } else { n },
                inclusive,
            },
        })
    }

    fn pattern_one(&mut self) -> PResult<Pattern> {
        let span = self.span();
        let kind = match self.peek().clone() {
            TokenKind::LParen => {
                self.bump();
                let mut items = Vec::new();
                while !self.at(&TokenKind::RParen) {
                    items.push(self.pattern()?);
                    if !self.eat(&TokenKind::Comma) {
                        break;
                    }
                }
                let end = self.expect(TokenKind::RParen)?;
                if items.len() == 1 {
                    return Ok(items.pop().expect("one item"));
                }
                return Ok(Pattern {
                    span: span.to(end),
                    kind: PatternKind::Tuple(items),
                });
            }
            TokenKind::Ident(name) if name == "_" => {
                self.bump();
                PatternKind::Wildcard
            }
            TokenKind::Ident(_) => {
                let name = self.ident()?;
                if self.eat(&TokenKind::Dot) {
                    // `Enum.Variant` or `module.Enum.Variant`.
                    let mut path = vec![name, self.ident()?];
                    while self.eat(&TokenKind::Dot) {
                        path.push(self.ident()?);
                    }
                    let variant = path.pop().expect("at least two segments");
                    let name = Ident {
                        name: path
                            .iter()
                            .map(|i| i.name.as_str())
                            .collect::<Vec<_>>()
                            .join("."),
                        span: path[0].span.to(path[path.len() - 1].span),
                    };
                    let mut args = Vec::new();
                    if self.eat(&TokenKind::LParen) {
                        while !self.at(&TokenKind::RParen) {
                            args.push(self.pattern()?);
                            if !self.eat(&TokenKind::Comma) {
                                break;
                            }
                        }
                        self.expect(TokenKind::RParen)?;
                    }
                    return Ok(Pattern {
                        span: span.to(self.prev_span()),
                        kind: PatternKind::Variant {
                            enum_name: name,
                            variant,
                            args,
                        },
                    });
                }
                if self.eat(&TokenKind::LParen) {
                    let mut args = Vec::new();
                    while !self.at(&TokenKind::RParen) {
                        args.push(self.pattern()?);
                        if !self.eat(&TokenKind::Comma) {
                            break;
                        }
                    }
                    let end = self.expect(TokenKind::RParen)?;
                    return Ok(Pattern {
                        span: span.to(end),
                        kind: PatternKind::Ctor { name, args },
                    });
                }
                PatternKind::Name(name.name)
            }
            TokenKind::Int(n) => {
                self.bump();
                if matches!(self.peek(), TokenKind::DotDot | TokenKind::DotDotEq) {
                    return self.range_pattern(n, span);
                }
                PatternKind::Int(n)
            }
            TokenKind::Float(f) => {
                self.bump();
                PatternKind::Float(f)
            }
            TokenKind::Minus => {
                self.bump();
                let kind = match self.peek().clone() {
                    TokenKind::Int(n) => PatternKind::Int(-n),
                    TokenKind::Float(f) => PatternKind::Float(-f),
                    _ => return Err(self.unexpected("a number", "عدد")),
                };
                self.bump();
                if let PatternKind::Int(n) = kind
                    && matches!(self.peek(), TokenKind::DotDot | TokenKind::DotDotEq)
                {
                    return self.range_pattern(n, span);
                }
                return Ok(Pattern {
                    kind,
                    span: span.to(self.prev_span()),
                });
            }
            TokenKind::Str(s) => {
                self.bump();
                PatternKind::Str(s)
            }
            TokenKind::True => {
                self.bump();
                PatternKind::Bool(true)
            }
            TokenKind::False => {
                self.bump();
                PatternKind::Bool(false)
            }
            _ => {
                return Err(Diagnostic::error(
                    "E0107",
                    span,
                    Text::new(
                        format!("expected a pattern, found {}", self.peek().describe()),
                        format!("المتوقع نمط، لكن وُجد {}", self.peek().describe_ar()),
                    ),
                ));
            }
        };
        Ok(Pattern { kind, span })
    }
}

enum Infix {
    Bin(BinOp),
    Range { inclusive: bool },
}

/// Binding powers (left, right); higher binds tighter. See spec §8.4.
fn infix_op(kind: &TokenKind) -> Option<(Infix, u8, u8)> {
    use TokenKind as T;
    let (infix, bp) = match kind {
        T::OrOr => (Infix::Bin(BinOp::Or), 1),
        T::AndAnd => (Infix::Bin(BinOp::And), 3),
        T::EqEq => (Infix::Bin(BinOp::Eq), 5),
        T::BangEq => (Infix::Bin(BinOp::Ne), 5),
        T::Lt => (Infix::Bin(BinOp::Lt), 7),
        T::Le => (Infix::Bin(BinOp::Le), 7),
        T::Gt => (Infix::Bin(BinOp::Gt), 7),
        T::Ge => (Infix::Bin(BinOp::Ge), 7),
        T::DotDot => (Infix::Range { inclusive: false }, 9),
        T::DotDotEq => (Infix::Range { inclusive: true }, 9),
        T::Plus => (Infix::Bin(BinOp::Add), 11),
        T::Minus => (Infix::Bin(BinOp::Sub), 11),
        T::Star => (Infix::Bin(BinOp::Mul), 13),
        T::Slash => (Infix::Bin(BinOp::Div), 13),
        T::Div => (Infix::Bin(BinOp::IntDiv), 13),
        T::Percent => (Infix::Bin(BinOp::Rem), 13),
        _ => return None,
    };
    // Ranges are non-associative: `a..b..c` stops after `a..b`.
    let r_bp = if matches!(infix, Infix::Range { .. }) {
        bp + 2
    } else {
        bp + 1
    };
    Some((infix, bp, r_bp))
}

fn is_place(expr: &Expr) -> bool {
    match &expr.kind {
        ExprKind::Name(_) => true,
        ExprKind::Field { target, .. } | ExprKind::Index { target, .. } => is_place(target),
        _ => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parse_ok(src: &str) -> Program {
        let (program, diags) = parse(src);
        assert!(diags.is_empty(), "unexpected diagnostics: {diags:#?}");
        program
    }

    fn main_stmts(src: &str) -> Vec<Stmt> {
        let program = parse_ok(src);
        match &program.items[0] {
            Item::Fn(f) => f.body.stmts.clone(),
            other => panic!("expected fn, got {other:?}"),
        }
    }

    fn first_expr(src: &str) -> Expr {
        match &main_stmts(&format!("fn main() {{\n{src}\n}}"))[0].kind {
            StmtKind::Expr(e) => e.clone(),
            StmtKind::Let { value, .. } => value.clone(),
            other => panic!("unexpected {other:?}"),
        }
    }

    /// Renders an expression as an S-expression for compact assertions.
    fn sexp(e: &Expr) -> String {
        match &e.kind {
            ExprKind::Int(n) => n.to_string(),
            ExprKind::Float(f) => format!("{f:?}"),
            ExprKind::Str(s) => format!("{s:?}"),
            ExprKind::Bool(b) => b.to_string(),
            ExprKind::Name(n) => n.clone(),
            ExprKind::List(items) => {
                format!("[{}]", items.iter().map(sexp).collect::<Vec<_>>().join(" "))
            }
            ExprKind::Record(fields) => format!(
                "{{{}}}",
                fields
                    .iter()
                    .map(|f| format!("{}:{}", f.name.name, sexp(&f.value)))
                    .collect::<Vec<_>>()
                    .join(" ")
            ),
            ExprKind::Unary { op, expr } => {
                let s = if *op == UnaryOp::Neg { "-" } else { "!" };
                format!("({s} {})", sexp(expr))
            }
            ExprKind::Binary { op, lhs, rhs } => {
                format!("({} {} {})", op.symbol(), sexp(lhs), sexp(rhs))
            }
            ExprKind::Range {
                start,
                end,
                inclusive,
            } => format!(
                "({} {} {})",
                if *inclusive { "..=" } else { ".." },
                sexp(start),
                sexp(end)
            ),
            ExprKind::Call {
                callee,
                type_args,
                args,
            } => {
                let targs = if type_args.is_empty() {
                    String::new()
                } else {
                    format!("<{}>", type_args.len())
                };
                let args: Vec<_> = args
                    .iter()
                    .map(|a| match &a.name {
                        Some(n) => format!("{}:{}", n.name, sexp(&a.value)),
                        None => sexp(&a.value),
                    })
                    .collect();
                format!("(call{targs} {} {})", sexp(callee), args.join(" "))
            }
            ExprKind::Field { target, name } => format!("(. {} {})", sexp(target), name.name),
            ExprKind::Index { target, index } => format!("([] {} {})", sexp(target), sexp(index)),
            ExprKind::Match { scrutinee, arms } => {
                format!("(match {} {})", sexp(scrutinee), arms.len())
            }
            ExprKind::Duration(ms) => format!("{ms}ms"),
            ExprKind::Tuple(items) => {
                let items: Vec<_> = items.iter().map(sexp).collect();
                format!("(tuple {})", items.join(" "))
            }
            ExprKind::Interp(parts) => {
                let parts: Vec<_> = parts
                    .iter()
                    .map(|p| match p {
                        InterpPart::Lit(t) => format!("{t:?}"),
                        InterpPart::Expr(e) => sexp(e),
                    })
                    .collect();
                format!("(str {})", parts.join(" "))
            }
            ExprKind::Await(e) => format!("(await {})", sexp(e)),
            ExprKind::Try(e) => format!("(? {})", sexp(e)),
            ExprKind::Lambda { params, body } => format!(
                "(|{}| {})",
                params
                    .iter()
                    .map(|p| p.name.name.clone())
                    .collect::<Vec<_>>()
                    .join(" "),
                match &**body {
                    ArmBody::Expr(e) => sexp(e),
                    ArmBody::Block(_) => "{...}".into(),
                }
            ),
            ExprKind::Map(entries) => format!(
                "[{}]",
                entries
                    .iter()
                    .map(|(k, v)| format!("{}:{}", sexp(k), sexp(v)))
                    .collect::<Vec<_>>()
                    .join(" ")
            ),
            ExprKind::Variant {
                enum_name,
                variant,
                args,
            } => format!("({enum_name}.{variant} {})", args.len()),
        }
    }

    #[test]
    fn precedence() {
        assert_eq!(sexp(&first_expr("1 + 2 * 3")), "(+ 1 (* 2 3))");
        assert_eq!(sexp(&first_expr("1 - 2 - 3")), "(- (- 1 2) 3)");
        assert_eq!(
            sexp(&first_expr("a || b && c == d")),
            "(|| a (&& b (== c d)))"
        );
        assert_eq!(sexp(&first_expr("-x.length")), "(- (. x length))");
        assert_eq!(sexp(&first_expr("7 div 2 % 3")), "(% (div 7 2) 3)");
        assert_eq!(sexp(&first_expr("0..n + 1")), "(.. 0 (+ n 1))");
        assert_eq!(sexp(&first_expr("a < b == c > d")), "(== (< a b) (> c d))");
    }

    #[test]
    fn calls_methods_and_indexing() {
        assert_eq!(
            sexp(&first_expr("xs.push(f(1, 2)[0])")),
            "(call (. xs push) ([] (call f 1 2) 0))"
        );
        assert_eq!(
            sexp(&first_expr("greet(name: \"Lina\", 3)")),
            "(call greet name:\"Lina\" 3)"
        );
    }

    #[test]
    fn generic_call_versus_comparison() {
        assert_eq!(
            sexp(&first_expr("ai.infer<Ticket>(x)")),
            "(call<1> (. ai infer) x)"
        );
        assert_eq!(sexp(&first_expr("f<List<Int>, String>()")), "(call<2> f )");
        assert_eq!(sexp(&first_expr("a < b")), "(< a b)");
        assert_eq!(sexp(&first_expr("a < b && c > d")), "(&& (< a b) (> c d))");
    }

    #[test]
    fn records_and_lists() {
        assert_eq!(
            sexp(&first_expr("let r = { name: \"Lina\", age }")),
            "{name:\"Lina\" age:age}"
        );
        assert_eq!(
            sexp(&first_expr("let r = {\n  a: 1\n  b: [1,\n 2]\n}")),
            "{a:1 b:[1 2]}"
        );
        assert_eq!(sexp(&first_expr("let e = {}")), "{}");
    }

    #[test]
    fn statements_without_semicolons() {
        let stmts = main_stmts(
            "fn main() {\n  var x = 1; x += 2\n  if x > 1 {\n    print(x)\n  }\n  else {\n  }\n  for i in 0..=3 { x = i }\n  while false { break }\n  return\n}",
        );
        assert_eq!(stmts.len(), 6);
        assert!(matches!(
            stmts[1].kind,
            StmtKind::Assign {
                op: AssignOp::Add,
                ..
            }
        ));
        assert!(matches!(
            stmts[2].kind,
            StmtKind::If {
                otherwise: Some(_),
                ..
            }
        ));
        assert!(matches!(stmts[5].kind, StmtKind::Return(None)));
    }

    #[test]
    fn functions_types_and_effects() {
        let program = parse_ok(
            "use std.io\ntype P = { x: Int, y: Int }\nlet ORIGIN: P = { x: 0, y: 0 }\npub fn show(p: P) -> String !{io} {\n  return \"p\"\n}",
        );
        assert_eq!(program.items.len(), 4);
        let Item::Fn(f) = &program.items[3] else {
            panic!()
        };
        assert!(f.is_pub);
        assert_eq!(f.effects.as_ref().unwrap().effects[0].name, "io");
    }

    #[test]
    fn match_arms() {
        let e = first_expr(
            "match r {\n  Ok(v) => print(v)\n  Err(_) => { print(1) }, None => 0\n  -1 => 2\n}",
        );
        let ExprKind::Match { arms, .. } = e.kind else {
            panic!()
        };
        assert_eq!(arms.len(), 4);
        assert!(matches!(arms[1].body, ArmBody::Block(_)));
        assert!(matches!(arms[3].pattern.kind, PatternKind::Int(-1)));
    }

    #[test]
    fn reports_errors_and_recovers() {
        let (program, diags) =
            parse("fn a() {\n  let = 1\n  print(2)\n}\nfn b() { 1 = 2 }\nfn c() {}");
        let codes: Vec<_> = diags.iter().map(|d| d.code).collect();
        assert_eq!(codes, vec!["E0101", "E0104"]);
        assert_eq!(program.items.len(), 3);
    }

    #[test]
    fn two_statements_on_one_line_need_a_separator() {
        let (_, diags) = parse("fn main() { let a = 1 let b = 2 }");
        assert_eq!(diags[0].code, "E0101");
    }

    #[test]
    fn trait_methods_have_no_bodies() {
        let (_, diags) = parse("trait T {\n  fn f(self) {}\n}");
        assert_eq!(diags[0].code, "E0101");
        let (_, diags) = parse("trait T {\n  fn f<U>(self)\n}");
        assert_eq!(diags[0].code, "E0103");
    }

    #[test]
    fn traits_and_impls() {
        let program = parse_ok(
            "pub trait Describe {\n  fn describe(self) -> String\n  fn rename(self, name: String) -> Self\n}\nimpl Describe for Student {\n  fn describe(self) -> String {\n    return self.name\n  }\n}\nimpl Shape {\n  pub fn area(self) -> Float {\n    return 1.0\n  }\n}\nfn show<T: Describe + Other, U>(x: T) {}",
        );
        let Item::Trait(t) = &program.items[0] else {
            panic!()
        };
        assert!(t.is_pub && t.methods.len() == 2 && t.methods[1].params.len() == 2);
        let Item::Impl(i) = &program.items[1] else {
            panic!()
        };
        assert_eq!(
            i.trait_name.as_ref().map(|n| n.name.as_str()),
            Some("Describe")
        );
        assert_eq!(i.methods[0].params[0].name.name, "self");
        let Item::Impl(i) = &program.items[2] else {
            panic!()
        };
        assert!(i.trait_name.is_none() && i.methods[0].is_pub);
        let Item::Fn(f) = &program.items[3] else {
            panic!()
        };
        assert_eq!(f.bounds.len(), 2);
        assert_eq!(f.type_params.len(), 2);
    }

    #[test]
    fn self_outside_methods_is_an_error() {
        let (_, diags) = parse("fn f(self) {}");
        assert_eq!(diags[0].code, "E0101");
        let (_, diags) = parse("impl X {\n  fn f(x: Int) {}\n}");
        assert_eq!(diags[0].code, "E0101");
    }

    #[test]
    fn closures_and_function_types() {
        assert_eq!(
            sexp(&first_expr("xs.map(|x| x * 2)")),
            "(call (. xs map) (|x| (* x 2)))"
        );
        assert_eq!(sexp(&first_expr("let f = || 1")), "(|| 1)");
        assert_eq!(
            sexp(&first_expr("let f = |a: Int, b| { return a }")),
            "(|a b| {...})"
        );
        let program = parse_ok("fn apply(f: fn(Int) -> Int, g: fn()) {}");
        let Item::Fn(f) = &program.items[0] else {
            panic!()
        };
        assert!(
            matches!(&f.params[0].ty.kind, TypeExprKind::Fn { params, .. } if params.len() == 1)
        );
    }

    #[test]
    fn named_imports() {
        let program = parse_ok("use app.school.students.{Student, average}\nuse app.lib");
        let Item::Use(u) = &program.items[0] else {
            panic!()
        };
        assert_eq!(u.path.len(), 3);
        let names: Vec<&str> = u.names.iter().map(|n| n.name.as_str()).collect();
        assert_eq!(names, ["Student", "average"]);
        let Item::Use(plain) = &program.items[1] else {
            panic!()
        };
        assert!(plain.names.is_empty());
    }

    #[test]
    fn pub_items_and_dotted_paths() {
        let program = parse_ok(
            "pub type T = { a: lib.Thing }
pub enum E { A }
pub let X = 1
fn f(x: lib.Level) {
  match x {
    lib.Level.High(v) => v
    _ => 0
  }
}",
        );
        assert!(matches!(&program.items[0], Item::Type(t) if t.is_pub));
        assert!(matches!(&program.items[1], Item::Enum(e) if e.is_pub));
        assert!(matches!(&program.items[2], Item::Const(c) if c.is_pub));
        let Item::Fn(f) = &program.items[3] else {
            panic!()
        };
        assert!(
            matches!(&f.params[0].ty.kind, TypeExprKind::Named { name, .. } if name.name == "lib.Level")
        );
        let StmtKind::Expr(Expr {
            kind: ExprKind::Match { arms, .. },
            ..
        }) = &f.body.stmts[0].kind
        else {
            panic!()
        };
        assert!(matches!(
            &arms[0].pattern.kind,
            PatternKind::Variant { enum_name, variant, .. } if enum_name.name == "lib.Level" && variant.name == "High"
        ));
    }

    #[test]
    fn enums_maps_and_try() {
        let program = parse_ok("enum Shape {\n  Circle(Float)\n  Rect(Float, Float),\n  Empty\n}");
        let Item::Enum(e) = &program.items[0] else {
            panic!()
        };
        assert_eq!(e.variants.len(), 3);
        assert_eq!(e.variants[1].fields.len(), 2);
        assert!(e.variants[2].fields.is_empty());

        assert_eq!(
            sexp(&first_expr("let m = [\"a\": 1, \"b\": 2]")),
            "[\"a\":1 \"b\":2]"
        );
        assert_eq!(sexp(&first_expr("let m = [:]")), "[]");
        assert_eq!(sexp(&first_expr("let xs = [1, 2]")), "[1 2]");
        assert_eq!(
            sexp(&first_expr("let v = f(x)?.len")),
            "(. (? (call f x)) len)"
        );

        let m =
            first_expr("match s {\n  Shape.Circle(r) => r\n  Shape.Empty => 0.0\n  _ => 1.0\n}");
        let ExprKind::Match { arms, .. } = m.kind else {
            panic!()
        };
        assert!(matches!(
            &arms[0].pattern.kind,
            PatternKind::Variant { variant, args, .. } if variant.name == "Circle" && args.len() == 1
        ));
        assert!(matches!(
            &arms[1].pattern.kind,
            PatternKind::Variant { args, .. } if args.is_empty()
        ));
    }

    #[test]
    fn async_await_and_durations() {
        let program = parse_ok("pub async fn f() -> Int {\n  return 1\n}");
        let Item::Fn(f) = &program.items[0] else {
            panic!()
        };
        assert!(f.is_pub && f.is_async);
        assert_eq!(
            sexp(&first_expr("await ai.infer<T>(input: x, timeout: 10s).len")),
            "(await (. (call<1> (. ai infer) input:x timeout:10000ms) len))"
        );
        assert_eq!(sexp(&first_expr("-await f()")), "(- (await (call f )))");
    }

    #[test]
    fn interpolated_strings() {
        assert_eq!(
            sexp(&first_expr("\"a {x + 1} b {f(y)}\"")),
            "(str \"a \" (+ x 1) \" b \" (call f y))"
        );
        let (_, diags) = parse(
            "fn main() {
  print(\"{x y}\")
}",
        );
        assert_eq!(diags[0].code, "E0101");
    }

    #[test]
    fn return_in_match_arm() {
        let (program, diags) = parse(
            "fn f(x: Option<Int>) -> Int {
  let n = match x {
    Some(n) => n
    None => return 0
  }
  return n
}",
        );
        assert!(diags.is_empty(), "{diags:?}");
        let Item::Fn(f) = &program.items[0] else {
            panic!("expected a function")
        };
        let StmtKind::Let { value, .. } = &f.body.stmts[0].kind else {
            panic!("expected let")
        };
        let ExprKind::Match { arms, .. } = &value.kind else {
            panic!("expected match")
        };
        assert!(matches!(&arms[1].body, ArmBody::Block(b) if b.stmts.len() == 1));
    }
}
