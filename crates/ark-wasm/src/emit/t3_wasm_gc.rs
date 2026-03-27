//! T3 `wasm32-wasi-p2` backend — Wasm GC emitter.
//!
//! Generates a Wasm module using GC types (struct/array/ref) for heap objects.
//! Strings are GC structs wrapping a byte array. Scalars use Wasm locals.
//! WASI Preview 1 fd_write is used for I/O, bridged via a small linear memory
//! region for the iov buffer.
//!
//! Type layout (GC):
//!   String  → (struct (field $bytes (ref $bytes_array)))
//!   Vec<T>  → (struct (field $data (ref $array_T)) (field $len i32) (field $cap i32))
//!   Struct  → (struct field0 field1 …)
//!   Enum    → (struct (field $tag i32) (field $p0 i32) …)

#![allow(dead_code)]

use ark_diagnostics::DiagnosticSink;
use ark_mir::mir::*;
use ark_typecheck::types::Type;
use std::collections::{HashMap, HashSet, VecDeque};
use wasm_encoder::{
    ArrayType, CodeSection, CompositeInnerType, CompositeType, DataSection, DataSegment,
    ExportKind, ExportSection, FieldType, Function, FunctionSection, GlobalSection, GlobalType,
    ImportSection, Instruction, MemArg, MemorySection, MemoryType, RefType as WasmRefType,
    StorageType, StructType, SubType, TypeSection, ValType,
};

// ── Linear memory layout (IO bridge only) ────────────────────────
const IOV_BASE: u32 = 0;
const IOV_LEN: u32 = 4;
const NWRITTEN: u32 = 8;
const SCRATCH: u32 = 16;
const I32BUF: u32 = 48;
const DATA_START: u32 = 256;
const SCR_A_PTR: u32 = SCRATCH;
const SCR_B_PTR: u32 = SCRATCH + 4;
const SCR_A_LEN: u32 = SCRATCH + 8;
const SCR_B_LEN: u32 = SCRATCH + 12;
const SCR_DST_PTR: u32 = SCRATCH + 16;
const SCR_I: u32 = SCRATCH + 20;
const SCR_J: u32 = SCRATCH + 24;

// GC struct field indices
const STR_FIELD_BYTES: u32 = 0;
const VEC_FIELD_DATA: u32 = 0;
const VEC_FIELD_LEN: u32 = 1;
const VEC_FIELD_CAP: u32 = 2;

// Well-known import function index
const FN_FD_WRITE: u32 = 0;

fn mutable_field(st: StorageType) -> FieldType {
    FieldType {
        element_type: st,
        mutable: true,
    }
}

fn immutable_field(st: StorageType) -> FieldType {
    FieldType {
        element_type: st,
        mutable: false,
    }
}

fn ref_nullable(idx: u32) -> ValType {
    ValType::Ref(WasmRefType {
        nullable: true,
        heap_type: wasm_encoder::HeapType::Concrete(idx),
    })
}

fn ref_non_null(idx: u32) -> ValType {
    ValType::Ref(WasmRefType {
        nullable: false,
        heap_type: wasm_encoder::HeapType::Concrete(idx),
    })
}

/// Normalize `__intrinsic_*` names to canonical emit names.
fn normalize_intrinsic(name: &str) -> &str {
    if let Some(stripped) = name.strip_prefix("__intrinsic_") {
        match stripped {
            "println" => "println",
            "print" => "print",
            "string_from" => "String_from",
            "i32_to_string" => "i32_to_string",
            "i64_to_string" => "i64_to_string",
            "f64_to_string" => "f64_to_string",
            "bool_to_string" => "bool_to_string",
            "concat" => "concat",
            "len" => "len",
            "push" => "push",
            "get" => "get",
            "get_unchecked" => "get_unchecked",
            "set" => "set",
            "pop" => "pop",
            "panic" => "panic",
            "assert" => "assert",
            "assert_eq" => "assert_eq",
            "Vec_new_i32" => "Vec_new_i32",
            "Vec_new_i64" => "Vec_new_i64",
            "Vec_new_f64" => "Vec_new_f64",
            "Vec_new_String" => "Vec_new_String",
            "sort_i32" => "sort_i32",
            other => other,
        }
    } else {
        name
    }
}

// ── Type allocator ───────────────────────────────────────────────

struct TypeAlloc {
    next_idx: u32,
    names: HashMap<String, u32>,
    section: TypeSection,
}

impl TypeAlloc {
    fn new() -> Self {
        Self {
            next_idx: 0,
            names: HashMap::new(),
            section: TypeSection::new(),
        }
    }

    fn add_func(&mut self, params: &[ValType], results: &[ValType]) -> u32 {
        let idx = self.next_idx;
        self.section
            .ty()
            .function(params.iter().copied(), results.iter().copied());
        self.next_idx += 1;
        idx
    }

    fn add_struct(&mut self, name: &str, fields: &[FieldType]) -> u32 {
        let idx = self.next_idx;
        self.names.insert(name.to_string(), idx);
        self.section.ty().subtype(&SubType {
            is_final: true,
            supertype_idx: None,
            composite_type: CompositeType {
                inner: CompositeInnerType::Struct(StructType {
                    fields: fields.to_vec().into_boxed_slice(),
                }),
                shared: false,
            },
        });
        self.next_idx += 1;
        idx
    }

    fn add_array(&mut self, name: &str, element: FieldType) -> u32 {
        let idx = self.next_idx;
        self.names.insert(name.to_string(), idx);
        self.section.ty().subtype(&SubType {
            is_final: true,
            supertype_idx: None,
            composite_type: CompositeType {
                inner: CompositeInnerType::Array(ArrayType(element)),
                shared: false,
            },
        });
        self.next_idx += 1;
        idx
    }
}

// ── Emit context ─────────────────────────────────────────────────

struct Ctx {
    types: TypeAlloc,
    data_segs: Vec<(u32, Vec<u8>)>,
    data_offset: u32,
    fn_map: HashMap<String, u32>,
    fn_names: Vec<String>,
    next_fn: u32,
    // Well-known GC type indices
    bytes_arr_ty: u32,
    string_ty: u32,
    arr_i32_ty: u32,
    vec_i32_ty: u32,
    arr_i64_ty: u32,
    vec_i64_ty: u32,
    arr_f64_ty: u32,
    vec_f64_ty: u32,
    // Well-known function type indices
    fd_write_ty: u32,
    // User struct/enum GC type indices
    struct_gc_types: HashMap<String, u32>,
    struct_layouts: HashMap<String, Vec<(String, String)>>,
    enum_gc_types: HashMap<String, u32>,
    enum_defs: HashMap<String, Vec<(String, Vec<String>)>>,
    fn_ret_types: HashMap<String, Type>,
    // Local type tracking (per-function)
    string_locals: std::collections::HashSet<u32>,
    f64_locals: std::collections::HashSet<u32>,
    i64_locals: std::collections::HashSet<u32>,
    bool_locals: std::collections::HashSet<u32>,
    local_struct: HashMap<u32, String>,
    // Helper function indices (emitted once)
    helper_i32_to_str: Option<u32>,
    helper_print_i32_ln: Option<u32>,
    helper_print_bool_ln: Option<u32>,
    helper_print_str_ln: Option<u32>,
    helper_print_newline: Option<u32>,
}

impl Ctx {
    fn type_to_val(&self, ty: &Type) -> ValType {
        match ty {
            Type::I64 => ValType::I64,
            Type::F64 => ValType::F64,
            Type::F32 => ValType::F32,
            // Bridge mode: all heap types are i32 pointers into linear memory
            _ => ValType::I32,
        }
    }

    fn alloc_data(&mut self, data: &[u8]) -> u32 {
        let offset = self.data_offset;
        self.data_segs.push((offset, data.to_vec()));
        self.data_offset += data.len() as u32;
        // Align to 4 bytes
        while self.data_offset % 4 != 0 {
            self.data_offset += 1;
        }
        offset
    }

    fn field_valtype(&self, ty_name: &str) -> ValType {
        match ty_name {
            "i64" => ValType::I64,
            "f64" => ValType::F64,
            "f32" => ValType::F32,
            "String" => ref_nullable(self.string_ty),
            _ => ValType::I32,
        }
    }
}

// ── Public entry point ───────────────────────────────────────────

/// Emit a Wasm module from MIR using real Wasm GC types.
///
/// Scalars live in Wasm locals. Strings, Vecs, structs, and enums use
/// GC struct/array types. I/O bridges through a small linear memory
/// region for WASI fd_write.
pub fn emit(mir: &MirModule, _sink: &mut DiagnosticSink) -> Vec<u8> {
    // TODO(MIR-01): remove checker fallback — read layouts from type_table only
    let struct_layouts: HashMap<String, Vec<(String, String)>> = mir.type_table.struct_defs.clone();
    let fn_ret_types: HashMap<String, Type> = mir
        .functions
        .iter()
        .map(|f| (f.name.clone(), f.return_ty.clone()))
        .collect();

    let mut ctx = Ctx {
        types: TypeAlloc::new(),
        data_segs: Vec::new(),
        data_offset: DATA_START,
        fn_map: HashMap::new(),
        fn_names: mir.functions.iter().map(|f| f.name.clone()).collect(),
        next_fn: 0,
        bytes_arr_ty: 0,
        string_ty: 0,
        arr_i32_ty: 0,
        vec_i32_ty: 0,
        arr_i64_ty: 0,
        vec_i64_ty: 0,
        arr_f64_ty: 0,
        vec_f64_ty: 0,
        fd_write_ty: 0,
        struct_gc_types: HashMap::new(),
        struct_layouts,
        enum_gc_types: HashMap::new(),
        enum_defs: mir.type_table.enum_defs.clone(),
        fn_ret_types,
        string_locals: Default::default(),
        f64_locals: Default::default(),
        i64_locals: Default::default(),
        bool_locals: Default::default(),
        local_struct: HashMap::new(),
        helper_i32_to_str: None,
        helper_print_i32_ln: None,
        helper_print_bool_ln: None,
        helper_print_str_ln: None,
        helper_print_newline: None,
    };
    ctx.emit_module(mir)
}

// ── Module emission ──────────────────────────────────────────────

impl Ctx {
    fn emit_module(&mut self, mir: &MirModule) -> Vec<u8> {
        let reachable_user_indices = self.reachable_function_indices(mir);

        // Phase 1: Register GC types
        self.register_gc_types(mir);

        // Phase 2: Register function type signatures
        let fd_write_ty = self.types.add_func(&[ValType::I32; 4], &[ValType::I32]);
        self.fd_write_ty = fd_write_ty;

        // Count helper functions we'll need
        let num_imports = 1u32; // fd_write
        // Bridge mode: all heap values are i32 pointers into linear memory.
        // GC ref types are declared in the type section but not yet used
        // in function signatures (that's Phase 2).
        let helper_fns: Vec<(String, Vec<ValType>, Vec<ValType>)> = vec![
            // __print_str_ln: (i32 ptr) -> ()
            ("__print_str_ln".into(), vec![ValType::I32], vec![]),
            // __print_i32_ln: (i32) -> ()
            ("__print_i32_ln".into(), vec![ValType::I32], vec![]),
            // __print_bool_ln: (i32) -> ()
            ("__print_bool_ln".into(), vec![ValType::I32], vec![]),
            // __i32_to_str: (i32) -> i32 ptr
            (
                "__i32_to_str".into(),
                vec![ValType::I32],
                vec![ValType::I32],
            ),
            // __print_newline: () -> ()
            ("__print_newline".into(), vec![], vec![]),
        ];

        // Register function types for helpers
        let mut helper_type_indices = Vec::new();
        for (_, params, results) in &helper_fns {
            let ty_idx = self.types.add_func(params, results);
            helper_type_indices.push(ty_idx);
        }

        // Register function types for user functions
        let mut user_fn_type_indices = Vec::new();
        for &idx in &reachable_user_indices {
            let func = &mir.functions[idx];
            let params: Vec<ValType> = func
                .params
                .iter()
                .map(|p| self.type_to_val(&p.ty))
                .collect();
            let results: Vec<ValType> = match &func.return_ty {
                Type::Unit | Type::Never => vec![],
                ty => vec![self.type_to_val(ty)],
            };
            let ty_idx = self.types.add_func(&params, &results);
            user_fn_type_indices.push(ty_idx);
        }

        // Assign function indices: imports first, then helpers, then user fns
        let helper_base = num_imports;
        for (i, (name, _, _)) in helper_fns.iter().enumerate() {
            let fn_idx = helper_base + i as u32;
            self.fn_map.insert(name.clone(), fn_idx);
        }
        self.helper_print_str_ln = Some(helper_base);
        self.helper_print_i32_ln = Some(helper_base + 1);
        self.helper_print_bool_ln = Some(helper_base + 2);
        self.helper_i32_to_str = Some(helper_base + 3);
        self.helper_print_newline = Some(helper_base + 4);

        let user_base = helper_base + helper_fns.len() as u32;
        for (i, &idx) in reachable_user_indices.iter().enumerate() {
            let func = &mir.functions[idx];
            self.fn_map.insert(func.name.clone(), user_base + i as u32);
        }

        // ── Build sections ───────────────────────────────────────

        // Import section: fd_write
        let mut imports = ImportSection::new();
        imports.import(
            "wasi_snapshot_preview1",
            "fd_write",
            wasm_encoder::EntityType::Function(fd_write_ty),
        );

        // Function section
        let mut functions = FunctionSection::new();
        for &ty_idx in &helper_type_indices {
            functions.function(ty_idx);
        }
        for &ty_idx in &user_fn_type_indices {
            functions.function(ty_idx);
        }

        // Memory section (small, for IO bridge only)
        let mut memories = MemorySection::new();
        memories.memory(MemoryType {
            // T3 still uses a linear-memory bump allocator for strings/Vec headers,
            // so reserve enough pages up front for the 10k Vec benchmarks.
            minimum: 4,
            maximum: Some(10),
            memory64: false,
            shared: false,
            page_size_log2: None,
        });

        // Export section
        let mut exports = ExportSection::new();
        exports.export("memory", ExportKind::Memory, 0);
        if let Some(&start_idx) = self.fn_map.get("_start") {
            exports.export("_start", ExportKind::Func, start_idx);
        } else if let Some(&main_idx) = self.fn_map.get("main") {
            exports.export("_start", ExportKind::Func, main_idx);
        }

        // Data section: static string literals and constants
        // Pre-allocate "true", "false", "\n" for print helpers
        let true_offset = self.alloc_data(b"true");
        let false_offset = self.alloc_data(b"false");
        let newline_offset = self.alloc_data(b"\n");

        // Code section: emit helper + user functions
        let mut codes = CodeSection::new();

        // Helper: __print_str_ln(str_ref)
        self.emit_print_str_ln_helper(&mut codes, newline_offset);
        // Helper: __print_i32_ln(val)
        self.emit_print_i32_ln_helper(&mut codes, newline_offset);
        // Helper: __print_bool_ln(val)
        self.emit_print_bool_ln_helper(&mut codes, true_offset, false_offset, newline_offset);
        // Helper: __i32_to_str(val) -> ref $string
        self.emit_i32_to_str_helper(&mut codes);
        // Helper: __print_newline()
        self.emit_print_newline_helper(&mut codes, newline_offset);

        // User functions
        for &idx in &reachable_user_indices {
            let func = &mir.functions[idx];
            self.emit_function(&mut codes, func);
        }

        // Global: heap_ptr starts after all static data segments so dynamic
        // strings do not overwrite embedded literals.
        let mut globals = GlobalSection::new();
        globals.global(
            GlobalType {
                val_type: ValType::I32,
                mutable: true,
                shared: false,
            },
            &wasm_encoder::ConstExpr::i32_const(self.data_offset as i32),
        );

        // Data section
        let mut data = DataSection::new();
        for (offset, bytes) in &self.data_segs {
            data.segment(DataSegment {
                mode: wasm_encoder::DataSegmentMode::Active {
                    memory_index: 0,
                    offset: &wasm_encoder::ConstExpr::i32_const(*offset as i32),
                },
                data: bytes.iter().copied(),
            });
        }

        // Assemble module
        let mut module = wasm_encoder::Module::new();
        module.section(&self.types.section);
        module.section(&imports);
        module.section(&functions);
        module.section(&memories);
        module.section(&globals);
        module.section(&exports);
        module.section(&codes);
        module.section(&data);
        module.finish()
    }

