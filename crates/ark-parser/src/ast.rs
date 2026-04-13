//! AST type definitions for Arukellt.

use ark_diagnostics::Span;

/// A parsed module (one source file).
#[derive(Debug, Clone)]
pub struct Module {
    pub docs: Vec<String>,
    pub imports: Vec<Import>,
    pub items: Vec<Item>,
}

/// Distinguishes the syntactic form of a `use` or `import` statement.
#[derive(Debug, Clone, PartialEq)]
pub enum ImportKind {
    /// Legacy flat import: `import name [as alias]`
    Simple,
    /// Module-path import: `use a::b::c [as alias]`
    ModulePath,
    /// Public module-path import: `pub use a::b::c [as alias]`
    PublicModulePath,
    /// Destructuring import: `use a::b::{c, d, e}`
    DestructureImport {
        /// The individual names extracted from the brace list.
        names: Vec<String>,
    },
}

#[derive(Debug, Clone)]
pub struct Import {
    pub module_name: String,
    pub alias: Option<String>,
    /// How this import was written syntactically.
    pub kind: ImportKind,
    pub span: Span,
}

#[derive(Debug, Clone)]
pub enum Item {
    FnDef(FnDef),
    StructDef(StructDef),
    EnumDef(EnumDef),
    TraitDef(TraitDef),
    ImplBlock(ImplBlock),
}

#[derive(Debug, Clone)]
pub struct FnDef {
    pub docs: Vec<String>,
    pub name: String,
    pub type_params: Vec<String>,
    pub type_param_bounds: Vec<(String, Vec<String>)>,
    pub params: Vec<Param>,
    pub return_type: Option<TypeExpr>,
    pub body: Block,
    pub is_pub: bool,
    pub span: Span,
}

#[derive(Debug, Clone)]
pub struct Param {
    pub name: String,
    pub ty: TypeExpr,
    pub span: Span,
}

#[derive(Debug, Clone)]
pub struct StructDef {
    pub docs: Vec<String>,
    pub name: String,
    pub type_params: Vec<String>,
    pub fields: Vec<Field>,
    pub is_pub: bool,
    pub span: Span,
}

#[derive(Debug, Clone)]
pub struct Field {
    pub name: String,
    pub ty: TypeExpr,
    pub span: Span,
}

#[derive(Debug, Clone)]
pub struct EnumDef {
    pub docs: Vec<String>,
    pub name: String,
    pub type_params: Vec<String>,
    pub variants: Vec<Variant>,
    pub is_pub: bool,
    pub span: Span,
}

#[derive(Debug, Clone)]
pub enum Variant {
    Unit {
        name: String,
        span: Span,
    },
    Tuple {
        name: String,
        fields: Vec<TypeExpr>,
        span: Span,
    },
    Struct {
        name: String,
        fields: Vec<Field>,
        span: Span,
    },
}

/// A method signature in a trait definition (no body).
#[derive(Debug, Clone)]
pub struct TraitMethodSig {
    pub docs: Vec<String>,
    pub name: String,
    pub params: Vec<Param>,
    pub return_type: Option<TypeExpr>,
    pub span: Span,
}

#[derive(Debug, Clone)]
pub struct TraitDef {
    pub docs: Vec<String>,
    pub name: String,
    pub type_params: Vec<String>,
    pub methods: Vec<TraitMethodSig>,
    pub is_pub: bool,
    pub span: Span,
}

#[derive(Debug, Clone)]
pub struct ImplBlock {
    pub docs: Vec<String>,
    pub trait_name: Option<String>,
    pub target_type: String,
    pub methods: Vec<FnDef>,
    pub span: Span,
}

/// Type expression in source.
#[derive(Debug, Clone)]
pub enum TypeExpr {
    Named {
        name: String,
        span: Span,
    },
    Generic {
        name: String,
        args: Vec<TypeExpr>,
        span: Span,
    },
    Tuple(Vec<TypeExpr>, Span),
    Array {
        elem: Box<TypeExpr>,
        size: u64,
        span: Span,
    },
    Slice {
        elem: Box<TypeExpr>,
        span: Span,
    },
    Function {
        params: Vec<TypeExpr>,
        ret: Box<TypeExpr>,
        span: Span,
    },
    Unit(Span),
    Qualified {
        module: String,
        name: String,
        span: Span,
    },
}

impl TypeExpr {
    pub fn span(&self) -> Span {
        match self {
            TypeExpr::Named { span, .. }
            | TypeExpr::Generic { span, .. }
            | TypeExpr::Array { span, .. }
            | TypeExpr::Slice { span, .. }
            | TypeExpr::Function { span, .. }
            | TypeExpr::Unit(span)
            | TypeExpr::Qualified { span, .. } => *span,
            TypeExpr::Tuple(_, span) => *span,
        }
    }
}

