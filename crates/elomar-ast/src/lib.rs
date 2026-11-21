//! Abstract syntax tree for Elomar-Lang. Every node carries a [`Span`].

pub use elomar_diagnostics::Span;

#[derive(Debug, Clone, PartialEq)]
pub struct Ident {
    pub name: String,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct Program {
    /// Items of every loaded file. Items of an imported module have names
    /// prefixed with the module name (`students.count`); see the loader.
    pub items: Vec<Item>,
    /// Imported modules that were found, e.g. `students` for `use app.students`.
    pub modules: Vec<String>,
    /// Names of the packages (dependencies) the program can use.
    pub packages: Vec<String>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Item {
    Use(UseDecl),
    Type(TypeDecl),
    Enum(EnumDecl),
    Const(ConstDecl),
    Fn(FnDecl),
    Trait(TraitDecl),
    Impl(ImplDecl),
}

/// `trait Describe { fn describe(self) -> String }`
#[derive(Debug, Clone, PartialEq)]
pub struct TraitDecl {
    pub is_pub: bool,
    pub name: Ident,
    pub methods: Vec<TraitMethod>,
    pub span: Span,
}

/// A method signature in a trait. `params[0]` is `self`, of type `Self`.
#[derive(Debug, Clone, PartialEq)]
pub struct TraitMethod {
    pub is_async: bool,
    pub name: Ident,
    pub params: Vec<Param>,
    pub ret: Option<TypeExpr>,
    pub effects: Option<EffectSet>,
    pub span: Span,
}

/// `impl Describe for Student { ... }` or `impl Shape { ... }`. Each
/// method's first parameter is `self`, typed as the target.
#[derive(Debug, Clone, PartialEq)]
pub struct ImplDecl {
    pub trait_name: Option<Ident>,
    pub target: TypeExpr,
    pub methods: Vec<FnDecl>,
    /// The module the impl is written in (set by the loader); empty for
    /// the main file.
    pub module: String,
    pub span: Span,
}

/// `use std.io`
#[derive(Debug, Clone, PartialEq)]
pub struct UseDecl {
    pub path: Vec<Ident>,
    /// `use app.students.{Student, average}`: the names imported directly.
    /// Empty for a plain `use`, which binds the module name instead.
    pub names: Vec<Ident>,
    /// The module this `use` appears in; empty for the main file.
    pub module: String,
    pub span: Span,
}

/// `type Student = { name: String }`
#[derive(Debug, Clone, PartialEq)]
pub struct TypeDecl {
    pub is_pub: bool,
    pub name: Ident,
    /// `type Pair<A, B> = ...`: type parameters, usually empty.
    pub type_params: Vec<Ident>,
    pub ty: TypeExpr,
    pub span: Span,
}

/// `enum Shape { Circle(Float), Square(Float), Empty }`
#[derive(Debug, Clone, PartialEq)]
pub struct EnumDecl {
    pub is_pub: bool,
    pub name: Ident,
    /// `enum Tree<T> { ... }`: type parameters, usually empty.
    pub type_params: Vec<Ident>,
    pub variants: Vec<VariantDecl>,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq)]
pub struct VariantDecl {
    pub name: Ident,
    /// Payload types; empty for a unit variant such as `Empty`.
    pub fields: Vec<TypeExpr>,
}

/// A file-level `let`.
#[derive(Debug, Clone, PartialEq)]
pub struct ConstDecl {
    pub is_pub: bool,
    pub name: Ident,
    pub ty: Option<TypeExpr>,
    pub value: Expr,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq)]
pub struct FnDecl {
    pub is_pub: bool,
    /// `async fn`: calling it returns a `Future`; `await` runs it.
    pub is_async: bool,
    pub name: Ident,
    /// `fn first<T>(...)`: type parameters, empty for a non-generic function.
    pub type_params: Vec<Ident>,
    /// `fn show<T: Describe>`: (type parameter, trait) pairs.
    pub bounds: Vec<(Ident, Ident)>,
    pub params: Vec<Param>,
    pub ret: Option<TypeExpr>,
    pub effects: Option<EffectSet>,
    pub body: Block,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Param {
    pub name: Ident,
    pub ty: TypeExpr,
}

/// `!{io, net}`
#[derive(Debug, Clone, PartialEq)]
pub struct EffectSet {
    pub effects: Vec<Ident>,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq)]
pub struct TypeExpr {
    pub kind: TypeExprKind,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq)]
pub enum TypeExprKind {
    /// `Int`, `List<Int>`, `Result<Float, String>`, `Student`
    Named { name: Ident, args: Vec<TypeExpr> },
    /// `{ name: String, age: Int }`
    Record(Vec<(Ident, TypeExpr)>),
    /// `fn(Int, String) -> Bool`
    Fn {
        params: Vec<TypeExpr>,
        ret: Box<TypeExpr>,
    },
}