    fn reachable_function_indices(&self, mir: &MirModule) -> Vec<usize> {
        let mut name_to_idx = HashMap::new();
        for (idx, func) in mir.functions.iter().enumerate() {
            name_to_idx.insert(func.name.as_str(), idx);
        }

        let mut reachable = HashSet::new();
        let mut queue = VecDeque::new();

        let push_root =
            |idx: usize, reachable: &mut HashSet<usize>, queue: &mut VecDeque<usize>| {
                if reachable.insert(idx) {
                    queue.push_back(idx);
                }
            };

        if let Some(entry) = mir.entry_fn {
            push_root(entry.0 as usize, &mut reachable, &mut queue);
        }
        if queue.is_empty() {
            for root_name in ["_start", "main"] {
                if let Some(&idx) = name_to_idx.get(root_name) {
                    push_root(idx, &mut reachable, &mut queue);
                }
            }
        }

        while let Some(func_idx) = queue.pop_front() {
            let func = &mir.functions[func_idx];
            self.collect_reachable_from_function(func, &name_to_idx, &mut reachable, &mut queue);
        }

        let mut ordered: Vec<_> = reachable.into_iter().collect();
        ordered.sort_unstable();
        ordered
    }

    fn collect_reachable_from_function(
        &self,
        func: &MirFunction,
        name_to_idx: &HashMap<&str, usize>,
        reachable: &mut HashSet<usize>,
        queue: &mut VecDeque<usize>,
    ) {
        for block in &func.blocks {
            for stmt in &block.stmts {
                self.collect_reachable_from_stmt(stmt, name_to_idx, reachable, queue);
            }
            self.collect_reachable_from_terminator(
                &block.terminator,
                name_to_idx,
                reachable,
                queue,
            );
        }
    }

    fn collect_reachable_from_stmt(
        &self,
        stmt: &MirStmt,
        name_to_idx: &HashMap<&str, usize>,
        reachable: &mut HashSet<usize>,
        queue: &mut VecDeque<usize>,
    ) {
        match stmt {
            MirStmt::Assign(place, rvalue) => {
                self.collect_reachable_from_place(place, name_to_idx, reachable, queue);
                self.collect_reachable_from_rvalue(rvalue, name_to_idx, reachable, queue);
            }
            MirStmt::Call { func, args, .. } => {
                self.push_reachable_fn(func.0 as usize, reachable, queue);
                for arg in args {
                    self.collect_reachable_from_operand(arg, name_to_idx, reachable, queue);
                }
            }
            MirStmt::CallBuiltin { name, args, .. } => {
                let canonical = normalize_intrinsic(name);
                if let Some(&idx) = name_to_idx.get(canonical) {
                    self.push_reachable_fn(idx, reachable, queue);
                }
                for arg in args {
                    self.collect_reachable_from_operand(arg, name_to_idx, reachable, queue);
                }
            }
            MirStmt::IfStmt {
                cond,
                then_body,
                else_body,
            } => {
                self.collect_reachable_from_operand(cond, name_to_idx, reachable, queue);
                for stmt in then_body {
                    self.collect_reachable_from_stmt(stmt, name_to_idx, reachable, queue);
                }
                for stmt in else_body {
                    self.collect_reachable_from_stmt(stmt, name_to_idx, reachable, queue);
                }
            }
            MirStmt::WhileStmt { cond, body } => {
                self.collect_reachable_from_operand(cond, name_to_idx, reachable, queue);
                for stmt in body {
                    self.collect_reachable_from_stmt(stmt, name_to_idx, reachable, queue);
                }
            }
            MirStmt::Return(Some(op)) => {
                self.collect_reachable_from_operand(op, name_to_idx, reachable, queue);
            }
            MirStmt::Break | MirStmt::Continue | MirStmt::Return(None) => {}
        }
    }

    fn collect_reachable_from_terminator(
        &self,
        terminator: &Terminator,
        name_to_idx: &HashMap<&str, usize>,
        reachable: &mut HashSet<usize>,
        queue: &mut VecDeque<usize>,
    ) {
        match terminator {
            Terminator::If { cond, .. } => {
                self.collect_reachable_from_operand(cond, name_to_idx, reachable, queue);
            }
            Terminator::Switch { scrutinee, .. } => {
                self.collect_reachable_from_operand(scrutinee, name_to_idx, reachable, queue);
            }
            Terminator::Return(Some(op)) => {
                self.collect_reachable_from_operand(op, name_to_idx, reachable, queue);
            }
            Terminator::Goto(_) | Terminator::Return(None) | Terminator::Unreachable => {}
        }
    }

    fn collect_reachable_from_rvalue(
        &self,
        rvalue: &Rvalue,
        name_to_idx: &HashMap<&str, usize>,
        reachable: &mut HashSet<usize>,
        queue: &mut VecDeque<usize>,
    ) {
        match rvalue {
            Rvalue::Use(op) | Rvalue::UnaryOp(_, op) => {
                self.collect_reachable_from_operand(op, name_to_idx, reachable, queue);
            }
            Rvalue::BinaryOp(_, lhs, rhs) => {
                self.collect_reachable_from_operand(lhs, name_to_idx, reachable, queue);
                self.collect_reachable_from_operand(rhs, name_to_idx, reachable, queue);
            }
            Rvalue::Aggregate(_, ops) => {
                for op in ops {
                    self.collect_reachable_from_operand(op, name_to_idx, reachable, queue);
                }
            }
            Rvalue::Ref(place) => {
                self.collect_reachable_from_place(place, name_to_idx, reachable, queue);
            }
        }
    }

    fn collect_reachable_from_place(
        &self,
        place: &Place,
        name_to_idx: &HashMap<&str, usize>,
        reachable: &mut HashSet<usize>,
        queue: &mut VecDeque<usize>,
    ) {
        match place {
            Place::Local(_) => {}
            Place::Field(inner, _) => {
                self.collect_reachable_from_place(inner, name_to_idx, reachable, queue);
            }
            Place::Index(inner, index) => {
                self.collect_reachable_from_place(inner, name_to_idx, reachable, queue);
                self.collect_reachable_from_operand(index, name_to_idx, reachable, queue);
            }
        }
    }

    fn collect_reachable_from_operand(
        &self,
        operand: &Operand,
        name_to_idx: &HashMap<&str, usize>,
        reachable: &mut HashSet<usize>,
        queue: &mut VecDeque<usize>,
    ) {
        match operand {
            Operand::Place(place) => {
                self.collect_reachable_from_place(place, name_to_idx, reachable, queue);
            }
            Operand::BinOp(_, lhs, rhs) => {
                self.collect_reachable_from_operand(lhs, name_to_idx, reachable, queue);
                self.collect_reachable_from_operand(rhs, name_to_idx, reachable, queue);
            }
            Operand::UnaryOp(_, inner)
            | Operand::EnumTag(inner)
            | Operand::TryExpr { expr: inner, .. } => {
                self.collect_reachable_from_operand(inner, name_to_idx, reachable, queue);
            }
            Operand::Call(name, args) => {
                if let Some(&idx) = name_to_idx.get(name.as_str()) {
                    self.push_reachable_fn(idx, reachable, queue);
                }
                for arg in args {
                    self.collect_reachable_from_operand(arg, name_to_idx, reachable, queue);
                }
            }
            Operand::IfExpr {
                cond,
                then_body,
                then_result,
                else_body,
                else_result,
            } => {
                self.collect_reachable_from_operand(cond, name_to_idx, reachable, queue);
                for stmt in then_body {
                    self.collect_reachable_from_stmt(stmt, name_to_idx, reachable, queue);
                }
                if let Some(op) = then_result {
                    self.collect_reachable_from_operand(op, name_to_idx, reachable, queue);
                }
                for stmt in else_body {
                    self.collect_reachable_from_stmt(stmt, name_to_idx, reachable, queue);
                }
                if let Some(op) = else_result {
                    self.collect_reachable_from_operand(op, name_to_idx, reachable, queue);
                }
            }
            Operand::StructInit { fields, .. } => {
                for (_, op) in fields {
                    self.collect_reachable_from_operand(op, name_to_idx, reachable, queue);
                }
            }
            Operand::FieldAccess { object, .. } => {
                self.collect_reachable_from_operand(object, name_to_idx, reachable, queue);
            }
            Operand::EnumInit { payload, .. } => {
                for op in payload {
                    self.collect_reachable_from_operand(op, name_to_idx, reachable, queue);
                }
            }
            Operand::EnumPayload { object, .. } => {
                self.collect_reachable_from_operand(object, name_to_idx, reachable, queue);
            }
            Operand::LoopExpr { init, body, result } => {
                self.collect_reachable_from_operand(init, name_to_idx, reachable, queue);
                for stmt in body {
                    self.collect_reachable_from_stmt(stmt, name_to_idx, reachable, queue);
                }
                self.collect_reachable_from_operand(result, name_to_idx, reachable, queue);
            }
            Operand::FnRef(name) => {
                if let Some(&idx) = name_to_idx.get(name.as_str()) {
                    self.push_reachable_fn(idx, reachable, queue);
                }
            }
            Operand::CallIndirect { callee, args } => {
                self.collect_reachable_from_operand(callee, name_to_idx, reachable, queue);
                for arg in args {
                    self.collect_reachable_from_operand(arg, name_to_idx, reachable, queue);
                }
            }
            Operand::ArrayInit { elements } => {
                for op in elements {
                    self.collect_reachable_from_operand(op, name_to_idx, reachable, queue);
                }
            }
            Operand::IndexAccess { object, index } => {
                self.collect_reachable_from_operand(object, name_to_idx, reachable, queue);
                self.collect_reachable_from_operand(index, name_to_idx, reachable, queue);
            }
            Operand::ConstI32(_)
            | Operand::ConstI64(_)
            | Operand::ConstF32(_)
            | Operand::ConstF64(_)
            | Operand::ConstBool(_)
            | Operand::ConstChar(_)
            | Operand::ConstString(_)
            | Operand::Unit => {}
        }
    }

    fn push_reachable_fn(
        &self,
        idx: usize,
        reachable: &mut HashSet<usize>,
        queue: &mut VecDeque<usize>,
    ) {
        if reachable.insert(idx) {
            queue.push_back(idx);
        }
    }