/// Expression.
#[derive(Debug, Clone)]
pub enum Expr {
    IntLit {
        value: i64,
        suffix: Option<String>,
        span: Span,
    },
    FloatLit {
        value: f64,
        suffix: Option<String>,
        span: Span,
    },
    StringLit {
        value: String,
        span: Span,
    },
    CharLit {
        value: char,
        span: Span,
    },
    BoolLit {
        value: bool,
        span: Span,
    },
    Ident {
        name: String,
        span: Span,
    },
    QualifiedIdent {
        module: String,
        name: String,
        span: Span,
    },
    Binary {
        left: Box<Expr>,
        op: BinOp,
        right: Box<Expr>,
        span: Span,
    },
    Unary {
        op: UnaryOp,
        operand: Box<Expr>,
        span: Span,
    },
    Call {
        callee: Box<Expr>,
        type_args: Vec<TypeExpr>,
        args: Vec<Expr>,
        span: Span,
    },
    FieldAccess {
        object: Box<Expr>,
        field: String,
        span: Span,
    },
    Index {
        object: Box<Expr>,
        index: Box<Expr>,
        span: Span,
    },
    If {
        cond: Box<Expr>,
        then_block: Block,
        else_block: Option<Block>,
        span: Span,
    },
    Match {
        scrutinee: Box<Expr>,
        arms: Vec<MatchArm>,
        span: Span,
    },
    Block(Block),
    Tuple {
        elements: Vec<Expr>,
        span: Span,
    },
    Array {
        elements: Vec<Expr>,
        span: Span,
    },
    ArrayRepeat {
        value: Box<Expr>,
        count: Box<Expr>,
        span: Span,
    },
    StructInit {
        name: String,
        fields: Vec<(String, Expr)>,
        base: Option<Box<Expr>>,
        span: Span,
    },
    Closure {
        params: Vec<ClosureParam>,
        return_type: Option<TypeExpr>,
        body: Box<Expr>,
        span: Span,
    },
    Return {
        value: Option<Box<Expr>>,
        span: Span,
    },
    Break {
        value: Option<Box<Expr>>,
        span: Span,
    },
    Continue {
        span: Span,
    },
    Try {
        expr: Box<Expr>,
        span: Span,
    },
    Assign {
        target: Box<Expr>,
        value: Box<Expr>,
        span: Span,
    },
    Loop {
        body: Block,
        span: Span,
    },
}

impl Expr {
    pub fn span(&self) -> Span {
        match self {
            Expr::IntLit { span, .. }
            | Expr::FloatLit { span, .. }
            | Expr::StringLit { span, .. }
            | Expr::CharLit { span, .. }
            | Expr::BoolLit { span, .. }
            | Expr::Ident { span, .. }
            | Expr::QualifiedIdent { span, .. }
            | Expr::Binary { span, .. }
            | Expr::Unary { span, .. }
            | Expr::Call { span, .. }
            | Expr::FieldAccess { span, .. }
            | Expr::Index { span, .. }
            | Expr::If { span, .. }
            | Expr::Match { span, .. }
            | Expr::Tuple { span, .. }
            | Expr::Array { span, .. }
            | Expr::ArrayRepeat { span, .. }
            | Expr::StructInit { span, .. }
            | Expr::Closure { span, .. }
            | Expr::Return { span, .. }
            | Expr::Break { span, .. }
            | Expr::Continue { span }
            | Expr::Try { span, .. }
            | Expr::Assign { span, .. }
            | Expr::Loop { span, .. } => *span,
            Expr::Block(block) => block.span,
        }
    }
}

#[derive(Debug, Clone)]
pub struct ClosureParam {
    pub name: String,
    pub ty: Option<TypeExpr>,
    pub span: Span,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BinOp {
    Add,
    Sub,
    Mul,
    Div,
    Mod,
    Eq,
    Ne,
    Lt,
    Le,
    Gt,
    Ge,
    And,
    Or,
    BitAnd,
    BitOr,
    BitXor,
    Shl,
    Shr,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UnaryOp {
    Neg,
    Not,
    BitNot,
}

#[derive(Debug, Clone)]
pub struct MatchArm {
    pub pattern: Pattern,
    pub guard: Option<Box<Expr>>,
    pub body: Expr,
    pub span: Span,
}

#[derive(Debug, Clone)]
pub enum Pattern {
    Wildcard(Span),
    Ident {
        name: String,
        span: Span,
    },
    IntLit {
        value: i64,
        suffix: Option<String>,
        span: Span,
    },
    FloatLit {
        value: f64,
        suffix: Option<String>,
        span: Span,
    },
    StringLit {
        value: String,
        span: Span,
    },
    CharLit {
        value: char,
        span: Span,
    },
    BoolLit {
        value: bool,
        span: Span,
    },
    Tuple {
        elements: Vec<Pattern>,
        span: Span,
    },
    Enum {
        path: String,
        variant: String,
        fields: Vec<Pattern>,
        span: Span,
    },
    Or {
        patterns: Vec<Pattern>,
        span: Span,
    },
    Struct {
        name: String,
        fields: Vec<(String, Option<Pattern>)>,
        span: Span,
    },
}

impl Pattern {
    pub fn span(&self) -> Span {
        match self {
            Pattern::Wildcard(span) => *span,
            Pattern::Ident { span, .. }
            | Pattern::IntLit { span, .. }
            | Pattern::FloatLit { span, .. }
            | Pattern::StringLit { span, .. }
            | Pattern::CharLit { span, .. }
            | Pattern::BoolLit { span, .. }
            | Pattern::Tuple { span, .. }
            | Pattern::Enum { span, .. }
            | Pattern::Or { span, .. }
            | Pattern::Struct { span, .. } => *span,
        }
    }
}

#[derive(Debug, Clone)]
pub enum Stmt {
    Let {
        name: String,
        name_span: Span,
        ty: Option<TypeExpr>,
        init: Expr,
        is_mut: bool,
        pattern: Option<Pattern>,
        span: Span,
    },
    Expr(Expr),
    While {
        cond: Expr,
        body: Block,
        span: Span,
    },
    Loop {
        body: Block,
        span: Span,
    },
    For {
        target: String,
        iter: ForIter,
        body: Block,
        span: Span,
    },
}

#[derive(Debug, Clone)]
pub enum ForIter {
    Range {
        start: Expr,
        end: Expr,
    },
    Values(Expr),
    /// Generic iterator: `for x in expr { ... }` where expr implements Iterator<T>
    Iter(Expr),
}

#[derive(Debug, Clone)]
pub struct Block {
    pub stmts: Vec<Stmt>,
    pub tail_expr: Option<Box<Expr>>,
    pub span: Span,
}
