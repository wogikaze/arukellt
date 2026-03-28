use crate::ids::*;
use crate::source_map::SourceMap;

#[derive(Debug, Clone)]
pub struct Program {
    pub id: ProgramId,
    pub modules: Vec<Module>,
    pub bodies: Vec<Body>,
    pub source_map: SourceMap,
}

#[derive(Debug, Clone)]
pub struct Module {
    pub id: ModuleId,
    pub name: String,
    pub imports: Vec<ImportRef>,
    pub items: Vec<Item>,
}

#[derive(Debug, Clone)]
pub struct ImportRef {
    pub module_name: String,
    pub alias: Option<String>,
}

#[derive(Debug, Clone)]
pub struct Item {
    pub id: ItemId,
    pub name: String,
    pub kind: ItemKind,
}

#[derive(Debug, Clone)]
pub enum ItemKind {
    Function(FunctionItem),
    Struct(StructItem),
    Enum(EnumItem),
    Trait(TraitItem),
    Impl(ImplItem),
}

#[derive(Debug, Clone)]
pub struct FunctionItem {
    pub body: BodyId,
    pub params: Vec<Param>,
    pub return_ty: Ty,
    pub type_params: Vec<String>,
    pub type_param_bounds: Vec<(String, Vec<String>)>,
    pub effects: EffectFlags,
}