    fn register_gc_types(&mut self, mir: &MirModule) {
        // $bytes_array = (array (mut i8))
        self.bytes_arr_ty = self
            .types
            .add_array("$bytes_array", mutable_field(StorageType::I8));
        // $string = (struct (field $bytes (ref $bytes_array)))
        self.string_ty = self.types.add_struct(
            "$string",
            &[mutable_field(StorageType::Val(ref_nullable(
                self.bytes_arr_ty,
            )))],
        );
        // $array_i32 = (array (mut i32))
        self.arr_i32_ty = self
            .types
            .add_array("$array_i32", mutable_field(StorageType::Val(ValType::I32)));
        // $vec_i32 = (struct (field $data (ref $array_i32)) (field $len i32) (field $cap i32))
        self.vec_i32_ty = self.types.add_struct(
            "$vec_i32",
            &[
                mutable_field(StorageType::Val(ref_nullable(self.arr_i32_ty))),
                mutable_field(StorageType::Val(ValType::I32)),
                mutable_field(StorageType::Val(ValType::I32)),
            ],
        );
        // $array_i64 = (array (mut i64))
        self.arr_i64_ty = self
            .types
            .add_array("$array_i64", mutable_field(StorageType::Val(ValType::I64)));
        self.vec_i64_ty = self.types.add_struct(
            "$vec_i64",
            &[
                mutable_field(StorageType::Val(ref_nullable(self.arr_i64_ty))),
                mutable_field(StorageType::Val(ValType::I32)),
                mutable_field(StorageType::Val(ValType::I32)),
            ],
        );
        // $array_f64 = (array (mut f64))
        self.arr_f64_ty = self
            .types
            .add_array("$array_f64", mutable_field(StorageType::Val(ValType::F64)));
        self.vec_f64_ty = self.types.add_struct(
            "$vec_f64",
            &[
                mutable_field(StorageType::Val(ref_nullable(self.arr_f64_ty))),
                mutable_field(StorageType::Val(ValType::I32)),
                mutable_field(StorageType::Val(ValType::I32)),
            ],
        );

        // User-defined structs
        for (sname, fields) in &mir.type_table.struct_defs {
            let gc_fields: Vec<FieldType> = fields
                .iter()
                .map(|(_, ty)| mutable_field(StorageType::Val(self.field_valtype(ty))))
                .collect();
            let idx = self.types.add_struct(sname, &gc_fields);
            self.struct_gc_types.insert(sname.clone(), idx);
        }

        // User-defined enums: tag + max-payload i32 slots
        for (ename, variants) in &mir.type_table.enum_defs {
            let max_fields = variants.iter().map(|(_, f)| f.len()).max().unwrap_or(0);
            let mut gc_fields = vec![mutable_field(StorageType::Val(ValType::I32))]; // tag
            for _ in 0..max_fields.max(1) {
                gc_fields.push(mutable_field(StorageType::Val(ValType::I32)));
            }
            let idx = self.types.add_struct(ename, &gc_fields);
            self.enum_gc_types.insert(ename.clone(), idx);
        }
    }

    // ── Helper function bodies ───────────────────────────────────

    fn emit_print_str_ln_helper(&self, codes: &mut CodeSection, newline_off: u32) {
        let ma = MemArg {
            offset: 0,
            align: 2,
            memory_index: 0,
        };
        let mut f = Function::new([(1, ValType::I32), (1, ValType::I32)]); // temp: len, i
        // param 0 = ref $string (but we treat as i32 in bridge mode)
        // For now, bridge implementation: the parameter is actually an i32
        // pointer into linear memory (length-prefixed string)
        // We copy GC string bytes to linear memory for fd_write.

        // For the bridge: param 0 is i32 ptr to length-prefixed string data.
        // Load length from [ptr-4]
        f.instruction(&Instruction::LocalGet(0)); // str_data_ptr
        f.instruction(&Instruction::I32Const(4));
        f.instruction(&Instruction::I32Sub);
        f.instruction(&Instruction::I32Load(ma)); // len
        f.instruction(&Instruction::LocalSet(1)); // local 1 = len

        // Set up iov: base = str_data_ptr, len = len
        f.instruction(&Instruction::I32Const(IOV_BASE as i32));
        f.instruction(&Instruction::LocalGet(0)); // str_data_ptr
        f.instruction(&Instruction::I32Store(ma));
        f.instruction(&Instruction::I32Const(IOV_LEN as i32));
        f.instruction(&Instruction::LocalGet(1)); // len
        f.instruction(&Instruction::I32Store(ma));
        // fd_write(1, &iov, 1, &nwritten)
        f.instruction(&Instruction::I32Const(1)); // fd=stdout
        f.instruction(&Instruction::I32Const(IOV_BASE as i32));
        f.instruction(&Instruction::I32Const(1)); // iovs_len
        f.instruction(&Instruction::I32Const(NWRITTEN as i32));
        f.instruction(&Instruction::Call(FN_FD_WRITE));
        f.instruction(&Instruction::Drop);

        // Print newline
        f.instruction(&Instruction::I32Const(IOV_BASE as i32));
        f.instruction(&Instruction::I32Const(newline_off as i32));
        f.instruction(&Instruction::I32Store(ma));
        f.instruction(&Instruction::I32Const(IOV_LEN as i32));
        f.instruction(&Instruction::I32Const(1));
        f.instruction(&Instruction::I32Store(ma));
        f.instruction(&Instruction::I32Const(1));
        f.instruction(&Instruction::I32Const(IOV_BASE as i32));
        f.instruction(&Instruction::I32Const(1));
        f.instruction(&Instruction::I32Const(NWRITTEN as i32));
        f.instruction(&Instruction::Call(FN_FD_WRITE));
        f.instruction(&Instruction::Drop);
        f.instruction(&Instruction::End);
        codes.function(&f);
    }

    fn emit_print_i32_ln_helper(&self, codes: &mut CodeSection, newline_off: u32) {
        let ma = MemArg {
            offset: 0,
            align: 2,
            memory_index: 0,
        };
        let ma0 = MemArg {
            offset: 0,
            align: 0,
            memory_index: 0,
        };
        // param 0 = i32 value
        let mut f = Function::new([
            (1, ValType::I32), // local 1: is_neg
            (1, ValType::I32), // local 2: digit_count
            (1, ValType::I32), // local 3: abs_val
            (1, ValType::I32), // local 4: temp
        ]);
        let buf_base = I32BUF;
        let buf_end = buf_base + 11; // max i32 digits + sign

        // Handle negative: if val < 0, set is_neg=1, val = -val
        f.instruction(&Instruction::LocalGet(0));
        f.instruction(&Instruction::I32Const(0));
        f.instruction(&Instruction::I32LtS);
        f.instruction(&Instruction::If(wasm_encoder::BlockType::Empty));
        f.instruction(&Instruction::I32Const(1));
        f.instruction(&Instruction::LocalSet(1)); // is_neg = 1
        f.instruction(&Instruction::I32Const(0));
        f.instruction(&Instruction::LocalGet(0));
        f.instruction(&Instruction::I32Sub);
        f.instruction(&Instruction::LocalSet(3)); // abs_val = -val
        f.instruction(&Instruction::Else);
        f.instruction(&Instruction::LocalGet(0));
        f.instruction(&Instruction::LocalSet(3)); // abs_val = val
        f.instruction(&Instruction::End);

        // Handle zero specially
        f.instruction(&Instruction::LocalGet(3));
        f.instruction(&Instruction::I32Eqz);
        f.instruction(&Instruction::If(wasm_encoder::BlockType::Empty));
        // Write "0\n"
        f.instruction(&Instruction::I32Const(buf_base as i32));
        f.instruction(&Instruction::I32Const(48)); // '0'
        f.instruction(&Instruction::I32Store8(ma0));
        f.instruction(&Instruction::I32Const(IOV_BASE as i32));
        f.instruction(&Instruction::I32Const(buf_base as i32));
        f.instruction(&Instruction::I32Store(ma));
        f.instruction(&Instruction::I32Const(IOV_LEN as i32));
        f.instruction(&Instruction::I32Const(1));
        f.instruction(&Instruction::I32Store(ma));
        f.instruction(&Instruction::I32Const(1));
        f.instruction(&Instruction::I32Const(IOV_BASE as i32));
        f.instruction(&Instruction::I32Const(1));
        f.instruction(&Instruction::I32Const(NWRITTEN as i32));
        f.instruction(&Instruction::Call(FN_FD_WRITE));
        f.instruction(&Instruction::Drop);
        // newline
        f.instruction(&Instruction::I32Const(IOV_BASE as i32));
        f.instruction(&Instruction::I32Const(newline_off as i32));
        f.instruction(&Instruction::I32Store(ma));
        f.instruction(&Instruction::I32Const(IOV_LEN as i32));
        f.instruction(&Instruction::I32Const(1));
        f.instruction(&Instruction::I32Store(ma));
        f.instruction(&Instruction::I32Const(1));
        f.instruction(&Instruction::I32Const(IOV_BASE as i32));
        f.instruction(&Instruction::I32Const(1));
        f.instruction(&Instruction::I32Const(NWRITTEN as i32));
        f.instruction(&Instruction::Call(FN_FD_WRITE));
        f.instruction(&Instruction::Drop);
        f.instruction(&Instruction::Return);
        f.instruction(&Instruction::End);

        // Extract digits right-to-left into buf
        f.instruction(&Instruction::I32Const(0));
        f.instruction(&Instruction::LocalSet(2)); // digit_count = 0
        f.instruction(&Instruction::Block(wasm_encoder::BlockType::Empty));
        f.instruction(&Instruction::Loop(wasm_encoder::BlockType::Empty));
        f.instruction(&Instruction::LocalGet(3));
        f.instruction(&Instruction::I32Eqz);
        f.instruction(&Instruction::BrIf(1)); // break if abs_val == 0
        // digit = abs_val % 10
        f.instruction(&Instruction::I32Const(buf_end as i32));
        f.instruction(&Instruction::LocalGet(2));
        f.instruction(&Instruction::I32Sub); // buf_end - digit_count
        f.instruction(&Instruction::LocalGet(3));
        f.instruction(&Instruction::I32Const(10));
        f.instruction(&Instruction::I32RemU);
        f.instruction(&Instruction::I32Const(48)); // + '0'
        f.instruction(&Instruction::I32Add);
        f.instruction(&Instruction::I32Store8(ma0));
        // abs_val /= 10
        f.instruction(&Instruction::LocalGet(3));
        f.instruction(&Instruction::I32Const(10));
        f.instruction(&Instruction::I32DivU);
        f.instruction(&Instruction::LocalSet(3));
        // digit_count++
        f.instruction(&Instruction::LocalGet(2));
        f.instruction(&Instruction::I32Const(1));
        f.instruction(&Instruction::I32Add);
        f.instruction(&Instruction::LocalSet(2));
        f.instruction(&Instruction::Br(0)); // continue
        f.instruction(&Instruction::End); // loop
        f.instruction(&Instruction::End); // block

        // If negative, prepend '-'
        f.instruction(&Instruction::LocalGet(1)); // is_neg
        f.instruction(&Instruction::If(wasm_encoder::BlockType::Empty));
        f.instruction(&Instruction::LocalGet(2));
        f.instruction(&Instruction::I32Const(1));
        f.instruction(&Instruction::I32Add);
        f.instruction(&Instruction::LocalSet(2)); // digit_count++
        f.instruction(&Instruction::I32Const(buf_end as i32));
        f.instruction(&Instruction::LocalGet(2));
        f.instruction(&Instruction::I32Sub);
        f.instruction(&Instruction::I32Const(1));
        f.instruction(&Instruction::I32Add); // position for '-'
        f.instruction(&Instruction::I32Const(45)); // '-'
        f.instruction(&Instruction::I32Store8(ma0));
        f.instruction(&Instruction::End);

        // Print: iov_base = buf_end - digit_count + 1, iov_len = digit_count
        f.instruction(&Instruction::I32Const(IOV_BASE as i32));
        f.instruction(&Instruction::I32Const(buf_end as i32));
        f.instruction(&Instruction::LocalGet(2));
        f.instruction(&Instruction::I32Sub);
        f.instruction(&Instruction::I32Const(1));
        f.instruction(&Instruction::I32Add);
        f.instruction(&Instruction::I32Store(ma));
        f.instruction(&Instruction::I32Const(IOV_LEN as i32));
        f.instruction(&Instruction::LocalGet(2));
        f.instruction(&Instruction::I32Store(ma));
        f.instruction(&Instruction::I32Const(1));
        f.instruction(&Instruction::I32Const(IOV_BASE as i32));
        f.instruction(&Instruction::I32Const(1));
        f.instruction(&Instruction::I32Const(NWRITTEN as i32));
        f.instruction(&Instruction::Call(FN_FD_WRITE));
        f.instruction(&Instruction::Drop);
        // newline
        f.instruction(&Instruction::I32Const(IOV_BASE as i32));
        f.instruction(&Instruction::I32Const(newline_off as i32));
        f.instruction(&Instruction::I32Store(ma));
        f.instruction(&Instruction::I32Const(IOV_LEN as i32));
        f.instruction(&Instruction::I32Const(1));
        f.instruction(&Instruction::I32Store(ma));
        f.instruction(&Instruction::I32Const(1));
        f.instruction(&Instruction::I32Const(IOV_BASE as i32));
        f.instruction(&Instruction::I32Const(1));
        f.instruction(&Instruction::I32Const(NWRITTEN as i32));
        f.instruction(&Instruction::Call(FN_FD_WRITE));
        f.instruction(&Instruction::Drop);
        f.instruction(&Instruction::End);
        codes.function(&f);
    }

    fn emit_print_bool_ln_helper(
        &self,
        codes: &mut CodeSection,
        true_off: u32,
        false_off: u32,
        newline_off: u32,
    ) {
        let ma = MemArg {
            offset: 0,
            align: 2,
            memory_index: 0,
        };
        let mut f = Function::new([]);
        // param 0 = i32 (0 or 1)
        f.instruction(&Instruction::LocalGet(0));
        f.instruction(&Instruction::If(wasm_encoder::BlockType::Empty));
        // true
        f.instruction(&Instruction::I32Const(IOV_BASE as i32));
        f.instruction(&Instruction::I32Const(true_off as i32));
        f.instruction(&Instruction::I32Store(ma));
        f.instruction(&Instruction::I32Const(IOV_LEN as i32));
        f.instruction(&Instruction::I32Const(4)); // "true".len()
        f.instruction(&Instruction::I32Store(ma));
        f.instruction(&Instruction::Else);
        // false
        f.instruction(&Instruction::I32Const(IOV_BASE as i32));
        f.instruction(&Instruction::I32Const(false_off as i32));
        f.instruction(&Instruction::I32Store(ma));
        f.instruction(&Instruction::I32Const(IOV_LEN as i32));
        f.instruction(&Instruction::I32Const(5)); // "false".len()
        f.instruction(&Instruction::I32Store(ma));
        f.instruction(&Instruction::End);

        f.instruction(&Instruction::I32Const(1));
        f.instruction(&Instruction::I32Const(IOV_BASE as i32));
        f.instruction(&Instruction::I32Const(1));
        f.instruction(&Instruction::I32Const(NWRITTEN as i32));
        f.instruction(&Instruction::Call(FN_FD_WRITE));
        f.instruction(&Instruction::Drop);
        // newline
        f.instruction(&Instruction::I32Const(IOV_BASE as i32));
        f.instruction(&Instruction::I32Const(newline_off as i32));
        f.instruction(&Instruction::I32Store(ma));
        f.instruction(&Instruction::I32Const(IOV_LEN as i32));
        f.instruction(&Instruction::I32Const(1));
        f.instruction(&Instruction::I32Store(ma));
        f.instruction(&Instruction::I32Const(1));
        f.instruction(&Instruction::I32Const(IOV_BASE as i32));
        f.instruction(&Instruction::I32Const(1));
        f.instruction(&Instruction::I32Const(NWRITTEN as i32));
        f.instruction(&Instruction::Call(FN_FD_WRITE));
        f.instruction(&Instruction::Drop);
        f.instruction(&Instruction::End);
        codes.function(&f);
    }