#[derive(Debug, Clone, PartialEq)]
pub struct Block {
    pub stmts: Vec<Stmt>,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Stmt {
    pub kind: StmtKind,
    pub span: Span,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AssignOp {
    Set,
    Add,
    Sub,
    Mul,
    Div,
}

#[derive(Debug, Clone, PartialEq)]
pub enum StmtKind {
    Let {
        mutable: bool,
        name: Ident,
        ty: Option<TypeExpr>,
        value: Expr,
    },
    Assign {
        target: Expr,
        op: AssignOp,
        value: Expr,
    },
    If {
        cond: Expr,
        then: Block,
        otherwise: Option<Else>,
    },
    While {
        cond: Expr,
        body: Block,
    },
    For {
        var: Ident,
        iter: Expr,
        body: Block,
    },
    Return(Option<Expr>),
    Break,
    Continue,
    Block(Block),
    Expr(Expr),
}

#[derive(Debug, Clone, PartialEq)]
pub enum Else {
    Block(Block),
    /// `else if ...`: always a `StmtKind::If`.
    If(Box<Stmt>),
}

#[derive(Debug, Clone, PartialEq)]
pub struct Expr {
    pub kind: ExprKind,
    pub span: Span,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UnaryOp {
    Neg,
    Not,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BinOp {
    Add,
    Sub,
    Mul,
    /// `/`: always produces a `Float`.
    Div,
    /// `div`: integer division, truncating toward zero.
    IntDiv,
    Rem,
    Eq,
    Ne,
    Lt,
    Le,
    Gt,
    Ge,
    And,
    Or,
}

impl BinOp {
    pub fn symbol(self) -> &'static str {
        match self {
            BinOp::Add => "+",
            BinOp::Sub => "-",
            BinOp::Mul => "*",
            BinOp::Div => "/",
            BinOp::IntDiv => "div",
            BinOp::Rem => "%",
            BinOp::Eq => "==",
            BinOp::Ne => "!=",
            BinOp::Lt => "<",
            BinOp::Le => "<=",
            BinOp::Gt => ">",
            BinOp::Ge => ">=",
            BinOp::And => "&&",
            BinOp::Or => "||",
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct Arg {
    pub name: Option<Ident>,
    pub value: Expr,
}

#[derive(Debug, Clone, PartialEq)]
pub struct FieldInit {
    pub name: Ident,
    pub value: Expr,
}

#[derive(Debug, Clone, PartialEq)]
pub enum ExprKind {
    Int(i64),
    Float(f64),
    Str(String),
    /// `"Hello {name}"`: literal text and expressions, joined as text.
    Interp(Vec<InterpPart>),
    Bool(bool),
    /// A duration literal, in milliseconds.
    Duration(i64),
    Name(String),
    List(Vec<Expr>),
    Record(Vec<FieldInit>),
    Unary {
        op: UnaryOp,
        expr: Box<Expr>,
    },
    Binary {
        op: BinOp,
        lhs: Box<Expr>,
        rhs: Box<Expr>,
    },
    Range {
        start: Box<Expr>,
        end: Box<Expr>,
        inclusive: bool,
    },
    /// A call. Method calls have a [`ExprKind::Field`] callee.
    Call {
        callee: Box<Expr>,
        type_args: Vec<TypeExpr>,
        args: Vec<Arg>,
    },
    Field {
        target: Box<Expr>,
        name: Ident,
    },
    Index {
        target: Box<Expr>,
        index: Box<Expr>,
    },
    Match {
        scrutinee: Box<Expr>,
        arms: Vec<MatchArm>,
    },
    /// `await future`
    Await(Box<Expr>),
    /// `["a": 1, "b": 2]`, or `[:]` when empty.
    Map(Vec<(Expr, Expr)>),
    /// `expr?`
    Try(Box<Expr>),
    /// A closure: `|x| x * 2`, `|a: Int, b: Int| { return a + b }`, `|| 1`.
    /// It captures the variables it uses by value when it is created.
    Lambda {
        params: Vec<LambdaParam>,
        body: Box<ArmBody>,
    },
    /// `Shape.Circle(1.0)` or `Priority.High`. The parser reads these as a
    /// field access or method call; the checker rewrites them to this.
    Variant {
        enum_name: String,
        variant: String,
        args: Vec<Expr>,
    },
}

#[derive(Debug, Clone, PartialEq)]
pub struct LambdaParam {
    pub name: Ident,
    pub ty: Option<TypeExpr>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct MatchArm {
    pub pattern: Pattern,
    pub body: ArmBody,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq)]
pub enum ArmBody {
    Expr(Expr),
    Block(Block),
}

#[derive(Debug, Clone, PartialEq)]
pub struct Pattern {
    pub kind: PatternKind,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq)]
pub enum PatternKind {
    Wildcard,
    /// A binding, or the constructor `None` (resolved by name).
    Name(String),
    /// `Some(p)`, `Ok(p)`, `Err(p)`.
    Ctor {
        name: Ident,
        args: Vec<Pattern>,
    },
    /// `Shape.Circle(r)`, `Priority.High`.
    Variant {
        enum_name: Ident,
        variant: Ident,
        args: Vec<Pattern>,
    },
    Int(i64),
    Float(f64),
    Str(String),
    Bool(bool),
    /// `"Fri" | "Sat"`: matches when any alternative matches. The
    /// alternatives bind no names.
    Or(Vec<Pattern>),
    /// `90..=100` or `0..10`: an `Int` in the range.
    Range {
        start: i64,
        end: i64,
        inclusive: bool,
    },
}

/// Names of the prelude's value constructors.
pub const CONSTRUCTORS: [&str; 4] = ["Some", "None", "Ok", "Err"];

/// A piece of an interpolated string.
#[derive(Debug, Clone, PartialEq)]
pub enum InterpPart {
    Lit(String),
    Expr(Expr),
}
