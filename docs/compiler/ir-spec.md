# CoreHIR / MIR Specification

This document describes the intermediate representations (IRs) used by the
Arukellt compiler, from parsed source through to Wasm output. It is the
authoritative reference for re-implementing the IR in another language.

> **Source of truth**: `crates/ark-mir/src/mir.rs`, `crates/ark-mir/src/lower/`,
> `crates/ark-mir/src/validate.rs`, `crates/ark-mir/src/opt/`,
> `crates/ark-parser/src/ast.rs`, `crates/ark-hir/src/hir.rs`,
> `crates/ark-typecheck/src/types.rs`.

---

## Table of Contents

1. [Pipeline Overview](#1-pipeline-overview)
2. [AST Specification](#2-ast-specification)
3. [CoreHIR Specification](#3-corehir-specification)
4. [Type System](#4-type-system)
5. [MIR Data Structures](#5-mir-data-structures)
6. [HIR → MIR Lowering Rules](#6-hir--mir-lowering-rules)
7. [MIR Optimization Passes](#7-mir-optimization-passes)
8. [MIR Validation Rules](#8-mir-validation-rules)
9. [MIR → Wasm Mapping](#9-mir--wasm-mapping)

---

## 1. Pipeline Overview

```
Source text
  │
  ▼
Lexer ──► Tokens
  │
  ▼
Parser ──► AST  (crates/ark-parser — ast.rs)
  │
  ▼
Resolver ──► Resolved AST  (crates/ark-resolve)
  │
  ▼
Type-checker ──► Typed HIR / CoreHIR  (crates/ark-typecheck, crates/ark-hir)
  │
  ▼
MIR Lowering ──► MirModule  (crates/ark-mir — lower/)
  │
  ▼
MIR Optimization ──► Optimized MirModule  (crates/ark-mir — opt/)
  │
  ▼
MIR Validation ──► Validated MirModule  (crates/ark-mir — validate.rs)
  │
  ▼
Wasm Emitter ──► .wasm binary  (crates/ark-wasm — emit/t3/)
```

| Phase | Input | Output | Crate |
|-------|-------|--------|-------|
| Lexer | Source text | Token stream | `ark-lexer` |
| Parser | Tokens | `ast::Module` | `ark-parser` |
| Resolver | `ast::Module` | Resolved AST with name bindings | `ark-resolve` |
| Type-checker | Resolved AST | `hir::Program` with typed nodes | `ark-typecheck` / `ark-hir` |
| MIR lowering | `hir::Program` or `ast::Module` | `MirModule` | `ark-mir` |
| MIR optimization | `MirModule` | Optimized `MirModule` | `ark-mir` |
| MIR validation | `MirModule` | Validated `MirModule` | `ark-mir` |
| Wasm emit | `MirModule` | `.wasm` binary | `ark-wasm` |

Two lowering paths exist:

- **Legacy path** (`MirSelection::Legacy`): lowers directly from AST. Default.
- **CoreHIR path** (`MirSelection::CoreHir`): lowers from typed HIR via
  `lower_check_output_to_mir`. Opt-in with `--mir-select corehir`.

---

## 2. AST Specification

Defined in `crates/ark-parser/src/ast.rs`.

### 2.1 Top-Level

```rust
struct Module {
    docs: Vec<String>,
    imports: Vec<Import>,
    items: Vec<Item>,
}

struct Import {
    module_name: String,
    alias: Option<String>,
    span: Span,
}

enum Item {
    FnDef(FnDef),
    StructDef(StructDef),
    EnumDef(EnumDef),
    TraitDef(TraitDef),
    ImplBlock(ImplBlock),
}
```

### 2.2 Definitions

```rust
struct FnDef {
    docs: Vec<String>,
    name: String,
    type_params: Vec<String>,
    type_param_bounds: Vec<(String, Vec<String>)>,
    params: Vec<Param>,
    return_type: Option<TypeExpr>,
    body: Block,
    is_pub: bool,
    span: Span,
}

struct Param {
    name: String,
    ty: TypeExpr,
    span: Span,
}

struct StructDef {
    docs: Vec<String>,
    name: String,
    type_params: Vec<String>,
    fields: Vec<Field>,
    is_pub: bool,
    span: Span,
}

struct Field { name: String, ty: TypeExpr, span: Span }

struct EnumDef {
    docs: Vec<String>,
    name: String,
    type_params: Vec<String>,
    variants: Vec<Variant>,
    is_pub: bool,
    span: Span,
}

enum Variant {
    Unit   { name: String, span: Span },
    Tuple  { name: String, fields: Vec<TypeExpr>, span: Span },
    Struct { name: String, fields: Vec<Field>, span: Span },
}

struct TraitDef {
    docs: Vec<String>,
    name: String,
    type_params: Vec<String>,
    methods: Vec<TraitMethodSig>,
    is_pub: bool,
    span: Span,
}

struct TraitMethodSig {
    docs: Vec<String>,
    name: String,
    params: Vec<Param>,
    return_type: Option<TypeExpr>,
    span: Span,
}

struct ImplBlock {
    docs: Vec<String>,
    trait_name: Option<String>,
    target_type: String,
    methods: Vec<FnDef>,
    span: Span,
}
```

### 2.3 Type Expressions

```rust
enum TypeExpr {
    Named     { name: String, span: Span },
    Generic   { name: String, args: Vec<TypeExpr>, span: Span },
    Tuple     (Vec<TypeExpr>, Span),
    Array     { elem: Box<TypeExpr>, size: u64, span: Span },
    Slice     { elem: Box<TypeExpr>, span: Span },
    Function  { params: Vec<TypeExpr>, ret: Box<TypeExpr>, span: Span },
    Unit      (Span),
    Qualified { module: String, name: String, span: Span },
}
```

### 2.4 Expressions

```rust
enum Expr {
    IntLit       { value: i64, suffix: Option<String>, span: Span },
    FloatLit     { value: f64, suffix: Option<String>, span: Span },
    StringLit    { value: String, span: Span },
    CharLit      { value: char, span: Span },
    BoolLit      { value: bool, span: Span },
    Ident        { name: String, span: Span },
    QualifiedIdent { module: String, name: String, span: Span },
    Binary       { left: Box<Expr>, op: BinOp, right: Box<Expr>, span: Span },
    Unary        { op: UnaryOp, operand: Box<Expr>, span: Span },
    Call         { callee: Box<Expr>, type_args: Vec<TypeExpr>, args: Vec<Expr>, span: Span },
    FieldAccess  { object: Box<Expr>, field: String, span: Span },
    Index        { object: Box<Expr>, index: Box<Expr>, span: Span },
    If           { cond: Box<Expr>, then_block: Block, else_block: Option<Block>, span: Span },
    Match        { scrutinee: Box<Expr>, arms: Vec<MatchArm>, span: Span },
    Block        (Block),
    Tuple        { elements: Vec<Expr>, span: Span },
    Array        { elements: Vec<Expr>, span: Span },
    ArrayRepeat  { value: Box<Expr>, count: Box<Expr>, span: Span },
    StructInit   { name: String, fields: Vec<(String, Expr)>, base: Option<Box<Expr>>, span: Span },
    Closure      { params: Vec<ClosureParam>, return_type: Option<TypeExpr>, body: Box<Expr>, span: Span },
    Return       { value: Option<Box<Expr>>, span: Span },
    Break        { value: Option<Box<Expr>>, span: Span },
    Continue     { span: Span },
    Try          { expr: Box<Expr>, span: Span },
    Assign       { target: Box<Expr>, value: Box<Expr>, span: Span },
    Loop         { body: Block, span: Span },
}

struct ClosureParam { name: String, ty: Option<TypeExpr>, span: Span }

enum BinOp {
    Add, Sub, Mul, Div, Mod,
    Eq, Ne, Lt, Le, Gt, Ge,
    And, Or,
    BitAnd, BitOr, BitXor, Shl, Shr,
}

enum UnaryOp { Neg, Not, BitNot }
```

### 2.5 Patterns

```rust
enum Pattern {
    Wildcard  (Span),
    Ident     { name: String, span: Span },
    IntLit    { value: i64, suffix: Option<String>, span: Span },
    FloatLit  { value: f64, suffix: Option<String>, span: Span },
    StringLit { value: String, span: Span },
    CharLit   { value: char, span: Span },
    BoolLit   { value: bool, span: Span },
    Tuple     { elements: Vec<Pattern>, span: Span },
    Enum      { path: String, variant: String, fields: Vec<Pattern>, span: Span },
    Or        { patterns: Vec<Pattern>, span: Span },
    Struct    { name: String, fields: Vec<(String, Option<Pattern>)>, span: Span },
}
```

### 2.6 Statements and Blocks

```rust
enum Stmt {
    Let  { name: String, ty: Option<TypeExpr>, init: Expr, is_mut: bool, pattern: Option<Pattern>, span: Span },
    Expr (Expr),
    While { cond: Expr, body: Block, span: Span },
    Loop  { body: Block, span: Span },
    For   { target: String, iter: ForIter, body: Block, span: Span },
}

enum ForIter {
    Range  { start: Expr, end: Expr },
    Values (Expr),
    Iter   (Expr),
}

struct Block {
    stmts: Vec<Stmt>,
    tail_expr: Option<Box<Expr>>,
    span: Span,
}

struct MatchArm {
    pattern: Pattern,
    guard: Option<Box<Expr>>,
    body: Expr,
    span: Span,
}
```

---

## 3. CoreHIR Specification

Defined in `crates/ark-hir/src/hir.rs`. CoreHIR is a typed, resolved IR that
sits between the raw AST and MIR. Method calls, operator syntax, and
qualified-import syntax are already resolved to canonical call targets.

### 3.1 Program Structure

```rust
struct Program {
    id: ProgramId,
    modules: Vec<Module>,
    bodies: Vec<Body>,
    source_map: SourceMap,
}

struct Module {
    id: ModuleId,
    name: String,
    imports: Vec<ImportRef>,
    items: Vec<Item>,
}

struct ImportRef { module_name: String, alias: Option<String> }

struct Item { id: ItemId, name: String, kind: ItemKind }

enum ItemKind {
    Function(FunctionItem),
    Struct(StructItem),
    Enum(EnumItem),
    Trait(TraitItem),
    Impl(ImplItem),
}
```

### 3.2 Item Definitions

```rust
struct FunctionItem {
    body: BodyId,
    params: Vec<Param>,
    return_ty: Ty,
    type_params: Vec<String>,
    type_param_bounds: Vec<(String, Vec<String>)>,
    effects: EffectFlags,
}

struct StructItem  { type_id: Option<TypeId>, fields: Vec<FieldDef>, type_params: Vec<String> }
struct EnumItem    { type_id: Option<TypeId>, variants: Vec<VariantDef>, type_params: Vec<String> }
struct TraitItem   { trait_id: Option<TraitId>, methods: Vec<TraitMethodSig>, type_params: Vec<String> }
struct ImplItem    { impl_id: Option<ImplId>, trait_name: Option<String>, target_type: String, methods: Vec<ItemId> }

struct VariantDef { name: String, fields: Vec<FieldDef> }
struct FieldDef   { name: String, ty: Ty }
struct Param      { local_id: LocalId, name: String, ty: Ty, value_mode: ValueMode }

struct EffectFlags {
    may_panic: bool,
    performs_io: bool,
    reads_clock: bool,
    reads_random: bool,
}
```

### 3.3 Bodies and Locals

```rust
struct Body {
    id: BodyId,
    params: Vec<Local>,
    locals: Vec<Local>,
    stmts: Vec<Stmt>,
    tail: Option<Expr>,
}

struct Local { id: LocalId, name: String, ty: Ty, value_mode: ValueMode }

enum ValueMode {
    ValueCopy,   // value-semantics deep copy
    SharedRef,   // alias-preserving shared reference
}
```

### 3.4 Statements

```rust
enum Stmt {
    Let(LetStmt),
    Expr(Expr),
    While     { cond: Expr, body: BodyId },
    Loop      { body: BodyId },
    ForRange  { local: Local, start: Expr, end: Expr, body: BodyId },
    ForValues { local: Local, iter: Expr, body: BodyId },
}

struct LetStmt {
    local: Local,
    init: Expr,
    pattern: Option<Pattern>,
    binding_mode: ValueMode,
}
```

### 3.5 Expressions

Every `Expr` carries an `ExprId`, a `Ty`, and an optional `Selection`.

```rust
struct Expr {
    id: ExprId,
    kind: ExprKind,
    ty: Ty,
    selection: Option<Selection>,
}

enum ExprKind {
    Const(ConstValue),
    Local(LocalId),
    Global(String),
    QualifiedGlobal { module: String, name: String },

    Call { target: CallTarget, args: Vec<CallArg> },

    BuiltinBinary { op: BuiltinBinaryOp, left: Box<Expr>, right: Box<Expr> },
    BuiltinUnary  { op: BuiltinUnaryOp, operand: Box<Expr> },

    If    { cond: Box<Expr>, then_body: BodyId, else_body: Option<BodyId> },
    Match { scrutinee: Box<Expr>, arms: Vec<MatchArm> },
    Block (BodyId),
    Loop  (BodyId),

    Tuple(Vec<Expr>),
    Array(Vec<Expr>),
    ArrayRepeat { value: Box<Expr>, count: Box<Expr> },

    StructInit  { name: String, fields: Vec<(String, Expr)> },
    FieldAccess { object: Box<Expr>, field: String },
    Index       { object: Box<Expr>, index: Box<Expr> },

    Return   (Option<Box<Expr>>),
    Break    (Option<Box<Expr>>),
    Continue,
    Try      (Box<Expr>),

    Assign { target: Box<Expr>, value: Box<Expr>, value_mode: ValueMode },

    Closure {
        params: Vec<Param>,
        body: BodyId,
        captures: Vec<CaptureInfo>,
    },

    StringConcatMany(Vec<Expr>),
}
```

### 3.6 Call Targets and Selection

```rust
enum CallTarget {
    Direct   { function: String },
    Selected { function: String, selection: Selection },
    Indirect (Box<Expr>),
}

struct Selection {
    kind: SelectionKind,
    impl_id: Option<ImplId>,
    method_item_id: Option<ItemId>,
    generic_substitutions: Vec<Ty>,
    self_ty: Option<Ty>,
    resolved_function: String,       // canonical backend-facing callee name
}

enum SelectionKind { Function, Method, TraitMethod, Operator, FromConversion }
```

If `CallTarget::Selected` is present, the frontend has already resolved the
callee. Downstream passes must not recompute this resolution.

### 3.7 Constants and Operators

```rust
enum ConstValue {
    Int(i64), Float(f64), String(String), Char(char), Bool(bool), Unit,
}

enum BuiltinBinaryOp {
    Add, Sub, Mul, Div, Mod,
    Eq, Ne, Lt, Le, Gt, Ge,
    And, Or, BitAnd, BitOr, BitXor, Shl, Shr,
}

enum BuiltinUnaryOp { Neg, Not, BitNot }
```

### 3.8 Patterns

```rust
struct Pattern { id: PatternId, kind: PatternKind, ty: Ty }

enum PatternKind {
    Wildcard,
    Binding { local: LocalId, name: String },
    Const(ConstValue),
    Tuple(Vec<Pattern>),
    Enum   { path: String, variant: String, fields: Vec<Pattern> },
    Or     (Vec<Pattern>),
    Struct { name: String, fields: Vec<(String, Option<Pattern>)> },
}
```

### 3.9 CoreHIR Types (Ty)

```rust
enum Ty {
    I32, I64, F32, F64,
    U8, U16, U32, U64, I8, I16,
    Bool, Char, Unit, String,
    Struct   { type_id: Option<TypeId>, name: String, args: Vec<Ty> },
    Enum     { type_id: Option<TypeId>, name: String, args: Vec<Ty> },
    Tuple    (Vec<Ty>),
    Array    (Box<Ty>, u64),
    Slice    (Box<Ty>),
    Vec      (Box<Ty>),
    Option   (Box<Ty>),
    Result   (Box<Ty>, Box<Ty>),
    Function { params: Vec<Ty>, ret: Box<Ty> },
    TypeParam(String),
    Never,
    Error,
}
```

### 3.10 Sugars Allowed in CoreHIR

These high-level constructs are permitted in validated CoreHIR and are desugared
during MIR lowering:

- `Match` — desugared to nested if-else chains with `EnumTag` / `EnumPayload`
- `Try` — desugared to match on `Result` with early return
- `ForRange` — desugared to while loop with counter
- `ForValues` — desugared to while loop with index
- `StringConcatMany` — desugared to chained concat calls

Method-call syntax, operator syntax, and qualified-import syntax must **not**
survive into CoreHIR; they appear as resolved `CallTarget::Selected` nodes.

---

## 4. Type System

### 4.1 Type-Checker Type (`Type`)

Defined in `crates/ark-typecheck/src/types.rs`. This is the type used by
`MirLocal.ty` and `MirFunction.return_ty`.

```rust
enum Type {
    // Primitives
    I32, I64, F32, F64, Bool, Char, Unit,

    // Extended scalars (v3 #040)
    U8,   // i32 with 0xFF mask
    U16,  // i32 with 0xFFFF mask
    U32,  // i32 (unsigned div/rem/cmp)
    U64,  // i64 (unsigned div/rem/cmp)
    I8,   // i32 with sign-extend from 8-bit
    I16,  // i32 with sign-extend from 16-bit

    // Reference types
    String,

    // Compound types
    Struct(TypeId),
    Enum(TypeId),
    Tuple(Vec<Type>),
    Array(Box<Type>, u64),          // [T; N]
    Slice(Box<Type>),               // [T]
    Vec(Box<Type>),                 // Vec<T>
    Option(Box<Type>),              // Option<T>
    Result(Box<Type>, Box<Type>),   // Result<T, E>

    // Function type
    Function { params: Vec<Type>, ret: Box<Type> },

    // Inference
    TypeVar(u32),   // unresolved type variable

    // Special
    Never,          // diverging (return, panic, break)
    Error,          // error recovery sentinel

    // Polymorphic
    Any,            // generic type param erased to anyref at Wasm level
}

struct TypeId(pub u32);
```

### 4.2 Wasm-Level Representation

| `Type` | Wasm `ValType` | Notes |
|--------|---------------|-------|
| `I32`, `Bool`, `Char`, `U8`, `U16`, `U32`, `I8`, `I16` | `i32` | Sub-word types use masking/sign-extension |
| `I64`, `U64` | `i64` | |
| `F32` | `f32` | |
| `F64` | `f64` | |
| `String` | `(ref $string)` | GC array of i8 |
| `Struct(_)` | `(ref $StructName)` | GC struct |
| `Enum(_)` | `(ref $EnumBase)` | Subtype hierarchy |
| `Vec(T)` | `(ref $vec_T)` | GC struct { ref array, i32 len } |
| `Option(T)` | `(ref $Option)` | Enum encoding |
| `Result(T, E)` | `(ref $Result)` | Enum encoding |
| `Function { .. }` | `funcref` / table index | Indirect call via `call_indirect` |
| `Any` | `anyref` | GC anyref; i32 boxed via `ref.i31` |
| `Unit` | — | No value; void return |
| `Never` | — | Unreachable |

---

## 5. MIR Data Structures

Defined in `crates/ark-mir/src/mir.rs`.

### 5.1 MirModule

Top-level container for the entire compiled program.

```rust
struct MirModule {
    functions: Vec<MirFunction>,
    entry_fn: Option<FnId>,
    type_table: TypeTable,
    struct_defs: HashMap<String, Vec<(String, String)>>,    // struct name → [(field, type)]
    enum_defs: HashMap<String, Vec<(String, Vec<String>)>>, // enum name → [(variant, [payload types])]
    imports: Vec<MirImport>,
    source_map: MirSourceMap,
    stats: MirStats,
}
```

### 5.2 TypeTable

Nominal type table — all type information the backend needs. The backend reads
types exclusively from this table rather than reaching back into the frontend.

```rust
struct TypeTable {
    struct_defs: HashMap<String, Vec<(String, String)>>,      // struct name → [(field name, field type name)]
    enum_defs: HashMap<String, Vec<(String, Vec<String>)>>,   // enum name → [(variant name, [payload type names])]
    fn_sigs: HashMap<String, MirFnSig>,                       // fn name → signature
}

struct MirFnSig {
    name: String,
    params: Vec<String>,   // parameter type names
    ret: String,           // return type name
}
```

### 5.3 MirImport

WIT-derived import for Component Model support.

```rust
struct MirImport {
    interface: String,          // e.g. "wasi:cli/stdout@0.2.0"
    name: String,               // function name within the interface
    param_types: Vec<String>,   // WIT parameter type strings
    return_type: Option<String>,// WIT return type string (None for void)
}
```

### 5.4 MirFunction

A single function in MIR form, using a control flow graph (CFG) of basic blocks.

```rust
struct MirFunction {
    id: FnId,
    name: String,
    instance: InstanceKey,
    params: Vec<MirLocal>,
    return_ty: Type,
    locals: Vec<MirLocal>,
    blocks: Vec<BasicBlock>,
    entry: BlockId,
    struct_typed_locals: HashMap<u32, String>,  // local id → struct type name
    enum_typed_locals: HashMap<u32, String>,    // local id → enum type name
    type_params: Vec<String>,                   // empty for non-generic functions
    source: SourceInfo,
    is_exported: bool,                          // true for `pub fn` declarations
}

struct FnId(pub u32);
```

### 5.5 InstanceKey

Tracks generic monomorphization instances.

```rust
struct InstanceKey {
    item: String,             // canonical function name
    substitution: Vec<String>,// type argument names (empty for non-generic)
    target_shape: String,     // "default" for most; distinguishes ABI shapes
}
```

### 5.6 MirLocal

A local variable or parameter.

```rust
struct MirLocal {
    id: LocalId,
    name: Option<String>,
    ty: Type,
}

struct LocalId(pub u32);
```

### 5.7 BasicBlock

A node in the control flow graph.

```rust
struct BasicBlock {
    id: BlockId,
    stmts: Vec<MirStmt>,
    terminator: Terminator,
    source: SourceInfo,
}

struct BlockId(pub u32);
```

### 5.8 MirStmt

All statement variants within a basic block.

```rust
enum MirStmt {
    /// Assign an rvalue to a place (local, field, or index).
    Assign(Place, Rvalue),

    /// Direct call to a known function by FnId.
    Call {
        dest: Option<Place>,
        func: FnId,
        args: Vec<Operand>,
    },

    /// Call a built-in function by name (e.g. "println", "concat").
    CallBuiltin {
        dest: Option<Place>,
        name: String,
        args: Vec<Operand>,
    },

    /// Structured if-then-else (preserved from source; not yet lowered to CFG branches).
    IfStmt {
        cond: Operand,
        then_body: Vec<MirStmt>,
        else_body: Vec<MirStmt>,
    },

    /// Structured while loop (preserved from source).
    WhileStmt {
        cond: Operand,
        body: Vec<MirStmt>,
    },

    /// Break out of the innermost loop.
    Break,

    /// Continue to the next iteration of the innermost loop.
    Continue,

    /// Return a value from the function.
    Return(Option<Operand>),

    /// GC allocation hint (pure annotation, no semantic effect).
    GcHint {
        local: LocalId,
        hint: GcHintKind,
    },
}
```

### 5.9 Terminator

How a basic block ends. Used by the CFG-based representation.

```rust
enum Terminator {
    /// Unconditional jump to another block.
    Goto(BlockId),

    /// Conditional branch.
    If {
        cond: Operand,
        then_block: BlockId,
        else_block: BlockId,
        hint: Option<BranchHint>,
    },

    /// Multi-way branch (compiled match).
    Switch {
        scrutinee: Operand,
        arms: Vec<(i64, BlockId)>,  // (value, target block)
        default: BlockId,
    },

    /// Return from the function.
    Return(Option<Operand>),

    /// Marks unreachable code.
    Unreachable,
}

enum BranchHint {
    Likely,     // then-path is expected to be taken
    Unlikely,   // then-path is expected to fall through
}
```

### 5.10 Place

An lvalue reference (assignment target).

```rust
enum Place {
    Local(LocalId),                     // a local variable
    Field(Box<Place>, String),          // struct field access: place.field_name
    Index(Box<Place>, Box<Operand>),    // array/vec index: place[index]
}
```

### 5.11 Rvalue

An rvalue expression assigned to a Place.

```rust
enum Rvalue {
    Use(Operand),                                   // simple value use
    BinaryOp(BinOp, Operand, Operand),              // binary arithmetic/logic
    UnaryOp(UnaryOp, Operand),                      // unary operation
    Aggregate(AggregateKind, Vec<Operand>),          // composite construction
    Ref(Place),                                     // reference to a place
}

enum AggregateKind {
    Tuple,
    Array,
    Struct(String),                      // struct name
    EnumVariant(String, String),         // (enum name, variant name)
}
```

### 5.12 Operand

All value operand variants. This is the richest enum in the MIR.

```rust
enum Operand {
    // ── Place ──
    Place(Place),

    // ── Scalar constants ──
    ConstI32(i32),
    ConstI64(i64),
    ConstF32(f32),
    ConstF64(f64),
    ConstBool(bool),
    ConstChar(char),
    ConstString(String),
    ConstU8(u8),        // i32 at Wasm level, masked to 0xFF
    ConstU16(u16),      // i32 at Wasm level, masked to 0xFFFF
    ConstU32(u32),      // i32 at Wasm level
    ConstU64(u64),      // i64 at Wasm level
    ConstI8(i8),        // i32 at Wasm level, sign-extended
    ConstI16(i16),      // i32 at Wasm level, sign-extended
    Unit,               // void / unit value

    // ── Operations ──
    BinOp(BinOp, Box<Operand>, Box<Operand>),
    UnaryOp(UnaryOp, Box<Operand>),

    // ── Calls ──
    Call(String, Vec<Operand>),                         // direct call by name
    CallIndirect { callee: Box<Operand>, args: Vec<Operand> },  // indirect call via fn pointer
    FnRef(String),                                      // reference to a named function

    // ── Control flow as expressions (high-level, NOT backend-legal) ──
    IfExpr {
        cond: Box<Operand>,
        then_body: Vec<MirStmt>,
        then_result: Option<Box<Operand>>,
        else_body: Vec<MirStmt>,
        else_result: Option<Box<Operand>>,
    },
    LoopExpr {
        init: Box<Operand>,
        body: Vec<MirStmt>,
        result: Box<Operand>,
    },
    TryExpr {
        expr: Box<Operand>,
        from_fn: Option<String>,    // From trait conversion function
    },

    // ── Struct operations ──
    StructInit {
        name: String,
        fields: Vec<(String, Operand)>,
    },
    FieldAccess {
        object: Box<Operand>,
        struct_name: String,
        field: String,
    },

    // ── Enum operations ──
    EnumInit {
        enum_name: String,
        variant: String,
        tag: i32,
        payload: Vec<Operand>,
    },
    EnumTag(Box<Operand>),              // extract tag (i32) from enum value
    EnumPayload {
        object: Box<Operand>,
        index: u32,                     // payload field index
        enum_name: String,
        variant_name: String,
    },

    // ── Array operations ──
    ArrayInit { elements: Vec<Operand> },
    IndexAccess { object: Box<Operand>, index: Box<Operand> },
}
```

### 5.13 Binary and Unary Operations

```rust
enum BinOp {
    Add, Sub, Mul, Div, Mod,            // arithmetic
    Eq, Ne, Lt, Le, Gt, Ge,            // comparison
    And, Or,                            // logical
    BitAnd, BitOr, BitXor, Shl, Shr,   // bitwise
}

enum UnaryOp {
    Neg,            // arithmetic negation
    Not,            // logical NOT
    BitNot,         // bitwise NOT
    SignExtend8,    // sign-extend low 8 bits  (i32.extend8_s / i64.extend8_s)
    SignExtend16,   // sign-extend low 16 bits (i32.extend16_s / i64.extend16_s)
    SignExtend32,   // sign-extend low 32 bits (i64.extend32_s, i64 only)
}
```

### 5.14 GC Hints

```rust
enum GcHintKind {
    ShortLived,     // allocation is expected to die quickly (loop-scoped)
}
```

### 5.15 Effect Analysis

```rust
enum EffectKind {
    Pure,           // no side effects
    ReadsMemory,    // reads from GC heap
    WritesMemory,   // writes to GC heap
    ControlFlow,    // affects control flow
    Call,           // may have arbitrary effects
    Unknown,        // conservative assumption
}
```

Every `MirStmt`, `Terminator`, `Rvalue`, and `Operand` implements
`effect_kind()` returning its effect classification. This is used by
optimization passes (e.g., CSE only eliminates `Pure` expressions).

### 5.16 Backend Legality

Three operands are **not backend-legal** and must be desugared before codegen:

- `Operand::IfExpr { .. }`
- `Operand::LoopExpr { .. }`
- `Operand::TryExpr { .. }`

The function `is_backend_legal_module(module)` returns `true` only if no
statement, operand, or terminator in the module contains these high-level
constructs. The validation function `validate_backend_legal_module()` checks
this and rejects modules that still contain backend-illegal nodes.

### 5.17 Provenance Tracking

```rust
enum MirProvenance {
    LegacyAst,          // lowered from AST (legacy path)
    CoreHir,            // lowered from CoreHIR
    CoreHirFallback,    // CoreHIR attempted, fell back to legacy
    LegacyOptimized,    // legacy path after optimization
    CoreHirOptimized,   // CoreHIR path after optimization
}
```

### 5.18 Source Mapping

```rust
struct SourceInfo { span: Option<Span> }

struct MirSourceMap {
    function_spans: HashMap<FnId, Span>,
    block_spans: HashMap<(FnId, BlockId), Span>,
    stmt_spans: HashMap<(FnId, BlockId, usize), Span>,  // usize = statement index
}

struct MirStats {
    instance_keys: Vec<InstanceKey>,
    direct_call_graph: HashMap<String, HashSet<String>>,
    optimization_rounds: u32,
    validation_runs: u32,
    provenance: Option<String>,
    optimization_trace: Vec<String>,
}
```

---

## 6. HIR → MIR Lowering Rules

Lowering is implemented in `crates/ark-mir/src/lower/`. Two paths exist:
the **legacy** path (AST → MIR) and the **CoreHIR** path (typed HIR → MIR).
Both produce the same `MirModule` structure.

### 6.1 Lowering Context

```rust
struct LowerCtx {
    // Tracks known enums, structs, local types, function return types, etc.
    // Provides fresh LocalId / FnId / BlockId allocation.
}
```

### 6.2 Expression Lowering (expr.rs)

Each source expression lowers to an `Operand`:

| Source Expression | MIR Operand |
|-------------------|-------------|
| Integer literal `42` | `Operand::ConstI32(42)` or `ConstI64(42)` based on suffix |
| Float literal `3.14` | `Operand::ConstF64(3.14)` |
| String literal `"hello"` | `Operand::ConstString("hello")` |
| Bool literal `true` | `Operand::ConstBool(true)` |
| Char literal `'a'` | `Operand::ConstChar('a')` |
| Variable `x` | `Operand::Place(Place::Local(local_id))` |
| Binary `a + b` | `Operand::BinOp(BinOp::Add, ⟨a⟩, ⟨b⟩)` |
| Unary `-x` | `Operand::UnaryOp(UnaryOp::Neg, ⟨x⟩)` |
| Function call `f(a, b)` | `Operand::Call("f", [⟨a⟩, ⟨b⟩])` |
| Method call `x.m(a)` | `Operand::Call("Type::m", [⟨x⟩, ⟨a⟩])` |
| Field access `p.x` | `Operand::FieldAccess { object: ⟨p⟩, struct_name, field: "x" }` |
| Index `a[i]` | `Operand::IndexAccess { object: ⟨a⟩, index: ⟨i⟩ }` |
| Struct init `P { x: 1 }` | `Operand::StructInit { name: "P", fields: [("x", ConstI32(1))] }` |
| Enum variant `Ok(v)` | `Operand::EnumInit { enum_name, variant: "Ok", tag: 0, payload: [⟨v⟩] }` |
| Array `[1, 2, 3]` | `Operand::ArrayInit { elements: [ConstI32(1), ConstI32(2), ConstI32(3)] }` |
| If expression | `Operand::IfExpr { cond, then_body, then_result, else_body, else_result }` |
| `expr?` (try) | `Operand::TryExpr { expr: ⟨expr⟩, from_fn }` |
| Closure `\|x\| x + 1` | Lambda-lifted to synthetic function + `Operand::FnRef("closure_N")` |
| Function reference | `Operand::FnRef("function_name")` |

**Short-circuit evaluation**: `&&` and `||` are lowered to `IfExpr`:
- `a && b` → `IfExpr { cond: a, then_result: b, else_result: ConstBool(false) }`
- `a || b` → `IfExpr { cond: a, then_result: ConstBool(true), else_result: b }`

### 6.3 Statement Lowering (stmt.rs)

| Source Statement | MIR Statement(s) |
|------------------|-------------------|
| `let x = expr;` | `MirStmt::Assign(Place::Local(x_id), Rvalue::Use(⟨expr⟩))` |
| `let (a, b) = expr;` | Tuple destructure: assign to temp, then extract fields |
| `x = expr;` | `MirStmt::Assign(Place::Local(x_id), Rvalue::Use(⟨expr⟩))` |
| `p.field = expr;` | `MirStmt::Assign(Place::Field(.., "field"), Rvalue::Use(⟨expr⟩))` |
| `f(a, b);` | `MirStmt::Call { dest: None, func: fn_id, args: [...] }` or `CallBuiltin` |
| `if c { ... } else { ... }` | `MirStmt::IfStmt { cond, then_body, else_body }` |
| `while c { ... }` | `MirStmt::WhileStmt { cond, body }` |
| `return expr;` | `MirStmt::Return(Some(⟨expr⟩))` |
| `break;` | `MirStmt::Break` |
| `continue;` | `MirStmt::Continue` |

### 6.4 Loop Desugaring (stmt.rs)

**Range for-loop** (`for i in start..end`):
```
let i = start;
while i < end {
    <body>
    i = i + 1;
}
```
Produces: `Assign(i, start)` → `WhileStmt { cond: BinOp(Lt, i, end), body: [<body>, Assign(i, Add(i, 1))] }`

**Value for-loop** (`for x in values(v)`):
```
let _idx = 0;
let _len = len(v);
while _idx < _len {
    let x = v[_idx];
    <body>
    _idx = _idx + 1;
}
```

**Iterator for-loop** (`for x in iter()`):
Similar desugaring to value loop using iterator protocol.

### 6.5 Pattern Match Lowering (pattern.rs)

Match expressions are desugared into nested if-else chains:

```
match scrutinee {
    Ok(v) => expr_a,
    Err(e) => expr_b,
}
```

Becomes:
```
let _tag = EnumTag(scrutinee);
if _tag == 0 {
    let v = EnumPayload { object: scrutinee, index: 0, variant_name: "Ok" };
    expr_a
} else {
    let e = EnumPayload { object: scrutinee, index: 0, variant_name: "Err" };
    expr_b
}
```

**Or-patterns** (`A | B => ...`) combine multiple conditions with logical OR.

**Struct patterns** (`Point { x, y } => ...`) extract fields via `FieldAccess`.

**Guard support**: `Pattern if guard => body` adds the guard as an additional
condition in the if-chain.

### 6.6 Closure Lowering (expr.rs)

Closures are **lambda-lifted**: each closure becomes a synthetic top-level
function with captured variables injected as extra parameters.

```
let y = 10;
let f = |x| x + y;
```

Becomes:
```
fn closure_0(x: i32, _capture_y: i32) -> i32 { x + _capture_y }
let f = FnRef("closure_0");
// Call sites pass captured values: Call("closure_0", [arg, y])
```

Free variable collection (`collect_free_vars`) identifies variables referenced
inside the closure but defined outside it.

### 6.7 Function Lowering (func.rs)

Each function definition produces a `MirFunction` with:
- Parameters registered as `MirLocal` entries
- Body statements lowered into a single `BasicBlock`
- Return type inferred from type annotations or checker output
- Type-tagged locals: `struct_typed_locals`, `enum_typed_locals`, and internal
  tracking sets for `string_locals`, `f64_locals`, `i64_locals`, `bool_locals`,
  `vec_locals`

### 6.8 Module Metadata

During lowering, the `TypeTable` is populated:
- All struct definitions → `type_table.struct_defs`
- All enum definitions (including `Option`, `Result`, specialized variants) → `type_table.enum_defs`
- Function signatures → `type_table.fn_sigs`
- Import declarations → `module.imports`

---

## 7. MIR Optimization Passes

Implemented in `crates/ark-mir/src/opt/`. The optimizer runs up to 3 rounds
(`MAX_OPT_ROUNDS = 3`) of the full pass pipeline. Each round runs all passes in
order; if no pass changes anything, iteration stops early.

Validation runs before and after each individual pass to ensure the MIR remains
well-formed.

### 7.1 Default Pass Order

```rust
const DEFAULT_PASS_ORDER: &[OptimizationPass] = &[
    ConstFold,
    BranchFold,
    CfgSimplify,
    LoopUnroll,
    CopyProp,
    ConstProp,
    TypeNarrowing,
    EscapeAnalysis,
    BoundsCheckElim,
    DeadLocalElim,
    DeadBlockElim,
    UnreachableCleanup,
    InlineSmallLeaf,
    StringConcatOpt,
    AggregateSimplify,
    AlgebraicSimplify,
    StrengthReduction,
    Cse,
    GcHint,
    BranchHintInfer,
];
```

### 7.2 Pass Descriptions

| Pass | Description |
|------|-------------|
| **ConstFold** | Evaluates constant binary operations at compile time and replaces them with their constant results (e.g., `5 + 3 → 8`). Handles i32, i64, and bool operations. |
| **BranchFold** | Folds constant-condition branches into unconditional gotos when the condition is a constant boolean value. |
| **CfgSimplify** | Simplifies the control flow graph by merging empty goto-only blocks. |
| **LoopUnroll** | Unrolls small loops with statically determined iteration counts. `MAX_UNROLL_ITERATIONS = 4`, `MAX_BODY_STMTS = 8`. |
| **CopyProp** | Eliminates copy assignments by replacing uses of copied locals with direct references to their source. |
| **ConstProp** | Propagates constant assignments within each basic block, replacing uses of constant-valued locals with the constant value. |
| **TypeNarrowing** | Narrows `i64` types to `i32` when values are provably within i32 range. Excludes parameters, return values, and unsafe uses. |
| **EscapeAnalysis** | Identifies struct allocations that don't escape the current function and applies Scalar Replacement of Aggregates (SROA): replaces each non-escaping struct field with individual scalar locals. |
| **BoundsCheckElim** | Removes redundant array bounds checks when the index is statically provable to be in range. Handles constant-index access, loop-induction-variable patterns, and duplicate bounds checks. |
| **DeadLocalElim** | Eliminates unused local variable declarations by collecting all referenced locals and removing unreferenced ones. |
| **DeadBlockElim** | Removes unreachable basic blocks via reachability analysis from the entry block. |
| **UnreachableCleanup** | Removes statements that appear after `return` within a block. |
| **InlineSmallLeaf** | Inlines small leaf functions (≤ `INLINE_SMALL_LEAF_BUDGET = 8` statements) into their callers. Also runs inter-function inlining at module level for functions ≤ 20 statements called ≤ 3 times. |
| **StringConcatOpt** | Normalizes string concatenation calls for potential optimization. |
| **AggregateSimplify** | Simplifies single-element `Aggregate` rvalues into direct `Use` assignments. |
| **AlgebraicSimplify** | Eliminates algebraic identities and absorbing elements (e.g., `x + 0 → x`, `x * 0 → 0`, `!!x → x`). |
| **StrengthReduction** | Replaces expensive operations with cheaper equivalents: `x * 2^n → x << n`, `x / 2^n → x >> n`. |
| **Cse** | Common Subexpression Elimination within basic blocks. Replaces duplicate pure binary/unary operations with references to the first computed result. Invalidates on side effects. |
| **GcHint** | Detects short-lived struct allocations inside loops and annotates them with `GcHint::ShortLived`. |
| **BranchHintInfer** | Infers branch likelihood hints by analyzing panic/error paths. Marks branches leading to panic/unreachable as `Unlikely`. |

### 7.3 Optimization Summary

Each pass returns an `OptimizationSummary` with counters:

```rust
struct OptimizationSummary {
    rounds: u32,
    const_folded: usize,
    branch_folded: usize,
    cfg_simplified: usize,
    copy_propagated: usize,
    const_propagated: usize,
    dead_locals_removed: usize,
    dead_blocks_removed: usize,
    unreachable_cleaned: usize,
    inline_small_leaf: usize,
    string_concat_normalized: usize,
    aggregate_simplified: usize,
    algebraic_simplified: usize,
    strength_reduced: usize,
    cse_eliminated: usize,
    loops_unrolled: usize,
    gc_hinted: usize,
    bounds_checks_eliminated: usize,
    scalar_replaced: usize,
    types_narrowed: usize,
    branch_hinted: usize,
}
```

### 7.4 Pipeline Control

The optimizer supports several modes:

- `optimize_module(module)` — run full pipeline (default)
- `optimize_module_named(module, &["const_fold", "copy_prop"])` — specific passes only
- `optimize_module_named_only(module, "const_fold")` — single pass, single round
- `optimize_module_named_without(module, "inline_small_leaf")` — all passes except one
- `optimize_module_named_until(module, "const_prop")` — all passes up to and including
- `optimize_module_none(module)` — validation only, no optimization

---

## 8. MIR Validation Rules

Implemented in `crates/ark-mir/src/validate.rs`.

### 8.1 Validation Entry Points

| Function | Description |
|----------|-------------|
| `validate_module(module)` | Structural validation; returns `Ok(())` or a list of errors |
| `validate_backend_legal_module(module)` | Structural validation **plus** backend-legality check (rejects `IfExpr`, `LoopExpr`, `TryExpr`) |

### 8.2 Structural Validation (`structural_errors`)

Module-level checks:

1. **No duplicate function IDs** — every `MirFunction.id` must be unique.
2. **Entry function exists** — if `entry_fn` is set, that `FnId` must exist among the module's functions.

### 8.3 Function Validation (`validate_function`)

Per-function checks:

1. **No duplicate block IDs** — every `BasicBlock.id` within a function must be unique.
2. **Entry block exists** — `func.entry` must refer to an existing block.
3. **No duplicate local IDs** — every local variable ID must be unique.
4. **Declared locals set** — all parameter and local IDs are collected; uses are checked against this set.
5. **Reachability analysis** — worklist-based CFG traversal from the entry block. Every successor block referenced by a terminator must exist.
6. **Per-block validation** — all statements and terminators in every block are recursively validated.

### 8.4 Statement Validation (`validate_stmt`)

| Statement | Checks |
|-----------|--------|
| `Assign(place, rvalue)` | Validate the place and the rvalue |
| `Call { dest, func, args }` | Function ID must exist in the module; validate dest place and all argument operands |
| `CallBuiltin { dest, args, .. }` | Validate dest place and all argument operands |
| `IfStmt { cond, then_body, else_body }` | Validate condition operand; recursively validate all nested statements |
| `WhileStmt { cond, body }` | Validate condition operand; recursively validate all nested statements |
| `Break`, `Continue` | No validation needed |
| `Return(value)` | Validate return value operand if present |
| `GcHint { .. }` | No validation needed |

### 8.5 Terminator Validation (`validate_terminator`)

| Terminator | Checks |
|------------|--------|
| `Goto(block)` | No operand validation (successor checked during reachability) |
| `If { cond, .. }` | Validate condition operand |
| `Switch { scrutinee, .. }` | Validate scrutinee operand |
| `Return(value)` | Validate return value operand if present |
| `Unreachable` | Entry block cannot terminate with `Unreachable` |

### 8.6 Rvalue Validation (`validate_rvalue`)

| Rvalue | Checks |
|--------|--------|
| `Use(operand)` | Validate the operand |
| `BinaryOp(op, lhs, rhs)` | Validate both operands; **division/modulo by zero constant** is rejected |
| `UnaryOp(op, operand)` | Validate the operand |
| `Aggregate(_, operands)` | Validate all operands |
| `Ref(place)` | Validate the place |

### 8.7 Place Validation (`validate_place`)

Recursive:

- `Place::Local(id)` — the local must be declared (in params or locals).
- `Place::Field(place, _)` — recursively validate the inner place.
- `Place::Index(place, index)` — recursively validate the inner place and the index operand.

### 8.8 Operand Validation (`validate_operand`)

All constant variants (`ConstI32`, `ConstI64`, `ConstF32`, `ConstF64`, `ConstU8`,
`ConstU16`, `ConstU32`, `ConstU64`, `ConstI8`, `ConstI16`, `ConstBool`,
`ConstChar`, `ConstString`, `Unit`, `FnRef`) require no further validation.

Compound operands are validated recursively:

| Operand | Validation |
|---------|------------|
| `Place(place)` | Validate the place |
| `BinOp(_, lhs, rhs)` | Validate both sub-operands |
| `UnaryOp(_, op)` / `EnumTag(op)` | Validate the inner operand |
| `Call(_, args)` | Validate all argument operands |
| `CallIndirect { callee, args }` | Validate callee and all argument operands |
| `IfExpr { cond, then_body, then_result, else_body, else_result }` | Validate condition, all nested statements, and both result operands |
| `LoopExpr { init, body, result }` | Validate init operand, all nested statements, and result operand |
| `TryExpr { expr, .. }` | Validate the inner expression operand |
| `StructInit { fields, .. }` | Validate all field value operands |
| `FieldAccess { object, .. }` | Validate the object operand |
| `EnumInit { payload, .. }` | Validate all payload operands |
| `EnumPayload { object, .. }` | Validate the object operand |
| `ArrayInit { elements }` | Validate all element operands |
| `IndexAccess { object, index }` | Validate both operands |

### 8.9 Error Reporting

```rust
struct MirValidationError {
    function: String,           // function name (or "<module>" for module-level)
    block: Option<BlockId>,     // block where error occurred (if applicable)
    message: String,            // human-readable error description
}
```

---

## 9. MIR → Wasm Mapping

The T3 backend (`crates/ark-wasm/src/emit/t3/`) emits Wasm GC instructions.
Linear memory (1 page) is reserved exclusively for WASI I/O marshaling.

### 9.1 Type Mapping

#### Strings: GC-Native Byte Arrays

```
MIR: Type::String  →  Wasm: (type $string (array (mut i8)))
```

- Creation: `array.new_data $string data_offset` (from static data section)
- Length: `array.len`
- Byte access: `array.get_u` (unsigned)
- Mutation: `array.set`
- Interning: at `opt_level >= 1`, static strings are cached in globals

#### Structs: GC Structs

```
MIR: struct_defs["Point"] = [("x", "i32"), ("y", "i32")]
  →
Wasm: (type $Point (struct (field i32) (field i32)))
```

- Construction: `struct.new $StructType`
- Field read: `struct.get $StructType field_index`
- Field write: `struct.set $StructType field_index`
- Fields never written after construction are declared immutable (eliminates GC
  write barriers).

#### Enums: Subtype Hierarchy (rec group)

```
MIR: enum_defs["Result"] = [("Ok", ["i32"]), ("Err", ["String"])]
  →
Wasm: (rec
  (type $Result (sub (struct)))          ;; base type (non-final)
  (type $Ok     (sub final $Result (struct (field i32))))
  (type $Err    (sub final $Result (struct (field (ref $string)))))
)
```

- Construction: `struct.new $VariantType`
- Tag extraction: `br_on_cast` chain (≥ 3 variants, `opt_level >= 1`) or
  `ref.test` chain (< 3 variants)
- Payload extraction: `ref.cast (non-null) $VariantType` → `struct.get`

#### Vectors: Struct + Array

```
MIR: Type::Vec(Type::I32)
  →
Wasm: (type $arr_i32 (array (mut i32)))
      (type $vec_i32 (struct
        (field (mut (ref $arr_i32)))   ;; backing data array
        (field (mut i32))              ;; length
      ))
```

Capacity = `array.len(data)`. Grow is implemented by allocating a new array and
copying.

### 9.2 Statement Mapping

#### Assign

```
MirStmt::Assign(Place::Local(id), Rvalue::Use(op))
  →
emit_operand(op)
local.set $id
```

```
MirStmt::Assign(Place::Field(Place::Local(id), field), Rvalue::Use(op))
  →
local.get $id
emit_operand(op)
struct.set $StructType field_index
```

#### Call

```
MirStmt::Call { dest, func, args }
  →
for arg in args: emit_operand_coerced(arg)
call $fn_index
if dest: local.set $dest_id
```

Tail-call optimization (`opt_level >= 1`): eligible `Return(Call(...))` patterns
are emitted as `return_call $fn_index`.

#### IfStmt

When `cond` is `ConstBool(true)` or `ConstBool(false)`, the branch structure is
eliminated (const-if elimination). Otherwise:

```
emit_operand(cond)
if
  emit_stmts(then_body)
else
  emit_stmts(else_body)
end
```

#### WhileStmt

```
block $break
  loop $continue
    emit_operand(cond)
    i32.eqz
    br_if $break            ;; exit if condition false
    emit_stmts(body)
    br $continue             ;; jump back to loop header
  end
end
```

#### Break / Continue

```
Break    → br $break_label   (depth adjusted for nesting)
Continue → br $continue_label
```

#### Return

```
emit_operand(value)
return
```

With anyref boxing: if function returns `Type::Any` and value is i32,
emit `ref.i31` before `return`.

#### GcHint

No Wasm instructions emitted (pure annotation consumed by the emitter for
allocation strategy decisions).

### 9.3 Operand Mapping

#### Constants

| Operand | Wasm |
|---------|------|
| `ConstI32(v)` | `i32.const v` |
| `ConstI64(v)` | `i64.const v` |
| `ConstF32(v)` | `f32.const v` |
| `ConstF64(v)` | `f64.const v` |
| `ConstBool(true)` | `i32.const 1` |
| `ConstBool(false)` | `i32.const 0` |
| `ConstChar(c)` | `i32.const (c as i32)` |
| `ConstU8(v)` | `i32.const v` |
| `ConstU16(v)` | `i32.const v` |
| `ConstU32(v)` | `i32.const v` |
| `ConstU64(v)` | `i64.const v` |
| `ConstI8(v)` | `i32.const v` |
| `ConstI16(v)` | `i32.const v` |
| `Unit` | (no value emitted) |
| `ConstString(s)` | `i32.const offset; i32.const len; array.new_data $string data_idx` (interned at opt ≥ 1) |

#### Place

```
Place::Local(id) → local.get $wasm_local_index
```

#### Binary Operations

Type-driven instruction selection:

| BinOp | i32 | i64 | f64 |
|-------|-----|-----|-----|
| `Add` | `i32.add` | `i64.add` | `f64.add` |
| `Sub` | `i32.sub` | `i64.sub` | `f64.sub` |
| `Mul` | `i32.mul` | `i64.mul` | `f64.mul` |
| `Div` | `i32.div_s` | `i64.div_s` | `f64.div` |
| `Mod` | `i32.rem_s` | `i64.rem_s` | — |
| `Eq` | `i32.eq` | `i64.eq` | `f64.eq` |
| `Ne` | `i32.ne` | `i64.ne` | `f64.ne` |
| `Lt` | `i32.lt_s` | `i64.lt_s` | `f64.lt` |
| `Le` | `i32.le_s` | `i64.le_s` | `f64.le` |
| `Gt` | `i32.gt_s` | `i64.gt_s` | `f64.gt` |
| `Ge` | `i32.ge_s` | `i64.ge_s` | `f64.ge` |
| `And` | `i32.and` | `i64.and` | — |
| `Or` | `i32.or` | `i64.or` | — |
| `BitAnd` | `i32.and` | `i64.and` | — |
| `BitOr` | `i32.or` | `i64.or` | — |
| `BitXor` | `i32.xor` | `i64.xor` | — |
| `Shl` | `i32.shl` | `i64.shl` | — |
| `Shr` | `i32.shr_s` | `i64.shr_s` | — |

**String equality**: when both operands are strings and the operation is
`Eq`/`Ne`, a byte-by-byte GC array comparison is emitted instead.

#### Unary Operations

| UnaryOp | i32 | i64 | f64 |
|---------|-----|-----|-----|
| `Neg` | `i32.const -1; i32.mul` | `i64.const 0; i64.sub (swap)` | `f64.neg` |
| `Not` | `i32.const -1; i32.xor` | — | — |
| `BitNot` | `i32.const -1; i32.xor` | `i64.const -1; i64.xor` | — |
| `SignExtend8` | `i32.extend8_s` | `i64.extend8_s` | — |
| `SignExtend16` | `i32.extend16_s` | `i64.extend16_s` | — |
| `SignExtend32` | — | `i64.extend32_s` | — |

#### Struct Operations

```
StructInit { name, fields }
  →
[emit each field value in definition order]
struct.new $StructType
```

```
FieldAccess { object, struct_name, field }
  →
emit_operand(object)
struct.get $StructType field_index
```

#### Enum Operations

```
EnumInit { enum_name, variant, payload }
  →
[emit payload values]
struct.new $VariantType
```

**Tag extraction** (optimized, `opt_level >= 1`, ≥ 3 variants):
```wasm
block $done (result i32)
  block $v0 (result (ref $Var0))
  block $v1 (result (ref $Var1))
    emit_operand(enum_value)
    br_on_cast 0 (any) (ref $Var0)
    br_on_cast 1 (any) (ref $Var1)
    unreachable
  end  ;; $v1
    drop
    i32.const 1
    br $done
  end  ;; $v0
    drop
    i32.const 0
end
```

**Tag extraction** (fallback, < 3 variants):
```wasm
emit_operand(enum_value)
ref.test (non-null) $Var0
if (result i32)
  i32.const 0
else
  i32.const 1
end
```

**Payload extraction**:
```
EnumPayload { object, index, variant_name }
  →
emit_operand(object)
ref.cast (non-null) $VariantType
struct.get $VariantType field_index
```

#### If Expression

```
emit_operand(cond)
if (result ValType)
  emit_stmts(then_body)
  emit_operand(then_result)
else
  emit_stmts(else_body)
  emit_operand(else_result)
end
```

When branches return different value types but one is anyref, values are boxed
via `ref.i31`.

#### Try Expression

```
TryExpr { expr, from_fn }
  →
emit_operand(expr)           ;; Result enum ref
local.set $scratch
local.get $scratch
ref.test (non-null) $Ok
if (result payload_type)
  ;; Ok path: extract payload
  local.get $scratch
  ref.cast (non-null) $Ok
  struct.get $Ok 0
else
  ;; Err path: convert error, wrap in outer Err, return
  local.get $scratch
  ref.cast (non-null) $Err
  struct.get $Err 0
  [if from_fn: call $from_fn]
  struct.new $OuterErr
  return
end
```

#### Call / CallIndirect

```
Call(name, args)
  →
for arg in args: emit_operand_coerced(arg)
call $fn_index
```

```
CallIndirect { callee, args }
  →
for arg in args: emit_operand(arg)
emit_operand(callee)
call_indirect type_index
```

### 9.4 Boxing and Unboxing (anyref ↔ concrete)

For generic functions using `Type::Any`:

**Boxing (i32 → anyref)**:
```wasm
ref.i31          ;; wrap i32 into i31ref
```

**Unboxing (anyref → i32)**:
```wasm
ref.cast (nullable) i31
i31.get_s        ;; extract signed i32 value
```

**String unboxing (anyref → ref $string)**:
```wasm
ref.cast (nullable) $string_type
```

### 9.5 Type Coercion

The emitter tracks locals by their concrete type using parallel sets
(`i64_locals`, `f64_locals`, `string_locals`, `struct_vec_locals`,
`any_locals`). When a function parameter expects a different type:

- `i32` constant → `i64`: emit `i64.const` directly
- `i32` constant → `f64`: emit `f64.const` directly
- `i32` value → anyref param: emit `ref.i31`
- anyref return → concrete type: emit appropriate `ref.cast` + extraction

### 9.6 Linear Memory Layout (WASI I/O Only)

Linear memory is **1 page** (64 KiB), reserved for WASI I/O marshaling. GC
values live on the Wasm GC heap, not in linear memory.

```
Offset   Purpose
──────   ───────
0        IOV_BASE: iov struct (ptr, len)
4        IOV_LEN: written count
16       SCRATCH: temporary scratch area
48       I32BUF: i32 conversions
56       SCR_VAL64: i64/f64 scratch
256+     DATA_START: static string/constant data
```