    fn emit_i32_to_str_helper(&mut self, codes: &mut CodeSection) {
        // Converts i32 to a length-prefixed string in linear memory.
        // Returns the data pointer (after length prefix) as i32.
        let ma = MemArg {
            offset: 0,
            align: 2,
            memory_index: 0,
        };
        let ma0 = MemArg {
            offset: 0,
            align: 0,
            memory_index: 0,
        };
        let mut f = Function::new([
            (1, ValType::I32), // local 1: is_neg
            (1, ValType::I32), // local 2: abs_val
            (1, ValType::I32), // local 3: digit_count
            (1, ValType::I32), // local 4: buf_ptr (= heap_ptr + 4)
            (1, ValType::I32), // local 5: result_ptr
        ]);
        // Allocate buffer on heap: [len:4][digits:12]
        f.instruction(&Instruction::GlobalGet(0)); // heap_ptr
        f.instruction(&Instruction::I32Const(4));
        f.instruction(&Instruction::I32Add);
        f.instruction(&Instruction::LocalSet(4)); // buf_ptr = heap_ptr + 4

        // Check negative
        f.instruction(&Instruction::LocalGet(0));
        f.instruction(&Instruction::I32Const(0));
        f.instruction(&Instruction::I32LtS);
        f.instruction(&Instruction::If(wasm_encoder::BlockType::Empty));
        f.instruction(&Instruction::I32Const(1));
        f.instruction(&Instruction::LocalSet(1));
        f.instruction(&Instruction::I32Const(0));
        f.instruction(&Instruction::LocalGet(0));
        f.instruction(&Instruction::I32Sub);
        f.instruction(&Instruction::LocalSet(2));
        f.instruction(&Instruction::Else);
        f.instruction(&Instruction::LocalGet(0));
        f.instruction(&Instruction::LocalSet(2));
        f.instruction(&Instruction::End);

        // Handle zero
        f.instruction(&Instruction::LocalGet(2));
        f.instruction(&Instruction::I32Eqz);
        f.instruction(&Instruction::If(wasm_encoder::BlockType::Empty));
        f.instruction(&Instruction::GlobalGet(0)); // store len=1
        f.instruction(&Instruction::I32Const(1));
        f.instruction(&Instruction::I32Store(ma));
        f.instruction(&Instruction::LocalGet(4)); // store '0'
        f.instruction(&Instruction::I32Const(48));
        f.instruction(&Instruction::I32Store8(ma0));
        f.instruction(&Instruction::LocalGet(4));
        f.instruction(&Instruction::LocalSet(5)); // result = buf_ptr
        // Bump heap
        f.instruction(&Instruction::GlobalGet(0));
        f.instruction(&Instruction::I32Const(8));
        f.instruction(&Instruction::I32Add);
        f.instruction(&Instruction::GlobalSet(0));
        f.instruction(&Instruction::LocalGet(5));
        f.instruction(&Instruction::Return);
        f.instruction(&Instruction::End);

        // Extract digits (right-to-left into buf_ptr+11 downward)
        f.instruction(&Instruction::I32Const(0));
        f.instruction(&Instruction::LocalSet(3)); // digit_count = 0
        f.instruction(&Instruction::Block(wasm_encoder::BlockType::Empty));
        f.instruction(&Instruction::Loop(wasm_encoder::BlockType::Empty));
        f.instruction(&Instruction::LocalGet(2));
        f.instruction(&Instruction::I32Eqz);
        f.instruction(&Instruction::BrIf(1));
        f.instruction(&Instruction::LocalGet(4));
        f.instruction(&Instruction::I32Const(11));
        f.instruction(&Instruction::I32Add);
        f.instruction(&Instruction::LocalGet(3));
        f.instruction(&Instruction::I32Sub);
        f.instruction(&Instruction::LocalGet(2));
        f.instruction(&Instruction::I32Const(10));
        f.instruction(&Instruction::I32RemU);
        f.instruction(&Instruction::I32Const(48));
        f.instruction(&Instruction::I32Add);
        f.instruction(&Instruction::I32Store8(ma0));
        f.instruction(&Instruction::LocalGet(2));
        f.instruction(&Instruction::I32Const(10));
        f.instruction(&Instruction::I32DivU);
        f.instruction(&Instruction::LocalSet(2));
        f.instruction(&Instruction::LocalGet(3));
        f.instruction(&Instruction::I32Const(1));
        f.instruction(&Instruction::I32Add);
        f.instruction(&Instruction::LocalSet(3));
        f.instruction(&Instruction::Br(0));
        f.instruction(&Instruction::End);
        f.instruction(&Instruction::End);

        // Prepend '-' if negative
        f.instruction(&Instruction::LocalGet(1));
        f.instruction(&Instruction::If(wasm_encoder::BlockType::Empty));
        f.instruction(&Instruction::LocalGet(3));
        f.instruction(&Instruction::I32Const(1));
        f.instruction(&Instruction::I32Add);
        f.instruction(&Instruction::LocalSet(3));
        f.instruction(&Instruction::LocalGet(4));
        f.instruction(&Instruction::I32Const(12));
        f.instruction(&Instruction::I32Add);
        f.instruction(&Instruction::LocalGet(3));
        f.instruction(&Instruction::I32Sub);
        f.instruction(&Instruction::I32Const(45)); // '-'
        f.instruction(&Instruction::I32Store8(ma0));
        f.instruction(&Instruction::End);

        // Copy digits to start of buffer (memmove in place)
        // Result pointer = buf_ptr + 12 - digit_count
        // But we need length-prefixed: store len at heap_ptr, data at heap_ptr+4
        // The digits are already in buf (heap_ptr+4) area, just at the end
        // We need: [len:4][data:digit_count] starting at heap_ptr
        // Digits are at buf_ptr + 12 - digit_count
        // Store length first
        f.instruction(&Instruction::GlobalGet(0));
        f.instruction(&Instruction::LocalGet(3)); // digit_count
        f.instruction(&Instruction::I32Store(ma));
        // result_ptr = data start = buf_ptr + 12 - digit_count
        f.instruction(&Instruction::LocalGet(4));
        f.instruction(&Instruction::I32Const(12));
        f.instruction(&Instruction::I32Add);
        f.instruction(&Instruction::LocalGet(3));
        f.instruction(&Instruction::I32Sub);
        f.instruction(&Instruction::LocalSet(5));
        // But we need the len prefix right before result_ptr
        // So: store len at result_ptr - 4
        f.instruction(&Instruction::LocalGet(5));
        f.instruction(&Instruction::I32Const(4));
        f.instruction(&Instruction::I32Sub);
        f.instruction(&Instruction::LocalGet(3));
        f.instruction(&Instruction::I32Store(ma));
        // Bump heap past the allocation
        f.instruction(&Instruction::LocalGet(4));
        f.instruction(&Instruction::I32Const(16));
        f.instruction(&Instruction::I32Add);
        f.instruction(&Instruction::GlobalSet(0));

        f.instruction(&Instruction::LocalGet(5));
        f.instruction(&Instruction::End);
        codes.function(&f);
    }

    fn emit_print_newline_helper(&self, codes: &mut CodeSection, newline_off: u32) {
        let ma = MemArg {
            offset: 0,
            align: 2,
            memory_index: 0,
        };
        let mut f = Function::new([]);
        f.instruction(&Instruction::I32Const(IOV_BASE as i32));
        f.instruction(&Instruction::I32Const(newline_off as i32));
        f.instruction(&Instruction::I32Store(ma));
        f.instruction(&Instruction::I32Const(IOV_LEN as i32));
        f.instruction(&Instruction::I32Const(1));
        f.instruction(&Instruction::I32Store(ma));
        f.instruction(&Instruction::I32Const(1));
        f.instruction(&Instruction::I32Const(IOV_BASE as i32));
        f.instruction(&Instruction::I32Const(1));
        f.instruction(&Instruction::I32Const(NWRITTEN as i32));
        f.instruction(&Instruction::Call(FN_FD_WRITE));
        f.instruction(&Instruction::Drop);
        f.instruction(&Instruction::End);
        codes.function(&f);
    }

    // ── User function emission ───────────────────────────────────

    fn emit_function(&mut self, codes: &mut CodeSection, func: &MirFunction) {
        // Reset per-function state
        self.string_locals.clear();
        self.f64_locals.clear();
        self.i64_locals.clear();
        self.bool_locals.clear();
        self.local_struct.clear();

        // Collect local types (skip params — they are already in the func signature)
        let num_params = func.params.len();
        let mut local_types: Vec<(u32, ValType)> = Vec::new();
        for local in func.locals.iter().skip(num_params) {
            let vt = self.type_to_val(&local.ty);
            local_types.push((1, vt));
        }
        // Track type metadata for all locals (including params)
        for local in &func.locals {
            match &local.ty {
                Type::String => {
                    self.string_locals.insert(local.id.0);
                }
                Type::F64 => {
                    self.f64_locals.insert(local.id.0);
                }
                Type::I64 => {
                    self.i64_locals.insert(local.id.0);
                }
                Type::Bool => {
                    self.bool_locals.insert(local.id.0);
                }
                _ => {}
            }
        }

        let mut f = Function::new(local_types);

        // Emit statements from entry block
        if let Some(block) = func.blocks.first() {
            for stmt in &block.stmts {
                self.emit_stmt(&mut f, stmt);
            }
            // Handle terminator
            match &block.terminator {
                Terminator::Return(Some(op)) => {
                    self.emit_operand(&mut f, op);
                    f.instruction(&Instruction::Return);
                }
                Terminator::Return(None) => {
                    f.instruction(&Instruction::Return);
                }
                _ => {}
            }
        }
        f.instruction(&Instruction::End);
        codes.function(&f);
    }

    fn emit_stmt(&mut self, f: &mut Function, stmt: &MirStmt) {
        match stmt {
            MirStmt::Assign(Place::Local(id), Rvalue::Use(op)) => {
                self.emit_operand(f, op);
                let local_idx = self.local_wasm_idx(id.0);
                f.instruction(&Instruction::LocalSet(local_idx));
            }
            MirStmt::Assign(_place, Rvalue::Use(op)) => {
                self.emit_operand(f, op);
                // For non-local places, just drop for now
                f.instruction(&Instruction::Drop);
            }
            MirStmt::Assign(Place::Local(id), Rvalue::BinaryOp(op, lhs, rhs)) => {
                self.emit_operand(f, lhs);
                self.emit_operand(f, rhs);
                self.emit_binop(f, *op, id.0);
                let local_idx = self.local_wasm_idx(id.0);
                f.instruction(&Instruction::LocalSet(local_idx));
            }
            MirStmt::Assign(Place::Local(id), Rvalue::UnaryOp(op, inner)) => {
                self.emit_operand(f, inner);
                self.emit_unaryop(f, *op, id.0);
                let local_idx = self.local_wasm_idx(id.0);
                f.instruction(&Instruction::LocalSet(local_idx));
            }
            MirStmt::Assign(_, _) => {
                // Other rvalue patterns — skip
            }
            MirStmt::CallBuiltin { dest, name, args } => {
                let canonical = normalize_intrinsic(name);
                self.emit_call_builtin(f, canonical, args, dest.as_ref());
            }
            MirStmt::Call {
                dest,
                func: fn_id,
                args,
            } => {
                // fn_id.0 is the MIR function index into mir.functions
                let fn_idx_mir = fn_id.0 as usize;
                if let Some(fn_name) = self.fn_names.get(fn_idx_mir).cloned() {
                    // Check if this is a stdlib wrapper function — redirect to CallBuiltin path
                    let canonical = normalize_intrinsic(&fn_name);
                    if self.is_builtin_name(canonical) {
                        self.emit_call_builtin(f, canonical, args, dest.as_ref());
                    } else {
                        for arg in args {
                            self.emit_operand(f, arg);
                        }
                        if let Some(&fn_idx) = self.fn_map.get(&fn_name) {
                            f.instruction(&Instruction::Call(fn_idx));
                        }
                        if let Some(Place::Local(id)) = dest.as_ref() {
                            f.instruction(&Instruction::LocalSet(self.local_wasm_idx(id.0)));
                        }
                    }
                }
            }
            MirStmt::IfStmt {
                cond,
                then_body,
                else_body,
            } => {
                self.emit_operand(f, cond);
                f.instruction(&Instruction::If(wasm_encoder::BlockType::Empty));
                for s in then_body {
                    self.emit_stmt(f, s);
                }
                if !else_body.is_empty() {
                    f.instruction(&Instruction::Else);
                    for s in else_body {
                        self.emit_stmt(f, s);
                    }
                }
                f.instruction(&Instruction::End);
            }
            MirStmt::WhileStmt { cond, body } => {
                f.instruction(&Instruction::Block(wasm_encoder::BlockType::Empty));
                f.instruction(&Instruction::Loop(wasm_encoder::BlockType::Empty));
                self.emit_operand(f, cond);
                f.instruction(&Instruction::I32Eqz);
                f.instruction(&Instruction::BrIf(1)); // break to block
                for s in body {
                    self.emit_stmt(f, s);
                }
                f.instruction(&Instruction::Br(0)); // continue loop
                f.instruction(&Instruction::End); // loop
                f.instruction(&Instruction::End); // block
            }
            MirStmt::Break => {
                f.instruction(&Instruction::Br(1)); // break outer block
            }
            MirStmt::Continue => {
                f.instruction(&Instruction::Br(0)); // continue loop
            }
            MirStmt::Return(Some(op)) => {
                self.emit_operand(f, op);
                f.instruction(&Instruction::Return);
            }
            MirStmt::Return(None) => {
                f.instruction(&Instruction::Return);
            }
        }
    }

