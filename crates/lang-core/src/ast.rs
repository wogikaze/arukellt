use crate::Type;

#[derive(Clone, Debug, PartialEq)]
pub struct Module {
    pub imports: Vec<CapabilityImport>,
    pub types: Vec<TypeDecl>,
    pub functions: Vec<Function>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct CapabilityImport {
    pub name: String,
}

#[derive(Clone, Debug, PartialEq)]
pub struct TypeDecl {
    pub name: String,
    pub variants: Vec<VariantDecl>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct VariantDecl {
    pub name: String,
    pub fields: Vec<VariantField>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct VariantField {
    pub name: String,
    pub ty: Type,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Function {
    pub public: bool,
    pub name: String,
    pub params: Vec<Param>,
    pub return_type: Type,
    pub body: Expr,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Param {
    pub name: String,
    pub ty: Type,
}

#[derive(Clone, Debug, PartialEq)]
pub enum Expr {
    Int(i64),
    Bool(bool),
    String(String),
    Ident(String),
    Binary {
        op: BinaryOp,
        left: Box<Expr>,
        right: Box<Expr>,
    },
    If {
        condition: Box<Expr>,
        then_branch: Box<Expr>,
        else_branch: Box<Expr>,
    },
    Match {
        subject: Box<Expr>,
        arms: Vec<MatchArm>,
    },
    Call {
        callee: String,
        args: Vec<Expr>,
    },
    Apply {
        func: Box<Expr>,
        args: Vec<Expr>,
    },
    List(Vec<Expr>),
    Tuple(Vec<Expr>),
    Range {
        start: Box<Expr>,
        end: Box<Expr>,
    },
    Lambda {
        param: String,
        body: Box<Expr>,
    },
    Let {
        name: String,
        value: Box<Expr>,
        body: Box<Expr>,
    },
    MethodCall {
        receiver: Box<Expr>,
        method: String,
        args: Vec<Expr>,
    },
    Index {
        receiver: Box<Expr>,
        index: Box<Expr>,
    },
    Error,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BinaryOp {
    Add,
    Subtract,
    Multiply,
    Divide,
    Modulo,
    Greater,
    Less,
    Equal,
    And,
    Or,
}

#[derive(Clone, Debug, PartialEq)]
pub struct MatchArm {
    pub pattern: Pattern,
    pub expr: Expr,
}

#[derive(Clone, Debug, PartialEq)]
pub enum Pattern {
    Wildcard,
    Variant { name: String, bindings: Vec<String> },
}
