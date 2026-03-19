use lang_core::{BinaryOp, Pattern, Type, TypeDecl};

#[derive(Clone, Debug, PartialEq)]
pub struct HighModule {
    pub imports: Vec<String>,
    pub types: Vec<TypeDecl>,
    pub functions: Vec<HighFunction>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct HighFunction {
    pub public: bool,
    pub name: String,
    pub params: Vec<HighParam>,
    pub return_type: Type,
    pub body: HighExpr,
}

#[derive(Clone, Debug, PartialEq)]
pub struct HighParam {
    pub name: String,
    pub ty: Type,
}

#[derive(Clone, Debug, PartialEq)]
pub struct HighExpr {
    pub kind: HighExprKind,
    pub ty: Type,
}

#[derive(Clone, Debug, PartialEq)]
pub enum HighExprKind {
    Int(i64),
    Bool(bool),
    String(String),
    Ident(String),
    List(Vec<HighExpr>),
    Tuple(Vec<HighExpr>),
    Lambda {
        param: String,
        body: Box<HighExpr>,
    },
    Binary {
        op: BinaryOp,
        left: Box<HighExpr>,
        right: Box<HighExpr>,
    },
    If {
        condition: Box<HighExpr>,
        then_branch: Box<HighExpr>,
        else_branch: Box<HighExpr>,
    },
    Match {
        subject: Box<HighExpr>,
        arms: Vec<HighMatchArm>,
    },
    Construct {
        variant: String,
        args: Vec<HighExpr>,
    },
    Call {
        callee: String,
        args: Vec<HighExpr>,
    },
    Error,
}

#[derive(Clone, Debug, PartialEq)]
pub struct HighMatchArm {
    pub pattern: Pattern,
    pub expr: HighExpr,
}