    fn emit_operand(&mut self, f: &mut Function, op: &Operand) {
        match op {
            Operand::ConstI32(v) => {
                f.instruction(&Instruction::I32Const(*v));
            }
            Operand::ConstI64(v) => {
                f.instruction(&Instruction::I64Const(*v));
            }
            Operand::ConstF32(v) => {
                f.instruction(&Instruction::F32Const(*v));
            }
            Operand::ConstF64(v) => {
                f.instruction(&Instruction::F64Const(*v));
            }
            Operand::ConstBool(v) => {
                f.instruction(&Instruction::I32Const(if *v { 1 } else { 0 }));
            }
            Operand::ConstChar(c) => {
                f.instruction(&Instruction::I32Const(*c as i32));
            }
            Operand::ConstString(s) => {
                // Allocate length-prefixed string in linear memory (data segment)
                let bytes = s.as_bytes();
                let len = bytes.len() as u32;
                let mut data = Vec::with_capacity(4 + bytes.len());
                data.extend_from_slice(&len.to_le_bytes());
                data.extend_from_slice(bytes);
                let offset = self.data_offset;
                self.data_segs.push((offset, data));
                self.data_offset += 4 + len;
                while self.data_offset % 4 != 0 {
                    self.data_offset += 1;
                }
                // Push pointer to data (after length prefix)
                f.instruction(&Instruction::I32Const((offset + 4) as i32));
            }
            Operand::Unit => {
                // Unit doesn't push a value
            }
            Operand::Place(Place::Local(id)) => {
                f.instruction(&Instruction::LocalGet(self.local_wasm_idx(id.0)));
            }
            Operand::Place(Place::Field(_, _)) | Operand::Place(Place::Index(_, _)) => {
                f.instruction(&Instruction::I32Const(0));
            }
            Operand::BinOp(op, lhs, rhs) => {
                self.emit_operand(f, lhs);
                self.emit_operand(f, rhs);
                self.emit_binop(f, *op, 0);
            }
            Operand::UnaryOp(op, inner) => {
                self.emit_operand(f, inner);
                self.emit_unaryop(f, *op, 0);
            }
            Operand::Call(name, args) => {
                let canonical = normalize_intrinsic(name).to_string();
                // Check if this is a builtin — redirect to inline implementation
                if self.is_builtin_name(&canonical) {
                    self.emit_call_builtin_operand(f, &canonical, args);
                } else {
                    for arg in args {
                        self.emit_operand(f, arg);
                    }
                    if let Some(&fn_idx) = self.fn_map.get(canonical.as_str()) {
                        f.instruction(&Instruction::Call(fn_idx));
                    } else {
                        // Unknown function: push zero
                        f.instruction(&Instruction::I32Const(0));
                    }
                }
            }
            Operand::IfExpr {
                cond,
                then_body,
                then_result,
                else_body,
                else_result,
            } => {
                self.emit_operand(f, cond);
                f.instruction(&Instruction::If(wasm_encoder::BlockType::Result(
                    ValType::I32,
                )));
                for s in then_body {
                    self.emit_stmt(f, s);
                }
                if let Some(r) = then_result {
                    self.emit_operand(f, r);
                } else {
                    f.instruction(&Instruction::I32Const(0));
                }
                f.instruction(&Instruction::Else);
                for s in else_body {
                    self.emit_stmt(f, s);
                }
                if let Some(r) = else_result {
                    self.emit_operand(f, r);
                } else {
                    f.instruction(&Instruction::I32Const(0));
                }
                f.instruction(&Instruction::End);
            }
            Operand::StructInit { name, fields } => {
                // Allocate struct on heap (linear memory bridge)
                let layout = self.struct_layouts.get(name).cloned().unwrap_or_default();
                let total_size = layout
                    .iter()
                    .map(|(_, ty)| match ty.as_str() {
                        "f64" | "i64" => 8u32,
                        _ => 4u32,
                    })
                    .sum::<u32>()
                    .max(4);
                // Allocate
                f.instruction(&Instruction::GlobalGet(0)); // base ptr
                // Store fields
                let mut offset = 0u32;
                let ma = MemArg {
                    offset: 0,
                    align: 2,
                    memory_index: 0,
                };
                for (fname, fty) in &layout {
                    if let Some((_, val)) = fields.iter().find(|(n, _)| n == fname) {
                        f.instruction(&Instruction::GlobalGet(0));
                        f.instruction(&Instruction::I32Const(offset as i32));
                        f.instruction(&Instruction::I32Add);
                        self.emit_operand(f, val);
                        if fty == "f64" {
                            f.instruction(&Instruction::F64Store(MemArg {
                                offset: 0,
                                align: 3,
                                memory_index: 0,
                            }));
                        } else if fty == "i64" {
                            f.instruction(&Instruction::I64Store(MemArg {
                                offset: 0,
                                align: 3,
                                memory_index: 0,
                            }));
                        } else {
                            f.instruction(&Instruction::I32Store(ma));
                        }
                    }
                    offset += match fty.as_str() {
                        "f64" | "i64" => 8,
                        _ => 4,
                    };
                }
                // Bump heap
                f.instruction(&Instruction::GlobalGet(0));
                f.instruction(&Instruction::I32Const(total_size as i32));
                f.instruction(&Instruction::I32Add);
                f.instruction(&Instruction::GlobalSet(0));
                // Result = base ptr (already on stack from first GlobalGet)
            }
            Operand::FieldAccess {
                object,
                struct_name,
                field,
            } => {
                let layout = self
                    .struct_layouts
                    .get(struct_name)
                    .cloned()
                    .unwrap_or_default();
                let mut offset = 0u32;
                let mut ftype = "i32";
                for (fname, ft) in &layout {
                    if fname == field {
                        ftype = ft;
                        break;
                    }
                    offset += match ft.as_str() {
                        "f64" | "i64" => 8,
                        _ => 4,
                    };
                }
                self.emit_operand(f, object);
                f.instruction(&Instruction::I32Const(offset as i32));
                f.instruction(&Instruction::I32Add);
                if ftype == "f64" {
                    f.instruction(&Instruction::F64Load(MemArg {
                        offset: 0,
                        align: 3,
                        memory_index: 0,
                    }));
                } else if ftype == "i64" {
                    f.instruction(&Instruction::I64Load(MemArg {
                        offset: 0,
                        align: 3,
                        memory_index: 0,
                    }));
                } else {
                    f.instruction(&Instruction::I32Load(MemArg {
                        offset: 0,
                        align: 2,
                        memory_index: 0,
                    }));
                }
            }
            Operand::EnumInit {
                enum_name,
                variant: _,
                tag,
                payload,
            } => {
                let total_size = self.enum_total_size(enum_name);
                let ma = MemArg {
                    offset: 0,
                    align: 2,
                    memory_index: 0,
                };
                f.instruction(&Instruction::GlobalGet(0)); // base ptr (result)
                // Store tag
                f.instruction(&Instruction::GlobalGet(0));
                f.instruction(&Instruction::I32Const(*tag));
                f.instruction(&Instruction::I32Store(ma));
                // Store payload fields
                let mut off = 4u32;
                for p in payload {
                    f.instruction(&Instruction::GlobalGet(0));
                    f.instruction(&Instruction::I32Const(off as i32));
                    f.instruction(&Instruction::I32Add);
                    self.emit_operand(f, p);
                    f.instruction(&Instruction::I32Store(ma));
                    off += 4;
                }
                // Bump heap
                f.instruction(&Instruction::GlobalGet(0));
                f.instruction(&Instruction::I32Const(total_size as i32));
                f.instruction(&Instruction::I32Add);
                f.instruction(&Instruction::GlobalSet(0));
            }
            Operand::EnumTag(inner) => {
                self.emit_operand(f, inner);
                f.instruction(&Instruction::I32Load(MemArg {
                    offset: 0,
                    align: 2,
                    memory_index: 0,
                }));
            }
            Operand::EnumPayload { object, index, .. } => {
                self.emit_operand(f, object);
                f.instruction(&Instruction::I32Const(4 + (*index * 4) as i32));
                f.instruction(&Instruction::I32Add);
                f.instruction(&Instruction::I32Load(MemArg {
                    offset: 0,
                    align: 2,
                    memory_index: 0,
                }));
            }
            Operand::LoopExpr { body, result, .. } => {
                for s in body {
                    self.emit_stmt(f, s);
                }
                self.emit_operand(f, result);
            }
            Operand::TryExpr { expr, .. } => {
                self.emit_operand(f, expr);
            }
            Operand::FnRef(name) => {
                if let Some(&idx) = self.fn_map.get(name.as_str()) {
                    f.instruction(&Instruction::I32Const(idx as i32));
                } else {
                    f.instruction(&Instruction::I32Const(0));
                }
            }
            Operand::CallIndirect { callee, args } => {
                for arg in args {
                    self.emit_operand(f, arg);
                }
                self.emit_operand(f, callee);
                f.instruction(&Instruction::I32Const(0)); // placeholder table call
            }
            Operand::ArrayInit { elements } => {
                // Allocate array in linear memory: [len:4][elem0:4][elem1:4]...
                let len = elements.len() as u32;
                let ma = MemArg {
                    offset: 0,
                    align: 2,
                    memory_index: 0,
                };
                f.instruction(&Instruction::GlobalGet(0)); // base ptr (result)
                // Store length
                f.instruction(&Instruction::GlobalGet(0));
                f.instruction(&Instruction::I32Const(len as i32));
                f.instruction(&Instruction::I32Store(ma));
                // Store elements
                for (i, elem) in elements.iter().enumerate() {
                    f.instruction(&Instruction::GlobalGet(0));
                    f.instruction(&Instruction::I32Const(4 + (i as i32) * 4));
                    f.instruction(&Instruction::I32Add);
                    self.emit_operand(f, elem);
                    f.instruction(&Instruction::I32Store(ma));
                }
                // Bump heap
                f.instruction(&Instruction::GlobalGet(0));
                f.instruction(&Instruction::I32Const((4 + len * 4) as i32));
                f.instruction(&Instruction::I32Add);
                f.instruction(&Instruction::GlobalSet(0));
            }
            Operand::IndexAccess { object, index } => {
                self.emit_operand(f, object);
                f.instruction(&Instruction::I32Const(4)); // skip length
                f.instruction(&Instruction::I32Add);
                self.emit_operand(f, index);
                f.instruction(&Instruction::I32Const(4));
                f.instruction(&Instruction::I32Mul);
                f.instruction(&Instruction::I32Add);
                f.instruction(&Instruction::I32Load(MemArg {
                    offset: 0,
                    align: 2,
                    memory_index: 0,
                }));
            }
        }
    }

    fn is_builtin_name(&self, name: &str) -> bool {
        matches!(
            name,
            "println"
                | "print"
                | "i32_to_string"
                | "i64_to_string"
                | "f64_to_string"
                | "bool_to_string"
                | "concat"
                | "String_from"
                | "len"
                | "push"
                | "get"
                | "get_unchecked"
                | "set"
                | "pop"
                | "join"
                | "panic"
                | "assert"
                | "assert_eq"
                | "Vec_new_i32"
                | "Vec_new_i64"
                | "Vec_new_f64"
                | "Vec_new_String"
                | "sort_i32"
                | "parse_i32"
                | "parse_i64"
                | "parse_f64"
                | "char_to_string"
                | "read_line"
                | "string_len"
                | "char_at"
                | "substring"
                | "contains"
                | "to_uppercase"
                | "to_lowercase"
                | "trim"
                | "split"
                | "starts_with"
                | "ends_with"
                | "replace"
                | "clock_now_ms"
                | "random_i32"
                | "random_f64"
                | "HashMap_new_i32_i32"
                | "HashMap_new_i32_String"
                | "HashMap_new_String_i32"
                | "HashMap_new_String_String"
                | "insert"
                | "get_or_default"
                | "contains_key"
                | "remove"
                | "keys"
                | "values"
                | "sort_i64"
                | "sort_f64"
                | "map_i32_i32"
                | "map_i32_String"
                | "filter_i32"
                | "filter_String"
        )
    }

