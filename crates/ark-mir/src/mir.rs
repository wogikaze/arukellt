//! MIR type definitions.

use std::collections::{HashMap, HashSet};

use ark_diagnostics::Span;
use ark_typecheck::types::Type;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EffectKind {
    Pure,
    ReadsMemory,
    WritesMemory,
    ControlFlow,
    Call,
    Unknown,
}

impl EffectKind {
    pub fn is_effectful(self) -> bool {
        !matches!(self, Self::Pure)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct InstanceKey {
    pub item: String,
    pub substitution: Vec<String>,
    pub target_shape: String,
}

impl InstanceKey {
    pub fn simple(item: impl Into<String>) -> Self {
        Self {
            item: item.into(),
            substitution: Vec::new(),
            target_shape: "default".to_string(),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SourceInfo {
    pub span: Option<Span>,
}

impl SourceInfo {
    pub fn unknown() -> Self {
        Self { span: None }
    }
}

impl Default for SourceInfo {
    fn default() -> Self {
        Self::unknown()
    }
}

#[derive(Debug, Clone, Default)]
pub struct MirSourceMap {
    pub function_spans: HashMap<FnId, Span>,
    pub block_spans: HashMap<(FnId, BlockId), Span>,
    pub stmt_spans: HashMap<(FnId, BlockId, usize), Span>,
}

#[derive(Debug, Clone, Default)]
pub struct MirStats {
    pub instance_keys: Vec<InstanceKey>,
    pub direct_call_graph: HashMap<String, HashSet<String>>,
    pub optimization_rounds: u32,
    pub validation_runs: u32,
    pub provenance: Option<String>,
    pub optimization_trace: Vec<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MirProvenance {
    LegacyAst,
    CoreHir,
    CoreHirFallback,
    LegacyOptimized,
    CoreHirOptimized,
}

impl MirProvenance {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::LegacyAst => "legacy-ast",
            Self::CoreHir => "corehir",
            Self::CoreHirFallback => "corehir-fallback",
            Self::LegacyOptimized => "legacy-optimized",
            Self::CoreHirOptimized => "corehir-optimized",
        }
    }
}

impl MirStats {
    pub fn register_function(&mut self, func: &MirFunction) {
        self.instance_keys.push(func.instance.clone());

        let callees = self.direct_call_graph.entry(func.name.clone()).or_default();
        for block in &func.blocks {
            for stmt in &block.stmts {
                if let MirStmt::Call { func, .. } = stmt {
                    callees.insert(fn_id_name(*func));
                }
            }
        }
    }
}

pub fn set_mir_provenance(module: &mut MirModule, provenance: MirProvenance) {
    module.stats.provenance = Some(provenance.as_str().to_string());
}

pub fn mir_provenance(module: &MirModule) -> Option<&str> {
    module.stats.provenance.as_deref()
}

pub fn push_optimization_trace(module: &mut MirModule, label: impl Into<String>) {
    module.stats.optimization_trace.push(label.into());
}

pub fn optimization_trace(module: &MirModule) -> &[String] {
    &module.stats.optimization_trace
}

pub fn function_by_name<'a>(module: &'a MirModule, name: &str) -> Option<&'a MirFunction> {
    module.functions.iter().find(|func| func.name == name)
}

pub fn module_snapshot(module: &MirModule) -> String {
    format!(
        "functions={} entry={:?} provenance={} rounds={} trace=[{}]",
        module.functions.len(),
        module.entry_fn.map(|id| id.0),
        mir_provenance(module).unwrap_or("unknown"),
        module.stats.optimization_rounds,
        module.stats.optimization_trace.join(", ")
    )
}

pub fn runtime_entry_name(module: &MirModule) -> Option<String> {
    if function_by_name(module, "_start").is_some() {
        Some("_start".to_string())
    } else {
        module
            .entry_fn
            .map(|id| function_name_or_fallback(module, id))
    }
}

pub fn compare_module_shapes(lhs: &MirModule, rhs: &MirModule) -> String {
    format!(
        "lhs: {}\nrhs: {}",
        module_snapshot(lhs),
        module_snapshot(rhs)
    )
}

fn fn_id_name(func: FnId) -> String {
    format!("fn#{}", func.0)
}

/// Unique block identifier.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct BlockId(pub u32);

/// Unique local variable identifier.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct LocalId(pub u32);

/// Unique function identifier.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct FnId(pub u32);

/// Function signature as seen by the backend (string-based types).
#[derive(Debug, Clone)]
pub struct MirFnSig {
    pub name: String,
    pub params: Vec<String>,
    pub ret: String,
}

/// Nominal type table — all type information the backend needs.
///
/// The backend (Wasm emitter, LLVM, WIT generator) should read types
/// exclusively from this table rather than reaching back into the
/// frontend `TypeChecker`.
#[derive(Debug, Clone, Default)]
pub struct TypeTable {
    /// Struct layouts: struct name → ordered (field name, field type name).
    pub struct_defs: HashMap<String, Vec<(String, String)>>,
    /// Enum variant types: enum name → vec of (variant name, payload type names).
    pub enum_defs: HashMap<String, Vec<(String, Vec<String>)>>,
    /// Function signatures: fn name → simplified signature.
    pub fn_sigs: HashMap<String, MirFnSig>,
}

/// A WIT-derived import declaration for Component Model support.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MirImport {
    /// WIT interface name (e.g., "wasi:cli/stdout@0.2.0")
    pub interface: String,
    /// Function name within the interface
    pub name: String,
    /// Parameter type names (WIT type strings)
    pub param_types: Vec<String>,
    /// Return type name (WIT type string), None for void
    pub return_type: Option<String>,
}

/// A MIR module (after monomorphization).
#[derive(Debug, Clone)]
pub struct MirModule {
    pub functions: Vec<MirFunction>,
    pub entry_fn: Option<FnId>,
    /// Nominal type table for backend consumers.
    pub type_table: TypeTable,
    /// Struct layouts: struct name -> ordered (field name, field type name)
    pub struct_defs: HashMap<String, Vec<(String, String)>>,
    /// Enum variant types: enum_name -> vec of (variant_name, vec of payload type names)
    pub enum_defs: HashMap<String, Vec<(String, Vec<String>)>>,
    /// WIT-derived imports for Component Model
    pub imports: Vec<MirImport>,
    pub source_map: MirSourceMap,
    pub stats: MirStats,
}

/// A single function in MIR form.
#[derive(Debug, Clone)]
pub struct MirFunction {
    pub id: FnId,
    pub name: String,
    pub instance: InstanceKey,
    pub params: Vec<MirLocal>,
    pub return_ty: Type,
    pub locals: Vec<MirLocal>,
    pub blocks: Vec<BasicBlock>,
    pub entry: BlockId,
    /// Maps local id → struct type name (for field access/store)
    pub struct_typed_locals: std::collections::HashMap<u32, String>,
    /// Maps local id → enum type name (for GC-native enum refs)
    pub enum_typed_locals: std::collections::HashMap<u32, String>,
    /// Type parameter names (empty for non-generic functions)
    pub type_params: Vec<String>,
    pub source: SourceInfo,
    /// Whether this function is user-defined `pub fn` (exported at component boundary).
    /// Defaults to false; set to true by the lowerer for `pub fn` declarations.
    pub is_exported: bool,
}

/// A local variable (parameter or temporary).
#[derive(Debug, Clone)]
pub struct MirLocal {
    pub id: LocalId,
    pub name: Option<String>,
    pub ty: Type,
}

/// A basic block in the CFG.
#[derive(Debug, Clone)]
pub struct BasicBlock {
    pub id: BlockId,
    pub stmts: Vec<MirStmt>,
    pub terminator: Terminator,
    pub source: SourceInfo,
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

impl MirStmt {
    pub fn effect_kind(&self) -> EffectKind {
        match self {
            Self::Assign(_, rvalue) => rvalue.effect_kind(),
            Self::Call { .. } | Self::CallBuiltin { .. } => EffectKind::Call,
            Self::IfStmt { .. }
            | Self::WhileStmt { .. }
            | Self::Break
            | Self::Continue
            | Self::Return(_) => EffectKind::ControlFlow,
        }
    }
}

impl Terminator {
    pub fn effect_kind(&self) -> EffectKind {
        match self {
            Self::Goto(_) | Self::If { .. } | Self::Switch { .. } | Self::Return(_) => {
                EffectKind::ControlFlow
            }
            Self::Unreachable => EffectKind::Unknown,
        }
    }
}

impl Rvalue {
    pub fn effect_kind(&self) -> EffectKind {
        match self {
            Self::Use(operand) => operand.effect_kind(),
            Self::BinaryOp(..) | Self::UnaryOp(..) | Self::Aggregate(..) => EffectKind::Pure,
            Self::Ref(_) => EffectKind::ReadsMemory,
        }
    }
}

impl Operand {
    pub fn effect_kind(&self) -> EffectKind {
        match self {
            Self::Place(_)
            | Self::ConstI32(_)
            | Self::ConstI64(_)
            | Self::ConstF32(_)
            | Self::ConstF64(_)
            | Self::ConstU8(_)
            | Self::ConstU16(_)
            | Self::ConstU32(_)
            | Self::ConstU64(_)
            | Self::ConstI8(_)
            | Self::ConstI16(_)
            | Self::ConstBool(_)
            | Self::ConstChar(_)
            | Self::ConstString(_)
            | Self::Unit
            | Self::FnRef(_) => EffectKind::Pure,
            Self::BinOp(_, lhs, rhs) => combine_effects(lhs.effect_kind(), rhs.effect_kind()),
            Self::UnaryOp(_, operand) => operand.effect_kind(),
            Self::Call(_, _)
            | Self::IfExpr { .. }
            | Self::LoopExpr { .. }
            | Self::TryExpr { .. }
            | Self::CallIndirect { .. } => EffectKind::Call,
            Self::StructInit { .. }
            | Self::FieldAccess { .. }
            | Self::EnumInit { .. }
            | Self::EnumTag(_)
            | Self::EnumPayload { .. }
            | Self::ArrayInit { .. }
            | Self::IndexAccess { .. } => EffectKind::ReadsMemory,
        }
    }
}

fn combine_effects(lhs: EffectKind, rhs: EffectKind) -> EffectKind {
    if lhs.is_effectful() { lhs } else { rhs }
}

pub fn is_backend_legal_operand(operand: &Operand) -> bool {
    !matches!(
        operand,
        Operand::IfExpr { .. } | Operand::LoopExpr { .. } | Operand::TryExpr { .. }
    )
}

pub fn is_backend_legal_module(module: &MirModule) -> bool {
    module
        .functions
        .iter()
        .flat_map(|func| func.blocks.iter())
        .all(|block| {
            block.stmts.iter().all(is_backend_legal_stmt)
                && terminator_backend_legal(&block.terminator)
        })
}

fn is_backend_legal_stmt(stmt: &MirStmt) -> bool {
    match stmt {
        MirStmt::Assign(_, rvalue) => match rvalue {
            Rvalue::Use(operand) => is_backend_legal_operand(operand),
            Rvalue::BinaryOp(_, lhs, rhs) => {
                is_backend_legal_operand(lhs) && is_backend_legal_operand(rhs)
            }
            Rvalue::UnaryOp(_, operand) => is_backend_legal_operand(operand),
            Rvalue::Aggregate(_, operands) => operands.iter().all(is_backend_legal_operand),
            Rvalue::Ref(_) => true,
        },
        MirStmt::Call { args, .. } | MirStmt::CallBuiltin { args, .. } => {
            args.iter().all(is_backend_legal_operand)
        }
        MirStmt::IfStmt {
            cond,
            then_body,
            else_body,
        } => {
            is_backend_legal_operand(cond)
                && then_body.iter().all(is_backend_legal_stmt)
                && else_body.iter().all(is_backend_legal_stmt)
        }
        MirStmt::WhileStmt { cond, body } => {
            is_backend_legal_operand(cond) && body.iter().all(is_backend_legal_stmt)
        }
        MirStmt::Break | MirStmt::Continue => true,
        MirStmt::Return(value) => value.as_ref().is_none_or(is_backend_legal_operand),
    }
}

fn terminator_backend_legal(terminator: &Terminator) -> bool {
    match terminator {
        Terminator::Goto(_) | Terminator::Unreachable => true,
        Terminator::If { cond, .. } => is_backend_legal_operand(cond),
        Terminator::Switch { scrutinee, .. } => is_backend_legal_operand(scrutinee),
        Terminator::Return(value) => value.as_ref().is_none_or(is_backend_legal_operand),
    }
}

pub fn function_name(module: &MirModule, id: FnId) -> Option<&str> {
    module
        .functions
        .iter()
        .find(|func| func.id == id)
        .map(|func| func.name.as_str())
}

pub fn function_name_or_fallback(module: &MirModule, id: FnId) -> String {
    function_name(module, id)
        .map(str::to_string)
        .unwrap_or_else(|| fn_id_name(id))
}

pub fn function_id_by_name(module: &MirModule, name: &str) -> Option<FnId> {
    module
        .functions
        .iter()
        .find(|func| func.name == name)
        .map(|func| func.id)
}

pub fn register_function_metadata(module: &mut MirModule, func: &MirFunction) {
    module.stats.register_function(func);
    if let Some(span) = func.source.span {
        module.source_map.function_spans.insert(func.id, span);
    }
    for block in &func.blocks {
        if let Some(span) = block.source.span {
            module
                .source_map
                .block_spans
                .insert((func.id, block.id), span);
        }
    }
}

pub fn sync_module_metadata(module: &mut MirModule) {
    module.source_map = MirSourceMap::default();
    module.stats = MirStats::default();
    let functions = module.functions.clone();
    for func in &functions {
        register_function_metadata(module, func);
    }
}

pub fn legacy_fn_id(name: &str) -> FnId {
    let mut hash = 0u32;
    for byte in name.bytes() {
        hash = hash.wrapping_mul(16777619) ^ byte as u32;
    }
    FnId(hash)
}

pub fn legacy_name_for_id(id: FnId) -> String {
    fn_id_name(id)
}

pub fn next_available_fn_id(module: &MirModule) -> FnId {
    let next = module
        .functions
        .iter()
        .map(|func| func.id.0)
        .max()
        .unwrap_or(0)
        .saturating_add(1);
    FnId(next)
}

pub fn next_available_block_id(function: &MirFunction) -> BlockId {
    let next = function
        .blocks
        .iter()
        .map(|block| block.id.0)
        .max()
        .unwrap_or(0)
        .saturating_add(1);
    BlockId(next)
}

pub fn next_available_local_id(function: &MirFunction) -> LocalId {
    let next = function
        .locals
        .iter()
        .chain(function.params.iter())
        .map(|local| local.id.0)
        .max()
        .unwrap_or(0)
        .saturating_add(1);
    LocalId(next)
}

pub fn instance_display(instance: &InstanceKey) -> String {
    if instance.substitution.is_empty() {
        return instance.item.clone();
    }

    format!(
        "{}<{}>@{}",
        instance.item,
        instance.substitution.join(","),
        instance.target_shape
    )
}

pub fn default_function_source() -> SourceInfo {
    SourceInfo::unknown()
}

pub fn default_block_source() -> SourceInfo {
    SourceInfo::unknown()
}

pub fn statement_span(
    module: &MirModule,
    func: FnId,
    block: BlockId,
    stmt_index: usize,
) -> Option<Span> {
    module
        .source_map
        .stmt_spans
        .get(&(func, block, stmt_index))
        .copied()
}

pub fn record_statement_span(
    module: &mut MirModule,
    func: FnId,
    block: BlockId,
    stmt_index: usize,
    span: Span,
) {
    module
        .source_map
        .stmt_spans
        .insert((func, block, stmt_index), span);
}

pub fn remove_statement_span(
    module: &mut MirModule,
    func: FnId,
    block: BlockId,
    stmt_index: usize,
) {
    module
        .source_map
        .stmt_spans
        .remove(&(func, block, stmt_index));
}

pub fn clear_statement_spans_for_block(module: &mut MirModule, func: FnId, block: BlockId) {
    module
        .source_map
        .stmt_spans
        .retain(|(existing_func, existing_block, _), _| {
            *existing_func != func || *existing_block != block
        });
}

pub fn renumber_statement_spans(module: &mut MirModule, func: FnId, block: BlockId, len: usize) {
    let mut spans = Vec::new();
    for idx in 0..len {
        if let Some(span) = statement_span(module, func, block, idx) {
            spans.push((idx, span));
        }
    }
    clear_statement_spans_for_block(module, func, block);
    for (idx, span) in spans {
        record_statement_span(module, func, block, idx, span);
    }
}

pub fn function_by_name_mut<'a>(
    module: &'a mut MirModule,
    name: &str,
) -> Option<&'a mut MirFunction> {
    module.functions.iter_mut().find(|func| func.name == name)
}

