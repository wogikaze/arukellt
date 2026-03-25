//! MIR type definitions.

use ark_typecheck::types::Type;

/// Unique block identifier.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct BlockId(pub u32);

/// Unique local variable identifier.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct LocalId(pub u32);

/// Unique function identifier.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct FnId(pub u32);

/// A MIR module (after monomorphization).
#[derive(Debug)]
pub struct MirModule {
    pub functions: Vec<MirFunction>,
    pub entry_fn: Option<FnId>,
    /// Struct layouts: struct name -> ordered (field name, field type name)
    pub struct_defs: std::collections::HashMap<String, Vec<(String, String)>>,
    /// Enum variant types: enum_name -> vec of (variant_name, vec of payload type names)
    pub enum_defs: std::collections::HashMap<String, Vec<(String, Vec<String>)>>,
}

/// A single function in MIR form.
#[derive(Debug)]
pub struct MirFunction {
    pub id: FnId,
    pub name: String,
    pub params: Vec<MirLocal>,
    pub return_ty: Type,
    pub locals: Vec<MirLocal>,
    pub blocks: Vec<BasicBlock>,
    pub entry: BlockId,
}

/// A local variable (parameter or temporary).
#[derive(Debug, Clone)]
pub struct MirLocal {
    pub id: LocalId,
    pub name: Option<String>,
    pub ty: Type,
}

/// A basic block in the CFG.
#[derive(Debug)]
pub struct BasicBlock {
    pub id: BlockId,
    pub stmts: Vec<MirStmt>,
    pub terminator: Terminator,
}

/// A MIR statement (within a basic block).
#[derive(Debug, Clone)]
pub enum MirStmt {
    Assign(Place, Rvalue),
    Call {
        dest: Option<Place>,
        func: FnId,
        args: Vec<Operand>,
    },
    CallBuiltin {
        dest: Option<Place>,
        name: String,
        args: Vec<Operand>,
    },
    IfStmt {
        cond: Operand,
        then_body: Vec<MirStmt>,
        else_body: Vec<MirStmt>,
    },
    WhileStmt {
        cond: Operand,
        body: Vec<MirStmt>,
    },
    Break,
    Continue,
    Return(Option<Operand>),
}

/// How a basic block ends.
#[derive(Debug)]
pub enum Terminator {
    Goto(BlockId),
    If {
        cond: Operand,
        then_block: BlockId,
        else_block: BlockId,
    },
    Switch {
        scrutinee: Operand,
        arms: Vec<(i64, BlockId)>,
        default: BlockId,
    },
    Return(Option<Operand>),
    Unreachable,
}

/// A value reference (lvalue).
#[derive(Debug, Clone)]
pub enum Place {
    Local(LocalId),
    Field(Box<Place>, String),
    Index(Box<Place>, Box<Operand>),
}

/// An rvalue expression.
#[derive(Debug, Clone)]
pub enum Rvalue {
    Use(Operand),
    BinaryOp(BinOp, Operand, Operand),
    UnaryOp(UnaryOp, Operand),
    Aggregate(AggregateKind, Vec<Operand>),
    Ref(Place),
}

/// A value operand.
#[derive(Debug, Clone)]
pub enum Operand {
    Place(Place),
    ConstI32(i32),
    ConstI64(i64),
    ConstF32(f32),
    ConstF64(f64),
    ConstBool(bool),
    ConstChar(char),
    ConstString(String),
    Unit,
    BinOp(BinOp, Box<Operand>, Box<Operand>),
    UnaryOp(UnaryOp, Box<Operand>),
    Call(String, Vec<Operand>),
    IfExpr {
        cond: Box<Operand>,
        then_body: Vec<MirStmt>,
        then_result: Option<Box<Operand>>,
        else_body: Vec<MirStmt>,
        else_result: Option<Box<Operand>>,
    },
    StructInit {
        name: String,
        fields: Vec<(String, Operand)>,
    },
    FieldAccess {
        object: Box<Operand>,
        struct_name: String,
        field: String,
    },
    /// Construct an enum variant with optional payload values.
    /// Layout in memory: [tag: i32] [payload0: i32] [payload1: i32] ...
    EnumInit {
        enum_name: String,
        variant: String,
        tag: i32,
        payload: Vec<Operand>,
    },
    /// Extract the tag from an enum value (i32.load at offset 0).
    EnumTag(Box<Operand>),
    /// Extract payload field at `index` from an enum value.
    EnumPayload {
        object: Box<Operand>,
        index: u32,
        enum_name: String,
        variant_name: String,
    },
}

/// Binary operations.
#[derive(Debug, Clone, Copy)]
pub enum BinOp {
    Add, Sub, Mul, Div, Mod,
    Eq, Ne, Lt, Le, Gt, Ge,
    And, Or,
    BitAnd, BitOr, BitXor, Shl, Shr,
}

/// Unary operations.
#[derive(Debug, Clone, Copy)]
pub enum UnaryOp {
    Neg, Not, BitNot,
}

/// Kind of aggregate being constructed.
#[derive(Debug, Clone)]
pub enum AggregateKind {
    Tuple,
    Array,
    Struct(String),
    EnumVariant(String, String), // enum name, variant name
}

impl MirModule {
    pub fn new() -> Self {
        Self { functions: Vec::new(), entry_fn: None, struct_defs: std::collections::HashMap::new(), enum_defs: std::collections::HashMap::new() }
    }
}

impl Default for MirModule {
    fn default() -> Self {
        Self::new()
    }
}