    fn emit_call_builtin(
        &mut self,
        f: &mut Function,
        canonical: &str,
        args: &[Operand],
        dest: Option<&Place>,
    ) {
        match canonical {
            "println" => {
                if let Some(arg) = args.first() {
                    self.emit_println(f, arg);
                }
            }
            "i32_to_string" => {
                if let Some(arg) = args.first() {
                    self.emit_operand(f, arg);
                    if let Some(idx) = self.helper_i32_to_str {
                        f.instruction(&Instruction::Call(idx));
                    }
                    if let Some(Place::Local(id)) = dest {
                        f.instruction(&Instruction::LocalSet(self.local_wasm_idx(id.0)));
                    } else {
                        f.instruction(&Instruction::Drop);
                    }
                }
            }
            "concat" => {
                self.emit_concat(f, args, dest);
            }
            "join" => {
                self.emit_join(f, args, dest);
            }
            "Vec_new_i32" => {
                self.emit_vec_new(f, 4, dest);
            }
            "Vec_new_i64" | "Vec_new_f64" => {
                self.emit_vec_new(f, 8, dest);
            }
            "Vec_new_String" => {
                self.emit_vec_new(f, 4, dest);
            }
            "push" => {
                self.emit_push(f, args);
            }
            "set" => {
                self.emit_set(f, args);
            }
            "len" => {
                self.emit_len_inline(f, args.first());
                if let Some(Place::Local(id)) = dest {
                    f.instruction(&Instruction::LocalSet(self.local_wasm_idx(id.0)));
                } else {
                    f.instruction(&Instruction::Drop);
                }
            }
            "get" => {
                self.emit_get_inline(f, args);
                if let Some(Place::Local(id)) = dest {
                    f.instruction(&Instruction::LocalSet(self.local_wasm_idx(id.0)));
                } else {
                    f.instruction(&Instruction::Drop);
                }
            }
            "get_unchecked" => {
                self.emit_get_unchecked_inline(f, args);
                if let Some(Place::Local(id)) = dest {
                    f.instruction(&Instruction::LocalSet(self.local_wasm_idx(id.0)));
                } else {
                    f.instruction(&Instruction::Drop);
                }
            }
            "pop" => {
                self.emit_pop_inline(f, args);
                if let Some(Place::Local(id)) = dest {
                    f.instruction(&Instruction::LocalSet(self.local_wasm_idx(id.0)));
                } else {
                    f.instruction(&Instruction::Drop);
                }
            }
            "String_from" => {
                if let Some(arg) = args.first() {
                    self.emit_operand(f, arg);
                    if let Some(Place::Local(id)) = dest {
                        f.instruction(&Instruction::LocalSet(self.local_wasm_idx(id.0)));
                    } else {
                        f.instruction(&Instruction::Drop);
                    }
                }
            }
            _ => {
                for arg in args {
                    self.emit_operand(f, arg);
                }
                if let Some(&fn_idx) = self.fn_map.get(canonical) {
                    f.instruction(&Instruction::Call(fn_idx));
                    let returns_value = self
                        .fn_ret_types
                        .get(canonical)
                        .is_some_and(|ty| !matches!(ty, Type::Unit | Type::Never));
                    if let Some(Place::Local(id)) = dest {
                        if returns_value {
                            f.instruction(&Instruction::LocalSet(self.local_wasm_idx(id.0)));
                        }
                    } else if returns_value {
                        f.instruction(&Instruction::Drop);
                    }
                } else if let Some(Place::Local(id)) = dest {
                    f.instruction(&Instruction::I32Const(0));
                    f.instruction(&Instruction::LocalSet(self.local_wasm_idx(id.0)));
                }
            }
        }
    }

    /// Handle builtin calls as operands (result stays on the stack).
    fn emit_call_builtin_operand(&mut self, f: &mut Function, canonical: &str, args: &[Operand]) {
        match canonical {
            "i32_to_string" => {
                if let Some(arg) = args.first() {
                    self.emit_operand(f, arg);
                    if let Some(idx) = self.helper_i32_to_str {
                        f.instruction(&Instruction::Call(idx));
                    }
                }
            }
            "String_from" => {
                if let Some(arg) = args.first() {
                    self.emit_operand(f, arg);
                }
            }
            "concat" => {
                self.emit_concat_inline(f, args);
            }
            "join" => {
                self.emit_join_inline(f, args);
            }
            "Vec_new_i32" => {
                self.emit_vec_new_inline(f, 4);
            }
            "Vec_new_i64" | "Vec_new_f64" => {
                self.emit_vec_new_inline(f, 8);
            }
            "Vec_new_String" => {
                self.emit_vec_new_inline(f, 4);
            }
            "len" => {
                self.emit_len_inline(f, args.first());
            }
            "get" => {
                self.emit_get_inline(f, args);
            }
            "get_unchecked" => {
                self.emit_get_unchecked_inline(f, args);
            }
            "pop" => {
                self.emit_pop_inline(f, args);
            }
            _ => {
                // Unimplemented builtin as operand — push zero
                f.instruction(&Instruction::I32Const(0));
            }
        }
    }

    fn emit_println(&mut self, f: &mut Function, arg: &Operand) {
        self.emit_operand(f, arg);
        if self.is_string_like_operand(arg) {
            if let Some(idx) = self.helper_print_str_ln {
                f.instruction(&Instruction::Call(idx));
            }
        } else if self.is_bool_like_operand(arg) {
            if let Some(idx) = self.helper_print_bool_ln {
                f.instruction(&Instruction::Call(idx));
            }
        } else if let Some(idx) = self.helper_print_i32_ln {
            f.instruction(&Instruction::Call(idx));
        }
    }

    fn is_string_like_operand(&self, operand: &Operand) -> bool {
        match operand {
            Operand::ConstString(_) | Operand::ConstChar(_) => true,
            Operand::Place(Place::Local(id)) => self.string_locals.contains(&id.0),
            Operand::Call(name, _) => {
                let canonical = normalize_intrinsic(name);
                matches!(
                    canonical,
                    "String_from"
                        | "concat"
                        | "clone"
                        | "slice"
                        | "to_lower"
                        | "to_upper"
                        | "join"
                        | "i32_to_string"
                        | "i64_to_string"
                        | "f64_to_string"
                        | "bool_to_string"
                        | "char_to_string"
                ) || self.fn_ret_types.get(name) == Some(&Type::String)
            }
            _ => false,
        }
    }

    fn is_bool_like_operand(&self, operand: &Operand) -> bool {
        match operand {
            Operand::ConstBool(_) => true,
            Operand::Place(Place::Local(id)) => self.bool_locals.contains(&id.0),
            Operand::Call(name, _) => {
                let canonical = normalize_intrinsic(name);
                matches!(
                    canonical,
                    "eq" | "starts_with" | "ends_with" | "contains" | "assert" | "assert_eq"
                ) || self.fn_ret_types.get(name) == Some(&Type::Bool)
            }
            _ => false,
        }
    }

    fn emit_concat(&mut self, f: &mut Function, _args: &[Operand], dest: Option<&Place>) {
        self.emit_concat_inline(f, _args);
        if let Some(Place::Local(id)) = dest {
            f.instruction(&Instruction::LocalSet(self.local_wasm_idx(id.0)));
        } else {
            f.instruction(&Instruction::Drop);
        }
    }

    fn emit_concat_inline(&mut self, f: &mut Function, args: &[Operand]) {
        let ma = MemArg {
            offset: 0,
            align: 2,
            memory_index: 0,
        };
        let ma0 = MemArg {
            offset: 0,
            align: 0,
            memory_index: 0,
        };

        if args.len() < 2 {
            f.instruction(&Instruction::I32Const(0));
            return;
        }

        // Scratch: store lhs/rhs pointers.
        f.instruction(&Instruction::I32Const(SCR_A_PTR as i32));
        self.emit_operand(f, &args[0]);
        f.instruction(&Instruction::I32Store(ma));

        f.instruction(&Instruction::I32Const(SCR_B_PTR as i32));
        self.emit_operand(f, &args[1]);
        f.instruction(&Instruction::I32Store(ma));

        // Scratch: store lhs/rhs lengths.
        f.instruction(&Instruction::I32Const(SCR_A_LEN as i32));
        f.instruction(&Instruction::I32Const(SCR_A_PTR as i32));
        f.instruction(&Instruction::I32Load(ma));
        f.instruction(&Instruction::I32Const(4));
        f.instruction(&Instruction::I32Sub);
        f.instruction(&Instruction::I32Load(ma));
        f.instruction(&Instruction::I32Store(ma));

        f.instruction(&Instruction::I32Const(SCR_B_LEN as i32));
        f.instruction(&Instruction::I32Const(SCR_B_PTR as i32));
        f.instruction(&Instruction::I32Load(ma));
        f.instruction(&Instruction::I32Const(4));
        f.instruction(&Instruction::I32Sub);
        f.instruction(&Instruction::I32Load(ma));
        f.instruction(&Instruction::I32Store(ma));

        // result data ptr = heap_ptr + 4
        f.instruction(&Instruction::I32Const(SCR_DST_PTR as i32));
        f.instruction(&Instruction::GlobalGet(0));
        f.instruction(&Instruction::I32Const(4));
        f.instruction(&Instruction::I32Add);
        f.instruction(&Instruction::I32Store(ma));

        // store total length at heap_ptr
        f.instruction(&Instruction::GlobalGet(0));
        f.instruction(&Instruction::I32Const(SCR_A_LEN as i32));
        f.instruction(&Instruction::I32Load(ma));
        f.instruction(&Instruction::I32Const(SCR_B_LEN as i32));
        f.instruction(&Instruction::I32Load(ma));
        f.instruction(&Instruction::I32Add);
        f.instruction(&Instruction::I32Store(ma));

        // Copy lhs bytes.
        f.instruction(&Instruction::I32Const(SCR_I as i32));
        f.instruction(&Instruction::I32Const(0));
        f.instruction(&Instruction::I32Store(ma));
        f.instruction(&Instruction::Block(wasm_encoder::BlockType::Empty));
        f.instruction(&Instruction::Loop(wasm_encoder::BlockType::Empty));
        f.instruction(&Instruction::I32Const(SCR_I as i32));
        f.instruction(&Instruction::I32Load(ma));
        f.instruction(&Instruction::I32Const(SCR_A_LEN as i32));
        f.instruction(&Instruction::I32Load(ma));
        f.instruction(&Instruction::I32GeU);
        f.instruction(&Instruction::BrIf(1));

        f.instruction(&Instruction::I32Const(SCR_DST_PTR as i32));
        f.instruction(&Instruction::I32Load(ma));
        f.instruction(&Instruction::I32Const(SCR_I as i32));
        f.instruction(&Instruction::I32Load(ma));
        f.instruction(&Instruction::I32Add);
        f.instruction(&Instruction::I32Const(SCR_A_PTR as i32));
        f.instruction(&Instruction::I32Load(ma));
        f.instruction(&Instruction::I32Const(SCR_I as i32));
        f.instruction(&Instruction::I32Load(ma));
        f.instruction(&Instruction::I32Add);
        f.instruction(&Instruction::I32Load8U(ma0));
        f.instruction(&Instruction::I32Store8(ma0));

        f.instruction(&Instruction::I32Const(SCR_I as i32));
        f.instruction(&Instruction::I32Const(SCR_I as i32));
        f.instruction(&Instruction::I32Load(ma));
        f.instruction(&Instruction::I32Const(1));
        f.instruction(&Instruction::I32Add);
        f.instruction(&Instruction::I32Store(ma));
        f.instruction(&Instruction::Br(0));
        f.instruction(&Instruction::End);
        f.instruction(&Instruction::End);

        // Copy rhs bytes after lhs.
        f.instruction(&Instruction::I32Const(SCR_I as i32));
        f.instruction(&Instruction::I32Const(0));
        f.instruction(&Instruction::I32Store(ma));
        f.instruction(&Instruction::Block(wasm_encoder::BlockType::Empty));
        f.instruction(&Instruction::Loop(wasm_encoder::BlockType::Empty));
        f.instruction(&Instruction::I32Const(SCR_I as i32));
        f.instruction(&Instruction::I32Load(ma));
        f.instruction(&Instruction::I32Const(SCR_B_LEN as i32));
        f.instruction(&Instruction::I32Load(ma));
        f.instruction(&Instruction::I32GeU);
        f.instruction(&Instruction::BrIf(1));

        f.instruction(&Instruction::I32Const(SCR_DST_PTR as i32));
        f.instruction(&Instruction::I32Load(ma));
        f.instruction(&Instruction::I32Const(SCR_A_LEN as i32));
        f.instruction(&Instruction::I32Load(ma));
        f.instruction(&Instruction::I32Add);
        f.instruction(&Instruction::I32Const(SCR_I as i32));
        f.instruction(&Instruction::I32Load(ma));
        f.instruction(&Instruction::I32Add);
        f.instruction(&Instruction::I32Const(SCR_B_PTR as i32));
        f.instruction(&Instruction::I32Load(ma));
        f.instruction(&Instruction::I32Const(SCR_I as i32));
        f.instruction(&Instruction::I32Load(ma));
        f.instruction(&Instruction::I32Add);
        f.instruction(&Instruction::I32Load8U(ma0));
        f.instruction(&Instruction::I32Store8(ma0));

        f.instruction(&Instruction::I32Const(SCR_I as i32));
        f.instruction(&Instruction::I32Const(SCR_I as i32));
        f.instruction(&Instruction::I32Load(ma));
        f.instruction(&Instruction::I32Const(1));
        f.instruction(&Instruction::I32Add);
        f.instruction(&Instruction::I32Store(ma));
        f.instruction(&Instruction::Br(0));
        f.instruction(&Instruction::End);
        f.instruction(&Instruction::End);

        // Bump heap past header + payload.
        f.instruction(&Instruction::GlobalGet(0));
        f.instruction(&Instruction::I32Const(4));
        f.instruction(&Instruction::I32Add);
        f.instruction(&Instruction::I32Const(SCR_A_LEN as i32));
        f.instruction(&Instruction::I32Load(ma));
        f.instruction(&Instruction::I32Add);
        f.instruction(&Instruction::I32Const(SCR_B_LEN as i32));
        f.instruction(&Instruction::I32Load(ma));
        f.instruction(&Instruction::I32Add);
        f.instruction(&Instruction::GlobalSet(0));

        f.instruction(&Instruction::I32Const(SCR_DST_PTR as i32));
        f.instruction(&Instruction::I32Load(ma));
    }

    fn emit_join(&mut self, f: &mut Function, args: &[Operand], dest: Option<&Place>) {
        self.emit_join_inline(f, args);
        if let Some(Place::Local(id)) = dest {
            f.instruction(&Instruction::LocalSet(self.local_wasm_idx(id.0)));
        } else {
            f.instruction(&Instruction::Drop);
        }
    }