#[derive(Debug, Clone)]
pub struct StructItem {
    pub type_id: Option<TypeId>,
    pub fields: Vec<FieldDef>,
    pub type_params: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct EnumItem {
    pub type_id: Option<TypeId>,
    pub variants: Vec<VariantDef>,
    pub type_params: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct TraitItem {
    pub trait_id: Option<TraitId>,
    pub methods: Vec<TraitMethodSig>,
    pub type_params: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct ImplItem {
    pub impl_id: Option<ImplId>,
    pub trait_name: Option<String>,
    pub target_type: String,
    pub methods: Vec<ItemId>,
}

#[derive(Debug, Clone)]
pub struct TraitMethodSig {
    pub name: String,
    pub params: Vec<Param>,
    pub return_ty: Ty,
}

#[derive(Debug, Clone)]
pub struct VariantDef {
    pub name: String,
    pub fields: Vec<FieldDef>,
}

#[derive(Debug, Clone)]
pub struct FieldDef {
    pub name: String,
    pub ty: Ty,
}

#[derive(Debug, Clone)]
pub struct Param {
    pub local_id: LocalId,
    pub name: String,
    pub ty: Ty,
    pub value_mode: ValueMode,
}

#[derive(Debug, Clone)]
pub struct Body {
    pub id: BodyId,
    pub params: Vec<Local>,
    pub locals: Vec<Local>,
    pub stmts: Vec<Stmt>,
    pub tail: Option<Expr>,
}

#[derive(Debug, Clone)]
pub struct Local {
    pub id: LocalId,
    pub name: String,
    pub ty: Ty,
    pub value_mode: ValueMode,
}

#[derive(Debug, Clone)]
pub enum Stmt {
    Let(LetStmt),
    Expr(Expr),
    While {
        cond: Expr,
        body: BodyId,
    },
    Loop {
        body: BodyId,
    },
    ForRange {
        local: Local,
        start: Expr,
        end: Expr,
        body: BodyId,
    },
    ForValues {
        local: Local,
        iter: Expr,
        body: BodyId,
    },
}

#[derive(Debug, Clone)]
pub struct LetStmt {
    pub local: Local,
    pub init: Expr,
    pub pattern: Option<Pattern>,
    pub binding_mode: ValueMode,
}

#[derive(Debug, Clone)]
pub struct Expr {
    pub id: ExprId,
    pub kind: ExprKind,
    pub ty: Ty,
    pub selection: Option<Selection>,
}

#[derive(Debug, Clone)]
pub enum ExprKind {
    Const(ConstValue),
    Local(LocalId),
    Global(String),
    QualifiedGlobal {
        module: String,
        name: String,
    },
    Call {
        target: CallTarget,
        args: Vec<CallArg>,
    },
    BuiltinBinary {
        op: BuiltinBinaryOp,
        left: Box<Expr>,
        right: Box<Expr>,
    },
    BuiltinUnary {
        op: BuiltinUnaryOp,
        operand: Box<Expr>,
    },
    If {
        cond: Box<Expr>,
        then_body: BodyId,
        else_body: Option<BodyId>,
    },
    Match {
        scrutinee: Box<Expr>,
        arms: Vec<MatchArm>,
    },
    Block(BodyId),
    Tuple(Vec<Expr>),
    Array(Vec<Expr>),
    ArrayRepeat {
        value: Box<Expr>,
        count: Box<Expr>,
    },
    StructInit {
        name: String,
        fields: Vec<(String, Expr)>,
    },
    FieldAccess {
        object: Box<Expr>,
        field: String,
    },
    Index {
        object: Box<Expr>,
        index: Box<Expr>,
    },
    Return(Option<Box<Expr>>),
    Break(Option<Box<Expr>>),
    Continue,
    Try(Box<Expr>),
    Assign {
        target: Box<Expr>,
        value: Box<Expr>,
        value_mode: ValueMode,
    },
    Loop(BodyId),
    Closure {
        params: Vec<Param>,
        body: BodyId,
        captures: Vec<CaptureInfo>,
    },
    StringConcatMany(Vec<Expr>),
}

#[derive(Debug, Clone)]
pub struct CallArg {
    pub expr: Expr,
    pub value_mode: ValueMode,
}

#[derive(Debug, Clone)]
pub struct MatchArm {
    pub pattern: Pattern,
    pub guard: Option<Expr>,
    pub body: Expr,
}

#[derive(Debug, Clone)]
pub struct Pattern {
    pub id: PatternId,
    pub kind: PatternKind,
    pub ty: Ty,
}

#[derive(Debug, Clone)]
pub enum PatternKind {
    Wildcard,
    Binding {
        local: LocalId,
        name: String,
    },
    Const(ConstValue),
    Tuple(Vec<Pattern>),
    Enum {
        path: String,
        variant: String,
        fields: Vec<Pattern>,
    },
    Or(Vec<Pattern>),
    Struct {
        name: String,
        fields: Vec<(String, Option<Pattern>)>,
    },
}

#[derive(Debug, Clone)]
pub enum ConstValue {
    Int(i64),
    Float(f64),
    String(String),
    Char(char),
    Bool(bool),
    Unit,
}

#[derive(Debug, Clone)]
pub enum CallTarget {
    Direct {
        function: String,
    },
    Selected {
        function: String,
        selection: Selection,
    },
    Indirect(Box<Expr>),
}

#[derive(Debug, Clone)]
pub struct Selection {
    pub kind: SelectionKind,
    pub impl_id: Option<ImplId>,
    pub method_item_id: Option<ItemId>,
    pub generic_substitutions: Vec<Ty>,
    pub self_ty: Option<Ty>,
    pub resolved_function: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SelectionKind {
    Function,
    Method,
    TraitMethod,
    Operator,
    FromConversion,
}

#[derive(Debug, Clone)]
pub struct CaptureInfo {
    pub local: LocalId,
    pub name: String,
    pub ty: Ty,
    pub value_mode: ValueMode,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ValueMode {
    ValueCopy,
    SharedRef,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct EffectFlags {
    pub may_panic: bool,
    pub performs_io: bool,
    pub reads_clock: bool,
    pub reads_random: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BuiltinBinaryOp {
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
pub enum BuiltinUnaryOp {
    Neg,
    Not,
    BitNot,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Ty {
    I32,
    I64,
    F32,
    F64,
    // Extended scalar types (v3 #040)
    U8,
    U16,
    U32,
    U64,
    I8,
    I16,
    Bool,
    Char,
    Unit,
    String,
    Struct {
        type_id: Option<TypeId>,
        name: String,
        args: Vec<Ty>,
    },
    Enum {
        type_id: Option<TypeId>,
        name: String,
        args: Vec<Ty>,
    },
    Tuple(Vec<Ty>),
    Array(Box<Ty>, u64),
    Slice(Box<Ty>),
    Vec(Box<Ty>),
    Option(Box<Ty>),
    Result(Box<Ty>, Box<Ty>),
    Function {
        params: Vec<Ty>,
        ret: Box<Ty>,
    },
    TypeParam(String),
    Never,
    Error,
}

impl Ty {
    pub fn is_reference(&self) -> bool {
        matches!(
            self,
            Ty::String
                | Ty::Struct { .. }
                | Ty::Enum { .. }
                | Ty::Vec(_)
                | Ty::Slice(_)
                | Ty::Option(_)
                | Ty::Result(_, _)
                | Ty::Function { .. }
        )
    }
}