pub fn function_by_id_mut(module: &mut MirModule, id: FnId) -> Option<&mut MirFunction> {
    module.functions.iter_mut().find(|func| func.id == id)
}

pub fn function_by_id(module: &MirModule, id: FnId) -> Option<&MirFunction> {
    module.functions.iter().find(|func| func.id == id)
}

pub fn collect_function_ids(module: &MirModule) -> HashSet<FnId> {
    module.functions.iter().map(|func| func.id).collect()
}

pub fn operand_calls(operand: &Operand, out: &mut Vec<String>) {
    match operand {
        Operand::Call(name, args) => {
            out.push(name.clone());
            for arg in args {
                operand_calls(arg, out);
            }
        }
        Operand::BinOp(_, lhs, rhs) => {
            operand_calls(lhs, out);
            operand_calls(rhs, out);
        }
        Operand::UnaryOp(_, inner) | Operand::EnumTag(inner) => {
            operand_calls(inner, out);
        }
        Operand::FnRef(_)
        | Operand::Place(_)
        | Operand::ConstI32(_)
        | Operand::ConstI64(_)
        | Operand::ConstF32(_)
        | Operand::ConstF64(_)
        | Operand::ConstU8(_)
        | Operand::ConstU16(_)
        | Operand::ConstU32(_)
        | Operand::ConstU64(_)
        | Operand::ConstI8(_)
        | Operand::ConstI16(_)
        | Operand::ConstBool(_)
        | Operand::ConstChar(_)
        | Operand::ConstString(_)
        | Operand::Unit => {}

        Operand::IfExpr {
            cond,
            then_body,
            then_result,
            else_body,
            else_result,
        } => {
            operand_calls(cond, out);
            for stmt in then_body {
                stmt_calls(stmt, out);
            }
            if let Some(result) = then_result {
                operand_calls(result, out);
            }
            for stmt in else_body {
                stmt_calls(stmt, out);
            }
            if let Some(result) = else_result {
                operand_calls(result, out);
            }
        }
        Operand::StructInit { fields, .. } => {
            for (_, operand) in fields {
                operand_calls(operand, out);
            }
        }
        Operand::FieldAccess { object, .. } => operand_calls(object, out),
        Operand::EnumInit { payload, .. } => {
            for operand in payload {
                operand_calls(operand, out);
            }
        }
        Operand::EnumPayload { object, .. } => operand_calls(object, out),
        Operand::LoopExpr { init, body, result } => {
            operand_calls(init, out);
            for stmt in body {
                stmt_calls(stmt, out);
            }
            operand_calls(result, out);
        }
        Operand::TryExpr { expr, from_fn } => {
            operand_calls(expr, out);
            if let Some(from_fn) = from_fn {
                out.push(from_fn.clone());
            }
        }
        Operand::CallIndirect { callee, args } => {
            operand_calls(callee, out);
            for arg in args {
                operand_calls(arg, out);
            }
        }
        Operand::ArrayInit { elements } => {
            for operand in elements {
                operand_calls(operand, out);
            }
        }
        Operand::IndexAccess { object, index } => {
            operand_calls(object, out);
            operand_calls(index, out);
        }
    }
}