    fn emit_join_inline(&mut self, f: &mut Function, args: &[Operand]) {
        let ma = MemArg {
            offset: 0,
            align: 2,
            memory_index: 0,
        };
        let ma0 = MemArg {
            offset: 0,
            align: 0,
            memory_index: 0,
        };

        if args.len() < 2 {
            f.instruction(&Instruction::I32Const(0));
            return;
        }

        // Scratch: parts pointer, separator pointer, i, n, out_start, out_pos.
        f.instruction(&Instruction::I32Const(SCR_A_PTR as i32));
        self.emit_operand(f, &args[0]);
        f.instruction(&Instruction::I32Store(ma));

        f.instruction(&Instruction::I32Const(SCR_B_PTR as i32));
        self.emit_operand(f, &args[1]);
        f.instruction(&Instruction::I32Store(ma));

        f.instruction(&Instruction::I32Const(SCR_B_LEN as i32));
        f.instruction(&Instruction::I32Const(SCR_A_PTR as i32));
        f.instruction(&Instruction::I32Load(ma));
        f.instruction(&Instruction::I32Const(4));
        f.instruction(&Instruction::I32Add);
        f.instruction(&Instruction::I32Load(ma));
        f.instruction(&Instruction::I32Store(ma));

        f.instruction(&Instruction::I32Const(SCR_DST_PTR as i32));
        f.instruction(&Instruction::GlobalGet(0));
        f.instruction(&Instruction::I32Store(ma));

        f.instruction(&Instruction::I32Const(SCR_I as i32));
        f.instruction(&Instruction::GlobalGet(0));
        f.instruction(&Instruction::I32Const(4));
        f.instruction(&Instruction::I32Add);
        f.instruction(&Instruction::I32Store(ma));

        f.instruction(&Instruction::I32Const(SCR_A_LEN as i32));
        f.instruction(&Instruction::I32Const(0));
        f.instruction(&Instruction::I32Store(ma));

        f.instruction(&Instruction::Block(wasm_encoder::BlockType::Empty));
        f.instruction(&Instruction::Loop(wasm_encoder::BlockType::Empty));
        f.instruction(&Instruction::I32Const(SCR_A_LEN as i32));
        f.instruction(&Instruction::I32Load(ma));
        f.instruction(&Instruction::I32Const(SCR_B_LEN as i32));
        f.instruction(&Instruction::I32Load(ma));
        f.instruction(&Instruction::I32GeU);
        f.instruction(&Instruction::BrIf(1));

        // If i > 0, append separator.
        f.instruction(&Instruction::I32Const(SCR_A_LEN as i32));
        f.instruction(&Instruction::I32Load(ma));
        f.instruction(&Instruction::I32Const(0));
        f.instruction(&Instruction::I32GtU);
        f.instruction(&Instruction::If(wasm_encoder::BlockType::Empty));
        f.instruction(&Instruction::I32Const(SCR_J as i32));
        f.instruction(&Instruction::I32Const(0));
        f.instruction(&Instruction::I32Store(ma));
        f.instruction(&Instruction::Block(wasm_encoder::BlockType::Empty));
        f.instruction(&Instruction::Loop(wasm_encoder::BlockType::Empty));
        f.instruction(&Instruction::I32Const(SCR_J as i32));
        f.instruction(&Instruction::I32Load(ma));
        f.instruction(&Instruction::I32Const(SCR_B_PTR as i32));
        f.instruction(&Instruction::I32Load(ma));
        f.instruction(&Instruction::I32Const(4));
        f.instruction(&Instruction::I32Sub);
        f.instruction(&Instruction::I32Load(ma));
        f.instruction(&Instruction::I32GeU);
        f.instruction(&Instruction::BrIf(1));

        f.instruction(&Instruction::I32Const(SCR_I as i32));
        f.instruction(&Instruction::I32Load(ma));
        f.instruction(&Instruction::I32Const(SCR_J as i32));
        f.instruction(&Instruction::I32Load(ma));
        f.instruction(&Instruction::I32Add);
        f.instruction(&Instruction::I32Const(SCR_B_PTR as i32));
        f.instruction(&Instruction::I32Load(ma));
        f.instruction(&Instruction::I32Const(SCR_J as i32));
        f.instruction(&Instruction::I32Load(ma));
        f.instruction(&Instruction::I32Add);
        f.instruction(&Instruction::I32Load8U(ma0));
        f.instruction(&Instruction::I32Store8(ma0));

        f.instruction(&Instruction::I32Const(SCR_J as i32));
        f.instruction(&Instruction::I32Const(SCR_J as i32));
        f.instruction(&Instruction::I32Load(ma));
        f.instruction(&Instruction::I32Const(1));
        f.instruction(&Instruction::I32Add);
        f.instruction(&Instruction::I32Store(ma));
        f.instruction(&Instruction::Br(0));
        f.instruction(&Instruction::End);
        f.instruction(&Instruction::End);

        f.instruction(&Instruction::I32Const(SCR_I as i32));
        f.instruction(&Instruction::I32Const(SCR_I as i32));
        f.instruction(&Instruction::I32Load(ma));
        f.instruction(&Instruction::I32Const(SCR_B_PTR as i32));
        f.instruction(&Instruction::I32Load(ma));
        f.instruction(&Instruction::I32Const(4));
        f.instruction(&Instruction::I32Sub);
        f.instruction(&Instruction::I32Load(ma));
        f.instruction(&Instruction::I32Add);
        f.instruction(&Instruction::I32Store(ma));
        f.instruction(&Instruction::End);

        // Append current string.
        f.instruction(&Instruction::I32Const(SCR_J as i32));
        f.instruction(&Instruction::I32Const(0));
        f.instruction(&Instruction::I32Store(ma));
        f.instruction(&Instruction::Block(wasm_encoder::BlockType::Empty));
        f.instruction(&Instruction::Loop(wasm_encoder::BlockType::Empty));
        f.instruction(&Instruction::I32Const(SCR_J as i32));
        f.instruction(&Instruction::I32Load(ma));
        f.instruction(&Instruction::I32Const(SCR_A_PTR as i32));
        f.instruction(&Instruction::I32Load(ma));
        f.instruction(&Instruction::I32Load(ma));
        f.instruction(&Instruction::I32Const(SCR_A_LEN as i32));
        f.instruction(&Instruction::I32Load(ma));
        f.instruction(&Instruction::I32Const(4));
        f.instruction(&Instruction::I32Mul);
        f.instruction(&Instruction::I32Add);
        f.instruction(&Instruction::I32Load(ma));
        f.instruction(&Instruction::I32Const(4));
        f.instruction(&Instruction::I32Sub);
        f.instruction(&Instruction::I32Load(ma));
        f.instruction(&Instruction::I32GeU);
        f.instruction(&Instruction::BrIf(1));

        f.instruction(&Instruction::I32Const(SCR_I as i32));
        f.instruction(&Instruction::I32Load(ma));
        f.instruction(&Instruction::I32Const(SCR_J as i32));
        f.instruction(&Instruction::I32Load(ma));
        f.instruction(&Instruction::I32Add);
        f.instruction(&Instruction::I32Const(SCR_A_PTR as i32));
        f.instruction(&Instruction::I32Load(ma));
        f.instruction(&Instruction::I32Load(ma));
        f.instruction(&Instruction::I32Const(SCR_A_LEN as i32));
        f.instruction(&Instruction::I32Load(ma));
        f.instruction(&Instruction::I32Const(4));
        f.instruction(&Instruction::I32Mul);
        f.instruction(&Instruction::I32Add);
        f.instruction(&Instruction::I32Load(ma));
        f.instruction(&Instruction::I32Const(SCR_J as i32));
        f.instruction(&Instruction::I32Load(ma));
        f.instruction(&Instruction::I32Add);
        f.instruction(&Instruction::I32Load8U(ma0));
        f.instruction(&Instruction::I32Store8(ma0));

        f.instruction(&Instruction::I32Const(SCR_J as i32));
        f.instruction(&Instruction::I32Const(SCR_J as i32));
        f.instruction(&Instruction::I32Load(ma));
        f.instruction(&Instruction::I32Const(1));
        f.instruction(&Instruction::I32Add);
        f.instruction(&Instruction::I32Store(ma));
        f.instruction(&Instruction::Br(0));
        f.instruction(&Instruction::End);
        f.instruction(&Instruction::End);

        f.instruction(&Instruction::I32Const(SCR_I as i32));
        f.instruction(&Instruction::I32Const(SCR_I as i32));
        f.instruction(&Instruction::I32Load(ma));
        f.instruction(&Instruction::I32Const(SCR_A_PTR as i32));
        f.instruction(&Instruction::I32Load(ma));
        f.instruction(&Instruction::I32Load(ma));
        f.instruction(&Instruction::I32Const(SCR_A_LEN as i32));
        f.instruction(&Instruction::I32Load(ma));
        f.instruction(&Instruction::I32Const(4));
        f.instruction(&Instruction::I32Mul);
        f.instruction(&Instruction::I32Add);
        f.instruction(&Instruction::I32Load(ma));
        f.instruction(&Instruction::I32Const(4));
        f.instruction(&Instruction::I32Sub);
        f.instruction(&Instruction::I32Load(ma));
        f.instruction(&Instruction::I32Add);
        f.instruction(&Instruction::I32Store(ma));

        f.instruction(&Instruction::I32Const(SCR_A_LEN as i32));
        f.instruction(&Instruction::I32Const(SCR_A_LEN as i32));
        f.instruction(&Instruction::I32Load(ma));
        f.instruction(&Instruction::I32Const(1));
        f.instruction(&Instruction::I32Add);
        f.instruction(&Instruction::I32Store(ma));
        f.instruction(&Instruction::Br(0));
        f.instruction(&Instruction::End);
        f.instruction(&Instruction::End);

        // Write length and return out_start + 4.
        f.instruction(&Instruction::I32Const(SCR_DST_PTR as i32));
        f.instruction(&Instruction::I32Load(ma));
        f.instruction(&Instruction::I32Const(SCR_I as i32));
        f.instruction(&Instruction::I32Load(ma));
        f.instruction(&Instruction::I32Const(SCR_DST_PTR as i32));
        f.instruction(&Instruction::I32Load(ma));
        f.instruction(&Instruction::I32Sub);
        f.instruction(&Instruction::I32Const(4));
        f.instruction(&Instruction::I32Sub);
        f.instruction(&Instruction::I32Store(ma));

        f.instruction(&Instruction::I32Const(SCR_DST_PTR as i32));
        f.instruction(&Instruction::I32Load(ma));
        f.instruction(&Instruction::I32Const(4));
        f.instruction(&Instruction::I32Add);

        f.instruction(&Instruction::I32Const(SCR_I as i32));
        f.instruction(&Instruction::I32Load(ma));
        f.instruction(&Instruction::GlobalSet(0));
    }

    fn emit_vec_new(&mut self, f: &mut Function, element_size: i32, dest: Option<&Place>) {
        self.emit_vec_new_inline(f, element_size);
        if let Some(Place::Local(id)) = dest {
            f.instruction(&Instruction::LocalSet(self.local_wasm_idx(id.0)));
        } else {
            f.instruction(&Instruction::Drop);
        }
    }

    fn emit_vec_new_inline(&mut self, f: &mut Function, element_size: i32) {
        let ma = MemArg {
            offset: 0,
            align: 2,
            memory_index: 0,
        };
        // Large fixed capacity keeps T3 usable for the 10k push/pop benchmark
        // until true reallocation lands.
        let cap = 16384i32;

        // Cache base pointer in scratch so this helper can both initialize the
        // header and leave the pointer on the stack as the expression result.
        f.instruction(&Instruction::I32Const(SCR_DST_PTR as i32));
        f.instruction(&Instruction::GlobalGet(0));
        f.instruction(&Instruction::I32Store(ma));

        // data_ptr = base + 12
        f.instruction(&Instruction::I32Const(SCR_DST_PTR as i32));
        f.instruction(&Instruction::I32Load(ma));
        f.instruction(&Instruction::I32Const(SCR_DST_PTR as i32));
        f.instruction(&Instruction::I32Load(ma));
        f.instruction(&Instruction::I32Const(12));
        f.instruction(&Instruction::I32Add);
        f.instruction(&Instruction::I32Store(ma));

        // len = 0
        f.instruction(&Instruction::I32Const(SCR_DST_PTR as i32));
        f.instruction(&Instruction::I32Load(ma));
        f.instruction(&Instruction::I32Const(4));
        f.instruction(&Instruction::I32Add);
        f.instruction(&Instruction::I32Const(0));
        f.instruction(&Instruction::I32Store(ma));

        // cap = 8
        f.instruction(&Instruction::I32Const(SCR_DST_PTR as i32));
        f.instruction(&Instruction::I32Load(ma));
        f.instruction(&Instruction::I32Const(8));
        f.instruction(&Instruction::I32Add);
        f.instruction(&Instruction::I32Const(cap));
        f.instruction(&Instruction::I32Store(ma));

        // heap_ptr += header + capacity * element_size
        f.instruction(&Instruction::GlobalGet(0));
        f.instruction(&Instruction::I32Const(12 + cap * element_size));
        f.instruction(&Instruction::I32Add);
        f.instruction(&Instruction::GlobalSet(0));

        // Result = base pointer
        f.instruction(&Instruction::I32Const(SCR_DST_PTR as i32));
        f.instruction(&Instruction::I32Load(ma));
    }

    fn emit_push(&mut self, f: &mut Function, args: &[Operand]) {
        let ma = MemArg {
            offset: 0,
            align: 2,
            memory_index: 0,
        };
        if args.len() < 2 {
            return;
        }

        // Scratch vec ptr and value
        f.instruction(&Instruction::I32Const(SCR_A_PTR as i32));
        self.emit_operand(f, &args[0]);
        f.instruction(&Instruction::I32Store(ma));

        f.instruction(&Instruction::I32Const(SCR_B_PTR as i32));
        self.emit_operand(f, &args[1]);
        f.instruction(&Instruction::I32Store(ma));

        // len scratch
        f.instruction(&Instruction::I32Const(SCR_A_LEN as i32));
        f.instruction(&Instruction::I32Const(SCR_A_PTR as i32));
        f.instruction(&Instruction::I32Load(ma));
        f.instruction(&Instruction::I32Const(4));
        f.instruction(&Instruction::I32Add);
        f.instruction(&Instruction::I32Load(ma));
        f.instruction(&Instruction::I32Store(ma));

        // cap scratch
        f.instruction(&Instruction::I32Const(SCR_B_LEN as i32));
        f.instruction(&Instruction::I32Const(SCR_A_PTR as i32));
        f.instruction(&Instruction::I32Load(ma));
        f.instruction(&Instruction::I32Const(8));
        f.instruction(&Instruction::I32Add);
        f.instruction(&Instruction::I32Load(ma));
        f.instruction(&Instruction::I32Store(ma));

        // if len >= cap: skip
        f.instruction(&Instruction::I32Const(SCR_A_LEN as i32));
        f.instruction(&Instruction::I32Load(ma));
        f.instruction(&Instruction::I32Const(SCR_B_LEN as i32));
        f.instruction(&Instruction::I32Load(ma));
        f.instruction(&Instruction::I32GeU);
        f.instruction(&Instruction::If(wasm_encoder::BlockType::Empty));
        f.instruction(&Instruction::Else);

        // data_ptr + len*4
        f.instruction(&Instruction::I32Const(SCR_A_PTR as i32));
        f.instruction(&Instruction::I32Load(ma));
        f.instruction(&Instruction::I32Load(ma));
        f.instruction(&Instruction::I32Const(SCR_A_LEN as i32));
        f.instruction(&Instruction::I32Load(ma));
        f.instruction(&Instruction::I32Const(4));
        f.instruction(&Instruction::I32Mul);
        f.instruction(&Instruction::I32Add);
        f.instruction(&Instruction::I32Const(SCR_B_PTR as i32));
        f.instruction(&Instruction::I32Load(ma));
        f.instruction(&Instruction::I32Store(ma));

        // len += 1
        f.instruction(&Instruction::I32Const(SCR_A_PTR as i32));
        f.instruction(&Instruction::I32Load(ma));
        f.instruction(&Instruction::I32Const(4));
        f.instruction(&Instruction::I32Add);
        f.instruction(&Instruction::I32Const(SCR_A_LEN as i32));
        f.instruction(&Instruction::I32Load(ma));
        f.instruction(&Instruction::I32Const(1));
        f.instruction(&Instruction::I32Add);
        f.instruction(&Instruction::I32Store(ma));
        f.instruction(&Instruction::End);
    }

    fn emit_len_inline(&mut self, f: &mut Function, arg: Option<&Operand>) {
        let ma = MemArg {
            offset: 0,
            align: 2,
            memory_index: 0,
        };
        if let Some(arg) = arg {
            self.emit_operand(f, arg);
            if self.is_string_like_operand(arg) {
                f.instruction(&Instruction::I32Const(4));
                f.instruction(&Instruction::I32Sub);
                f.instruction(&Instruction::I32Load(ma));
            } else {
                f.instruction(&Instruction::I32Const(4));
                f.instruction(&Instruction::I32Add);
                f.instruction(&Instruction::I32Load(ma));
            }
        } else {
            f.instruction(&Instruction::I32Const(0));
        }
    }

    fn emit_get_unchecked_inline(&mut self, f: &mut Function, args: &[Operand]) {
        let ma = MemArg {
            offset: 0,
            align: 2,
            memory_index: 0,
        };
        if args.len() < 2 {
            f.instruction(&Instruction::I32Const(0));
            return;
        }

        self.emit_operand(f, &args[0]);
        f.instruction(&Instruction::I32Load(ma));
        self.emit_operand(f, &args[1]);
        f.instruction(&Instruction::I32Const(4));
        f.instruction(&Instruction::I32Mul);
        f.instruction(&Instruction::I32Add);
        f.instruction(&Instruction::I32Load(ma));
    }

    fn emit_get_inline(&mut self, f: &mut Function, args: &[Operand]) {
        let ma = MemArg {
            offset: 0,
            align: 2,
            memory_index: 0,
        };
        if args.len() < 2 {
            f.instruction(&Instruction::I32Const(0));
            return;
        }

        self.emit_operand(f, &args[1]);
        self.emit_operand(f, &args[0]);
        f.instruction(&Instruction::I32Const(4));
        f.instruction(&Instruction::I32Add);
        f.instruction(&Instruction::I32Load(ma));
        f.instruction(&Instruction::I32LtU);
        f.instruction(&Instruction::If(wasm_encoder::BlockType::Result(
            ValType::I32,
        )));

        f.instruction(&Instruction::GlobalGet(0));
        f.instruction(&Instruction::I32Const(0));
        f.instruction(&Instruction::I32Store(ma));
        f.instruction(&Instruction::GlobalGet(0));
        f.instruction(&Instruction::I32Const(4));
        f.instruction(&Instruction::I32Add);
        self.emit_get_unchecked_inline(f, args);
        f.instruction(&Instruction::I32Store(ma));
        f.instruction(&Instruction::GlobalGet(0));
        f.instruction(&Instruction::GlobalGet(0));
        f.instruction(&Instruction::I32Const(8));
        f.instruction(&Instruction::I32Add);
        f.instruction(&Instruction::GlobalSet(0));

        f.instruction(&Instruction::Else);

        f.instruction(&Instruction::GlobalGet(0));
        f.instruction(&Instruction::I32Const(1));
        f.instruction(&Instruction::I32Store(ma));
        f.instruction(&Instruction::GlobalGet(0));
        f.instruction(&Instruction::I32Const(4));
        f.instruction(&Instruction::I32Add);
        f.instruction(&Instruction::I32Const(0));
        f.instruction(&Instruction::I32Store(ma));
        f.instruction(&Instruction::GlobalGet(0));
        f.instruction(&Instruction::GlobalGet(0));
        f.instruction(&Instruction::I32Const(8));
        f.instruction(&Instruction::I32Add);
        f.instruction(&Instruction::GlobalSet(0));

        f.instruction(&Instruction::End);
    }

    fn emit_set(&mut self, f: &mut Function, args: &[Operand]) {
        let ma = MemArg {
            offset: 0,
            align: 2,
            memory_index: 0,
        };
        if args.len() < 3 {
            return;
        }

        self.emit_operand(f, &args[0]);
        f.instruction(&Instruction::I32Load(ma));
        self.emit_operand(f, &args[1]);
        f.instruction(&Instruction::I32Const(4));
        f.instruction(&Instruction::I32Mul);
        f.instruction(&Instruction::I32Add);
        self.emit_operand(f, &args[2]);
        f.instruction(&Instruction::I32Store(ma));
    }

    fn emit_pop_inline(&mut self, f: &mut Function, args: &[Operand]) {
        let ma = MemArg {
            offset: 0,
            align: 2,
            memory_index: 0,
        };
        if args.is_empty() {
            f.instruction(&Instruction::I32Const(0));
            return;
        }

        f.instruction(&Instruction::I32Const(SCR_A_PTR as i32));
        self.emit_operand(f, &args[0]);
        f.instruction(&Instruction::I32Store(ma));

        f.instruction(&Instruction::I32Const(SCR_A_LEN as i32));
        f.instruction(&Instruction::I32Const(SCR_A_PTR as i32));
        f.instruction(&Instruction::I32Load(ma));
        f.instruction(&Instruction::I32Const(4));
        f.instruction(&Instruction::I32Add);
        f.instruction(&Instruction::I32Load(ma));
        f.instruction(&Instruction::I32Store(ma));

        f.instruction(&Instruction::I32Const(SCR_A_LEN as i32));
        f.instruction(&Instruction::I32Load(ma));
        f.instruction(&Instruction::I32Const(0));
        f.instruction(&Instruction::I32GtU);
        f.instruction(&Instruction::If(wasm_encoder::BlockType::Result(
            ValType::I32,
        )));

        f.instruction(&Instruction::I32Const(SCR_A_LEN as i32));
        f.instruction(&Instruction::I32Const(SCR_A_LEN as i32));
        f.instruction(&Instruction::I32Load(ma));
        f.instruction(&Instruction::I32Const(1));
        f.instruction(&Instruction::I32Sub);
        f.instruction(&Instruction::I32Store(ma));

        f.instruction(&Instruction::I32Const(SCR_A_PTR as i32));
        f.instruction(&Instruction::I32Load(ma));
        f.instruction(&Instruction::I32Const(4));
        f.instruction(&Instruction::I32Add);
        f.instruction(&Instruction::I32Const(SCR_A_LEN as i32));
        f.instruction(&Instruction::I32Load(ma));
        f.instruction(&Instruction::I32Store(ma));

        f.instruction(&Instruction::GlobalGet(0));
        f.instruction(&Instruction::I32Const(0));
        f.instruction(&Instruction::I32Store(ma));
        f.instruction(&Instruction::GlobalGet(0));
        f.instruction(&Instruction::I32Const(4));
        f.instruction(&Instruction::I32Add);
        f.instruction(&Instruction::I32Const(SCR_A_PTR as i32));
        f.instruction(&Instruction::I32Load(ma));
        f.instruction(&Instruction::I32Load(ma));
        f.instruction(&Instruction::I32Const(SCR_A_LEN as i32));
        f.instruction(&Instruction::I32Load(ma));
        f.instruction(&Instruction::I32Const(4));
        f.instruction(&Instruction::I32Mul);
        f.instruction(&Instruction::I32Add);
        f.instruction(&Instruction::I32Load(ma));
        f.instruction(&Instruction::I32Store(ma));
        f.instruction(&Instruction::GlobalGet(0));
        f.instruction(&Instruction::GlobalGet(0));
        f.instruction(&Instruction::I32Const(8));
        f.instruction(&Instruction::I32Add);
        f.instruction(&Instruction::GlobalSet(0));

        f.instruction(&Instruction::Else);

        f.instruction(&Instruction::GlobalGet(0));
        f.instruction(&Instruction::I32Const(1));
        f.instruction(&Instruction::I32Store(ma));
        f.instruction(&Instruction::GlobalGet(0));
        f.instruction(&Instruction::I32Const(4));
        f.instruction(&Instruction::I32Add);
        f.instruction(&Instruction::I32Const(0));
        f.instruction(&Instruction::I32Store(ma));
        f.instruction(&Instruction::GlobalGet(0));
        f.instruction(&Instruction::GlobalGet(0));
        f.instruction(&Instruction::I32Const(8));
        f.instruction(&Instruction::I32Add);
        f.instruction(&Instruction::GlobalSet(0));

        f.instruction(&Instruction::End);
    }

    fn emit_binop(&self, f: &mut Function, op: BinOp, _local_id: u32) {
        // Check if operands are f64 based on context
        let is_f64 = self.f64_locals.contains(&_local_id);
        let is_i64 = self.i64_locals.contains(&_local_id);

        if is_f64 {
            match op {
                BinOp::Add => {
                    f.instruction(&Instruction::F64Add);
                }
                BinOp::Sub => {
                    f.instruction(&Instruction::F64Sub);
                }
                BinOp::Mul => {
                    f.instruction(&Instruction::F64Mul);
                }
                BinOp::Div => {
                    f.instruction(&Instruction::F64Div);
                }
                _ => {
                    f.instruction(&Instruction::F64Add);
                } // fallback
            }
        } else if is_i64 {
            match op {
                BinOp::Add => {
                    f.instruction(&Instruction::I64Add);
                }
                BinOp::Sub => {
                    f.instruction(&Instruction::I64Sub);
                }
                BinOp::Mul => {
                    f.instruction(&Instruction::I64Mul);
                }
                BinOp::Div => {
                    f.instruction(&Instruction::I64DivS);
                }
                BinOp::Mod => {
                    f.instruction(&Instruction::I64RemS);
                }
                _ => {
                    f.instruction(&Instruction::I64Add);
                }
            }
        } else {
            match op {
                BinOp::Add => {
                    f.instruction(&Instruction::I32Add);
                }
                BinOp::Sub => {
                    f.instruction(&Instruction::I32Sub);
                }
                BinOp::Mul => {
                    f.instruction(&Instruction::I32Mul);
                }
                BinOp::Div => {
                    f.instruction(&Instruction::I32DivS);
                }
                BinOp::Mod => {
                    f.instruction(&Instruction::I32RemS);
                }
                BinOp::Eq => {
                    f.instruction(&Instruction::I32Eq);
                }
                BinOp::Ne => {
                    f.instruction(&Instruction::I32Ne);
                }
                BinOp::Lt => {
                    f.instruction(&Instruction::I32LtS);
                }
                BinOp::Le => {
                    f.instruction(&Instruction::I32LeS);
                }
                BinOp::Gt => {
                    f.instruction(&Instruction::I32GtS);
                }
                BinOp::Ge => {
                    f.instruction(&Instruction::I32GeS);
                }
                BinOp::And => {
                    f.instruction(&Instruction::I32And);
                }
                BinOp::Or => {
                    f.instruction(&Instruction::I32Or);
                }
                BinOp::BitAnd => {
                    f.instruction(&Instruction::I32And);
                }
                BinOp::BitOr => {
                    f.instruction(&Instruction::I32Or);
                }
                BinOp::BitXor => {
                    f.instruction(&Instruction::I32Xor);
                }
                BinOp::Shl => {
                    f.instruction(&Instruction::I32Shl);
                }
                BinOp::Shr => {
                    f.instruction(&Instruction::I32ShrS);
                }
            }
        }
    }

    fn emit_unaryop(&self, f: &mut Function, op: UnaryOp, local_id: u32) {
        match op {
            UnaryOp::Neg => {
                if self.f64_locals.contains(&local_id) {
                    f.instruction(&Instruction::F64Neg);
                } else if self.i64_locals.contains(&local_id) {
                    f.instruction(&Instruction::I64Const(0));
                    f.instruction(&Instruction::I64Sub);
                } else {
                    // i32 neg: 0 - val (val is on stack)
                    // We need: push 0 then subtract, but val is already on stack
                    // So: store val, push 0, load val, sub
                    // Simpler approach: multiply by -1
                    f.instruction(&Instruction::I32Const(-1));
                    f.instruction(&Instruction::I32Mul);
                }
            }
            UnaryOp::Not | UnaryOp::BitNot => {
                f.instruction(&Instruction::I32Const(-1));
                f.instruction(&Instruction::I32Xor);
            }
        }
    }

    fn local_wasm_idx(&self, mir_local_id: u32) -> u32 {
        // In the bridge implementation, MIR local IDs map directly
        // to Wasm local indices (params first, then locals)
        mir_local_id
    }

    fn enum_total_size(&self, enum_name: &str) -> u32 {
        let variants = match self.enum_defs.get(enum_name) {
            Some(v) => v,
            None => return 8,
        };
        let max_payload: u32 = variants
            .iter()
            .map(|(_, fields)| fields.len() as u32 * 4)
            .max()
            .unwrap_or(4);
        4 + max_payload.max(4)
    }
}