fn stmt_calls(stmt: &MirStmt, out: &mut Vec<String>) {
    match stmt {
        MirStmt::Assign(_, rvalue) => match rvalue {
            Rvalue::Use(operand) => operand_calls(operand, out),
            Rvalue::BinaryOp(_, lhs, rhs) => {
                operand_calls(lhs, out);
                operand_calls(rhs, out);
            }
            Rvalue::UnaryOp(_, operand) => operand_calls(operand, out),
            Rvalue::Aggregate(_, operands) => {
                for operand in operands {
                    operand_calls(operand, out);
                }
            }
            Rvalue::Ref(_) => {}
        },
        MirStmt::Call { func, args, .. } => {
            out.push(fn_id_name(*func));
            for operand in args {
                operand_calls(operand, out);
            }
        }
        MirStmt::CallBuiltin { args, .. } => {
            for operand in args {
                operand_calls(operand, out);
            }
        }
        MirStmt::IfStmt {
            cond,
            then_body,
            else_body,
        } => {
            operand_calls(cond, out);
            for stmt in then_body {
                stmt_calls(stmt, out);
            }
            for stmt in else_body {
                stmt_calls(stmt, out);
            }
        }
        MirStmt::WhileStmt { cond, body } => {
            operand_calls(cond, out);
            for stmt in body {
                stmt_calls(stmt, out);
            }
        }
        MirStmt::Break | MirStmt::Continue => {}
        MirStmt::Return(value) => {
            if let Some(value) = value {
                operand_calls(value, out);
            }
        }
    }
}

/// How a basic block ends.
#[derive(Debug, Clone)]
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
    // Extended scalar constants (v3 #040)
    // At Wasm level, U8/U16/U32/I8/I16 are i32; U64 is i64.
    // The type tag is preserved so the emitter can apply masking/sign-extension.
    ConstU8(u8),
    ConstU16(u16),
    ConstU32(u32),
    ConstU64(u64),
    ConstI8(i8),
    ConstI16(i16),
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
    /// Loop as expression: execute body (which includes WhileStmt), then return result.
    LoopExpr {
        init: Box<Operand>,
        body: Vec<MirStmt>,
        result: Box<Operand>,
    },
    /// Try expression (`expr?`): unwrap Ok or early-return Err.
    TryExpr {
        expr: Box<Operand>,
        /// If set, call this function to convert the Err value (From trait).
        from_fn: Option<String>,
    },
    /// Reference to a named function (for passing as value).
    FnRef(String),
    /// Indirect call via function pointer.
    CallIndirect {
        callee: Box<Operand>,
        args: Vec<Operand>,
    },
    /// Construct a fixed-size array from elements.
    ArrayInit {
        elements: Vec<Operand>,
    },
    /// Index into an array: array[index].
    IndexAccess {
        object: Box<Operand>,
        index: Box<Operand>,
    },
}

/// Binary operations.
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

/// Unary operations.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UnaryOp {
    Neg,
    Not,
    BitNot,
}

/// Kind of aggregate being constructed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AggregateKind {
    Tuple,
    Array,
    Struct(String),
    EnumVariant(String, String), // enum name, variant name
}

impl MirModule {
    pub fn new() -> Self {
        Self {
            functions: Vec::new(),
            entry_fn: None,
            type_table: TypeTable::default(),
            struct_defs: HashMap::new(),
            enum_defs: HashMap::new(),
            imports: Vec::new(),
            source_map: MirSourceMap::default(),
            stats: MirStats::default(),
        }
    }
}

impl Default for MirModule {
    fn default() -> Self {
        Self::new()
    }
}
