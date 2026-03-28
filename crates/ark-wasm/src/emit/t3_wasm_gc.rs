//! T3 `wasm32-wasi-p2` backend — Wasm GC emitter (transitioning to GC-native).
//!
//! GC types are declared in the type section with proper WasmGC structure:
//!   - String:  bare `(array (mut i8))` — no wrapper struct
//!   - Vec<T>:  `(struct (field (mut (ref $arr_T))) (field (mut i32)))` — cap = array.len
//!   - Struct:  `(struct (field (mut T))...)` — typed fields
//!   - Enum:    subtype hierarchy — non-final base + final variant subtypes
//!
//! Currently in bridge-mode: runtime values still use linear memory with i32
//! pointers. GC-native emission is being implemented phase by phase.
//!
//! Linear memory is reserved for WASI I/O only (1 page, not growable).
//! No bump allocator — heap_ptr global has been removed.

#![allow(dead_code)]

use ark_diagnostics::DiagnosticSink;
use ark_mir::mir::*;
use ark_typecheck::types::Type;
use std::collections::{HashMap, HashSet, VecDeque};
use wasm_encoder::{
    ArrayType, CodeSection, CompositeInnerType, CompositeType, DataSection, DataSegment,
    ExportKind, ExportSection, FieldType, Function, FunctionSection, GlobalSection, GlobalType,
    HeapType, ImportSection, Instruction, MemArg, MemorySection, MemoryType,
    RefType as WasmRefType, StorageType, StructType, SubType, TypeSection, ValType,
};

// ── Linear memory layout (IO bridge only) ────────────────────────
const IOV_BASE: u32 = 0;
const IOV_LEN: u32 = 4;
const NWRITTEN: u32 = 8;
const SCRATCH: u32 = 16;
const I32BUF: u32 = 48;
const SCR_VAL64: u32 = 56; // 8-byte scratch for i64/f64 values
const DATA_START: u32 = 256;
const SCR_A_PTR: u32 = SCRATCH;
const SCR_B_PTR: u32 = SCRATCH + 4;
const SCR_A_LEN: u32 = SCRATCH + 8;
const SCR_B_LEN: u32 = SCRATCH + 12;
const SCR_DST_PTR: u32 = SCRATCH + 16;
const SCR_I: u32 = SCRATCH + 20;
const SCR_J: u32 = SCRATCH + 24;
const SCR_MATCH: u32 = SCRATCH + 28;
const SCR_RESULT: u32 = SCRATCH + 32;

// GC struct field indices
const STR_FIELD_BYTES: u32 = 0;
const VEC_FIELD_DATA: u32 = 0;
const VEC_FIELD_LEN: u32 = 1;
const VEC_FIELD_CAP: u32 = 2;

// Well-known import function indices (set dynamically based on usage)
// const FN_FD_WRITE: u32 = 0;  -- now self.wasi_fd_write
// const FN_PATH_OPEN: u32 = 1; -- now self.wasi_path_open  
// const FN_FD_READ: u32 = 2;   -- now self.wasi_fd_read
// const FN_FD_CLOSE: u32 = 3;  -- now self.wasi_fd_close

// I/O scratch memory layout
const FS_SCRATCH: u32 = 160;
const FS_BUF_SIZE: u32 = 4096;

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
    func_cache: HashMap<(Vec<ValType>, Vec<ValType>), u32>,
    section: TypeSection,
}

impl TypeAlloc {
    fn new() -> Self {
        Self {
            next_idx: 0,
            names: HashMap::new(),
            func_cache: HashMap::new(),
            section: TypeSection::new(),
        }
    }

    fn add_func(&mut self, params: &[ValType], results: &[ValType]) -> u32 {
        let key = (params.to_vec(), results.to_vec());
        if let Some(&idx) = self.func_cache.get(&key) {
            return idx;
        }
        let idx = self.next_idx;
        self.section
            .ty()
            .function(params.iter().copied(), results.iter().copied());
        self.func_cache.insert(key, idx);
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

    /// Add a non-final base struct type for enum subtype hierarchies.
    fn add_sub_struct_base(&mut self, name: &str) -> u32 {
        let idx = self.next_idx;
        self.names.insert(name.to_string(), idx);
        self.section.ty().subtype(&SubType {
            is_final: false,
            supertype_idx: None,
            composite_type: CompositeType {
                inner: CompositeInnerType::Struct(StructType {
                    fields: Box::new([]),
                }),
                shared: false,
            },
        });
        self.next_idx += 1;
        idx
    }

    /// Add a final variant struct subtype with the given supertype.
    fn add_sub_struct_variant(&mut self, name: &str, super_idx: u32, fields: &[FieldType]) -> u32 {
        let idx = self.next_idx;
        self.names.insert(name.to_string(), idx);
        self.section.ty().subtype(&SubType {
            is_final: true,
            supertype_idx: Some(super_idx),
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

    /// Add an entire enum hierarchy as a single `rec` group so that
    /// structurally identical variants get distinct types (isorecursive
    /// type equivalence is per-position within a rec group).
    /// Returns (base_idx, vec of (variant_name, variant_idx)).
    fn add_enum_rec_group(
        &mut self,
        base_name: &str,
        variants: &[(String, Vec<FieldType>)],
    ) -> (u32, Vec<(String, u32)>) {
        let base_idx = self.next_idx;
        self.names.insert(base_name.to_string(), base_idx);

        // Build the rec group: base type at position 0, then each variant
        let mut subtypes = Vec::with_capacity(1 + variants.len());

        // Base: non-final empty struct
        subtypes.push(SubType {
            is_final: false,
            supertype_idx: None,
            composite_type: CompositeType {
                inner: CompositeInnerType::Struct(StructType {
                    fields: Box::new([]),
                }),
                shared: false,
            },
        });

        // Each variant: final subtype of base (index 0 within this rec group = base_idx)
        for (_, fields) in variants {
            subtypes.push(SubType {
                is_final: true,
                supertype_idx: Some(base_idx),
                composite_type: CompositeType {
                    inner: CompositeInnerType::Struct(StructType {
                        fields: fields.clone().into_boxed_slice(),
                    }),
                    shared: false,
                },
            });
        }

        // Emit the rec group
        self.section.ty().rec(subtypes);

        // Register names and compute indices
        let mut result = Vec::with_capacity(variants.len());
        for (i, (vname, _)) in variants.iter().enumerate() {
            let v_idx = base_idx + 1 + i as u32;
            let full_name = format!("{}.{}", base_name, vname);
            self.names.insert(full_name, v_idx);
            result.push((vname.clone(), v_idx));
        }

        self.next_idx = base_idx + 1 + variants.len() as u32;
        (base_idx, result)
    }
}

// ── Emit context ─────────────────────────────────────────────────

struct Ctx {
    types: TypeAlloc,
    /// Active data segments (loaded into linear memory) for I/O scratch only.
    data_segs: Vec<(u32, Vec<u8>)>,
    data_offset: u32,
    /// Passive data segments for string literals (consumed by array.new_data).
    string_data_segs: Vec<Vec<u8>>,
    /// Deduplication cache: maps string bytes → segment index.
    string_seg_cache: HashMap<Vec<u8>, u32>,
    fn_map: HashMap<String, u32>,
    fn_names: Vec<String>,
    next_fn: u32,
    // Well-known GC type indices
    string_ty: u32,
    arr_i32_ty: u32,
    vec_i32_ty: u32,
    arr_i64_ty: u32,
    vec_i64_ty: u32,
    arr_f64_ty: u32,
    vec_f64_ty: u32,
    arr_string_ty: u32,
    vec_string_ty: u32,
    // HashMap GC type index
    hashmap_i32_i32_ty: u32,
    // Well-known function type indices
    fd_write_ty: u32,
    // User struct GC type indices
    struct_gc_types: HashMap<String, u32>,
    struct_layouts: HashMap<String, Vec<(String, String)>>,
    // Enum GC type indices: subtype hierarchy
    enum_base_types: HashMap<String, u32>,
    enum_variant_types: HashMap<String, HashMap<String, u32>>,
    enum_variant_field_types: HashMap<(String, String), Vec<String>>,
    enum_defs: HashMap<String, Vec<(String, Vec<String>)>>,
    fn_ret_types: HashMap<String, Type>,
    fn_param_types: HashMap<String, Vec<Type>>,
    fn_ret_type_names: HashMap<String, String>,
    fn_param_type_names: HashMap<String, Vec<String>>,
    // Local type tracking (per-function)
    string_locals: std::collections::HashSet<u32>,
    f64_locals: std::collections::HashSet<u32>,
    i64_locals: std::collections::HashSet<u32>,
    bool_locals: std::collections::HashSet<u32>,
    any_locals: std::collections::HashSet<u32>,
    f64_vec_locals: std::collections::HashSet<u32>,
    i64_vec_locals: std::collections::HashSet<u32>,
    i32_vec_locals: std::collections::HashSet<u32>,
    string_vec_locals: std::collections::HashSet<u32>,
    // Vec<Struct> support: struct_name → (arr_type_idx, vec_type_idx)
    custom_vec_types: HashMap<String, (u32, u32)>,
    // Per-function tracking of which locals are Vec<StructName>
    struct_vec_locals: HashMap<u32, String>,
    local_struct: HashMap<u32, String>,
    local_enum: HashMap<u32, String>,
    // Helper function indices (emitted once)
    helper_i32_to_str: Option<u32>,
    helper_i64_to_str: Option<u32>,
    helper_f64_to_str: Option<u32>,
    helper_print_i32_ln: Option<u32>,
    helper_print_bool_ln: Option<u32>,
    helper_print_str_ln: Option<u32>,
    helper_print_newline: Option<u32>,
    helper_parse_i32: Option<u32>,
    helper_parse_i64: Option<u32>,
    helper_parse_f64: Option<u32>,
    err_string_seg: Option<u32>,
    err_float_string_seg: Option<u32>,
    // Pre-registered indirect call type indices
    indirect_types: HashMap<(Vec<ValType>, Vec<ValType>), u32>,
    // Scratch local base index (set per-function, for GC string ops)
    scratch_base: u32,
    // Whether the current function being emitted is _start/main (drops return value)
    is_start_fn: bool,
    // Extra nesting depth for break/continue inside if/else within loops
    loop_break_extra_depth: u32,
    // Generic function context: type_params of the function being emitted
    current_fn_type_params: Vec<String>,
    // Return type of the function being emitted
    current_fn_return_ty: Type,
    // WASI import indices (dynamically assigned based on usage)
    wasi_fd_write: u32,
    wasi_path_open: u32,
    wasi_fd_read: u32,
    wasi_fd_close: u32,
    wasi_needs_fs: bool,
    /// Optimization level (0 = O0, 1 = O1, 2 = O2).
    /// Tail-call emission (`return_call`) is enabled at opt_level >= 1.
    opt_level: u8,
}

impl Ctx {
    fn type_to_val(&self, ty: &Type) -> ValType {
        match ty {
            Type::I64 | Type::U64 => ValType::I64,
            Type::F64 => ValType::F64,
            Type::F32 => ValType::F32,
            Type::U8 | Type::U16 | Type::U32 | Type::I8 | Type::I16 => ValType::I32,
            Type::String => ref_nullable(self.string_ty),
            Type::Any => ValType::Ref(WasmRefType {
                nullable: true,
                heap_type: HeapType::ANY,
            }),
            Type::Vec(elem) => match elem.as_ref() {
                Type::I64 => ref_nullable(self.vec_i64_ty),
                Type::F64 => ref_nullable(self.vec_f64_ty),
                Type::String => ref_nullable(self.vec_string_ty),
                _ => ref_nullable(self.vec_i32_ty),
            },
            _ => ValType::I32,
        }
    }

    /// Resolve a type name (from fn_sigs or struct/enum defs) to a ValType.
    fn type_name_to_val(&self, name: &str) -> ValType {
        match name {
            "i32" | "bool" | "char" | "()" | "u8" | "u16" | "u32" | "i8" | "i16" => ValType::I32,
            "i64" | "u64" => ValType::I64,
            "f64" => ValType::F64,
            "f32" => ValType::F32,
            "String" => ref_nullable(self.string_ty),
            _ => {
                if let Some(&ty_idx) = self.struct_gc_types.get(name) {
                    return ref_nullable(ty_idx);
                }
                if let Some(&base_idx) = self.enum_base_types.get(name) {
                    return ref_nullable(base_idx);
                }
                // Vec types: Vec<i32>, Vec<String>, etc.
                if name.starts_with("Vec<") {
                    let inner = &name[4..name.len() - 1];
                    match inner {
                        "i32" => return ref_nullable(self.vec_i32_ty),
                        "i64" => return ref_nullable(self.vec_i64_ty),
                        "f64" => return ref_nullable(self.vec_f64_ty),
                        "String" => return ref_nullable(self.vec_string_ty),
                        _ => {
                            if let Some(&(_, vec_ty)) = self.custom_vec_types.get(inner) {
                                return ref_nullable(vec_ty);
                            }
                            // For generic Vec<T> (unknown inner type), default to Vec<i32>
                            return ref_nullable(self.vec_i32_ty);
                        }
                    }
                }
                // Option<T> → use "Option" base enum type
                if name.starts_with("Option<") || name == "Option" {
                    if let Some(&base_idx) = self.enum_base_types.get("Option") {
                        return ref_nullable(base_idx);
                    }
                }
                // Result<T, E> → use "Result" base enum type
                if name.starts_with("Result<") || name == "Result" {
                    if let Some(&base_idx) = self.enum_base_types.get("Result") {
                        return ref_nullable(base_idx);
                    }
                }
                ValType::I32
            }
        }
    }

    /// Resolve the Wasm ValType for a MIR local, using struct/enum typed-local
    /// side-channel maps to return GC ref types instead of i32.
    #[allow(clippy::type_complexity)]
    fn local_val_type(
        &self,
        local: &MirLocal,
        struct_typed_locals: &HashMap<u32, String>,
        enum_typed_locals: &HashMap<u32, String>,
        vec_sets: Option<(&HashSet<u32>, &HashSet<u32>, &HashSet<u32>, &HashSet<u32>)>,
    ) -> ValType {
        // Check struct side-channel first
        if let Some(sname) = struct_typed_locals.get(&local.id.0) {
            if let Some(&ty_idx) = self.struct_gc_types.get(sname) {
                return ref_nullable(ty_idx);
            }
        }
        // Check enum side-channel
        if let Some(ename) = enum_typed_locals.get(&local.id.0) {
            if let Some(&base_idx) = self.enum_base_types.get(ename) {
                return ref_nullable(base_idx);
            }
        }
        // Check Vec<Struct> side-channel
        if let Some(sname) = self.struct_vec_locals.get(&local.id.0) {
            if let Some(&(_, vec_ty)) = self.custom_vec_types.get(sname.as_str()) {
                return ref_nullable(vec_ty);
            }
        }
        // Check propagated vec types
        if let Some((vi32, vi64, vf64, vstr)) = vec_sets {
            let lid = local.id.0;
            if vi64.contains(&lid) {
                return ref_nullable(self.vec_i64_ty);
            }
            if vf64.contains(&lid) {
                return ref_nullable(self.vec_f64_ty);
            }
            if vstr.contains(&lid) {
                return ref_nullable(self.vec_string_ty);
            }
            if vi32.contains(&lid) {
                return ref_nullable(self.vec_i32_ty);
            }
        }
        self.type_to_val(&local.ty)
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

    /// Allocate a passive data segment for a string literal.
    /// Returns the segment index (used by array.new_data).
    /// Deduplicates identical byte sequences.
    fn alloc_string_data(&mut self, data: &[u8]) -> u32 {
        if let Some(&idx) = self.string_seg_cache.get(data) {
            return idx;
        }
        let idx = self.string_data_segs.len() as u32;
        self.string_data_segs.push(data.to_vec());
        self.string_seg_cache.insert(data.to_vec(), idx);
        idx
    }

    fn field_valtype(&self, ty_name: &str) -> ValType {
        match ty_name {
            "i64" => ValType::I64,
            "f64" => ValType::F64,
            "f32" => ValType::F32,
            "String" => ref_nullable(self.string_ty),
            "anyref" => ValType::Ref(WasmRefType {
                nullable: true,
                heap_type: HeapType::ANY,
            }),
            _ => {
                if let Some(&ty_idx) = self.struct_gc_types.get(ty_name) {
                    return ref_nullable(ty_idx);
                }
                if let Some(&base_idx) = self.enum_base_types.get(ty_name) {
                    return ref_nullable(base_idx);
                }
                ValType::I32
            }
        }
    }

    /// Scan a statement for Vec_new_* calls referencing struct names
    fn scan_operands_for_vec_struct(
        &self,
        stmt: &MirStmt,
        struct_defs: &HashMap<String, Vec<(String, String)>>,
        out: &mut HashSet<String>,
    ) {
        match stmt {
            MirStmt::Assign(_, Rvalue::Use(op)) => {
                self.scan_op_for_vec_struct(op, struct_defs, out);
            }
            MirStmt::CallBuiltin { name, args, .. } => {
                if let Some(sname) = name.strip_prefix("Vec_new_") {
                    if struct_defs.contains_key(sname) {
                        out.insert(sname.to_string());
                    }
                }
                for a in args {
                    self.scan_op_for_vec_struct(a, struct_defs, out);
                }
            }
            _ => {}
        }
    }

    fn scan_op_for_vec_struct(
        &self,
        op: &Operand,
        struct_defs: &HashMap<String, Vec<(String, String)>>,
        out: &mut HashSet<String>,
    ) {
        if let Operand::Call(name, _) = op {
            if let Some(sname) = name.strip_prefix("Vec_new_") {
                if struct_defs.contains_key(sname) {
                    out.insert(sname.to_string());
                }
            }
        }
    }
}

// ── Public entry point ───────────────────────────────────────────

/// Emit a Wasm module from MIR using real Wasm GC types.
///
/// Scalars live in Wasm locals. Strings, Vecs, structs, and enums use
/// GC struct/array types. I/O bridges through a small linear memory
/// region for WASI fd_write.
pub fn emit(mir: &MirModule, _sink: &mut DiagnosticSink, opt_level: u8) -> Vec<u8> {
    // TODO(MIR-01): remove checker fallback — read layouts from type_table only
    let struct_layouts: HashMap<String, Vec<(String, String)>> = mir.type_table.struct_defs.clone();
    let fn_ret_types: HashMap<String, Type> = mir
        .functions
        .iter()
        .map(|f| (f.name.clone(), f.return_ty.clone()))
        .collect();
    let fn_param_types: HashMap<String, Vec<Type>> = mir
        .functions
        .iter()
        .map(|f| {
            (
                f.name.clone(),
                f.params.iter().map(|p| p.ty.clone()).collect(),
            )
        })
        .collect();
    // Build fn return type NAME map from fn_sigs for struct/enum return resolution
    let fn_ret_type_names: HashMap<String, String> = mir
        .type_table
        .fn_sigs
        .iter()
        .map(|(name, sig)| (name.clone(), sig.ret.clone()))
        .collect();
    // Build fn param type NAME map from fn_sigs for struct/enum/vec param resolution
    let fn_param_type_names: HashMap<String, Vec<String>> = mir
        .type_table
        .fn_sigs
        .iter()
        .map(|(name, sig)| (name.clone(), sig.params.clone()))
        .collect();

    let mut ctx = Ctx {
        types: TypeAlloc::new(),
        data_segs: Vec::new(),
        data_offset: DATA_START,
        string_data_segs: Vec::new(),
        string_seg_cache: HashMap::new(),
        fn_map: HashMap::new(),
        fn_names: mir.functions.iter().map(|f| f.name.clone()).collect(),
        next_fn: 0,
        string_ty: 0,
        arr_i32_ty: 0,
        vec_i32_ty: 0,
        arr_i64_ty: 0,
        vec_i64_ty: 0,
        arr_f64_ty: 0,
        vec_f64_ty: 0,
        arr_string_ty: 0,
        vec_string_ty: 0,
        hashmap_i32_i32_ty: 0,
        fd_write_ty: 0,
        struct_gc_types: HashMap::new(),
        struct_layouts,
        enum_base_types: HashMap::new(),
        enum_variant_types: HashMap::new(),
        enum_variant_field_types: HashMap::new(),
        enum_defs: mir.type_table.enum_defs.clone(),
        fn_ret_types,
        fn_param_types,
        fn_ret_type_names,
        fn_param_type_names,
        string_locals: Default::default(),
        f64_locals: Default::default(),
        i64_locals: Default::default(),
        bool_locals: Default::default(),
        any_locals: Default::default(),
        f64_vec_locals: Default::default(),
        i64_vec_locals: Default::default(),
        i32_vec_locals: Default::default(),
        string_vec_locals: Default::default(),
        custom_vec_types: HashMap::new(),
        struct_vec_locals: HashMap::new(),
        local_struct: HashMap::new(),
        local_enum: HashMap::new(),
        helper_i32_to_str: None,
        helper_i64_to_str: None,
        helper_f64_to_str: None,
        helper_print_i32_ln: None,
        helper_print_bool_ln: None,
        helper_print_str_ln: None,
        helper_print_newline: None,
        helper_parse_i32: None,
        helper_parse_i64: None,
        helper_parse_f64: None,
        err_string_seg: None,
        err_float_string_seg: None,
        indirect_types: HashMap::new(),
        scratch_base: 0,
        is_start_fn: false,
        loop_break_extra_depth: 0,
        current_fn_type_params: vec![],
        current_fn_return_ty: Type::Unit,
        wasi_fd_write: 0,
        wasi_path_open: 0,
        wasi_fd_read: 0,
        wasi_fd_close: 0,
        wasi_needs_fs: false,
        opt_level,
    };
    ctx.emit_module(mir)
}

// ── Module emission ──────────────────────────────────────────────

impl Ctx {
    fn emit_module(&mut self, mir: &MirModule) -> Vec<u8> {
        let reachable_user_indices = self.reachable_function_indices(mir);

        // Scan MIR to determine which WASI imports are needed
        let needs_fs = Self::mir_uses_fs(mir, &reachable_user_indices);
        self.wasi_needs_fs = needs_fs;

        // Phase 1: Register GC types
        self.register_gc_types(mir);

        // Pre-allocate "invalid number" error string for parse helpers
        if self.enum_base_types.contains_key("Result")
            || self.enum_base_types.contains_key("Result_i64_String")
            || self.enum_base_types.contains_key("Result_f64_String")
        {
            let seg = self.alloc_string_data(b"parse error: invalid integer");
            self.err_string_seg = Some(seg);
            let seg_f = self.alloc_string_data(b"parse error: invalid float");
            self.err_float_string_seg = Some(seg_f);
        }

        // Phase 2: Register function type signatures
        let fd_write_ty = self.types.add_func(&[ValType::I32; 4], &[ValType::I32]);
        self.fd_write_ty = fd_write_ty;
        // path_open: (i32,i32,i32,i32,i32,i64,i64,i32,i32) -> i32
        let path_open_ty = self.types.add_func(
            &[
                ValType::I32,
                ValType::I32,
                ValType::I32,
                ValType::I32,
                ValType::I32,
                ValType::I64,
                ValType::I64,
                ValType::I32,
                ValType::I32,
            ],
            &[ValType::I32],
        );
        // fd_read: same as fd_write (i32,i32,i32,i32) -> i32
        let fd_read_ty = fd_write_ty;
        // fd_close: (i32) -> i32
        let fd_close_ty = self.types.add_func(&[ValType::I32], &[ValType::I32]);

        // Count helper functions we'll need
        // Dynamic WASI import count: fd_write is always needed; FS imports only if used
        let num_imports = if needs_fs { 4u32 } else { 1u32 };
        self.wasi_fd_write = 0; // fd_write is always index 0
        if needs_fs {
            self.wasi_path_open = 1;
            self.wasi_fd_read = 2;
            self.wasi_fd_close = 3;
        }
        // GC-native helper function signatures
        let str_ref = ref_nullable(self.string_ty);
        let mut helper_fns: Vec<(String, Vec<ValType>, Vec<ValType>)> = vec![
            // __print_str_ln: (ref $string) -> ()
            ("__print_str_ln".into(), vec![str_ref], vec![]),
            // __print_i32_ln: (i32) -> ()
            ("__print_i32_ln".into(), vec![ValType::I32], vec![]),
            // __print_bool_ln: (i32) -> ()
            ("__print_bool_ln".into(), vec![ValType::I32], vec![]),
            // __i32_to_str: (i32) -> (ref $string)
            ("__i32_to_str".into(), vec![ValType::I32], vec![str_ref]),
            // __print_newline: () -> ()
            ("__print_newline".into(), vec![], vec![]),
            // __i64_to_str: (i64) -> (ref $string)
            ("__i64_to_str".into(), vec![ValType::I64], vec![str_ref]),
            // __f64_to_str: (f64) -> (ref $string)
            ("__f64_to_str".into(), vec![ValType::F64], vec![str_ref]),
        ];

        // Conditionally add parse helpers if the relevant Result enum types exist
        let parse_i32_helper_idx = if self.enum_base_types.contains_key("Result") {
            let result_ref = ref_nullable(*self.enum_base_types.get("Result").unwrap());
            let idx = helper_fns.len();
            helper_fns.push(("__parse_i32".into(), vec![str_ref], vec![result_ref]));
            Some(idx)
        } else {
            None
        };
        let parse_i64_helper_idx = if self.enum_base_types.contains_key("Result_i64_String") {
            let result_ref = ref_nullable(*self.enum_base_types.get("Result_i64_String").unwrap());
            let idx = helper_fns.len();
            helper_fns.push(("__parse_i64".into(), vec![str_ref], vec![result_ref]));
            Some(idx)
        } else {
            None
        };
        let parse_f64_helper_idx = if self.enum_base_types.contains_key("Result_f64_String") {
            let result_ref = ref_nullable(*self.enum_base_types.get("Result_f64_String").unwrap());
            let idx = helper_fns.len();
            helper_fns.push(("__parse_f64".into(), vec![str_ref], vec![result_ref]));
            Some(idx)
        } else {
            None
        };

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
            let params: Vec<ValType> =
                if let Some(param_names) = self.fn_param_type_names.get(&func.name) {
                    param_names
                        .iter()
                        .enumerate()
                        .map(|(i, p)| {
                            // If MIR says the param is Any (generic), use anyref regardless of AST type name
                            if func.params.get(i).is_some_and(|mp| mp.ty == Type::Any) {
                                ValType::Ref(WasmRefType {
                                    nullable: true,
                                    heap_type: HeapType::ANY,
                                })
                            } else {
                                self.type_name_to_val(p)
                            }
                        })
                        .collect()
                } else {
                    func.params
                        .iter()
                        .map(|p| {
                            self.local_val_type(
                                p,
                                &func.struct_typed_locals,
                                &func.enum_typed_locals,
                                None,
                            )
                        })
                        .collect()
                };
            let results: Vec<ValType> = if func.name == "main" || func.name == "_start" {
                // WASI _start must be () -> ()
                vec![]
            } else {
                match &func.return_ty {
                    Type::Unit | Type::Never => vec![],
                    ty => {
                        // Vec_new_* functions return GC vec refs
                        let result_ty = if func.name.starts_with("Vec_new_") {
                            let sname = &func.name[8..];
                            match sname {
                                "i32" => ref_nullable(self.vec_i32_ty),
                                "i64" => ref_nullable(self.vec_i64_ty),
                                "f64" => ref_nullable(self.vec_f64_ty),
                                "String" => ref_nullable(self.vec_string_ty),
                                _ => {
                                    if let Some(&(_, vec_ty)) = self.custom_vec_types.get(sname) {
                                        ref_nullable(vec_ty)
                                    } else {
                                        self.type_to_val(ty)
                                    }
                                }
                            }
                        } else {
                            // For struct/enum returns, use fn_ret_type_names for accurate type
                            if matches!(ty, Type::I32) {
                                // Check for generic tuple return: scan terminators for StructInit("__tupleN_any")
                                let tuple_any_ret = if !func.type_params.is_empty() {
                                    func.blocks.iter().find_map(|blk| {
                                        if let Terminator::Return(Some(Operand::StructInit {
                                            name,
                                            ..
                                        })) = &blk.terminator
                                        {
                                            if name.starts_with("__tuple") && name.ends_with("_any")
                                            {
                                                self.struct_gc_types
                                                    .get(name)
                                                    .map(|&idx| ref_nullable(idx))
                                            } else {
                                                None
                                            }
                                        } else {
                                            None
                                        }
                                    })
                                } else {
                                    None
                                };
                                if let Some(tuple_ty) = tuple_any_ret {
                                    tuple_ty
                                } else if let Some(ret_name) =
                                    self.fn_ret_type_names.get(&func.name)
                                {
                                    self.type_name_to_val(ret_name)
                                } else {
                                    ValType::I32
                                }
                            } else {
                                self.type_to_val(ty)
                            }
                        };
                        vec![result_ty]
                    }
                }
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
        self.helper_i64_to_str = Some(helper_base + 5);
        self.helper_f64_to_str = Some(helper_base + 6);
        if let Some(idx) = parse_i32_helper_idx {
            self.helper_parse_i32 = Some(helper_base + idx as u32);
        }
        if let Some(idx) = parse_i64_helper_idx {
            self.helper_parse_i64 = Some(helper_base + idx as u32);
        }
        if let Some(idx) = parse_f64_helper_idx {
            self.helper_parse_f64 = Some(helper_base + idx as u32);
        }

        let user_base = helper_base + helper_fns.len() as u32;
        for (i, &idx) in reachable_user_indices.iter().enumerate() {
            let func = &mir.functions[idx];
            self.fn_map.insert(func.name.clone(), user_base + i as u32);
        }

        // Pre-register indirect call type signatures for HOF operations
        {
            let sigs: Vec<(Vec<ValType>, Vec<ValType>)> = vec![
                (vec![ValType::I32], vec![ValType::I32]), // (i32) -> i32
                (vec![ValType::I64], vec![ValType::I32]), // (i64) -> i32 (predicate)
                (vec![ValType::F64], vec![ValType::I32]), // (f64) -> i32 (predicate)
                (vec![ValType::I32], vec![ValType::I64]), // (i32) -> i64 (map)
                (vec![ValType::I64], vec![ValType::I64]), // (i64) -> i64 (map)
                (vec![ValType::F64], vec![ValType::F64]), // (f64) -> f64 (map)
                (vec![ValType::I64, ValType::I64], vec![ValType::I64]), // (i64,i64) -> i64 (fold)
            ];
            for (params, results) in sigs {
                let ty_idx = self.types.add_func(&params, &results);
                self.indirect_types.insert((params, results), ty_idx);
            }
        }

        // ── Build sections ───────────────────────────────────────

        // Import section: WASI functions (only those actually used)
        let mut imports = ImportSection::new();
        imports.import(
            "wasi_snapshot_preview1",
            "fd_write",
            wasm_encoder::EntityType::Function(fd_write_ty),
        );
        if needs_fs {
            imports.import(
                "wasi_snapshot_preview1",
                "path_open",
                wasm_encoder::EntityType::Function(path_open_ty),
            );
            imports.import(
                "wasi_snapshot_preview1",
                "fd_read",
                wasm_encoder::EntityType::Function(fd_read_ty),
            );
            imports.import(
                "wasi_snapshot_preview1",
                "fd_close",
                wasm_encoder::EntityType::Function(fd_close_ty),
            );
        }

        // Function section
        let mut functions = FunctionSection::new();
        for &ty_idx in &helper_type_indices {
            functions.function(ty_idx);
        }
        for &ty_idx in &user_fn_type_indices {
            functions.function(ty_idx);
        }

        // Memory section — BRIDGE COMPAT: keep 4-10 pages until fully GC-native.
        // Target: 1 page fixed (WASI I/O only) once all allocations use GC heap.
        let mut memories = MemorySection::new();
        memories.memory(MemoryType {
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

        // Export user pub functions for Component Model (kebab-case names for WIT)
        for func in &mir.functions {
            if func.is_exported && func.name != "main" && !func.name.starts_with("__") {
                if let Some(&idx) = self.fn_map.get(func.name.as_str()) {
                    let export_name = func.name.replace('_', "-");
                    exports.export(&export_name, ExportKind::Func, idx);
                }
            }
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
        // Helper: __i64_to_str(i64) -> ref $string
        self.emit_i64_to_str_helper(&mut codes);
        // Helper: __f64_to_str(f64) -> ref $string
        self.emit_f64_to_str_helper(&mut codes);
        // Helper: __parse_i32(ref $string) -> ref $Result
        if self.helper_parse_i32.is_some() {
            self.emit_parse_i32_helper(&mut codes);
        }
        // Helper: __parse_i64(ref $string) -> ref $Result_i64_String
        if self.helper_parse_i64.is_some() {
            self.emit_parse_i64_helper(&mut codes);
        }
        // Helper: __parse_f64(ref $string) -> ref $Result_f64_String
        if self.helper_parse_f64.is_some() {
            self.emit_parse_f64_helper(&mut codes);
        }

        // User functions
        for &idx in &reachable_user_indices {
            let func = &mir.functions[idx];
            let canonical = normalize_intrinsic(&func.name);
            if self.is_builtin_name(canonical) {
                // Builtin functions are inlined at call sites — emit a stub body
                // that just returns a default value to satisfy validation.
                self.emit_builtin_stub(&mut codes, func);
            } else {
                self.emit_function(&mut codes, func);
            }
        }

        // Global: heap_ptr for legacy I/O buffer allocation and VecLiteral fallback.
        // Retained for backward compatibility with call_indirect-based HOF dispatch.
        let mut globals = GlobalSection::new();
        globals.global(
            GlobalType {
                val_type: ValType::I32,
                mutable: true,
                shared: false,
            },
            &wasm_encoder::ConstExpr::i32_const(self.data_offset as i32),
        );

        // Data section: active segments first, then passive (string literals)
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
        // Passive segments for string literals (consumed by array.new_data)
        for bytes in &self.string_data_segs {
            data.segment(DataSegment {
                mode: wasm_encoder::DataSegmentMode::Passive,
                data: bytes.iter().copied(),
            });
        }

        // Table section — for indirect calls (higher-order functions)
        let total_funcs =
            num_imports + helper_fns.len() as u32 + reachable_user_indices.len() as u32;
        let mut tables = wasm_encoder::TableSection::new();
        tables.table(wasm_encoder::TableType {
            element_type: wasm_encoder::RefType::FUNCREF,
            minimum: total_funcs as u64,
            maximum: Some(total_funcs as u64),
            table64: false,
            shared: false,
        });

        // Element section — populate table with all function refs
        let mut elements = wasm_encoder::ElementSection::new();
        let func_indices: Vec<u32> = (0..total_funcs).collect();
        elements.active(
            Some(0),
            &wasm_encoder::ConstExpr::i32_const(0),
            wasm_encoder::Elements::Functions(std::borrow::Cow::Borrowed(&func_indices)),
        );

        // Assemble module
        let mut module = wasm_encoder::Module::new();
        module.section(&self.types.section);
        module.section(&imports);
        module.section(&functions);
        module.section(&tables);
        module.section(&memories);
        module.section(&globals);
        module.section(&exports);
        module.section(&elements);
        // DataCount section required for passive data segments (array.new_data)
        let total_data_segs = self.data_segs.len() as u32 + self.string_data_segs.len() as u32;
        module.section(&wasm_encoder::DataCountSection {
            count: total_data_segs,
        });
        module.section(&codes);
        module.section(&data);

        // Name section: emit function names for debug/profiling
        let mut name_section = wasm_encoder::NameSection::new();
        name_section.module("arukellt");
        let mut func_names = wasm_encoder::NameMap::new();
        // Import names (dynamically assigned)
        func_names.append(0, "wasi:fd_write");
        if needs_fs {
            func_names.append(1, "wasi:path_open");
            func_names.append(2, "wasi:fd_read");
            func_names.append(3, "wasi:fd_close");
        }
        // Helper function names (sorted by index for NameMap)
        let mut helpers: Vec<(u32, &str)> = self.fn_map.iter()
            .filter(|(_, idx)| **idx >= num_imports && **idx < user_base)
            .map(|(name, idx)| (*idx, name.as_str()))
            .collect();
        helpers.sort_by_key(|(idx, _)| *idx);
        for (idx, name) in helpers {
            func_names.append(idx, name);
        }
        // User function names
        for (i, &mir_idx) in reachable_user_indices.iter().enumerate() {
            let wasm_idx = user_base + i as u32;
            func_names.append(wasm_idx, &mir.functions[mir_idx].name);
        }
        name_section.functions(&func_names);
        module.section(&name_section);

        module.finish()
    }

    /// Scan MIR for filesystem builtins (fs_read_file, fs_write_file) to
    /// determine if path_open/fd_read/fd_close WASI imports are needed.
    fn mir_uses_fs(mir: &MirModule, reachable: &[usize]) -> bool {
        for &idx in reachable {
            let func = &mir.functions[idx];
            for block in &func.blocks {
                for stmt in &block.stmts {
                    if Self::stmt_uses_fs(stmt) {
                        return true;
                    }
                }
            }
        }
        false
    }

    fn stmt_uses_fs(stmt: &MirStmt) -> bool {
        match stmt {
            MirStmt::CallBuiltin { name, .. } => {
                name == "fs_read_file" || name == "fs_write_file"
            }
            MirStmt::Assign(_, rvalue) => Self::rvalue_uses_fs(rvalue),
            MirStmt::IfStmt { cond, then_body, else_body } => {
                Self::operand_uses_fs(cond)
                    || then_body.iter().any(|s| Self::stmt_uses_fs(s))
                    || else_body.iter().any(|s| Self::stmt_uses_fs(s))
            }
            MirStmt::WhileStmt { cond, body } => {
                Self::operand_uses_fs(cond)
                    || body.iter().any(|s| Self::stmt_uses_fs(s))
            }
            MirStmt::Return(Some(op)) => Self::operand_uses_fs(op),
            _ => false,
        }
    }

    fn rvalue_uses_fs(rvalue: &Rvalue) -> bool {
        match rvalue {
            Rvalue::Use(op) => Self::operand_uses_fs(op),
            Rvalue::BinaryOp(_, l, r) => Self::operand_uses_fs(l) || Self::operand_uses_fs(r),
            Rvalue::UnaryOp(_, op) => Self::operand_uses_fs(op),
            _ => false,
        }
    }

    fn operand_uses_fs(op: &Operand) -> bool {
        match op {
            Operand::Call(name, args) => {
                if name == "fs_read_file" || name == "fs_write_file" {
                    return true;
                }
                args.iter().any(|a| Self::operand_uses_fs(a))
            }
            _ => false,
        }
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

        // Exported functions are also roots (for Component Model exports)
        for (idx, func) in mir.functions.iter().enumerate() {
            if func.is_exported && func.name != "main" && !func.name.starts_with("__") {
                push_root(idx, &mut reachable, &mut queue);
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
        // ── String: bare packed i8 array (no wrapper struct) ──
        // (type $string (array (mut i8)))
        self.string_ty = self
            .types
            .add_array("$string", mutable_field(StorageType::I8));

        // ── Vec backing arrays ──
        // (type $arr_i32 (array (mut i32)))
        self.arr_i32_ty = self
            .types
            .add_array("$arr_i32", mutable_field(StorageType::Val(ValType::I32)));
        // (type $arr_i64 (array (mut i64)))
        self.arr_i64_ty = self
            .types
            .add_array("$arr_i64", mutable_field(StorageType::Val(ValType::I64)));
        // (type $arr_f64 (array (mut f64)))
        self.arr_f64_ty = self
            .types
            .add_array("$arr_f64", mutable_field(StorageType::Val(ValType::F64)));
        // (type $arr_string (array (mut (ref null $string))))
        self.arr_string_ty = self.types.add_array(
            "$arr_string",
            mutable_field(StorageType::Val(ref_nullable(self.string_ty))),
        );

        // ── Vec structs: data ref + len (capacity = array.len) ──
        // (type $vec_i32 (struct (field (mut (ref $arr_i32))) (field (mut i32))))
        self.vec_i32_ty = self.types.add_struct(
            "$vec_i32",
            &[
                mutable_field(StorageType::Val(ref_nullable(self.arr_i32_ty))),
                mutable_field(StorageType::Val(ValType::I32)),
            ],
        );
        self.vec_i64_ty = self.types.add_struct(
            "$vec_i64",
            &[
                mutable_field(StorageType::Val(ref_nullable(self.arr_i64_ty))),
                mutable_field(StorageType::Val(ValType::I32)),
            ],
        );
        self.vec_f64_ty = self.types.add_struct(
            "$vec_f64",
            &[
                mutable_field(StorageType::Val(ref_nullable(self.arr_f64_ty))),
                mutable_field(StorageType::Val(ValType::I32)),
            ],
        );
        self.vec_string_ty = self.types.add_struct(
            "$vec_string",
            &[
                mutable_field(StorageType::Val(ref_nullable(self.arr_string_ty))),
                mutable_field(StorageType::Val(ValType::I32)),
            ],
        );

        // HashMap<i32, i32>: struct { keys: ref $arr_i32, values: ref $arr_i32, count: i32 }
        self.hashmap_i32_i32_ty = self.types.add_struct(
            "$hashmap_i32_i32",
            &[
                mutable_field(StorageType::Val(ref_nullable(self.arr_i32_ty))),
                mutable_field(StorageType::Val(ref_nullable(self.arr_i32_ty))),
                mutable_field(StorageType::Val(ValType::I32)),
            ],
        );
        self.struct_gc_types
            .insert("__hashmap_i32_i32".to_string(), self.hashmap_i32_i32_ty);

        // ── User-defined structs ──
        // Topologically sort structs so field-type dependencies are registered first
        let struct_defs = &mir.type_table.struct_defs;
        let mut sorted_structs: Vec<&String> = Vec::new();
        let mut visited: HashSet<&String> = HashSet::new();
        fn topo_visit<'a>(
            name: &'a String,
            defs: &'a HashMap<String, Vec<(String, String)>>,
            visited: &mut HashSet<&'a String>,
            sorted: &mut Vec<&'a String>,
        ) {
            if visited.contains(name) {
                return;
            }
            visited.insert(name);
            if let Some(fields) = defs.get(name) {
                for (_, fty) in fields {
                    if defs.contains_key(fty.as_str()) {
                        topo_visit(fty, defs, visited, sorted);
                    }
                }
            }
            sorted.push(name);
        }
        for sname in struct_defs.keys() {
            topo_visit(sname, struct_defs, &mut visited, &mut sorted_structs);
        }
        for sname in &sorted_structs {
            let fields = &struct_defs[*sname];
            let gc_fields: Vec<FieldType> = fields
                .iter()
                .map(|(_, ty)| mutable_field(StorageType::Val(self.field_valtype(ty))))
                .collect();
            let idx = self.types.add_struct(sname, &gc_fields);
            self.struct_gc_types.insert((*sname).clone(), idx);
        }

        // ── Vec<Struct> types: scan MIR for Vec_new_* calls with struct names ──
        {
            let mut vec_struct_names: HashSet<String> = HashSet::new();
            for func in &mir.functions {
                for block in &func.blocks {
                    for stmt in &block.stmts {
                        self.scan_operands_for_vec_struct(stmt, struct_defs, &mut vec_struct_names);
                    }
                }
            }
            for sname in &vec_struct_names {
                if let Some(&struct_ty_idx) = self.struct_gc_types.get(sname) {
                    let arr_ty = self.types.add_array(
                        &format!("$arr_{}", sname),
                        mutable_field(StorageType::Val(ref_nullable(struct_ty_idx))),
                    );
                    let vec_ty = self.types.add_struct(
                        &format!("$vec_{}", sname),
                        &[
                            mutable_field(StorageType::Val(ref_nullable(arr_ty))),
                            mutable_field(StorageType::Val(ValType::I32)),
                        ],
                    );
                    self.custom_vec_types
                        .insert(sname.clone(), (arr_ty, vec_ty));
                }
            }
        }

        // ── User-defined enums: subtype hierarchy (rec group) ──
        // Each enum is emitted as one rec group so that structurally
        // identical variants (e.g., unit variants) are type-distinct.
        // Topological sort: enums whose variant fields reference other enums must be processed after them.
        let enum_names: Vec<String> = mir.type_table.enum_defs.keys().cloned().collect();
        let mut enum_order: Vec<String> = Vec::new();
        let mut enum_visited: HashSet<String> = HashSet::new();
        fn enum_topo_visit(
            name: &str,
            enum_defs: &HashMap<String, Vec<(String, Vec<String>)>>,
            visited: &mut HashSet<String>,
            order: &mut Vec<String>,
        ) {
            if visited.contains(name) {
                return;
            }
            visited.insert(name.to_string());
            if let Some(variants) = enum_defs.get(name) {
                for (_, field_types) in variants {
                    for ft in field_types {
                        if enum_defs.contains_key(ft.as_str()) {
                            enum_topo_visit(ft, enum_defs, visited, order);
                        }
                    }
                }
            }
            order.push(name.to_string());
        }
        for ename in &enum_names {
            enum_topo_visit(
                ename,
                &mir.type_table.enum_defs,
                &mut enum_visited,
                &mut enum_order,
            );
        }
        for ename in &enum_order {
            let variants = mir.type_table.enum_defs.get(ename).unwrap();
            let variant_fields: Vec<(String, Vec<FieldType>)> = variants
                .iter()
                .map(|(vname, field_types)| {
                    let gc_fields: Vec<FieldType> = field_types
                        .iter()
                        .map(|ty| mutable_field(StorageType::Val(self.field_valtype(ty))))
                        .collect();
                    (vname.clone(), gc_fields)
                })
                .collect();

            let (base_idx, variant_indices) = self.types.add_enum_rec_group(ename, &variant_fields);
            self.enum_base_types.insert(ename.clone(), base_idx);

            let mut variant_map = HashMap::new();
            for (vname, v_idx) in variant_indices {
                variant_map.insert(vname, v_idx);
            }
            self.enum_variant_types.insert(ename.clone(), variant_map);

            // Store field type names for enum payload type resolution
            for (vname, field_types) in variants {
                self.enum_variant_field_types
                    .insert((ename.clone(), vname.clone()), field_types.clone());
            }
        }
    }

    // ── Helper function bodies ───────────────────────────────────

    fn emit_print_str_ln_helper(&self, codes: &mut CodeSection, newline_off: u32) {
        let ma = MemArg {
            offset: 0,
            align: 2,
            memory_index: 0,
        };
        let ma1 = MemArg {
            offset: 0,
            align: 0,
            memory_index: 0,
        };
        let _str_ref = ref_nullable(self.string_ty);
        // Param 0 = (ref null $string). Locals: len (i32), i (i32).
        // Note: param 0 uses slot 0 but is ref type (not declared in local_types).
        let mut f = Function::new([(1, ValType::I32), (1, ValType::I32)]);

        // Get string length: array.len
        f.instruction(&Instruction::LocalGet(0)); // ref $string
        f.instruction(&Instruction::ArrayLen);
        f.instruction(&Instruction::LocalSet(1)); // local 1 = len

        // Copy GC string bytes to linear memory at SCRATCH.
        // for i = 0; i < len; i++ { mem[SCRATCH + i] = array.get_u $string (param0) i }
        f.instruction(&Instruction::I32Const(0));
        f.instruction(&Instruction::LocalSet(2)); // i = 0
        f.instruction(&Instruction::Block(wasm_encoder::BlockType::Empty));
        f.instruction(&Instruction::Loop(wasm_encoder::BlockType::Empty));
        // if i >= len, break
        f.instruction(&Instruction::LocalGet(2));
        f.instruction(&Instruction::LocalGet(1));
        f.instruction(&Instruction::I32GeU);
        f.instruction(&Instruction::BrIf(1)); // break outer block
        // mem[SCRATCH + i] = array.get_u $string param0 i
        f.instruction(&Instruction::I32Const(SCRATCH as i32));
        f.instruction(&Instruction::LocalGet(2));
        f.instruction(&Instruction::I32Add); // SCRATCH + i
        f.instruction(&Instruction::LocalGet(0)); // ref $string
        f.instruction(&Instruction::LocalGet(2)); // i
        f.instruction(&Instruction::ArrayGetU(self.string_ty));
        f.instruction(&Instruction::I32Store8(ma1));
        // i++
        f.instruction(&Instruction::LocalGet(2));
        f.instruction(&Instruction::I32Const(1));
        f.instruction(&Instruction::I32Add);
        f.instruction(&Instruction::LocalSet(2));
        f.instruction(&Instruction::Br(0)); // continue loop
        f.instruction(&Instruction::End); // end loop
        f.instruction(&Instruction::End); // end block

        // Set up IOV: base = SCRATCH, len = string length
        f.instruction(&Instruction::I32Const(IOV_BASE as i32));
        f.instruction(&Instruction::I32Const(SCRATCH as i32));
        f.instruction(&Instruction::I32Store(ma));
        f.instruction(&Instruction::I32Const(IOV_LEN as i32));
        f.instruction(&Instruction::LocalGet(1)); // len
        f.instruction(&Instruction::I32Store(ma));
        // fd_write(1, &iov, 1, &nwritten)
        f.instruction(&Instruction::I32Const(1)); // fd=stdout
        f.instruction(&Instruction::I32Const(IOV_BASE as i32));
        f.instruction(&Instruction::I32Const(1)); // iovs_len
        f.instruction(&Instruction::I32Const(NWRITTEN as i32));
        f.instruction(&Instruction::Call(self.wasi_fd_write));
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
        f.instruction(&Instruction::Call(self.wasi_fd_write));
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
        f.instruction(&Instruction::Call(self.wasi_fd_write));
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
        f.instruction(&Instruction::Call(self.wasi_fd_write));
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
        f.instruction(&Instruction::Call(self.wasi_fd_write));
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
        f.instruction(&Instruction::Call(self.wasi_fd_write));
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
        f.instruction(&Instruction::Call(self.wasi_fd_write));
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
        f.instruction(&Instruction::Call(self.wasi_fd_write));
        f.instruction(&Instruction::Drop);
        f.instruction(&Instruction::End);
        codes.function(&f);
    }

    fn emit_i32_to_str_helper(&mut self, codes: &mut CodeSection) {
        // GC-native: converts i32 → (ref $string) as a GC byte array.
        // Algorithm: extract digits to scratch memory, then build GC array.
        let ma0 = MemArg {
            offset: 0,
            align: 0,
            memory_index: 0,
        };
        let str_ref = ref_nullable(self.string_ty);
        let mut f = Function::new([
            (1, ValType::I32), // local 1: is_neg
            (1, ValType::I32), // local 2: abs_val
            (1, ValType::I32), // local 3: digit_count (total string length)
            (1, ValType::I32), // local 4: temp/loop counter
            (1, str_ref),      // local 5: result GC string
        ]);

        // Determine sign and absolute value
        f.instruction(&Instruction::LocalGet(0)); // value
        f.instruction(&Instruction::I32Const(0));
        f.instruction(&Instruction::I32LtS);
        f.instruction(&Instruction::If(wasm_encoder::BlockType::Empty));
        f.instruction(&Instruction::I32Const(1));
        f.instruction(&Instruction::LocalSet(1)); // is_neg = 1
        f.instruction(&Instruction::I32Const(0));
        f.instruction(&Instruction::LocalGet(0));
        f.instruction(&Instruction::I32Sub);
        f.instruction(&Instruction::LocalSet(2)); // abs_val = -value
        f.instruction(&Instruction::Else);
        f.instruction(&Instruction::LocalGet(0));
        f.instruction(&Instruction::LocalSet(2)); // abs_val = value
        f.instruction(&Instruction::End);

        // Handle zero specially
        f.instruction(&Instruction::LocalGet(2));
        f.instruction(&Instruction::I32Eqz);
        f.instruction(&Instruction::If(wasm_encoder::BlockType::Result(str_ref)));
        // Create "0" as a 1-byte GC array: array.new $string fill=48('0') len=1
        f.instruction(&Instruction::I32Const(48)); // fill value = '0'
        f.instruction(&Instruction::I32Const(1)); // length = 1
        f.instruction(&Instruction::ArrayNew(self.string_ty));
        f.instruction(&Instruction::Return);
        f.instruction(&Instruction::Else);
        // dummy ref for type consistency (never used)
        f.instruction(&Instruction::I32Const(0)); // fill value
        f.instruction(&Instruction::I32Const(0)); // length
        f.instruction(&Instruction::ArrayNew(self.string_ty));
        f.instruction(&Instruction::End);
        f.instruction(&Instruction::Drop); // drop the else-branch dummy

        // Extract digits to scratch memory in reverse order
        // scratch[0..11] holds digits (max 11 for "-2147483648")
        f.instruction(&Instruction::I32Const(0));
        f.instruction(&Instruction::LocalSet(3)); // digit_count = 0
        f.instruction(&Instruction::Block(wasm_encoder::BlockType::Empty));
        f.instruction(&Instruction::Loop(wasm_encoder::BlockType::Empty));
        f.instruction(&Instruction::LocalGet(2));
        f.instruction(&Instruction::I32Eqz);
        f.instruction(&Instruction::BrIf(1)); // break if abs_val == 0
        // scratch[11 - digit_count] = (abs_val % 10) + '0'
        f.instruction(&Instruction::I32Const(SCRATCH as i32 + 11));
        f.instruction(&Instruction::LocalGet(3));
        f.instruction(&Instruction::I32Sub); // addr = SCRATCH+11-digit_count
        f.instruction(&Instruction::LocalGet(2));
        f.instruction(&Instruction::I32Const(10));
        f.instruction(&Instruction::I32RemU);
        f.instruction(&Instruction::I32Const(48));
        f.instruction(&Instruction::I32Add); // digit char
        f.instruction(&Instruction::I32Store8(ma0));
        // abs_val /= 10
        f.instruction(&Instruction::LocalGet(2));
        f.instruction(&Instruction::I32Const(10));
        f.instruction(&Instruction::I32DivU);
        f.instruction(&Instruction::LocalSet(2));
        // digit_count++
        f.instruction(&Instruction::LocalGet(3));
        f.instruction(&Instruction::I32Const(1));
        f.instruction(&Instruction::I32Add);
        f.instruction(&Instruction::LocalSet(3));
        f.instruction(&Instruction::Br(0)); // continue
        f.instruction(&Instruction::End); // end loop
        f.instruction(&Instruction::End); // end block

        // If negative, prepend '-'
        f.instruction(&Instruction::LocalGet(1));
        f.instruction(&Instruction::If(wasm_encoder::BlockType::Empty));
        f.instruction(&Instruction::LocalGet(3));
        f.instruction(&Instruction::I32Const(1));
        f.instruction(&Instruction::I32Add);
        f.instruction(&Instruction::LocalSet(3)); // digit_count++ for '-'
        // scratch[12 - digit_count] = '-'
        f.instruction(&Instruction::I32Const(SCRATCH as i32 + 12));
        f.instruction(&Instruction::LocalGet(3));
        f.instruction(&Instruction::I32Sub);
        f.instruction(&Instruction::I32Const(45)); // '-'
        f.instruction(&Instruction::I32Store8(ma0));
        f.instruction(&Instruction::End);

        // Create GC string from scratch bytes
        // The string starts at scratch[12 - digit_count] and has digit_count bytes
        f.instruction(&Instruction::I32Const(0)); // fill value (will be overwritten)
        f.instruction(&Instruction::LocalGet(3)); // length
        f.instruction(&Instruction::ArrayNew(self.string_ty)); // array.new $string fill=0 len=digit_count
        f.instruction(&Instruction::LocalSet(5)); // store GC string

        // Copy bytes from scratch to GC array
        f.instruction(&Instruction::I32Const(0));
        f.instruction(&Instruction::LocalSet(4)); // i = 0
        f.instruction(&Instruction::Block(wasm_encoder::BlockType::Empty));
        f.instruction(&Instruction::Loop(wasm_encoder::BlockType::Empty));
        f.instruction(&Instruction::LocalGet(4));
        f.instruction(&Instruction::LocalGet(3));
        f.instruction(&Instruction::I32GeU);
        f.instruction(&Instruction::BrIf(1)); // break if i >= digit_count
        // array.set $string result[i] = mem[SCRATCH + 12 - digit_count + i]
        f.instruction(&Instruction::LocalGet(5)); // ref $string
        f.instruction(&Instruction::LocalGet(4)); // i
        f.instruction(&Instruction::I32Const(SCRATCH as i32 + 12));
        f.instruction(&Instruction::LocalGet(3));
        f.instruction(&Instruction::I32Sub);
        f.instruction(&Instruction::LocalGet(4));
        f.instruction(&Instruction::I32Add); // addr = SCRATCH+12-digit_count+i
        f.instruction(&Instruction::I32Load8U(ma0)); // byte from scratch
        f.instruction(&Instruction::ArraySet(self.string_ty));
        // i++
        f.instruction(&Instruction::LocalGet(4));
        f.instruction(&Instruction::I32Const(1));
        f.instruction(&Instruction::I32Add);
        f.instruction(&Instruction::LocalSet(4));
        f.instruction(&Instruction::Br(0)); // continue
        f.instruction(&Instruction::End); // end loop
        f.instruction(&Instruction::End); // end block

        // Return the GC string
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
        f.instruction(&Instruction::Call(self.wasi_fd_write));
        f.instruction(&Instruction::Drop);
        f.instruction(&Instruction::End);
        codes.function(&f);
    }

    /// Emit __i64_to_str(val: i64) -> ref $string
    /// Converts an i64 to decimal string representation as GC byte array.
    fn emit_i64_to_str_helper(&mut self, codes: &mut CodeSection) {
        let ma0 = MemArg {
            offset: 0,
            align: 0,
            memory_index: 0,
        };
        let str_ref = ref_nullable(self.string_ty);
        // Use scratch area at SCRATCH offset. Max i64 string = 20 chars ("-9223372036854775808")
        // We'll use scratch[SCRATCH..SCRATCH+24] as digit buffer
        let buf_end = SCRATCH as i32 + 23; // rightmost digit position

        let mut f = Function::new([
            (1, ValType::I32), // local 1: is_neg
            (1, ValType::I64), // local 2: abs_val (i64)
            (1, ValType::I32), // local 3: digit_count
            (1, ValType::I32), // local 4: loop counter i
            (1, str_ref),      // local 5: result GC string
        ]);

        // Determine sign and absolute value
        f.instruction(&Instruction::LocalGet(0)); // i64 value
        f.instruction(&Instruction::I64Const(0));
        f.instruction(&Instruction::I64LtS);
        f.instruction(&Instruction::If(wasm_encoder::BlockType::Empty));
        f.instruction(&Instruction::I32Const(1));
        f.instruction(&Instruction::LocalSet(1)); // is_neg = 1
        f.instruction(&Instruction::I64Const(0));
        f.instruction(&Instruction::LocalGet(0));
        f.instruction(&Instruction::I64Sub);
        f.instruction(&Instruction::LocalSet(2)); // abs_val = -value
        f.instruction(&Instruction::Else);
        f.instruction(&Instruction::LocalGet(0));
        f.instruction(&Instruction::LocalSet(2)); // abs_val = value
        f.instruction(&Instruction::End);

        // Handle zero specially
        f.instruction(&Instruction::LocalGet(2));
        f.instruction(&Instruction::I64Eqz);
        f.instruction(&Instruction::If(wasm_encoder::BlockType::Result(str_ref)));
        f.instruction(&Instruction::I32Const(48)); // '0'
        f.instruction(&Instruction::I32Const(1));
        f.instruction(&Instruction::ArrayNew(self.string_ty));
        f.instruction(&Instruction::Return);
        f.instruction(&Instruction::Else);
        f.instruction(&Instruction::I32Const(0));
        f.instruction(&Instruction::I32Const(0));
        f.instruction(&Instruction::ArrayNew(self.string_ty));
        f.instruction(&Instruction::End);
        f.instruction(&Instruction::Drop);

        // Extract digits to scratch memory in reverse order
        f.instruction(&Instruction::I32Const(0));
        f.instruction(&Instruction::LocalSet(3)); // digit_count = 0
        f.instruction(&Instruction::Block(wasm_encoder::BlockType::Empty));
        f.instruction(&Instruction::Loop(wasm_encoder::BlockType::Empty));
        f.instruction(&Instruction::LocalGet(2));
        f.instruction(&Instruction::I64Eqz);
        f.instruction(&Instruction::BrIf(1)); // break if abs_val == 0
        // scratch[buf_end - digit_count] = (abs_val % 10) + '0'
        f.instruction(&Instruction::I32Const(buf_end));
        f.instruction(&Instruction::LocalGet(3));
        f.instruction(&Instruction::I32Sub); // addr
        f.instruction(&Instruction::LocalGet(2));
        f.instruction(&Instruction::I64Const(10));
        f.instruction(&Instruction::I64RemU);
        f.instruction(&Instruction::I32WrapI64); // digit as i32
        f.instruction(&Instruction::I32Const(48));
        f.instruction(&Instruction::I32Add); // digit char
        f.instruction(&Instruction::I32Store8(ma0));
        // abs_val /= 10
        f.instruction(&Instruction::LocalGet(2));
        f.instruction(&Instruction::I64Const(10));
        f.instruction(&Instruction::I64DivU);
        f.instruction(&Instruction::LocalSet(2));
        // digit_count++
        f.instruction(&Instruction::LocalGet(3));
        f.instruction(&Instruction::I32Const(1));
        f.instruction(&Instruction::I32Add);
        f.instruction(&Instruction::LocalSet(3));
        f.instruction(&Instruction::Br(0));
        f.instruction(&Instruction::End); // end loop
        f.instruction(&Instruction::End); // end block

        // If negative, prepend '-'
        f.instruction(&Instruction::LocalGet(1));
        f.instruction(&Instruction::If(wasm_encoder::BlockType::Empty));
        f.instruction(&Instruction::LocalGet(3));
        f.instruction(&Instruction::I32Const(1));
        f.instruction(&Instruction::I32Add);
        f.instruction(&Instruction::LocalSet(3));
        f.instruction(&Instruction::I32Const(buf_end + 1));
        f.instruction(&Instruction::LocalGet(3));
        f.instruction(&Instruction::I32Sub);
        f.instruction(&Instruction::I32Const(45)); // '-'
        f.instruction(&Instruction::I32Store8(ma0));
        f.instruction(&Instruction::End);

        // Create GC string and copy bytes
        f.instruction(&Instruction::I32Const(0));
        f.instruction(&Instruction::LocalGet(3));
        f.instruction(&Instruction::ArrayNew(self.string_ty));
        f.instruction(&Instruction::LocalSet(5));

        f.instruction(&Instruction::I32Const(0));
        f.instruction(&Instruction::LocalSet(4)); // i = 0
        f.instruction(&Instruction::Block(wasm_encoder::BlockType::Empty));
        f.instruction(&Instruction::Loop(wasm_encoder::BlockType::Empty));
        f.instruction(&Instruction::LocalGet(4));
        f.instruction(&Instruction::LocalGet(3));
        f.instruction(&Instruction::I32GeU);
        f.instruction(&Instruction::BrIf(1));
        f.instruction(&Instruction::LocalGet(5)); // ref $string
        f.instruction(&Instruction::LocalGet(4)); // i
        f.instruction(&Instruction::I32Const(buf_end + 1));
        f.instruction(&Instruction::LocalGet(3));
        f.instruction(&Instruction::I32Sub);
        f.instruction(&Instruction::LocalGet(4));
        f.instruction(&Instruction::I32Add); // addr = buf_end+1-digit_count+i
        f.instruction(&Instruction::I32Load8U(ma0));
        f.instruction(&Instruction::ArraySet(self.string_ty));
        f.instruction(&Instruction::LocalGet(4));
        f.instruction(&Instruction::I32Const(1));
        f.instruction(&Instruction::I32Add);
        f.instruction(&Instruction::LocalSet(4));
        f.instruction(&Instruction::Br(0));
        f.instruction(&Instruction::End);
        f.instruction(&Instruction::End);

        f.instruction(&Instruction::LocalGet(5));
        f.instruction(&Instruction::End);
        codes.function(&f);
    }

    /// Emit __f64_to_str(val: f64) -> ref $string
    /// Converts an f64 to decimal string with up to 15 significant digits.
    /// Uses integer + fractional parts separately.
    fn emit_f64_to_str_helper(&mut self, codes: &mut CodeSection) {
        let ma0 = MemArg {
            offset: 0,
            align: 0,
            memory_index: 0,
        };
        let str_ref = ref_nullable(self.string_ty);
        // Strategy: write characters to scratch memory, then copy to GC array.
        // Max f64 printed length: sign + 20 int digits + '.' + 15 frac digits ≈ 40
        // Use scratch area at offset 64..128
        let buf_base = 64_i32;
        let _buf_size = 64_i32;

        let mut f = Function::new([
            (1, ValType::I32), // local 1: is_neg
            (1, ValType::F64), // local 2: abs_val
            (1, ValType::I32), // local 3: write_pos (cursor into scratch)
            (1, ValType::I64), // local 4: int_part (as i64)
            (1, ValType::F64), // local 5: frac_part
            (1, ValType::I32), // local 6: temp / digit_count for int part
            (1, ValType::I32), // local 7: int_start position in scratch
            (1, ValType::I32), // local 8: loop counter
            (1, str_ref),      // local 9: result GC string
            (1, ValType::I32), // local 10: total_len
            (1, ValType::I64), // local 11: temp i64 for int digit extraction
            (1, ValType::I32), // local 12: frac_digits count
        ]);

        // Determine sign
        f.instruction(&Instruction::LocalGet(0)); // f64 value
        f.instruction(&Instruction::F64Const(0.0));
        f.instruction(&Instruction::F64Lt);
        f.instruction(&Instruction::If(wasm_encoder::BlockType::Empty));
        f.instruction(&Instruction::I32Const(1));
        f.instruction(&Instruction::LocalSet(1)); // is_neg = 1
        f.instruction(&Instruction::LocalGet(0));
        f.instruction(&Instruction::F64Neg);
        f.instruction(&Instruction::LocalSet(2)); // abs_val = -value
        f.instruction(&Instruction::Else);
        f.instruction(&Instruction::LocalGet(0));
        f.instruction(&Instruction::LocalSet(2)); // abs_val = value
        f.instruction(&Instruction::End);

        // int_part = trunc(abs_val) as i64
        f.instruction(&Instruction::LocalGet(2));
        f.instruction(&Instruction::I64TruncSatF64U);
        f.instruction(&Instruction::LocalSet(4)); // int_part

        // frac_part = abs_val - f64(int_part)
        f.instruction(&Instruction::LocalGet(2));
        f.instruction(&Instruction::LocalGet(4));
        f.instruction(&Instruction::F64ConvertI64U);
        f.instruction(&Instruction::F64Sub);
        f.instruction(&Instruction::LocalSet(5)); // frac_part

        // Write position starts after potential '-' sign
        f.instruction(&Instruction::I32Const(buf_base));
        f.instruction(&Instruction::LocalSet(3)); // write_pos = buf_base

        // If negative, write '-'
        f.instruction(&Instruction::LocalGet(1));
        f.instruction(&Instruction::If(wasm_encoder::BlockType::Empty));
        f.instruction(&Instruction::LocalGet(3));
        f.instruction(&Instruction::I32Const(45)); // '-'
        f.instruction(&Instruction::I32Store8(ma0));
        f.instruction(&Instruction::LocalGet(3));
        f.instruction(&Instruction::I32Const(1));
        f.instruction(&Instruction::I32Add);
        f.instruction(&Instruction::LocalSet(3));
        f.instruction(&Instruction::End);

        // Convert integer part to digits.
        // Handle zero case
        f.instruction(&Instruction::LocalGet(4));
        f.instruction(&Instruction::I64Eqz);
        f.instruction(&Instruction::If(wasm_encoder::BlockType::Empty));
        // Write '0'
        f.instruction(&Instruction::LocalGet(3));
        f.instruction(&Instruction::I32Const(48)); // '0'
        f.instruction(&Instruction::I32Store8(ma0));
        f.instruction(&Instruction::LocalGet(3));
        f.instruction(&Instruction::I32Const(1));
        f.instruction(&Instruction::I32Add);
        f.instruction(&Instruction::LocalSet(3));
        f.instruction(&Instruction::Else);
        // Non-zero: extract digits in reverse to a temp area, then copy forward
        // Use scratch at buf_base+40..buf_base+60 as temp digit buffer
        {
            let temp_end = buf_base + 59; // rightmost temp digit position
            f.instruction(&Instruction::I32Const(0));
            f.instruction(&Instruction::LocalSet(6)); // digit_count = 0
            f.instruction(&Instruction::LocalGet(4));
            f.instruction(&Instruction::LocalSet(11)); // temp_i64 = int_part
            f.instruction(&Instruction::Block(wasm_encoder::BlockType::Empty));
            f.instruction(&Instruction::Loop(wasm_encoder::BlockType::Empty));
            f.instruction(&Instruction::LocalGet(11));
            f.instruction(&Instruction::I64Eqz);
            f.instruction(&Instruction::BrIf(1));
            // temp[temp_end - digit_count] = (temp_i64 % 10) + '0'
            f.instruction(&Instruction::I32Const(temp_end));
            f.instruction(&Instruction::LocalGet(6));
            f.instruction(&Instruction::I32Sub);
            f.instruction(&Instruction::LocalGet(11));
            f.instruction(&Instruction::I64Const(10));
            f.instruction(&Instruction::I64RemU);
            f.instruction(&Instruction::I32WrapI64);
            f.instruction(&Instruction::I32Const(48));
            f.instruction(&Instruction::I32Add);
            f.instruction(&Instruction::I32Store8(ma0));
            // temp_i64 /= 10
            f.instruction(&Instruction::LocalGet(11));
            f.instruction(&Instruction::I64Const(10));
            f.instruction(&Instruction::I64DivU);
            f.instruction(&Instruction::LocalSet(11));
            // digit_count++
            f.instruction(&Instruction::LocalGet(6));
            f.instruction(&Instruction::I32Const(1));
            f.instruction(&Instruction::I32Add);
            f.instruction(&Instruction::LocalSet(6));
            f.instruction(&Instruction::Br(0));
            f.instruction(&Instruction::End); // end loop
            f.instruction(&Instruction::End); // end block

            // Copy digits from temp to write_pos
            f.instruction(&Instruction::I32Const(0));
            f.instruction(&Instruction::LocalSet(8)); // i = 0
            f.instruction(&Instruction::Block(wasm_encoder::BlockType::Empty));
            f.instruction(&Instruction::Loop(wasm_encoder::BlockType::Empty));
            f.instruction(&Instruction::LocalGet(8));
            f.instruction(&Instruction::LocalGet(6));
            f.instruction(&Instruction::I32GeU);
            f.instruction(&Instruction::BrIf(1));
            // scratch[write_pos + i] = scratch[temp_end+1-digit_count+i]
            f.instruction(&Instruction::LocalGet(3));
            f.instruction(&Instruction::LocalGet(8));
            f.instruction(&Instruction::I32Add); // dest addr
            f.instruction(&Instruction::I32Const(temp_end + 1));
            f.instruction(&Instruction::LocalGet(6));
            f.instruction(&Instruction::I32Sub);
            f.instruction(&Instruction::LocalGet(8));
            f.instruction(&Instruction::I32Add); // src addr
            f.instruction(&Instruction::I32Load8U(ma0));
            f.instruction(&Instruction::I32Store8(ma0));
            // i++
            f.instruction(&Instruction::LocalGet(8));
            f.instruction(&Instruction::I32Const(1));
            f.instruction(&Instruction::I32Add);
            f.instruction(&Instruction::LocalSet(8));
            f.instruction(&Instruction::Br(0));
            f.instruction(&Instruction::End);
            f.instruction(&Instruction::End);

            // write_pos += digit_count
            f.instruction(&Instruction::LocalGet(3));
            f.instruction(&Instruction::LocalGet(6));
            f.instruction(&Instruction::I32Add);
            f.instruction(&Instruction::LocalSet(3));
        }
        f.instruction(&Instruction::End); // end if/else for zero check

        // Write '.'
        f.instruction(&Instruction::LocalGet(3));
        f.instruction(&Instruction::I32Const(46)); // '.'
        f.instruction(&Instruction::I32Store8(ma0));
        f.instruction(&Instruction::LocalGet(3));
        f.instruction(&Instruction::I32Const(1));
        f.instruction(&Instruction::I32Add);
        f.instruction(&Instruction::LocalSet(3));

        // Write fractional digits
        // Multiply frac by 10 repeatedly, extract digit, up to 15 digits max
        // But trim trailing zeros (keep at least 1 digit)
        f.instruction(&Instruction::I32Const(0));
        f.instruction(&Instruction::LocalSet(12)); // frac_digits = 0
        f.instruction(&Instruction::Block(wasm_encoder::BlockType::Empty));
        f.instruction(&Instruction::Loop(wasm_encoder::BlockType::Empty));
        // if frac_digits >= 15, break
        f.instruction(&Instruction::LocalGet(12));
        f.instruction(&Instruction::I32Const(15));
        f.instruction(&Instruction::I32GeU);
        f.instruction(&Instruction::BrIf(1));
        // frac_part *= 10
        f.instruction(&Instruction::LocalGet(5));
        f.instruction(&Instruction::F64Const(10.0));
        f.instruction(&Instruction::F64Mul);
        f.instruction(&Instruction::LocalSet(5));
        // digit = trunc(frac_part) as i32
        f.instruction(&Instruction::LocalGet(5));
        f.instruction(&Instruction::I32TruncSatF64S);
        f.instruction(&Instruction::LocalSet(6)); // temp digit
        // frac_part -= f64(digit)
        f.instruction(&Instruction::LocalGet(5));
        f.instruction(&Instruction::LocalGet(6));
        f.instruction(&Instruction::F64ConvertI32S);
        f.instruction(&Instruction::F64Sub);
        f.instruction(&Instruction::LocalSet(5));
        // write digit + '0'
        f.instruction(&Instruction::LocalGet(3));
        f.instruction(&Instruction::LocalGet(6));
        f.instruction(&Instruction::I32Const(48));
        f.instruction(&Instruction::I32Add);
        f.instruction(&Instruction::I32Store8(ma0));
        f.instruction(&Instruction::LocalGet(3));
        f.instruction(&Instruction::I32Const(1));
        f.instruction(&Instruction::I32Add);
        f.instruction(&Instruction::LocalSet(3));
        // frac_digits++
        f.instruction(&Instruction::LocalGet(12));
        f.instruction(&Instruction::I32Const(1));
        f.instruction(&Instruction::I32Add);
        f.instruction(&Instruction::LocalSet(12));
        // if frac_part is effectively zero AND frac_digits >= 1, break
        f.instruction(&Instruction::LocalGet(5));
        f.instruction(&Instruction::F64Const(1e-14));
        f.instruction(&Instruction::F64Lt);
        f.instruction(&Instruction::LocalGet(12));
        f.instruction(&Instruction::I32Const(1));
        f.instruction(&Instruction::I32GeU);
        f.instruction(&Instruction::I32And);
        f.instruction(&Instruction::BrIf(1)); // break if frac ~= 0 and at least 1 digit
        f.instruction(&Instruction::Br(0)); // continue
        f.instruction(&Instruction::End); // end loop
        f.instruction(&Instruction::End); // end block

        // Now trim trailing zeros (but keep at least 1 fractional digit)
        // write_pos currently points past the last digit written
        // Scan backwards while scratch[write_pos-1] == '0' AND more than 1 frac digit
        f.instruction(&Instruction::Block(wasm_encoder::BlockType::Empty));
        f.instruction(&Instruction::Loop(wasm_encoder::BlockType::Empty));
        // if frac_digits <= 1, stop trimming
        f.instruction(&Instruction::LocalGet(12));
        f.instruction(&Instruction::I32Const(1));
        f.instruction(&Instruction::I32LeU);
        f.instruction(&Instruction::BrIf(1));
        // if scratch[write_pos - 1] != '0', stop
        f.instruction(&Instruction::LocalGet(3));
        f.instruction(&Instruction::I32Const(1));
        f.instruction(&Instruction::I32Sub);
        f.instruction(&Instruction::I32Load8U(ma0));
        f.instruction(&Instruction::I32Const(48)); // '0'
        f.instruction(&Instruction::I32Ne);
        f.instruction(&Instruction::BrIf(1));
        // write_pos--
        f.instruction(&Instruction::LocalGet(3));
        f.instruction(&Instruction::I32Const(1));
        f.instruction(&Instruction::I32Sub);
        f.instruction(&Instruction::LocalSet(3));
        // frac_digits--
        f.instruction(&Instruction::LocalGet(12));
        f.instruction(&Instruction::I32Const(1));
        f.instruction(&Instruction::I32Sub);
        f.instruction(&Instruction::LocalSet(12));
        f.instruction(&Instruction::Br(0));
        f.instruction(&Instruction::End);
        f.instruction(&Instruction::End);

        // total_len = write_pos - buf_base
        f.instruction(&Instruction::LocalGet(3));
        f.instruction(&Instruction::I32Const(buf_base));
        f.instruction(&Instruction::I32Sub);
        f.instruction(&Instruction::LocalSet(10)); // total_len

        // Create GC string
        f.instruction(&Instruction::I32Const(0));
        f.instruction(&Instruction::LocalGet(10));
        f.instruction(&Instruction::ArrayNew(self.string_ty));
        f.instruction(&Instruction::LocalSet(9));

        // Copy bytes from scratch to GC array
        f.instruction(&Instruction::I32Const(0));
        f.instruction(&Instruction::LocalSet(8)); // i = 0
        f.instruction(&Instruction::Block(wasm_encoder::BlockType::Empty));
        f.instruction(&Instruction::Loop(wasm_encoder::BlockType::Empty));
        f.instruction(&Instruction::LocalGet(8));
        f.instruction(&Instruction::LocalGet(10));
        f.instruction(&Instruction::I32GeU);
        f.instruction(&Instruction::BrIf(1));
        f.instruction(&Instruction::LocalGet(9));
        f.instruction(&Instruction::LocalGet(8));
        f.instruction(&Instruction::I32Const(buf_base));
        f.instruction(&Instruction::LocalGet(8));
        f.instruction(&Instruction::I32Add);
        f.instruction(&Instruction::I32Load8U(ma0));
        f.instruction(&Instruction::ArraySet(self.string_ty));
        f.instruction(&Instruction::LocalGet(8));
        f.instruction(&Instruction::I32Const(1));
        f.instruction(&Instruction::I32Add);
        f.instruction(&Instruction::LocalSet(8));
        f.instruction(&Instruction::Br(0));
        f.instruction(&Instruction::End);
        f.instruction(&Instruction::End);

        f.instruction(&Instruction::LocalGet(9));
        f.instruction(&Instruction::End);
        codes.function(&f);
    }

    /// Emit __parse_i32(s: ref $string) -> ref $Result
    /// Parses a decimal integer from a GC string, returns Ok(i32) or Err(String)
    fn emit_parse_i32_helper(&self, codes: &mut CodeSection) {
        let result_base = *self.enum_base_types.get("Result").unwrap();
        let result_ok = *self
            .enum_variant_types
            .get("Result")
            .unwrap()
            .get("Ok")
            .unwrap();
        let result_err = *self
            .enum_variant_types
            .get("Result")
            .unwrap()
            .get("Err")
            .unwrap();
        let _result_ref = ref_nullable(result_base);

        // locals: s(param0), len(1), i(2), neg(3), result(4), ch(5)
        let mut f = Function::new(vec![
            (1, ValType::I32), // len
            (1, ValType::I32), // i
            (1, ValType::I32), // neg
            (1, ValType::I32), // result
            (1, ValType::I32), // ch
        ]);

        // len = array.len(s)
        f.instruction(&Instruction::LocalGet(0));
        f.instruction(&Instruction::ArrayLen);
        f.instruction(&Instruction::LocalSet(1));

        // if len == 0: return Err("invalid number")
        f.instruction(&Instruction::LocalGet(1));
        f.instruction(&Instruction::I32Eqz);
        f.instruction(&Instruction::If(wasm_encoder::BlockType::Empty));
        {
            let seg = self.find_or_make_err_string();
            f.instruction(&Instruction::I32Const(0));
            f.instruction(&Instruction::I32Const(28));
            f.instruction(&Instruction::ArrayNewData {
                array_type_index: self.string_ty,
                array_data_index: seg,
            });
            f.instruction(&Instruction::StructNew(result_err));
            f.instruction(&Instruction::Return);
        }
        f.instruction(&Instruction::End);

        // Check for leading '-'
        f.instruction(&Instruction::LocalGet(0));
        f.instruction(&Instruction::I32Const(0));
        f.instruction(&Instruction::ArrayGetU(self.string_ty));
        f.instruction(&Instruction::I32Const(45)); // '-'
        f.instruction(&Instruction::I32Eq);
        f.instruction(&Instruction::If(wasm_encoder::BlockType::Empty));
        {
            f.instruction(&Instruction::I32Const(1));
            f.instruction(&Instruction::LocalSet(3)); // neg = 1
            f.instruction(&Instruction::I32Const(1));
            f.instruction(&Instruction::LocalSet(2)); // i = 1
            // if len == 1 (just "-"): return Err
            f.instruction(&Instruction::LocalGet(1));
            f.instruction(&Instruction::I32Const(1));
            f.instruction(&Instruction::I32Eq);
            f.instruction(&Instruction::If(wasm_encoder::BlockType::Empty));
            {
                let seg = self.find_or_make_err_string();
                f.instruction(&Instruction::I32Const(0));
                f.instruction(&Instruction::I32Const(28));
                f.instruction(&Instruction::ArrayNewData {
                    array_type_index: self.string_ty,
                    array_data_index: seg,
                });
                f.instruction(&Instruction::StructNew(result_err));
                f.instruction(&Instruction::Return);
            }
            f.instruction(&Instruction::End);
        }
        f.instruction(&Instruction::End);

        // Parse loop
        f.instruction(&Instruction::Block(wasm_encoder::BlockType::Empty));
        f.instruction(&Instruction::Loop(wasm_encoder::BlockType::Empty));
        {
            // if i >= len: break
            f.instruction(&Instruction::LocalGet(2));
            f.instruction(&Instruction::LocalGet(1));
            f.instruction(&Instruction::I32GeU);
            f.instruction(&Instruction::BrIf(1)); // break outer block

            // ch = array.get_u(s, i)
            f.instruction(&Instruction::LocalGet(0));
            f.instruction(&Instruction::LocalGet(2));
            f.instruction(&Instruction::ArrayGetU(self.string_ty));
            f.instruction(&Instruction::LocalSet(5));

            // if ch < '0' || ch > '9': return Err
            f.instruction(&Instruction::LocalGet(5));
            f.instruction(&Instruction::I32Const(48));
            f.instruction(&Instruction::I32LtU);
            f.instruction(&Instruction::LocalGet(5));
            f.instruction(&Instruction::I32Const(57));
            f.instruction(&Instruction::I32GtU);
            f.instruction(&Instruction::I32Or);
            f.instruction(&Instruction::If(wasm_encoder::BlockType::Empty));
            {
                let seg = self.find_or_make_err_string();
                f.instruction(&Instruction::I32Const(0));
                f.instruction(&Instruction::I32Const(28));
                f.instruction(&Instruction::ArrayNewData {
                    array_type_index: self.string_ty,
                    array_data_index: seg,
                });
                f.instruction(&Instruction::StructNew(result_err));
                f.instruction(&Instruction::Return);
            }
            f.instruction(&Instruction::End);

            // result = result * 10 + (ch - '0')
            f.instruction(&Instruction::LocalGet(4));
            f.instruction(&Instruction::I32Const(10));
            f.instruction(&Instruction::I32Mul);
            f.instruction(&Instruction::LocalGet(5));
            f.instruction(&Instruction::I32Const(48));
            f.instruction(&Instruction::I32Sub);
            f.instruction(&Instruction::I32Add);
            f.instruction(&Instruction::LocalSet(4));

            // i += 1
            f.instruction(&Instruction::LocalGet(2));
            f.instruction(&Instruction::I32Const(1));
            f.instruction(&Instruction::I32Add);
            f.instruction(&Instruction::LocalSet(2));

            f.instruction(&Instruction::Br(0)); // continue loop
        }
        f.instruction(&Instruction::End); // end loop
        f.instruction(&Instruction::End); // end block

        // if neg: result = -result
        f.instruction(&Instruction::LocalGet(3));
        f.instruction(&Instruction::If(wasm_encoder::BlockType::Empty));
        {
            f.instruction(&Instruction::I32Const(0));
            f.instruction(&Instruction::LocalGet(4));
            f.instruction(&Instruction::I32Sub);
            f.instruction(&Instruction::LocalSet(4));
        }
        f.instruction(&Instruction::End);

        // return Ok(result)
        f.instruction(&Instruction::LocalGet(4));
        f.instruction(&Instruction::StructNew(result_ok));

        f.instruction(&Instruction::End);
        codes.function(&f);
    }

    /// Emit __parse_i64(s: ref $string) -> ref $Result_i64_String
    fn emit_parse_i64_helper(&self, codes: &mut CodeSection) {
        let _result_base = *self.enum_base_types.get("Result_i64_String").unwrap();
        let result_ok = *self
            .enum_variant_types
            .get("Result_i64_String")
            .unwrap()
            .get("Ok")
            .unwrap();
        let result_err = *self
            .enum_variant_types
            .get("Result_i64_String")
            .unwrap()
            .get("Err")
            .unwrap();

        // locals: s(param0), len(1), i(2), neg(3), result_lo(4):i32, result_hi(5):i64, ch(6)
        let mut f = Function::new(vec![
            (1, ValType::I32), // len
            (1, ValType::I32), // i
            (1, ValType::I32), // neg
            (1, ValType::I64), // result
            (1, ValType::I32), // ch
        ]);

        // len = array.len(s)
        f.instruction(&Instruction::LocalGet(0));
        f.instruction(&Instruction::ArrayLen);
        f.instruction(&Instruction::LocalSet(1));

        // if len == 0: return Err
        f.instruction(&Instruction::LocalGet(1));
        f.instruction(&Instruction::I32Eqz);
        f.instruction(&Instruction::If(wasm_encoder::BlockType::Empty));
        {
            let seg = self.find_or_make_err_string();
            f.instruction(&Instruction::I32Const(0));
            f.instruction(&Instruction::I32Const(28));
            f.instruction(&Instruction::ArrayNewData {
                array_type_index: self.string_ty,
                array_data_index: seg,
            });
            f.instruction(&Instruction::StructNew(result_err));
            f.instruction(&Instruction::Return);
        }
        f.instruction(&Instruction::End);

        // Check for '-'
        f.instruction(&Instruction::LocalGet(0));
        f.instruction(&Instruction::I32Const(0));
        f.instruction(&Instruction::ArrayGetU(self.string_ty));
        f.instruction(&Instruction::I32Const(45));
        f.instruction(&Instruction::I32Eq);
        f.instruction(&Instruction::If(wasm_encoder::BlockType::Empty));
        {
            f.instruction(&Instruction::I32Const(1));
            f.instruction(&Instruction::LocalSet(3));
            f.instruction(&Instruction::I32Const(1));
            f.instruction(&Instruction::LocalSet(2));
            f.instruction(&Instruction::LocalGet(1));
            f.instruction(&Instruction::I32Const(1));
            f.instruction(&Instruction::I32Eq);
            f.instruction(&Instruction::If(wasm_encoder::BlockType::Empty));
            {
                let seg = self.find_or_make_err_string();
                f.instruction(&Instruction::I32Const(0));
                f.instruction(&Instruction::I32Const(28));
                f.instruction(&Instruction::ArrayNewData {
                    array_type_index: self.string_ty,
                    array_data_index: seg,
                });
                f.instruction(&Instruction::StructNew(result_err));
                f.instruction(&Instruction::Return);
            }
            f.instruction(&Instruction::End);
        }
        f.instruction(&Instruction::End);

        // Parse loop (i64 accumulation)
        f.instruction(&Instruction::Block(wasm_encoder::BlockType::Empty));
        f.instruction(&Instruction::Loop(wasm_encoder::BlockType::Empty));
        {
            f.instruction(&Instruction::LocalGet(2));
            f.instruction(&Instruction::LocalGet(1));
            f.instruction(&Instruction::I32GeU);
            f.instruction(&Instruction::BrIf(1));

            f.instruction(&Instruction::LocalGet(0));
            f.instruction(&Instruction::LocalGet(2));
            f.instruction(&Instruction::ArrayGetU(self.string_ty));
            f.instruction(&Instruction::LocalSet(5));

            f.instruction(&Instruction::LocalGet(5));
            f.instruction(&Instruction::I32Const(48));
            f.instruction(&Instruction::I32LtU);
            f.instruction(&Instruction::LocalGet(5));
            f.instruction(&Instruction::I32Const(57));
            f.instruction(&Instruction::I32GtU);
            f.instruction(&Instruction::I32Or);
            f.instruction(&Instruction::If(wasm_encoder::BlockType::Empty));
            {
                let seg = self.find_or_make_err_string();
                f.instruction(&Instruction::I32Const(0));
                f.instruction(&Instruction::I32Const(28));
                f.instruction(&Instruction::ArrayNewData {
                    array_type_index: self.string_ty,
                    array_data_index: seg,
                });
                f.instruction(&Instruction::StructNew(result_err));
                f.instruction(&Instruction::Return);
            }
            f.instruction(&Instruction::End);

            // result = result * 10 + (ch - 48) as i64
            f.instruction(&Instruction::LocalGet(4));
            f.instruction(&Instruction::I64Const(10));
            f.instruction(&Instruction::I64Mul);
            f.instruction(&Instruction::LocalGet(5));
            f.instruction(&Instruction::I32Const(48));
            f.instruction(&Instruction::I32Sub);
            f.instruction(&Instruction::I64ExtendI32U);
            f.instruction(&Instruction::I64Add);
            f.instruction(&Instruction::LocalSet(4));

            f.instruction(&Instruction::LocalGet(2));
            f.instruction(&Instruction::I32Const(1));
            f.instruction(&Instruction::I32Add);
            f.instruction(&Instruction::LocalSet(2));

            f.instruction(&Instruction::Br(0));
        }
        f.instruction(&Instruction::End);
        f.instruction(&Instruction::End);

        // if neg: result = -result
        f.instruction(&Instruction::LocalGet(3));
        f.instruction(&Instruction::If(wasm_encoder::BlockType::Empty));
        {
            f.instruction(&Instruction::I64Const(0));
            f.instruction(&Instruction::LocalGet(4));
            f.instruction(&Instruction::I64Sub);
            f.instruction(&Instruction::LocalSet(4));
        }
        f.instruction(&Instruction::End);

        // return Ok(result)
        f.instruction(&Instruction::LocalGet(4));
        f.instruction(&Instruction::StructNew(result_ok));

        f.instruction(&Instruction::End);
        codes.function(&f);
    }

    /// Emit __parse_f64(s: ref $string) -> ref $Result_f64_String
    /// Simplified: parse integer part + optional decimal part
    fn emit_parse_f64_helper(&self, codes: &mut CodeSection) {
        let _result_base = *self.enum_base_types.get("Result_f64_String").unwrap();
        let result_ok = *self
            .enum_variant_types
            .get("Result_f64_String")
            .unwrap()
            .get("Ok")
            .unwrap();
        let result_err = *self
            .enum_variant_types
            .get("Result_f64_String")
            .unwrap()
            .get("Err")
            .unwrap();

        // locals: s(0), len(1), i(2), neg(3), int_part(4):f64, frac_part(5):f64, divisor(6):f64, ch(7), has_dot(8)
        let mut f = Function::new(vec![
            (1, ValType::I32), // len
            (1, ValType::I32), // i
            (1, ValType::I32), // neg
            (1, ValType::F64), // int_part
            (1, ValType::F64), // frac_part
            (1, ValType::F64), // divisor
            (1, ValType::I32), // ch
            (1, ValType::I32), // has_dot
        ]);

        // len = array.len(s)
        f.instruction(&Instruction::LocalGet(0));
        f.instruction(&Instruction::ArrayLen);
        f.instruction(&Instruction::LocalSet(1));

        // if len == 0: return Err
        f.instruction(&Instruction::LocalGet(1));
        f.instruction(&Instruction::I32Eqz);
        f.instruction(&Instruction::If(wasm_encoder::BlockType::Empty));
        {
            let seg = self.find_or_make_err_float_string();
            f.instruction(&Instruction::I32Const(0));
            f.instruction(&Instruction::I32Const(26));
            f.instruction(&Instruction::ArrayNewData {
                array_type_index: self.string_ty,
                array_data_index: seg,
            });
            f.instruction(&Instruction::StructNew(result_err));
            f.instruction(&Instruction::Return);
        }
        f.instruction(&Instruction::End);

        // divisor = 1.0
        f.instruction(&Instruction::F64Const(1.0));
        f.instruction(&Instruction::LocalSet(6));

        // Check for '-'
        f.instruction(&Instruction::LocalGet(0));
        f.instruction(&Instruction::I32Const(0));
        f.instruction(&Instruction::ArrayGetU(self.string_ty));
        f.instruction(&Instruction::I32Const(45));
        f.instruction(&Instruction::I32Eq);
        f.instruction(&Instruction::If(wasm_encoder::BlockType::Empty));
        {
            f.instruction(&Instruction::I32Const(1));
            f.instruction(&Instruction::LocalSet(3));
            f.instruction(&Instruction::I32Const(1));
            f.instruction(&Instruction::LocalSet(2));
            f.instruction(&Instruction::LocalGet(1));
            f.instruction(&Instruction::I32Const(1));
            f.instruction(&Instruction::I32Eq);
            f.instruction(&Instruction::If(wasm_encoder::BlockType::Empty));
            {
                let seg = self.find_or_make_err_float_string();
                f.instruction(&Instruction::I32Const(0));
                f.instruction(&Instruction::I32Const(26));
                f.instruction(&Instruction::ArrayNewData {
                    array_type_index: self.string_ty,
                    array_data_index: seg,
                });
                f.instruction(&Instruction::StructNew(result_err));
                f.instruction(&Instruction::Return);
            }
            f.instruction(&Instruction::End);
        }
        f.instruction(&Instruction::End);

        // Parse loop
        f.instruction(&Instruction::Block(wasm_encoder::BlockType::Empty));
        f.instruction(&Instruction::Loop(wasm_encoder::BlockType::Empty));
        {
            f.instruction(&Instruction::LocalGet(2));
            f.instruction(&Instruction::LocalGet(1));
            f.instruction(&Instruction::I32GeU);
            f.instruction(&Instruction::BrIf(1));

            f.instruction(&Instruction::LocalGet(0));
            f.instruction(&Instruction::LocalGet(2));
            f.instruction(&Instruction::ArrayGetU(self.string_ty));
            f.instruction(&Instruction::LocalSet(7));

            // Check for '.'
            f.instruction(&Instruction::LocalGet(7));
            f.instruction(&Instruction::I32Const(46)); // '.'
            f.instruction(&Instruction::I32Eq);
            f.instruction(&Instruction::If(wasm_encoder::BlockType::Empty));
            {
                f.instruction(&Instruction::I32Const(1));
                f.instruction(&Instruction::LocalSet(8)); // has_dot = 1
                f.instruction(&Instruction::LocalGet(2));
                f.instruction(&Instruction::I32Const(1));
                f.instruction(&Instruction::I32Add);
                f.instruction(&Instruction::LocalSet(2));
                f.instruction(&Instruction::Br(1)); // continue loop
            }
            f.instruction(&Instruction::End);

            // Check digit range
            f.instruction(&Instruction::LocalGet(7));
            f.instruction(&Instruction::I32Const(48));
            f.instruction(&Instruction::I32LtU);
            f.instruction(&Instruction::LocalGet(7));
            f.instruction(&Instruction::I32Const(57));
            f.instruction(&Instruction::I32GtU);
            f.instruction(&Instruction::I32Or);
            f.instruction(&Instruction::If(wasm_encoder::BlockType::Empty));
            {
                let seg = self.find_or_make_err_float_string();
                f.instruction(&Instruction::I32Const(0));
                f.instruction(&Instruction::I32Const(26));
                f.instruction(&Instruction::ArrayNewData {
                    array_type_index: self.string_ty,
                    array_data_index: seg,
                });
                f.instruction(&Instruction::StructNew(result_err));
                f.instruction(&Instruction::Return);
            }
            f.instruction(&Instruction::End);

            // digit = (ch - 48) as f64
            f.instruction(&Instruction::LocalGet(8)); // has_dot
            f.instruction(&Instruction::If(wasm_encoder::BlockType::Empty));
            {
                // frac_part = frac_part * 10 + digit; divisor *= 10
                f.instruction(&Instruction::LocalGet(5));
                f.instruction(&Instruction::F64Const(10.0));
                f.instruction(&Instruction::F64Mul);
                f.instruction(&Instruction::LocalGet(7));
                f.instruction(&Instruction::I32Const(48));
                f.instruction(&Instruction::I32Sub);
                f.instruction(&Instruction::F64ConvertI32U);
                f.instruction(&Instruction::F64Add);
                f.instruction(&Instruction::LocalSet(5));
                f.instruction(&Instruction::LocalGet(6));
                f.instruction(&Instruction::F64Const(10.0));
                f.instruction(&Instruction::F64Mul);
                f.instruction(&Instruction::LocalSet(6));
            }
            f.instruction(&Instruction::Else);
            {
                // int_part = int_part * 10 + digit
                f.instruction(&Instruction::LocalGet(4));
                f.instruction(&Instruction::F64Const(10.0));
                f.instruction(&Instruction::F64Mul);
                f.instruction(&Instruction::LocalGet(7));
                f.instruction(&Instruction::I32Const(48));
                f.instruction(&Instruction::I32Sub);
                f.instruction(&Instruction::F64ConvertI32U);
                f.instruction(&Instruction::F64Add);
                f.instruction(&Instruction::LocalSet(4));
            }
            f.instruction(&Instruction::End);

            f.instruction(&Instruction::LocalGet(2));
            f.instruction(&Instruction::I32Const(1));
            f.instruction(&Instruction::I32Add);
            f.instruction(&Instruction::LocalSet(2));

            f.instruction(&Instruction::Br(0));
        }
        f.instruction(&Instruction::End);
        f.instruction(&Instruction::End);

        // result = int_part + frac_part / divisor → store in int_part (local 4)
        f.instruction(&Instruction::LocalGet(4));
        f.instruction(&Instruction::LocalGet(5));
        f.instruction(&Instruction::LocalGet(6));
        f.instruction(&Instruction::F64Div);
        f.instruction(&Instruction::F64Add);
        f.instruction(&Instruction::LocalSet(4)); // result in local 4

        // if neg: result = -result
        f.instruction(&Instruction::LocalGet(3));
        f.instruction(&Instruction::If(wasm_encoder::BlockType::Empty));
        {
            f.instruction(&Instruction::LocalGet(4));
            f.instruction(&Instruction::F64Neg);
            f.instruction(&Instruction::LocalSet(4));
        }
        f.instruction(&Instruction::End);

        // return Ok(result)
        f.instruction(&Instruction::LocalGet(4));
        f.instruction(&Instruction::StructNew(result_ok));

        f.instruction(&Instruction::End);
        codes.function(&f);
    }

    /// Find or allocate the "invalid number" error string data segment
    fn find_or_make_err_string(&self) -> u32 {
        self.data_segs.len() as u32 + self.err_string_seg.unwrap_or(0)
    }

    fn find_or_make_err_float_string(&self) -> u32 {
        self.data_segs.len() as u32 + self.err_float_string_seg.unwrap_or(0)
    }

    // ── User function emission ───────────────────────────────────

    /// Emit a stub body for a builtin function (inlined at call sites).
    /// The body is `unreachable` — it should never be called at runtime.
    fn emit_builtin_stub(&self, codes: &mut CodeSection, _func: &MirFunction) {
        let f_locals: Vec<(u32, ValType)> = vec![];
        let mut f = Function::new(f_locals);
        f.instruction(&Instruction::Unreachable);
        f.instruction(&Instruction::End);
        codes.function(&f);
    }

    fn emit_function(&mut self, codes: &mut CodeSection, func: &MirFunction) {
        // Track whether this is the _start/main function
        self.is_start_fn = func.name == "main" || func.name == "_start";
        // Reset per-function state
        self.string_locals.clear();
        self.f64_locals.clear();
        self.i64_locals.clear();
        self.bool_locals.clear();
        self.any_locals.clear();
        self.f64_vec_locals.clear();
        self.i64_vec_locals.clear();
        self.i32_vec_locals.clear();
        self.string_vec_locals.clear();
        self.struct_vec_locals.clear();
        self.local_struct.clear();
        self.local_enum.clear();
        self.current_fn_type_params = func.type_params.clone();
        self.current_fn_return_ty = func.return_ty.clone();

        // ── Type propagation: scan ALL assignments (including nested) to infer ref types ──
        let mut extra_enum: HashMap<u32, String> = HashMap::new();
        let mut extra_struct: HashMap<u32, String> = HashMap::new();
        let mut extra_vec_i32: HashSet<u32> = HashSet::new();
        let mut extra_vec_i64: HashSet<u32> = HashSet::new();
        let mut extra_vec_f64: HashSet<u32> = HashSet::new();
        let mut extra_vec_string: HashSet<u32> = HashSet::new();

        // Collect ALL statements recursively (flattens IfStmt/WhileStmt bodies)
        fn collect_stmts_recursive<'a>(stmts: &'a [MirStmt], out: &mut Vec<&'a MirStmt>) {
            for stmt in stmts {
                out.push(stmt);
                match stmt {
                    MirStmt::IfStmt {
                        then_body,
                        else_body,
                        ..
                    } => {
                        collect_stmts_recursive(then_body, out);
                        collect_stmts_recursive(else_body, out);
                    }
                    MirStmt::WhileStmt { body, .. } => {
                        collect_stmts_recursive(body, out);
                    }
                    _ => {}
                }
            }
        }
        // Also collect stmts embedded in Operand trees (e.g. IfExpr.then_body inside Return)
        fn collect_stmts_from_operand<'a>(op: &'a Operand, out: &mut Vec<&'a MirStmt>) {
            match op {
                Operand::IfExpr {
                    cond,
                    then_body,
                    then_result,
                    else_body,
                    else_result,
                } => {
                    collect_stmts_from_operand(cond, out);
                    collect_stmts_recursive(then_body, out);
                    collect_stmts_recursive(else_body, out);
                    if let Some(tr) = then_result {
                        collect_stmts_from_operand(tr, out);
                    }
                    if let Some(er) = else_result {
                        collect_stmts_from_operand(er, out);
                    }
                }
                Operand::LoopExpr { body, result, .. } => {
                    collect_stmts_recursive(body, out);
                    collect_stmts_from_operand(result, out);
                }
                Operand::BinOp(_, a, b) => {
                    collect_stmts_from_operand(a, out);
                    collect_stmts_from_operand(b, out);
                }
                Operand::UnaryOp(_, a) => {
                    collect_stmts_from_operand(a, out);
                }
                Operand::TryExpr { expr, .. } => {
                    collect_stmts_from_operand(expr, out);
                }
                _ => {}
            }
        }
        let mut all_stmts: Vec<&MirStmt> = Vec::new();
        for block in &func.blocks {
            collect_stmts_recursive(&block.stmts, &mut all_stmts);
            // Also scan operands inside the terminator (especially Return)
            match &block.terminator {
                Terminator::Return(Some(op)) => collect_stmts_from_operand(op, &mut all_stmts),
                Terminator::If { cond, .. } => collect_stmts_from_operand(cond, &mut all_stmts),
                Terminator::Switch { scrutinee, .. } => {
                    collect_stmts_from_operand(scrutinee, &mut all_stmts)
                }
                _ => {}
            }
        }

        // Multi-pass to propagate through chains (e.g., a = b = c where c is typed)
        for _pass in 0..3 {
            for stmt in &all_stmts {
                match stmt {
                    MirStmt::Assign(Place::Local(dst), Rvalue::Use(op)) => {
                        match op {
                            Operand::Place(Place::Local(src)) => {
                                let ename_opt = func
                                    .enum_typed_locals
                                    .get(&src.0)
                                    .or_else(|| extra_enum.get(&src.0))
                                    .cloned();
                                if let Some(ename) = ename_opt {
                                    extra_enum.entry(dst.0).or_insert(ename);
                                }
                                let sname_opt = func
                                    .struct_typed_locals
                                    .get(&src.0)
                                    .or_else(|| extra_struct.get(&src.0))
                                    .cloned();
                                if let Some(sname) = sname_opt {
                                    extra_struct.entry(dst.0).or_insert(sname);
                                }
                                if extra_vec_i32.contains(&src.0) {
                                    extra_vec_i32.insert(dst.0);
                                }
                                if extra_vec_i64.contains(&src.0) {
                                    extra_vec_i64.insert(dst.0);
                                }
                                if extra_vec_f64.contains(&src.0) {
                                    extra_vec_f64.insert(dst.0);
                                }
                                if extra_vec_string.contains(&src.0) {
                                    extra_vec_string.insert(dst.0);
                                }
                                // Propagate struct-vec
                                if let Some(svn) = self.struct_vec_locals.get(&src.0).cloned() {
                                    self.struct_vec_locals.entry(dst.0).or_insert(svn);
                                }
                            }
                            Operand::EnumInit { enum_name, .. } => {
                                extra_enum.entry(dst.0).or_insert_with(|| enum_name.clone());
                            }
                            Operand::EnumPayload {
                                enum_name,
                                variant_name,
                                index,
                                ..
                            } => {
                                // The payload type comes from enum_variant_field_types
                                let key = (enum_name.clone(), variant_name.clone());
                                if let Some(field_types) = self.enum_variant_field_types.get(&key) {
                                    if let Some(ft) = field_types.get(*index as usize) {
                                        if self.enum_base_types.contains_key(ft.as_str()) {
                                            extra_enum.entry(dst.0).or_insert_with(|| ft.clone());
                                        }
                                        if self.struct_gc_types.contains_key(ft.as_str()) {
                                            extra_struct.entry(dst.0).or_insert_with(|| ft.clone());
                                        }
                                    }
                                }
                            }
                            Operand::StructInit { name, .. } => {
                                extra_struct.entry(dst.0).or_insert_with(|| name.clone());
                            }
                            Operand::Call(name, _) => {
                                let canonical = normalize_intrinsic(name);
                                if let Some(sname) = canonical.strip_prefix("Vec_new_") {
                                    match sname {
                                        "i32" => {
                                            extra_vec_i32.insert(dst.0);
                                        }
                                        "i64" => {
                                            extra_vec_i64.insert(dst.0);
                                        }
                                        "f64" => {
                                            extra_vec_f64.insert(dst.0);
                                        }
                                        "String" => {
                                            extra_vec_string.insert(dst.0);
                                        }
                                        _ => {
                                            if self.custom_vec_types.contains_key(sname) {
                                                self.struct_vec_locals
                                                    .insert(dst.0, sname.to_string());
                                            }
                                        }
                                    }
                                }
                                // Propagate vec type from filter/map calls
                                match canonical {
                                    "filter_i32" => {
                                        extra_vec_i32.insert(dst.0);
                                    }
                                    "filter_i64" => {
                                        extra_vec_i64.insert(dst.0);
                                    }
                                    "filter_f64" => {
                                        extra_vec_f64.insert(dst.0);
                                    }
                                    "filter_String" => {
                                        extra_vec_string.insert(dst.0);
                                    }
                                    "map_i32_i32" => {
                                        extra_vec_i32.insert(dst.0);
                                    }
                                    "map_i64_i64" => {
                                        extra_vec_i64.insert(dst.0);
                                    }
                                    "map_f64_f64" => {
                                        extra_vec_f64.insert(dst.0);
                                    }
                                    "map_i32_String" | "map_i64_String" | "map_f64_String"
                                    | "map_String_String" => {
                                        extra_vec_string.insert(dst.0);
                                    }
                                    "parse_i32" => {
                                        extra_enum
                                            .entry(dst.0)
                                            .or_insert_with(|| "Result".to_string());
                                    }
                                    "parse_i64" => {
                                        extra_enum
                                            .entry(dst.0)
                                            .or_insert_with(|| "Result_i64_String".to_string());
                                    }
                                    "parse_f64" => {
                                        extra_enum
                                            .entry(dst.0)
                                            .or_insert_with(|| "Result_f64_String".to_string());
                                    }
                                    "fs_read_file" => {
                                        extra_enum
                                            .entry(dst.0)
                                            .or_insert_with(|| "Result_String_String".to_string());
                                    }
                                    "fs_write_file" => {
                                        extra_enum
                                            .entry(dst.0)
                                            .or_insert_with(|| "Result".to_string());
                                    }
                                    "find_i32" | "find_String" => {
                                        extra_enum
                                            .entry(dst.0)
                                            .or_insert_with(|| "Option".to_string());
                                    }
                                    "HashMap_i32_i32_new" => {
                                        extra_struct
                                            .entry(dst.0)
                                            .or_insert_with(|| "__hashmap_i32_i32".to_string());
                                    }
                                    "HashMap_i32_i32_get" => {
                                        extra_enum
                                            .entry(dst.0)
                                            .or_insert_with(|| "Option".to_string());
                                    }
                                    _ => {}
                                }
                                // Also check fn_ret_type_names for enum return types
                                if let Some(ret_name) = self.fn_ret_type_names.get(canonical) {
                                    if self.enum_base_types.contains_key(ret_name.as_str()) {
                                        extra_enum.entry(dst.0).or_insert_with(|| ret_name.clone());
                                    }
                                }
                            }
                            _ => {}
                        }
                    }
                    MirStmt::CallBuiltin {
                        dest: Some(Place::Local(dst)),
                        name,
                        ..
                    } => {
                        let canonical = normalize_intrinsic(name);
                        if let Some(sname) = canonical.strip_prefix("Vec_new_") {
                            match sname {
                                "i32" => {
                                    extra_vec_i32.insert(dst.0);
                                }
                                "i64" => {
                                    extra_vec_i64.insert(dst.0);
                                }
                                "f64" => {
                                    extra_vec_f64.insert(dst.0);
                                }
                                "String" => {
                                    extra_vec_string.insert(dst.0);
                                }
                                _ => {
                                    if self.custom_vec_types.contains_key(sname) {
                                        self.struct_vec_locals.insert(dst.0, sname.to_string());
                                    }
                                }
                            }
                        }
                        match canonical {
                            "filter_i32" => {
                                extra_vec_i32.insert(dst.0);
                            }
                            "filter_i64" => {
                                extra_vec_i64.insert(dst.0);
                            }
                            "filter_f64" => {
                                extra_vec_f64.insert(dst.0);
                            }
                            "filter_String" => {
                                extra_vec_string.insert(dst.0);
                            }
                            "map_i32_i32" => {
                                extra_vec_i32.insert(dst.0);
                            }
                            "map_i64_i64" => {
                                extra_vec_i64.insert(dst.0);
                            }
                            "map_f64_f64" => {
                                extra_vec_f64.insert(dst.0);
                            }
                            "map_i32_String" | "map_i64_String" | "map_f64_String"
                            | "map_String_String" => {
                                extra_vec_string.insert(dst.0);
                            }
                            "parse_i32" => {
                                extra_enum
                                    .entry(dst.0)
                                    .or_insert_with(|| "Result".to_string());
                            }
                            "parse_i64" => {
                                extra_enum
                                    .entry(dst.0)
                                    .or_insert_with(|| "Result_i64_String".to_string());
                            }
                            "parse_f64" => {
                                extra_enum
                                    .entry(dst.0)
                                    .or_insert_with(|| "Result_f64_String".to_string());
                            }
                            "fs_read_file" => {
                                extra_enum
                                    .entry(dst.0)
                                    .or_insert_with(|| "Result_String_String".to_string());
                            }
                            "fs_write_file" => {
                                extra_enum
                                    .entry(dst.0)
                                    .or_insert_with(|| "Result".to_string());
                            }
                            "find_i32" | "find_String" => {
                                extra_enum
                                    .entry(dst.0)
                                    .or_insert_with(|| "Option".to_string());
                            }
                            "HashMap_i32_i32_new" => {
                                extra_struct
                                    .entry(dst.0)
                                    .or_insert_with(|| "__hashmap_i32_i32".to_string());
                            }
                            "HashMap_i32_i32_get" => {
                                extra_enum
                                    .entry(dst.0)
                                    .or_insert_with(|| "Option".to_string());
                            }
                            _ => {}
                        }
                    }
                    _ => {}
                }
            }
            // Also propagate from MIR-declared vec types
            for local in func.params.iter().chain(func.locals.iter()) {
                if let Type::Vec(elem) = &local.ty {
                    match elem.as_ref() {
                        Type::I32 | Type::Bool => {
                            extra_vec_i32.insert(local.id.0);
                        }
                        Type::I64 => {
                            extra_vec_i64.insert(local.id.0);
                        }
                        Type::F64 => {
                            extra_vec_f64.insert(local.id.0);
                        }
                        Type::String => {
                            extra_vec_string.insert(local.id.0);
                        }
                        _ => {
                            extra_vec_i32.insert(local.id.0);
                        }
                    }
                }
            }
        }

        // Merge extra maps into the function's typed_locals for local_val_type
        let mut merged_struct = func.struct_typed_locals.clone();
        for (k, v) in &extra_struct {
            merged_struct.entry(*k).or_insert_with(|| v.clone());
        }
        let mut merged_enum = func.enum_typed_locals.clone();
        for (k, v) in &extra_enum {
            // Specialized names (e.g. Result_i64_String) override generic (Result)
            let should_override = v.starts_with("Result_") || v.starts_with("Option_");
            if should_override {
                merged_enum.insert(*k, v.clone());
            } else {
                merged_enum.entry(*k).or_insert_with(|| v.clone());
            }
        }

        // Merge vec sets: propagated from assignment scan + type scan
        let vec_sets = Some((
            &extra_vec_i32,
            &extra_vec_i64,
            &extra_vec_f64,
            &extra_vec_string,
        ));

        // Collect local types (skip params — they are already in the func signature)
        let num_params = func.params.len();
        let mut local_types: Vec<(u32, ValType)> = Vec::new();
        for local in func.locals.iter().skip(num_params) {
            let vt = self.local_val_type(local, &merged_struct, &merged_enum, vec_sets);
            local_types.push((1, vt));
        }
        // Track type metadata for all locals (including params)
        for local in func.params.iter().chain(func.locals.iter()) {
            match &local.ty {
                Type::String => {
                    self.string_locals.insert(local.id.0);
                }
                Type::F64 => {
                    self.f64_locals.insert(local.id.0);
                }
                Type::I64 | Type::U64 => {
                    self.i64_locals.insert(local.id.0);
                }
                Type::Bool => {
                    self.bool_locals.insert(local.id.0);
                }
                Type::Vec(elem) => match elem.as_ref() {
                    Type::F64 => {
                        self.f64_vec_locals.insert(local.id.0);
                    }
                    Type::I64 => {
                        self.i64_vec_locals.insert(local.id.0);
                    }
                    Type::String => {
                        self.string_vec_locals.insert(local.id.0);
                    }
                    _ => {
                        self.i32_vec_locals.insert(local.id.0);
                    }
                },
                Type::Any => {
                    self.any_locals.insert(local.id.0);
                }
                _ => {}
            }
        }
        // Populate struct/enum local maps from side-channel + propagated types
        for (lid, sname) in &merged_struct {
            self.local_struct
                .entry(*lid)
                .or_insert_with(|| sname.clone());
        }
        for (lid, ename) in &merged_enum {
            self.local_enum.entry(*lid).or_insert_with(|| ename.clone());
        }
        // Merge propagated vec types into runtime sets
        for lid in &extra_vec_i32 {
            self.i32_vec_locals.insert(*lid);
        }
        for lid in &extra_vec_i64 {
            self.i64_vec_locals.insert(*lid);
        }
        for lid in &extra_vec_f64 {
            self.f64_vec_locals.insert(*lid);
        }
        for lid in &extra_vec_string {
            self.string_vec_locals.insert(*lid);
        }
        // Track generic function params as enum/vec locals based on fn_param_type_names
        if !func.type_params.is_empty() {
            if let Some(param_names) = self.fn_param_type_names.get(&func.name).cloned() {
                for (i, pname) in param_names.iter().enumerate() {
                    if let Some(p) = func.params.get(i) {
                        if pname.starts_with("Option") {
                            self.local_enum.insert(p.id.0, "Option".to_string());
                        } else if pname.starts_with("Result") {
                            let rname = if pname.contains("i64") {
                                "Result_i64_String"
                            } else if pname.contains("f64") {
                                "Result_f64_String"
                            } else if pname.contains("String, String")
                                || pname.contains("String,String")
                            {
                                "Result_String_String"
                            } else {
                                "Result"
                            };
                            self.local_enum.insert(p.id.0, rname.to_string());
                        } else if pname.starts_with("Vec<") {
                            let inner = &pname[4..pname.len().saturating_sub(1)];
                            match inner {
                                "i64" => {
                                    self.i64_vec_locals.insert(p.id.0);
                                }
                                "f64" => {
                                    self.f64_vec_locals.insert(p.id.0);
                                }
                                "String" => {
                                    self.string_vec_locals.insert(p.id.0);
                                }
                                _ => {
                                    self.i32_vec_locals.insert(p.id.0);
                                }
                            }
                        }
                    }
                }
            }
        }
        // Add scratch locals for GC string operations:
        // +0: i32 (counter/index), +1: i32, +2: i32, +3: i32,
        // +4: ref $string, +5: ref $string
        // +6: i64, +7: f64, +8: ref $string (extra for vec ops)
        // +9: i32 (extra scratch for nested loops)
        // +10: anyref (for TryExpr ? operator)
        self.scratch_base = func.locals.len() as u32;
        let str_ref = ref_nullable(self.string_ty);
        local_types.push((1, ValType::I32));
        local_types.push((1, ValType::I32));
        local_types.push((1, ValType::I32));
        local_types.push((1, ValType::I32));
        local_types.push((1, str_ref));
        local_types.push((1, str_ref));
        local_types.push((1, ValType::I64)); // si(6): i64 scratch
        local_types.push((1, ValType::F64)); // si(7): f64 scratch
        local_types.push((1, str_ref)); // si(8): extra ref scratch
        local_types.push((1, ValType::I32)); // si(9): extra i32 scratch
        // si(10): anyref scratch for TryExpr (? operator)
        let anyref_ty = ValType::Ref(WasmRefType {
            nullable: true,
            heap_type: HeapType::Abstract {
                shared: false,
                ty: wasm_encoder::AbstractHeapType::Any,
            },
        });
        local_types.push((1, anyref_ty));
        // si(11): anyref scratch #2 for HOF operations
        local_types.push((1, anyref_ty));

        let mut f = Function::new(local_types);

        // Emit statements from entry block
        if let Some(block) = func.blocks.first() {
            for stmt in &block.stmts {
                self.emit_stmt(&mut f, stmt);
            }
            // Handle terminator
            match &block.terminator {
                Terminator::Return(Some(op)) => {
                    if func.name == "main" || func.name == "_start" {
                        // WASI _start must be () -> (); emit for side effects but discard result
                        if !matches!(op, Operand::Unit) {
                            self.emit_operand(&mut f, op);
                            if self.operand_produces_value(op) {
                                f.instruction(&Instruction::Drop);
                            }
                        }
                    } else {
                        self.emit_operand(&mut f, op);
                        // Box value types when returning from generic function with anyref return
                        if self.current_fn_return_ty == Type::Any {
                            let op_vt = self.infer_operand_type(op);
                            if op_vt == ValType::I32 {
                                f.instruction(&Instruction::RefI31);
                            }
                        }
                    }
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
                // Unbox anyref from __tupleN_any FieldAccess to the destination local's concrete type
                if let Operand::FieldAccess { struct_name, .. } = op {
                    if struct_name.starts_with("__tuple") && struct_name.ends_with("_any") {
                        if self.string_locals.contains(&id.0) {
                            f.instruction(&Instruction::RefCastNullable(HeapType::Concrete(
                                self.string_ty,
                            )));
                        } else {
                            // Default: i32 — unbox from i31ref
                            f.instruction(&Instruction::RefCastNullable(HeapType::Abstract {
                                shared: false,
                                ty: wasm_encoder::AbstractHeapType::I31,
                            }));
                            f.instruction(&Instruction::I31GetS);
                        }
                    }
                }
                let local_idx = self.local_wasm_idx(id.0);
                f.instruction(&Instruction::LocalSet(local_idx));
            }
            MirStmt::Assign(Place::Field(inner, field_name), Rvalue::Use(op)) => {
                // struct.set: obj_ref field_idx value
                if let Place::Local(id) = inner.as_ref() {
                    let struct_name = self.local_struct.get(&id.0).cloned();
                    if let Some(ref sname) = struct_name {
                        if let Some(&ty_idx) = self.struct_gc_types.get(sname) {
                            let field_idx = self
                                .struct_layouts
                                .get(sname)
                                .and_then(|fields| fields.iter().position(|(n, _)| n == field_name))
                                .unwrap_or(0) as u32;
                            let local_idx = self.local_wasm_idx(id.0);
                            f.instruction(&Instruction::LocalGet(local_idx));
                            self.emit_operand(f, op);
                            f.instruction(&Instruction::StructSet {
                                struct_type_index: ty_idx,
                                field_index: field_idx,
                            });
                            return;
                        }
                    }
                }
                // Fallback: drop
                self.emit_operand(f, op);
                f.instruction(&Instruction::Drop);
            }
            MirStmt::Assign(_place, Rvalue::Use(op)) => {
                self.emit_operand(f, op);
                // For non-local places, just drop for now
                f.instruction(&Instruction::Drop);
            }
            MirStmt::Assign(Place::Local(id), Rvalue::BinaryOp(op, lhs, rhs)) => {
                let lhs_i64 = self.is_i64_like_operand(lhs);
                let lhs_f64 = self.is_f64_like_operand(lhs);
                let rhs_i64 = self.is_i64_like_operand(rhs);
                let rhs_f64 = self.is_f64_like_operand(rhs);
                let need_i64 = lhs_i64 || rhs_i64;
                let need_f64 = lhs_f64 || rhs_f64;
                let lhs_str = self.is_string_like_operand(lhs);
                if lhs_str && matches!(op, BinOp::Eq | BinOp::Ne) {
                    // String comparison: byte-by-byte GC array compare
                    self.emit_operand(f, lhs);
                    self.emit_operand(f, rhs);
                    self.emit_string_eq_gc(f);
                    if *op == BinOp::Ne {
                        f.instruction(&Instruction::I32Eqz);
                    }
                } else {
                    self.emit_operand_coerced(f, lhs, need_i64, need_f64);
                    self.emit_operand_coerced(f, rhs, need_i64, need_f64);
                    self.emit_binop(f, *op, Some(lhs));
                }
                let local_idx = self.local_wasm_idx(id.0);
                f.instruction(&Instruction::LocalSet(local_idx));
            }
            MirStmt::Assign(Place::Local(id), Rvalue::UnaryOp(op, inner)) => {
                self.emit_operand(f, inner);
                let is_f64 = self.f64_locals.contains(&id.0) || self.is_f64_like_operand(inner);
                let is_i64 = self.i64_locals.contains(&id.0) || self.is_i64_like_operand(inner);
                match op {
                    UnaryOp::Neg => {
                        if is_f64 {
                            f.instruction(&Instruction::F64Neg);
                        } else if is_i64 {
                            let scratch = self.si(6);
                            f.instruction(&Instruction::LocalSet(scratch));
                            f.instruction(&Instruction::I64Const(0));
                            f.instruction(&Instruction::LocalGet(scratch));
                            f.instruction(&Instruction::I64Sub);
                        } else {
                            f.instruction(&Instruction::I32Const(-1));
                            f.instruction(&Instruction::I32Mul);
                        }
                    }
                    UnaryOp::Not | UnaryOp::BitNot => {
                        f.instruction(&Instruction::I32Const(-1));
                        f.instruction(&Instruction::I32Xor);
                    }
                }
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
                        let param_types = self.fn_param_types.get(&fn_name).cloned();
                        for (i, arg) in args.iter().enumerate() {
                            let need_i64 = param_types
                                .as_ref()
                                .and_then(|pt| pt.get(i))
                                .is_some_and(|t| matches!(t, Type::I64));
                            let need_f64 = param_types
                                .as_ref()
                                .and_then(|pt| pt.get(i))
                                .is_some_and(|t| matches!(t, Type::F64));
                            let need_any = param_types
                                .as_ref()
                                .and_then(|pt| pt.get(i))
                                .is_some_and(|t| matches!(t, Type::Any));
                            self.emit_operand_coerced(f, arg, need_i64, need_f64);
                            // Box i32/bool → ref.i31 for anyref params
                            if need_any {
                                let arg_vt = self.infer_operand_type(arg);
                                if arg_vt == ValType::I32 {
                                    f.instruction(&Instruction::RefI31);
                                }
                            }
                        }
                        if let Some(&fn_idx) = self.fn_map.get(&fn_name) {
                            f.instruction(&Instruction::Call(fn_idx));
                        }
                        // Unbox anyref return if needed
                        let ret_ty = self.fn_ret_types.get(&fn_name).cloned();
                        if let Some(ref rt) = ret_ty {
                            if *rt == Type::Any && dest.is_some() {
                                let concrete = self.infer_generic_return_type(&fn_name, args);
                                self.emit_anyref_unbox(f, &concrete);
                            }
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
                // Const-if elimination: skip the branch structure for constant conditions
                if let Operand::ConstBool(value) = cond {
                    let body = if *value { then_body } else { else_body };
                    for s in body {
                        self.emit_stmt(f, s);
                    }
                    return;
                }
                self.emit_operand(f, cond);
                f.instruction(&Instruction::If(wasm_encoder::BlockType::Empty));
                self.loop_break_extra_depth += 1;
                for s in then_body {
                    self.emit_stmt(f, s);
                }
                if !else_body.is_empty() {
                    f.instruction(&Instruction::Else);
                    for s in else_body {
                        self.emit_stmt(f, s);
                    }
                }
                self.loop_break_extra_depth -= 1;
                f.instruction(&Instruction::End);
            }
            MirStmt::WhileStmt { cond, body } => {
                let saved_depth = self.loop_break_extra_depth;
                self.loop_break_extra_depth = 0;
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
                self.loop_break_extra_depth = saved_depth;
            }
            MirStmt::Break => {
                // br targets the outer block of the while loop
                // depth: block(outer) > loop > [if*] > here
                f.instruction(&Instruction::Br(1 + self.loop_break_extra_depth));
            }
            MirStmt::Continue => {
                // br targets the loop (beginning)
                f.instruction(&Instruction::Br(self.loop_break_extra_depth));
            }
            MirStmt::Return(Some(op)) => {
                // ── Tail-call optimisation (return_call) ──
                // When opt_level >= 1 and the returned value is a direct call
                // whose Wasm return type matches the current function, emit
                // `return_call` instead of `call` + `return`.
                if self.opt_level >= 1 && !self.is_start_fn {
                    if let Operand::Call(name, args) = op {
                        let canonical = normalize_intrinsic(name).to_string();
                        if !self.is_builtin_name(&canonical) {
                            let callee_ret_is_any = self
                                .fn_ret_types
                                .get(canonical.as_str())
                                .is_some_and(|t| *t == Type::Any);
                            let current_ret_is_any = self.current_fn_return_ty == Type::Any;
                            // return_call is valid only when no boxing/unboxing
                            // is needed between the callee result and our return.
                            if callee_ret_is_any == current_ret_is_any {
                                if let Some(&fn_idx) = self.fn_map.get(canonical.as_str()) {
                                    let param_types =
                                        self.fn_param_types.get(canonical.as_str()).cloned();
                                    for (i, arg) in args.iter().enumerate() {
                                        self.emit_operand(f, arg);
                                        if let Some(ref pts) = param_types {
                                            if i < pts.len() && pts[i] == Type::Any {
                                                let arg_vt = self.infer_operand_type(arg);
                                                if arg_vt == ValType::I32 {
                                                    f.instruction(&Instruction::RefI31);
                                                }
                                            }
                                        }
                                    }
                                    f.instruction(&Instruction::ReturnCall(fn_idx));
                                    return;
                                }
                            }
                        }
                    }
                    if let Operand::CallIndirect { callee, args } = op {
                        if self.current_fn_return_ty != Type::Any {
                            for arg in args {
                                self.emit_operand(f, arg);
                            }
                            self.emit_operand(f, callee);
                            let params: Vec<ValType> = args
                                .iter()
                                .map(|a| {
                                    if self.is_f64_like_operand(a) {
                                        ValType::F64
                                    } else if self.is_i64_like_operand(a) {
                                        ValType::I64
                                    } else {
                                        ValType::I32
                                    }
                                })
                                .collect();
                            let results = vec![ValType::I32];
                            let type_index = self
                                .indirect_types
                                .get(&(params, results))
                                .copied()
                                .unwrap_or(0);
                            f.instruction(&Instruction::ReturnCallIndirect {
                                type_index,
                                table_index: 0,
                            });
                            return;
                        }
                    }
                }
                // ── Normal (non-tail-call) path ──
                if self.is_start_fn && !matches!(op, Operand::Unit) {
                    self.emit_operand(f, op);
                    if self.operand_produces_value(op) {
                        f.instruction(&Instruction::Drop);
                    }
                } else {
                    self.emit_operand(f, op);
                    // Box value types when returning from generic function with anyref return
                    if self.current_fn_return_ty == Type::Any {
                        let op_vt = self.infer_operand_type(op);
                        if op_vt == ValType::I32 {
                            f.instruction(&Instruction::RefI31);
                        }
                    }
                }
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
            Operand::ConstU8(v) => {
                f.instruction(&Instruction::I32Const(*v as i32));
            }
            Operand::ConstU16(v) => {
                f.instruction(&Instruction::I32Const(*v as i32));
            }
            Operand::ConstU32(v) => {
                f.instruction(&Instruction::I32Const(*v as i32));
            }
            Operand::ConstU64(v) => {
                f.instruction(&Instruction::I64Const(*v as i64));
            }
            Operand::ConstI8(v) => {
                f.instruction(&Instruction::I32Const(*v as i32));
            }
            Operand::ConstI16(v) => {
                f.instruction(&Instruction::I32Const(*v as i32));
            }
            Operand::ConstBool(v) => {
                f.instruction(&Instruction::I32Const(if *v { 1 } else { 0 }));
            }
            Operand::ConstChar(c) => {
                f.instruction(&Instruction::I32Const(*c as i32));
            }
            Operand::ConstString(s) => {
                // GC-native: create a GC string via array.new_data
                let bytes = s.as_bytes();
                let len = bytes.len() as u32;
                let seg_idx = self.alloc_string_data(bytes);
                // The absolute segment index = num_active + seg_idx
                // (computed at assembly time, stored as relative for now)
                let abs_seg = self.data_segs.len() as u32 + seg_idx;
                f.instruction(&Instruction::I32Const(0)); // src offset in data segment
                f.instruction(&Instruction::I32Const(len as i32)); // length
                f.instruction(&Instruction::ArrayNewData {
                    array_type_index: self.string_ty,
                    array_data_index: abs_seg,
                });
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
                let lhs_i64 = self.is_i64_like_operand(lhs);
                let lhs_f64 = self.is_f64_like_operand(lhs);
                let rhs_i64 = self.is_i64_like_operand(rhs);
                let rhs_f64 = self.is_f64_like_operand(rhs);
                let need_i64 = lhs_i64 || rhs_i64;
                let need_f64 = lhs_f64 || rhs_f64;
                let lhs_str = self.is_string_like_operand(lhs);
                if lhs_str && matches!(op, BinOp::Eq | BinOp::Ne) {
                    self.emit_operand(f, lhs);
                    self.emit_operand(f, rhs);
                    self.emit_string_eq_gc(f);
                    if *op == BinOp::Ne {
                        f.instruction(&Instruction::I32Eqz);
                    }
                } else {
                    self.emit_operand_coerced(f, lhs, need_i64, need_f64);
                    self.emit_operand_coerced(f, rhs, need_i64, need_f64);
                    self.emit_binop(f, *op, Some(lhs));
                }
            }
            Operand::UnaryOp(op, inner) => {
                self.emit_operand(f, inner);
                let is_f64 = self.is_f64_like_operand(inner);
                let is_i64 = self.is_i64_like_operand(inner);
                match op {
                    UnaryOp::Neg => {
                        if is_f64 {
                            f.instruction(&Instruction::F64Neg);
                        } else if is_i64 {
                            let scratch = self.si(6);
                            f.instruction(&Instruction::LocalSet(scratch));
                            f.instruction(&Instruction::I64Const(0));
                            f.instruction(&Instruction::LocalGet(scratch));
                            f.instruction(&Instruction::I64Sub);
                        } else {
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
            Operand::Call(name, args) => {
                let canonical = normalize_intrinsic(name).to_string();
                // Check if this is a builtin — redirect to inline implementation
                if self.is_builtin_name(&canonical) {
                    self.emit_call_builtin_operand(f, &canonical, args);
                } else {
                    // Check if callee has Any-typed (generic) params needing boxing
                    let param_types = self.fn_param_types.get(canonical.as_str()).cloned();
                    for (i, arg) in args.iter().enumerate() {
                        self.emit_operand(f, arg);
                        // Box i32/bool/char → ref.i31 when callee expects anyref
                        if let Some(ref pts) = param_types {
                            if i < pts.len() && pts[i] == Type::Any {
                                let arg_vt = self.infer_operand_type(arg);
                                if arg_vt == ValType::I32 {
                                    f.instruction(&Instruction::RefI31);
                                }
                                // ref types (String, struct, enum) are anyref-compatible
                            }
                        }
                    }
                    if let Some(&fn_idx) = self.fn_map.get(canonical.as_str()) {
                        f.instruction(&Instruction::Call(fn_idx));
                    } else {
                        // Unknown function: push zero
                        f.instruction(&Instruction::I32Const(0));
                    }
                    // Unbox anyref return → concrete type based on arg-inferred substitution
                    if let Some(ret_ty) = self.fn_ret_types.get(canonical.as_str()).cloned() {
                        if ret_ty == Type::Any {
                            // Infer concrete type from first Any-typed arg
                            let concrete = self.infer_generic_return_type(&canonical, args);
                            self.emit_anyref_unbox(f, &concrete);
                        }
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
                // Determine the result type from non-Unit branches
                let result_vt =
                    self.infer_if_result_type(then_result.as_deref(), else_result.as_deref());
                let result_is_anyref = matches!(
                    result_vt,
                    ValType::Ref(WasmRefType {
                        heap_type: HeapType::Abstract {
                            ty: wasm_encoder::AbstractHeapType::Any,
                            ..
                        },
                        ..
                    })
                );
                self.emit_operand(f, cond);
                f.instruction(&Instruction::If(wasm_encoder::BlockType::Result(result_vt)));
                for s in then_body {
                    self.emit_stmt(f, s);
                }
                match then_result.as_deref() {
                    Some(Operand::Unit) | None => {
                        self.emit_default_value(f, &result_vt);
                    }
                    Some(r) => {
                        self.emit_operand(f, r);
                        // Box value types to anyref when block expects anyref
                        if result_is_anyref {
                            let r_vt = self.infer_operand_type(r);
                            if r_vt == ValType::I32 {
                                f.instruction(&Instruction::RefI31);
                            }
                        }
                    }
                }
                f.instruction(&Instruction::Else);
                for s in else_body {
                    self.emit_stmt(f, s);
                }
                match else_result.as_deref() {
                    Some(Operand::Unit) | None => {
                        self.emit_default_value(f, &result_vt);
                    }
                    Some(r) => {
                        self.emit_operand(f, r);
                        // Box value types to anyref when block expects anyref
                        if result_is_anyref {
                            let r_vt = self.infer_operand_type(r);
                            if r_vt == ValType::I32 {
                                f.instruction(&Instruction::RefI31);
                            }
                        }
                    }
                }
                f.instruction(&Instruction::End);
            }
            Operand::StructInit { name, fields } => {
                // GC-native: push field values in order, then struct.new
                let layout = self.struct_layouts.get(name).cloned().unwrap_or_default();
                let ty_idx = self.struct_gc_types.get(name).copied().unwrap_or(0);
                for (fname, _fty) in &layout {
                    if let Some((_, val)) = fields.iter().find(|(n, _)| n == fname) {
                        self.emit_operand(f, val);
                    } else {
                        // Default value for missing field
                        f.instruction(&Instruction::I32Const(0));
                    }
                }
                f.instruction(&Instruction::StructNew(ty_idx));
            }
            Operand::FieldAccess {
                object,
                struct_name,
                field,
            } => {
                // GC-native: struct.get $T $field_idx
                let layout = self
                    .struct_layouts
                    .get(struct_name)
                    .cloned()
                    .unwrap_or_default();
                let ty_idx = self.struct_gc_types.get(struct_name).copied().unwrap_or(0);
                let field_idx = layout
                    .iter()
                    .position(|(fname, _)| fname == field)
                    .unwrap_or(0) as u32;
                self.emit_operand(f, object);
                f.instruction(&Instruction::StructGet {
                    struct_type_index: ty_idx,
                    field_index: field_idx,
                });
            }
            Operand::EnumInit {
                enum_name,
                variant,
                tag: _,
                payload,
            } => {
                // GC-native: push payload fields, then struct.new $Variant
                let ty_idx = self
                    .enum_variant_types
                    .get(enum_name.as_str())
                    .and_then(|vs| vs.get(variant.as_str()))
                    .copied()
                    .unwrap_or(0);
                for p in payload.iter() {
                    self.emit_operand(f, p);
                }
                f.instruction(&Instruction::StructNew(ty_idx));
            }
            Operand::EnumTag(inner) => {
                // GC-native: produce ordinal via ref.test chain
                let enum_name = self.infer_enum_name(inner);
                if let Some(variants) = self.enum_defs.get(&enum_name).cloned() {
                    let variant_types: Vec<u32> = variants
                        .iter()
                        .map(|(vname, _)| {
                            self.enum_variant_types
                                .get(enum_name.as_str())
                                .and_then(|vs| vs.get(vname.as_str()))
                                .copied()
                                .unwrap_or(0)
                        })
                        .collect();
                    if variant_types.len() <= 1 {
                        f.instruction(&Instruction::I32Const(0));
                    } else {
                        // Nested if-else: ref.test $V0 ? 0 : ref.test $V1 ? 1 : ...
                        for (i, &vty) in variant_types.iter().enumerate() {
                            if i == variant_types.len() - 1 {
                                f.instruction(&Instruction::I32Const(i as i32));
                            } else {
                                self.emit_operand(f, inner);
                                f.instruction(&Instruction::RefTestNonNull(HeapType::Concrete(
                                    vty,
                                )));
                                f.instruction(&Instruction::If(wasm_encoder::BlockType::Result(
                                    ValType::I32,
                                )));
                                f.instruction(&Instruction::I32Const(i as i32));
                                f.instruction(&Instruction::Else);
                            }
                        }
                        for _ in 0..variant_types.len() - 1 {
                            f.instruction(&Instruction::End);
                        }
                    }
                } else {
                    f.instruction(&Instruction::I32Const(0));
                }
            }
            Operand::EnumPayload {
                object,
                index,
                enum_name,
                variant_name,
            } => {
                // GC-native: ref.cast to variant type, then struct.get
                let variant_ty = self
                    .enum_variant_types
                    .get(enum_name.as_str())
                    .and_then(|vs| vs.get(variant_name.as_str()))
                    .copied()
                    .unwrap_or(0);
                self.emit_operand(f, object);
                f.instruction(&Instruction::RefCastNonNull(HeapType::Concrete(variant_ty)));
                f.instruction(&Instruction::StructGet {
                    struct_type_index: variant_ty,
                    field_index: *index,
                });
            }
            Operand::LoopExpr { body, result, .. } => {
                for s in body {
                    self.emit_stmt(f, s);
                }
                self.emit_operand(f, result);
            }
            Operand::TryExpr { expr, .. } => {
                // ? operator: evaluate expr (returns Result enum ref),
                // if Ok → extract payload, if Err → early return
                self.emit_try_expr(f, expr);
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
                // Determine signature from arg types
                let params: Vec<ValType> = args
                    .iter()
                    .map(|a| {
                        if self.is_f64_like_operand(a) {
                            ValType::F64
                        } else if self.is_i64_like_operand(a) {
                            ValType::I64
                        } else {
                            ValType::I32
                        }
                    })
                    .collect();
                let results = vec![ValType::I32];
                let type_index = self
                    .indirect_types
                    .get(&(params, results))
                    .copied()
                    .unwrap_or(0);
                f.instruction(&Instruction::CallIndirect {
                    type_index,
                    table_index: 0,
                });
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
                | "to_string"
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
                | "string_slice"
                | "contains"
                | "to_uppercase"
                | "to_lowercase"
                | "to_upper"
                | "to_lower"
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
                | "HashMap_i32_i32_new"
                | "HashMap_i32_i32_insert"
                | "HashMap_i32_i32_get"
                | "HashMap_i32_i32_contains_key"
                | "HashMap_i32_i32_len"
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
                | "filter_i64"
                | "filter_f64"
                | "map_i64_i64"
                | "map_f64_f64"
                | "fold_i64_i64"
                | "String_new"
                | "string_new"
                | "is_empty"
                | "clone"
                | "contains_i32"
                | "contains_String"
                | "reverse_i32"
                | "reverse_String"
                | "remove_i32"
                | "sum_i32"
                | "sum_i64"
                | "sum_f64"
                | "product_i32"
                | "product_i64"
                | "product_f64"
                | "any_i32"
                | "any_String"
                | "find_i32"
                | "find_String"
                | "fold_i32_i32"
                | "fold_f64_f64"
                | "map_String_String"
                | "map_i64_String"
                | "map_f64_String"
                | "fs_read_file"
                | "fs_write_file"
        ) || (name.starts_with("Vec_new_") && self.custom_vec_types.contains_key(&name[8..]))
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
                self.emit_join_gc(f, args);
                if let Some(Place::Local(id)) = dest {
                    f.instruction(&Instruction::LocalSet(self.local_wasm_idx(id.0)));
                } else {
                    f.instruction(&Instruction::Drop);
                }
            }
            "Vec_new_i32" => {
                self.emit_vec_new_gc(f, self.vec_i32_ty, self.arr_i32_ty);
                if let Some(Place::Local(id)) = dest {
                    f.instruction(&Instruction::LocalSet(self.local_wasm_idx(id.0)));
                } else {
                    f.instruction(&Instruction::Drop);
                }
            }
            "Vec_new_i64" => {
                self.emit_vec_new_gc(f, self.vec_i64_ty, self.arr_i64_ty);
                if let Some(Place::Local(id)) = dest {
                    f.instruction(&Instruction::LocalSet(self.local_wasm_idx(id.0)));
                } else {
                    f.instruction(&Instruction::Drop);
                }
            }
            "Vec_new_f64" => {
                self.emit_vec_new_gc(f, self.vec_f64_ty, self.arr_f64_ty);
                if let Some(Place::Local(id)) = dest {
                    f.instruction(&Instruction::LocalSet(self.local_wasm_idx(id.0)));
                } else {
                    f.instruction(&Instruction::Drop);
                }
            }
            "Vec_new_String" => {
                self.emit_vec_new_gc(f, self.vec_string_ty, self.arr_string_ty);
                if let Some(Place::Local(id)) = dest {
                    f.instruction(&Instruction::LocalSet(self.local_wasm_idx(id.0)));
                } else {
                    f.instruction(&Instruction::Drop);
                }
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
            "string_len" | "char_at" | "substring" | "string_slice" | "clone" | "to_uppercase"
            | "to_lowercase" | "to_upper" | "to_lower" | "trim" | "contains" | "starts_with"
            | "ends_with" | "replace" | "split" => {
                // Delegate to operand version then store/drop
                self.emit_call_builtin_operand(f, canonical, args);
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
            "String_new" | "string_new" => {
                // Empty GC string: array.new $string 0
                f.instruction(&Instruction::I32Const(0)); // fill value
                f.instruction(&Instruction::I32Const(0)); // length
                f.instruction(&Instruction::ArrayNew(self.string_ty));
                if let Some(Place::Local(id)) = dest {
                    f.instruction(&Instruction::LocalSet(self.local_wasm_idx(id.0)));
                } else {
                    f.instruction(&Instruction::Drop);
                }
            }
            "is_empty" => {
                // is_empty(s) → array.len(s) == 0
                if let Some(arg) = args.first() {
                    self.emit_operand(f, arg);
                    f.instruction(&Instruction::ArrayLen);
                    f.instruction(&Instruction::I32Eqz);
                    if let Some(Place::Local(id)) = dest {
                        f.instruction(&Instruction::LocalSet(self.local_wasm_idx(id.0)));
                    } else {
                        f.instruction(&Instruction::Drop);
                    }
                }
            }
            "to_string" => {
                if let Some(arg) = args.first() {
                    if self.is_string_like_operand(arg) {
                        self.emit_operand(f, arg);
                    } else if self.is_f64_like_operand(arg) {
                        let converted = Operand::Call("f64_to_string".to_string(), args.to_vec());
                        self.emit_operand(f, &converted);
                    } else if self.is_i64_like_operand(arg) {
                        let converted = Operand::Call("i64_to_string".to_string(), args.to_vec());
                        self.emit_operand(f, &converted);
                    } else if self.is_bool_like_operand(arg) {
                        let converted = Operand::Call("bool_to_string".to_string(), args.to_vec());
                        self.emit_operand(f, &converted);
                    } else {
                        let converted = Operand::Call("i32_to_string".to_string(), args.to_vec());
                        self.emit_operand(f, &converted);
                    }
                    if let Some(Place::Local(id)) = dest {
                        f.instruction(&Instruction::LocalSet(self.local_wasm_idx(id.0)));
                    } else {
                        f.instruction(&Instruction::Drop);
                    }
                }
            }
            "filter_i64" | "filter_f64" | "filter_i32" | "filter_String" => {
                self.emit_filter_hof_gc(f, canonical, args);
                if let Some(Place::Local(id)) = dest {
                    f.instruction(&Instruction::LocalSet(self.local_wasm_idx(id.0)));
                } else {
                    f.instruction(&Instruction::Drop);
                }
            }
            "map_i64_i64" | "map_f64_f64" | "map_i32_i32" | "map_i32_String" => {
                self.emit_map_hof_gc(f, canonical, args);
                if let Some(Place::Local(id)) = dest {
                    f.instruction(&Instruction::LocalSet(self.local_wasm_idx(id.0)));
                } else {
                    f.instruction(&Instruction::Drop);
                }
            }
            "fold_i64_i64" | "fold_i32_i32" | "fold_f64_f64" => {
                self.emit_fold_hof_gc(f, canonical, args);
                if let Some(Place::Local(id)) = dest {
                    f.instruction(&Instruction::LocalSet(self.local_wasm_idx(id.0)));
                } else {
                    f.instruction(&Instruction::Drop);
                }
            }
            "map_String_String" | "map_i64_String" | "map_f64_String" => {
                self.emit_map_hof_gc(f, canonical, args);
                if let Some(Place::Local(id)) = dest {
                    f.instruction(&Instruction::LocalSet(self.local_wasm_idx(id.0)));
                } else {
                    f.instruction(&Instruction::Drop);
                }
            }
            "contains_i32" | "contains_String" => {
                self.emit_contains_inline(f, canonical, args);
                if let Some(Place::Local(id)) = dest {
                    f.instruction(&Instruction::LocalSet(self.local_wasm_idx(id.0)));
                } else {
                    f.instruction(&Instruction::Drop);
                }
            }
            "reverse_i32" | "reverse_String" => {
                self.emit_reverse_inline(f, canonical, args);
            }
            "remove_i32" => {
                self.emit_remove_inline(f, args);
            }
            "sum_i32" | "sum_i64" | "sum_f64" | "product_i32" | "product_i64" | "product_f64" => {
                self.emit_sum_product_inline(f, canonical, args);
                if let Some(Place::Local(id)) = dest {
                    f.instruction(&Instruction::LocalSet(self.local_wasm_idx(id.0)));
                } else {
                    f.instruction(&Instruction::Drop);
                }
            }
            "any_i32" | "any_String" => {
                self.emit_any_hof_gc(f, canonical, args);
                if let Some(Place::Local(id)) = dest {
                    f.instruction(&Instruction::LocalSet(self.local_wasm_idx(id.0)));
                } else {
                    f.instruction(&Instruction::Drop);
                }
            }
            "find_i32" | "find_String" => {
                self.emit_find_hof_gc(f, canonical, args);
                if let Some(Place::Local(id)) = dest {
                    f.instruction(&Instruction::LocalSet(self.local_wasm_idx(id.0)));
                } else {
                    f.instruction(&Instruction::Drop);
                }
            }
            "fs_read_file" => {
                self.emit_fs_read_file_gc(f, args);
                if let Some(Place::Local(id)) = dest {
                    f.instruction(&Instruction::LocalSet(self.local_wasm_idx(id.0)));
                } else {
                    f.instruction(&Instruction::Drop);
                }
            }
            "fs_write_file" => {
                self.emit_fs_write_file_gc(f, args);
                if let Some(Place::Local(id)) = dest {
                    f.instruction(&Instruction::LocalSet(self.local_wasm_idx(id.0)));
                } else {
                    f.instruction(&Instruction::Drop);
                }
            }
            _ if canonical.starts_with("Vec_new_") => {
                let sname = &canonical[8..];
                if let Some(&(arr_ty, vec_ty)) = self.custom_vec_types.get(sname) {
                    self.emit_vec_new_gc(f, vec_ty, arr_ty);
                    if let Some(Place::Local(id)) = dest {
                        f.instruction(&Instruction::LocalSet(self.local_wasm_idx(id.0)));
                    } else {
                        f.instruction(&Instruction::Drop);
                    }
                }
            }
            "HashMap_i32_i32_new" => {
                // Create keys and values arrays (initial capacity 16), then struct
                let hm_ty = self.hashmap_i32_i32_ty;
                let arr_ty = self.arr_i32_ty;
                f.instruction(&Instruction::I32Const(0)); // fill value
                f.instruction(&Instruction::I32Const(16)); // initial capacity
                f.instruction(&Instruction::ArrayNew(arr_ty));
                f.instruction(&Instruction::I32Const(0));
                f.instruction(&Instruction::I32Const(16));
                f.instruction(&Instruction::ArrayNew(arr_ty));
                f.instruction(&Instruction::I32Const(0)); // count = 0
                f.instruction(&Instruction::StructNew(hm_ty));
                if let Some(Place::Local(id)) = dest {
                    f.instruction(&Instruction::LocalSet(self.local_wasm_idx(id.0)));
                } else {
                    f.instruction(&Instruction::Drop);
                }
            }
            "HashMap_i32_i32_insert" => {
                self.emit_hashmap_i32_i32_insert(f, args);
            }
            "HashMap_i32_i32_get" => {
                self.emit_hashmap_i32_i32_get(f, args);
                if let Some(Place::Local(id)) = dest {
                    f.instruction(&Instruction::LocalSet(self.local_wasm_idx(id.0)));
                } else {
                    f.instruction(&Instruction::Drop);
                }
            }
            "HashMap_i32_i32_len" => {
                // struct.get $hashmap_i32_i32 2 (count field)
                if let Some(arg) = args.first() {
                    self.emit_operand(f, arg);
                    f.instruction(&Instruction::StructGet {
                        struct_type_index: self.hashmap_i32_i32_ty,
                        field_index: 2,
                    });
                } else {
                    f.instruction(&Instruction::I32Const(0));
                }
                if let Some(Place::Local(id)) = dest {
                    f.instruction(&Instruction::LocalSet(self.local_wasm_idx(id.0)));
                } else {
                    f.instruction(&Instruction::Drop);
                }
            }
            "HashMap_i32_i32_contains_key" => {
                self.emit_hashmap_i32_i32_contains_key(f, args);
                if let Some(Place::Local(id)) = dest {
                    f.instruction(&Instruction::LocalSet(self.local_wasm_idx(id.0)));
                } else {
                    f.instruction(&Instruction::Drop);
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
            "bool_to_string" => {
                // GC-native: if arg then "true" else "false" as GC arrays
                if let Some(arg) = args.first() {
                    let str_ref = ref_nullable(self.string_ty);
                    self.emit_operand(f, arg);
                    f.instruction(&Instruction::If(wasm_encoder::BlockType::Result(str_ref)));
                    let seg = self.alloc_string_data(b"true");
                    let abs_seg = self.data_segs.len() as u32 + seg;
                    f.instruction(&Instruction::I32Const(0));
                    f.instruction(&Instruction::I32Const(4));
                    f.instruction(&Instruction::ArrayNewData {
                        array_type_index: self.string_ty,
                        array_data_index: abs_seg,
                    });
                    f.instruction(&Instruction::Else);
                    let seg = self.alloc_string_data(b"false");
                    let abs_seg = self.data_segs.len() as u32 + seg;
                    f.instruction(&Instruction::I32Const(0));
                    f.instruction(&Instruction::I32Const(5));
                    f.instruction(&Instruction::ArrayNewData {
                        array_type_index: self.string_ty,
                        array_data_index: abs_seg,
                    });
                    f.instruction(&Instruction::End);
                }
            }
            "char_to_string" => {
                // GC-native: create 1-byte GC string from char value
                if let Some(arg) = args.first() {
                    self.emit_operand(f, arg); // char value (i32 = byte)
                    f.instruction(&Instruction::I32Const(1)); // length = 1
                    f.instruction(&Instruction::ArrayNew(self.string_ty));
                }
            }
            "i64_to_string" => {
                if let Some(arg) = args.first() {
                    self.emit_operand(f, arg);
                    if let Some(idx) = self.helper_i64_to_str {
                        f.instruction(&Instruction::Call(idx));
                    }
                }
            }
            "f64_to_string" => {
                if let Some(arg) = args.first() {
                    self.emit_operand(f, arg);
                    if let Some(idx) = self.helper_f64_to_str {
                        f.instruction(&Instruction::Call(idx));
                    }
                }
            }
            "String_from" | "String_new" | "string_new" => {
                if let Some(arg) = args.first() {
                    self.emit_operand(f, arg);
                } else {
                    // String::new() → empty GC string
                    f.instruction(&Instruction::I32Const(0)); // fill
                    f.instruction(&Instruction::I32Const(0)); // length
                    f.instruction(&Instruction::ArrayNew(self.string_ty));
                }
            }
            "to_string" => {
                if let Some(arg) = args.first() {
                    if self.is_string_like_operand(arg) {
                        self.emit_operand(f, arg);
                    } else if self.is_f64_like_operand(arg) {
                        let converted = Operand::Call("f64_to_string".to_string(), args.to_vec());
                        self.emit_operand(f, &converted);
                    } else if self.is_i64_like_operand(arg) {
                        let converted = Operand::Call("i64_to_string".to_string(), args.to_vec());
                        self.emit_operand(f, &converted);
                    } else if self.is_bool_like_operand(arg) {
                        let converted = Operand::Call("bool_to_string".to_string(), args.to_vec());
                        self.emit_operand(f, &converted);
                    } else {
                        let converted = Operand::Call("i32_to_string".to_string(), args.to_vec());
                        self.emit_operand(f, &converted);
                    }
                }
            }
            "concat" => {
                self.emit_concat_gc(f, args);
            }
            "join" => {
                self.emit_join_gc(f, args);
            }
            "Vec_new_i32" => {
                self.emit_vec_new_gc(f, self.vec_i32_ty, self.arr_i32_ty);
            }
            "Vec_new_i64" => {
                self.emit_vec_new_gc(f, self.vec_i64_ty, self.arr_i64_ty);
            }
            "Vec_new_f64" => {
                self.emit_vec_new_gc(f, self.vec_f64_ty, self.arr_f64_ty);
            }
            "Vec_new_String" => {
                self.emit_vec_new_gc(f, self.vec_string_ty, self.arr_string_ty);
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
            "string_len" => {
                // GC-native: array.len on GC string ref
                if let Some(arg) = args.first() {
                    self.emit_operand(f, arg);
                    f.instruction(&Instruction::ArrayLen);
                } else {
                    f.instruction(&Instruction::I32Const(0));
                }
            }
            "char_at" => {
                // GC-native: array.get_u $string ref idx
                if args.len() >= 2 {
                    self.emit_operand(f, &args[0]); // string ref
                    self.emit_operand(f, &args[1]); // index
                    f.instruction(&Instruction::ArrayGetU(self.string_ty));
                } else {
                    f.instruction(&Instruction::I32Const(0));
                }
            }
            "substring" | "string_slice" => {
                // GC-native: new array + array.copy slice
                if args.len() >= 3 {
                    self.emit_substring_gc(f, &args[0], &args[1], &args[2]);
                } else {
                    f.instruction(&Instruction::I32Const(0)); // fill
                    f.instruction(&Instruction::I32Const(0)); // len
                    f.instruction(&Instruction::ArrayNew(self.string_ty));
                }
            }
            "clone" => {
                // GC-native: copy entire string to new GC array
                if let Some(arg) = args.first() {
                    self.emit_string_clone_gc(f, arg);
                } else {
                    f.instruction(&Instruction::I32Const(0));
                    f.instruction(&Instruction::I32Const(0));
                    f.instruction(&Instruction::ArrayNew(self.string_ty));
                }
            }
            "to_uppercase" | "to_lowercase" | "to_upper" | "to_lower" => {
                if let Some(arg) = args.first() {
                    self.emit_case_transform_gc(
                        f,
                        arg,
                        canonical == "to_uppercase" || canonical == "to_upper",
                    );
                } else {
                    f.instruction(&Instruction::I32Const(0));
                    f.instruction(&Instruction::I32Const(0));
                    f.instruction(&Instruction::ArrayNew(self.string_ty));
                }
            }
            "trim" => {
                if let Some(arg) = args.first() {
                    self.emit_trim_gc(f, arg);
                } else {
                    f.instruction(&Instruction::I32Const(0));
                    f.instruction(&Instruction::I32Const(0));
                    f.instruction(&Instruction::ArrayNew(self.string_ty));
                }
            }
            "contains" => {
                if args.len() >= 2 {
                    self.emit_contains_gc(f, &args[0], &args[1]);
                } else {
                    f.instruction(&Instruction::I32Const(0));
                }
            }
            "starts_with" => {
                if args.len() >= 2 {
                    self.emit_starts_with_gc(f, &args[0], &args[1]);
                } else {
                    f.instruction(&Instruction::I32Const(0));
                }
            }
            "ends_with" => {
                if args.len() >= 2 {
                    self.emit_ends_with_gc(f, &args[0], &args[1]);
                } else {
                    f.instruction(&Instruction::I32Const(0));
                }
            }
            "replace" => {
                // Stub: return clone of input
                if let Some(arg) = args.first() {
                    self.emit_string_clone_gc(f, arg);
                } else {
                    f.instruction(&Instruction::I32Const(0));
                    f.instruction(&Instruction::I32Const(0));
                    f.instruction(&Instruction::ArrayNew(self.string_ty));
                }
            }
            "split" => {
                // Stub: return empty Vec<String>
                self.emit_vec_new_gc(f, self.vec_string_ty, self.arr_string_ty);
            }
            "filter_i64" | "filter_f64" | "filter_i32" | "filter_String" => {
                self.emit_filter_hof_gc(f, canonical, args);
            }
            "map_i64_i64" | "map_f64_f64" | "map_i32_i32" | "map_i32_String" => {
                self.emit_map_hof_gc(f, canonical, args);
            }
            "fold_i64_i64" => {
                self.emit_fold_hof_gc(f, canonical, args);
            }
            "contains_i32" | "contains_String" => {
                self.emit_contains_inline(f, canonical, args);
            }
            "reverse_i32" | "reverse_String" => {
                self.emit_reverse_inline(f, canonical, args);
                // reverse is void; push dummy 0
                f.instruction(&Instruction::I32Const(0));
            }
            "remove_i32" => {
                self.emit_remove_inline(f, args);
                f.instruction(&Instruction::I32Const(0));
            }
            "sum_i32" | "sum_i64" | "sum_f64" | "product_i32" | "product_i64" | "product_f64" => {
                self.emit_sum_product_inline(f, canonical, args);
            }
            "fold_i32_i32" | "fold_f64_f64" => {
                self.emit_fold_hof_gc(f, canonical, args);
            }
            "map_String_String" | "map_i64_String" | "map_f64_String" => {
                self.emit_map_hof_gc(f, canonical, args);
            }
            "any_i32" | "any_String" => {
                self.emit_any_hof_gc(f, canonical, args);
            }
            "find_i32" | "find_String" => {
                self.emit_find_hof_gc(f, canonical, args);
            }
            "is_empty" => {
                // is_empty(s) → array.len(s) == 0
                if let Some(arg) = args.first() {
                    self.emit_operand(f, arg);
                    f.instruction(&Instruction::ArrayLen);
                    f.instruction(&Instruction::I32Eqz);
                } else {
                    f.instruction(&Instruction::I32Const(1));
                }
            }
            _ if canonical.starts_with("Vec_new_") => {
                let sname = &canonical[8..];
                if let Some(&(arr_ty, vec_ty)) = self.custom_vec_types.get(sname) {
                    self.emit_vec_new_gc(f, vec_ty, arr_ty);
                } else {
                    f.instruction(&Instruction::I32Const(0));
                }
            }
            "parse_i32" => {
                if let Some(arg) = args.first() {
                    self.emit_operand(f, arg);
                    if let Some(idx) = self.helper_parse_i32 {
                        f.instruction(&Instruction::Call(idx));
                    }
                }
            }
            "parse_i64" => {
                if let Some(arg) = args.first() {
                    self.emit_operand(f, arg);
                    if let Some(idx) = self.helper_parse_i64 {
                        f.instruction(&Instruction::Call(idx));
                    }
                }
            }
            "parse_f64" => {
                if let Some(arg) = args.first() {
                    self.emit_operand(f, arg);
                    if let Some(idx) = self.helper_parse_f64 {
                        f.instruction(&Instruction::Call(idx));
                    }
                }
            }
            "fs_read_file" => {
                self.emit_fs_read_file_gc(f, args);
            }
            "fs_write_file" => {
                self.emit_fs_write_file_gc(f, args);
            }
            "HashMap_i32_i32_new" => {
                let hm_ty = self.hashmap_i32_i32_ty;
                let arr_ty = self.arr_i32_ty;
                f.instruction(&Instruction::I32Const(0));
                f.instruction(&Instruction::I32Const(16));
                f.instruction(&Instruction::ArrayNew(arr_ty));
                f.instruction(&Instruction::I32Const(0));
                f.instruction(&Instruction::I32Const(16));
                f.instruction(&Instruction::ArrayNew(arr_ty));
                f.instruction(&Instruction::I32Const(0));
                f.instruction(&Instruction::StructNew(hm_ty));
            }
            "HashMap_i32_i32_get" => {
                self.emit_hashmap_i32_i32_get(f, args);
            }
            "HashMap_i32_i32_len" => {
                if let Some(arg) = args.first() {
                    self.emit_operand(f, arg);
                    f.instruction(&Instruction::StructGet {
                        struct_type_index: self.hashmap_i32_i32_ty,
                        field_index: 2,
                    });
                } else {
                    f.instruction(&Instruction::I32Const(0));
                }
            }
            "HashMap_i32_i32_contains_key" => {
                self.emit_hashmap_i32_i32_contains_key(f, args);
            }
            _ => {
                // Unimplemented builtin as operand — push null ref for string types
                // or zero for scalars
                f.instruction(&Instruction::I32Const(0));
            }
        }
    }

    /// Emit the ? operator: evaluate expr (Result enum), extract Ok payload or early-return Err
    fn emit_try_expr(&mut self, f: &mut Function, expr: &Operand) {
        // Determine which Result enum type the expr produces
        let result_type = self.infer_operand_type(expr);

        // Find the matching enum and its variants
        let (enum_name, ok_ty, err_ty) = if let ValType::Ref(rt) = result_type {
            match rt.heap_type {
                HeapType::Concrete(idx) => {
                    // Find which enum has this base type
                    let mut found = None;
                    for (name, &base) in &self.enum_base_types {
                        if base == idx {
                            found = Some(name.clone());
                            break;
                        }
                    }
                    if let Some(ename) = found {
                        let variants = self.enum_variant_types.get(&ename).cloned();
                        if let Some(v) = variants {
                            let ok_idx = v.get("Ok").copied();
                            let err_idx = v.get("Err").copied();
                            (Some(ename), ok_idx, err_idx)
                        } else {
                            (None, None, None)
                        }
                    } else {
                        (None, None, None)
                    }
                }
                _ => (None, None, None),
            }
        } else {
            (None, None, None)
        };

        if let (Some(_ename), Some(ok_variant), Some(err_variant)) = (enum_name, ok_ty, err_ty) {
            let anyref_scratch = self.si(10);

            // Emit inner expression → ref $Result on stack
            self.emit_operand(f, expr);

            // Save to anyref scratch
            f.instruction(&Instruction::LocalSet(anyref_scratch));

            // Check if Ok variant using ref.test
            f.instruction(&Instruction::LocalGet(anyref_scratch));
            f.instruction(&Instruction::RefTestNonNull(HeapType::Concrete(ok_variant)));

            // Determine Ok payload type from enum_defs
            let ok_payload_ty = self.get_ok_payload_type(&_ename);

            f.instruction(&Instruction::If(wasm_encoder::BlockType::Result(
                ok_payload_ty,
            )));
            {
                // Ok path: cast and extract payload
                f.instruction(&Instruction::LocalGet(anyref_scratch));
                f.instruction(&Instruction::RefCastNonNull(HeapType::Concrete(ok_variant)));
                f.instruction(&Instruction::StructGet {
                    struct_type_index: ok_variant,
                    field_index: 0,
                });
            }
            f.instruction(&Instruction::Else);
            {
                // Err path: cast to Err variant, re-wrap in function return type, and return
                // In the common case (same Result type), just return the original ref
                f.instruction(&Instruction::LocalGet(anyref_scratch));
                f.instruction(&Instruction::RefCastNonNull(HeapType::Concrete(
                    err_variant,
                )));
                // Extract Err payload (ref $string)
                f.instruction(&Instruction::StructGet {
                    struct_type_index: err_variant,
                    field_index: 0,
                });
                // Re-wrap in function's return Result.Err
                // For simplicity, assume function returns same Result type
                f.instruction(&Instruction::StructNew(err_variant));
                f.instruction(&Instruction::Return);
            }
            f.instruction(&Instruction::End);
        } else {
            // Fallback: just emit inner expression
            self.emit_operand(f, expr);
        }
    }

    /// Get the Wasm ValType of an Ok variant's payload from enum_defs
    fn get_ok_payload_type(&self, enum_name: &str) -> ValType {
        if let Some(variants) = self.enum_defs.get(enum_name) {
            for (vname, fields) in variants {
                if vname == "Ok" {
                    if let Some(field_type) = fields.first() {
                        return self.type_name_to_val(field_type);
                    }
                }
            }
        }
        ValType::I32 // default
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
        let vt = self.infer_operand_type(operand);
        matches!(vt, ValType::Ref(rt) if rt.heap_type == HeapType::Concrete(self.string_ty))
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
            Operand::BinOp(op, _, _) => matches!(
                op,
                BinOp::Eq
                    | BinOp::Ne
                    | BinOp::Lt
                    | BinOp::Le
                    | BinOp::Gt
                    | BinOp::Ge
                    | BinOp::And
                    | BinOp::Or
            ),
            Operand::UnaryOp(op, _) => matches!(op, UnaryOp::Not),
            _ => false,
        }
    }

    fn is_f64_like_operand(&self, operand: &Operand) -> bool {
        match operand {
            Operand::ConstF64(_) | Operand::ConstF32(_) => true,
            Operand::Place(Place::Local(id)) => self.f64_locals.contains(&id.0),
            Operand::BinOp(_, l, r) => self.is_f64_like_operand(l) || self.is_f64_like_operand(r),
            Operand::UnaryOp(_, inner) => self.is_f64_like_operand(inner),
            Operand::Call(name, _) => {
                let canonical = normalize_intrinsic(name);
                matches!(canonical, "sqrt" | "random_f64")
                    || self.fn_ret_types.get(name) == Some(&Type::F64)
            }
            _ => false,
        }
    }

    fn is_i64_like_operand(&self, operand: &Operand) -> bool {
        match operand {
            Operand::ConstI64(_) | Operand::ConstU64(_) => true,
            Operand::Place(Place::Local(id)) => self.i64_locals.contains(&id.0),
            Operand::BinOp(_, l, r) => self.is_i64_like_operand(l) || self.is_i64_like_operand(r),
            Operand::UnaryOp(_, inner) => self.is_i64_like_operand(inner),
            Operand::Call(name, _) => {
                let canonical = normalize_intrinsic(name);
                matches!(canonical, "clock_now") || self.fn_ret_types.get(name) == Some(&Type::I64)
            }
            _ => false,
        }
    }

    /// Determine Vec element size from the vec operand (checks f64_vec_locals/i64_vec_locals).
    fn vec_elem_size(&self, vec_operand: &Operand) -> i32 {
        match vec_operand {
            Operand::Place(Place::Local(id)) => {
                if self.f64_vec_locals.contains(&id.0) || self.i64_vec_locals.contains(&id.0) {
                    8
                } else {
                    4
                }
            }
            _ => 4,
        }
    }

    /// Check if a Vec operand holds f64 elements.
    fn is_f64_vec_operand(&self, operand: &Operand) -> bool {
        matches!(operand, Operand::Place(Place::Local(id)) if self.f64_vec_locals.contains(&id.0))
    }

    /// Check if a Vec operand holds i64 elements.
    fn is_i64_vec_operand(&self, operand: &Operand) -> bool {
        matches!(operand, Operand::Place(Place::Local(id)) if self.i64_vec_locals.contains(&id.0))
    }

    /// Check if a Vec operand holds string elements.
    fn is_string_vec_operand(&self, operand: &Operand) -> bool {
        let vt = self.infer_operand_type(operand);
        matches!(vt, ValType::Ref(rt) if rt.heap_type == HeapType::Concrete(self.vec_string_ty))
    }

    /// Infer the GC vec struct type index for a vec operand.
    fn infer_vec_type_idx(&self, operand: &Operand) -> u32 {
        if self.is_f64_vec_operand(operand) {
            self.vec_f64_ty
        } else if self.is_i64_vec_operand(operand) {
            self.vec_i64_ty
        } else if self.is_string_vec_operand(operand) {
            self.vec_string_ty
        } else if let Some(sname) = self.get_struct_vec_name(operand) {
            self.custom_vec_types
                .get(&sname)
                .map(|&(_, v)| v)
                .unwrap_or(self.vec_i32_ty)
        } else {
            self.vec_i32_ty
        }
    }

    /// Infer the GC array type index for a vec operand's backing array.
    fn infer_arr_type_idx(&self, operand: &Operand) -> u32 {
        if self.is_f64_vec_operand(operand) {
            self.arr_f64_ty
        } else if self.is_i64_vec_operand(operand) {
            self.arr_i64_ty
        } else if self.is_string_vec_operand(operand) {
            self.arr_string_ty
        } else if let Some(sname) = self.get_struct_vec_name(operand) {
            self.custom_vec_types
                .get(&sname)
                .map(|&(a, _)| a)
                .unwrap_or(self.arr_i32_ty)
        } else {
            self.arr_i32_ty
        }
    }

    /// If operand is a local that holds a Vec<Struct>, return the struct name.
    fn get_struct_vec_name(&self, operand: &Operand) -> Option<String> {
        match operand {
            Operand::Place(Place::Local(id)) => self.struct_vec_locals.get(&id.0).cloned(),
            _ => None,
        }
    }

    fn emit_concat(&mut self, f: &mut Function, _args: &[Operand], dest: Option<&Place>) {
        self.emit_concat_gc(f, _args);
        if let Some(Place::Local(id)) = dest {
            f.instruction(&Instruction::LocalSet(self.local_wasm_idx(id.0)));
        } else {
            f.instruction(&Instruction::Drop);
        }
    }

    // ── GC-native string operations ─────────────────────────────────

    /// scratch local indices relative to scratch_base
    fn si(&self, offset: u32) -> u32 {
        self.scratch_base + offset
    }

    /// Infer the enum name from an operand (for EnumTag dispatch).
    fn infer_enum_name(&self, operand: &Operand) -> String {
        match operand {
            Operand::Place(Place::Local(id)) => {
                self.local_enum.get(&id.0).cloned().unwrap_or_default()
            }
            _ => String::new(),
        }
    }

    /// Infer the Wasm ValType for an if-expression result from its branches.
    fn infer_if_result_type(
        &self,
        then_result: Option<&Operand>,
        else_result: Option<&Operand>,
    ) -> ValType {
        let mut types = vec![];
        for result in [then_result, else_result] {
            match result {
                Some(Operand::Unit) | None => continue,
                Some(op) => types.push(self.infer_operand_type(op)),
            }
        }
        let anyref_vt = ValType::Ref(WasmRefType {
            nullable: true,
            heap_type: HeapType::Abstract {
                shared: false,
                ty: wasm_encoder::AbstractHeapType::Any,
            },
        });
        // If any branch produces anyref, the result type must be anyref
        // (value types in the other branch will be boxed via ref.i31)
        if types.iter().any(|t| *t == anyref_vt) {
            return anyref_vt;
        }
        // If both branches produce enum refs and one is non-null, prefer nullable
        types.first().cloned().unwrap_or(ValType::I32)
    }

    /// Check whether an operand produces a value on the Wasm stack.
    fn operand_produces_value(&self, op: &Operand) -> bool {
        match op {
            Operand::Unit => false,
            Operand::Call(name, _) => self
                .fn_ret_types
                .get(name.as_str())
                .is_some_and(|ty| !matches!(ty, Type::Unit | Type::Never)),
            _ => true,
        }
    }

    /// Infer the Wasm ValType that an operand produces.
    fn infer_operand_type(&self, op: &Operand) -> ValType {
        match op {
            Operand::ConstI32(_)
            | Operand::ConstBool(_)
            | Operand::ConstChar(_)
            | Operand::ConstU8(_)
            | Operand::ConstU16(_)
            | Operand::ConstU32(_)
            | Operand::ConstI8(_)
            | Operand::ConstI16(_) => ValType::I32,
            Operand::ConstI64(_) | Operand::ConstU64(_) => ValType::I64,
            Operand::ConstF64(_) => ValType::F64,
            Operand::ConstF32(_) => ValType::F32,
            Operand::ConstString(_) => ref_nullable(self.string_ty),
            Operand::Place(Place::Local(id)) => {
                if self.string_locals.contains(&id.0) {
                    return ref_nullable(self.string_ty);
                }
                if self.f64_locals.contains(&id.0) {
                    return ValType::F64;
                }
                if self.i64_locals.contains(&id.0) {
                    return ValType::I64;
                }
                if self.any_locals.contains(&id.0) {
                    return ValType::Ref(WasmRefType {
                        nullable: true,
                        heap_type: HeapType::ANY,
                    });
                }
                if let Some(sname) = self.local_struct.get(&id.0) {
                    if let Some(&ty_idx) = self.struct_gc_types.get(sname) {
                        return ref_nullable(ty_idx);
                    }
                }
                if let Some(ename) = self.local_enum.get(&id.0) {
                    if let Some(&base_idx) = self.enum_base_types.get(ename) {
                        return ref_nullable(base_idx);
                    }
                }
                // Vec locals
                if self.i32_vec_locals.contains(&id.0) {
                    return ref_nullable(self.vec_i32_ty);
                }
                if self.i64_vec_locals.contains(&id.0) {
                    return ref_nullable(self.vec_i64_ty);
                }
                if self.f64_vec_locals.contains(&id.0) {
                    return ref_nullable(self.vec_f64_ty);
                }
                if self.string_vec_locals.contains(&id.0) {
                    return ref_nullable(self.vec_string_ty);
                }
                if let Some(sname) = self.struct_vec_locals.get(&id.0) {
                    if let Some(&(_, vec_ty)) = self.custom_vec_types.get(sname) {
                        return ref_nullable(vec_ty);
                    }
                }
                ValType::I32
            }
            Operand::StructInit { name, .. } => {
                if let Some(&ty_idx) = self.struct_gc_types.get(name) {
                    ref_nullable(ty_idx)
                } else {
                    ValType::I32
                }
            }
            Operand::EnumInit { enum_name, .. } => {
                if let Some(&base_idx) = self.enum_base_types.get(enum_name.as_str()) {
                    ref_nullable(base_idx)
                } else {
                    ValType::I32
                }
            }
            Operand::Call(name, args) => {
                let canonical = normalize_intrinsic(name);
                // Handle Vec_new_* builtins directly
                match canonical {
                    "Vec_new_i32" => return ref_nullable(self.vec_i32_ty),
                    "Vec_new_i64" => return ref_nullable(self.vec_i64_ty),
                    "Vec_new_f64" => return ref_nullable(self.vec_f64_ty),
                    "Vec_new_String" => return ref_nullable(self.vec_string_ty),
                    // get_unchecked/get on typed vec → infer element type
                    "get_unchecked" | "get" if !args.is_empty() => {
                        if self.is_string_vec_operand(&args[0]) {
                            return ref_nullable(self.string_ty);
                        } else if self.is_f64_vec_operand(&args[0]) {
                            return ValType::F64;
                        } else if self.is_i64_vec_operand(&args[0]) {
                            return ValType::I64;
                        } else if let Some(sname) = self.get_struct_vec_name(&args[0]) {
                            if let Some(&ty_idx) = self.struct_gc_types.get(&sname) {
                                return ref_nullable(ty_idx);
                            }
                        }
                        // For get(), it returns Result — fall through to fn_ret_type_names
                        if canonical == "get_unchecked" {
                            return ValType::I32;
                        }
                    }
                    "concat" | "clone" | "to_uppercase" | "to_lowercase" | "to_upper"
                    | "to_lower" | "trim" | "replace" | "substring" | "string_slice"
                    | "String_from" | "String_new" | "string_new" | "char_to_string"
                    | "i32_to_string" | "i64_to_string" | "f64_to_string" | "bool_to_string"
                    | "to_string" => {
                        return ref_nullable(self.string_ty);
                    }
                    "contains_i32" | "contains_String" | "len" | "string_len" | "char_at"
                    | "sum_i32" | "product_i32" => {
                        return ValType::I32;
                    }
                    "sum_i64" | "product_i64" => return ValType::I64,
                    "sum_f64" | "product_f64" => return ValType::F64,
                    "parse_i32" => {
                        if let Some(&base_idx) = self.enum_base_types.get("Result") {
                            return ref_nullable(base_idx);
                        }
                    }
                    "parse_i64" => {
                        if let Some(&base_idx) = self.enum_base_types.get("Result_i64_String") {
                            return ref_nullable(base_idx);
                        }
                    }
                    "parse_f64" => {
                        if let Some(&base_idx) = self.enum_base_types.get("Result_f64_String") {
                            return ref_nullable(base_idx);
                        }
                    }
                    _ => {}
                }
                // Check Vec_new_* for struct names
                if let Some(sname) = canonical.strip_prefix("Vec_new_") {
                    if let Some(&(_, vec_ty)) = self.custom_vec_types.get(sname) {
                        return ref_nullable(vec_ty);
                    }
                }
                if let Some(ret_name) = self.fn_ret_type_names.get(name) {
                    // Check if the function returns Any (generic) — infer concrete type
                    if let Some(ret_ty) = self.fn_ret_types.get(name) {
                        if *ret_ty == Type::Any {
                            return self.infer_generic_return_type(name, args);
                        }
                    }
                    self.type_name_to_val(ret_name)
                } else if let Some(ret_ty) = self.fn_ret_types.get(name) {
                    if *ret_ty == Type::Any {
                        return self.infer_generic_return_type(name, args);
                    }
                    self.type_to_val(ret_ty)
                } else {
                    ValType::I32
                }
            }
            Operand::FieldAccess {
                struct_name, field, ..
            } => {
                let layout = self
                    .struct_layouts
                    .get(struct_name)
                    .cloned()
                    .unwrap_or_default();
                if let Some((_, fty)) = layout.iter().find(|(n, _)| n == field) {
                    self.type_name_to_val(fty)
                } else {
                    ValType::I32
                }
            }
            Operand::BinOp { .. } | Operand::UnaryOp { .. } | Operand::EnumTag(_) => ValType::I32,
            Operand::IfExpr {
                then_result,
                else_result,
                ..
            } => self.infer_if_result_type(then_result.as_deref(), else_result.as_deref()),
            _ => ValType::I32,
        }
    }

    /// Infer the concrete return type of a generic function call from the argument types.
    /// For `identity<T>(x: T) -> T`, if called with a String arg, returns String.
    fn infer_generic_return_type(&self, fn_name: &str, args: &[Operand]) -> ValType {
        let param_types = self.fn_param_types.get(fn_name);
        if let Some(pts) = param_types {
            // Find the first Any-typed param and use its corresponding arg's type
            for (i, pt) in pts.iter().enumerate() {
                if *pt == Type::Any {
                    if let Some(arg) = args.get(i) {
                        return self.infer_operand_type(arg);
                    }
                }
            }
        }
        // Fallback: return i32
        ValType::I32
    }

    /// Emit unboxing instructions to convert an anyref on the stack to a concrete type.
    fn emit_anyref_unbox(&self, f: &mut Function, target_vt: &ValType) {
        match target_vt {
            ValType::I32 => {
                // anyref → ref.cast (ref i31) → i31.get_s
                f.instruction(&Instruction::RefCastNullable(HeapType::Abstract {
                    shared: false,
                    ty: wasm_encoder::AbstractHeapType::I31,
                }));
                f.instruction(&Instruction::I31GetS);
            }
            ValType::Ref(rt) => {
                // anyref → ref.cast (ref $concrete_type)
                f.instruction(&Instruction::RefCastNullable(rt.heap_type));
            }
            _ => {
                // For i64/f64 we'd need struct boxing — not yet implemented
            }
        }
    }

    /// Emit a default/zero value for a given ValType (used for Unit branches in if-expressions).
    fn emit_default_value(&self, f: &mut Function, vt: &ValType) {
        match vt {
            ValType::I32 => f.instruction(&Instruction::I32Const(0)),
            ValType::I64 => f.instruction(&Instruction::I64Const(0)),
            ValType::F32 => f.instruction(&Instruction::F32Const(0.0)),
            ValType::F64 => f.instruction(&Instruction::F64Const(0.0)),
            ValType::Ref(rt) => f.instruction(&Instruction::RefNull(rt.heap_type)),
            _ => f.instruction(&Instruction::I32Const(0)),
        };
    }

    /// concat(a, b) → new GC string = a ++ b
    fn emit_concat_gc(&mut self, f: &mut Function, args: &[Operand]) {
        if args.len() < 2 {
            f.instruction(&Instruction::I32Const(0));
            f.instruction(&Instruction::I32Const(0));
            f.instruction(&Instruction::ArrayNew(self.string_ty));
            return;
        }
        let s0 = self.si(4); // a_ref
        let s1 = self.si(5); // b_ref
        let s_result = self.si(8); // result_ref (3rd ref scratch)
        let len_a = self.si(0);
        let len_b = self.si(1);
        let sty = self.string_ty;

        // Evaluate operands into scratch ref locals
        self.emit_operand(f, &args[0]);
        f.instruction(&Instruction::LocalSet(s0));
        self.emit_operand(f, &args[1]);
        f.instruction(&Instruction::LocalSet(s1));

        // Get lengths
        f.instruction(&Instruction::LocalGet(s0));
        f.instruction(&Instruction::ArrayLen);
        f.instruction(&Instruction::LocalSet(len_a));
        f.instruction(&Instruction::LocalGet(s1));
        f.instruction(&Instruction::ArrayLen);
        f.instruction(&Instruction::LocalSet(len_b));

        // Create result array of len_a + len_b, store in s_result
        f.instruction(&Instruction::I32Const(0)); // fill value
        f.instruction(&Instruction::LocalGet(len_a));
        f.instruction(&Instruction::LocalGet(len_b));
        f.instruction(&Instruction::I32Add);
        f.instruction(&Instruction::ArrayNew(sty));
        f.instruction(&Instruction::LocalSet(s_result));

        // array.copy result[0..len_a] = a[0..len_a]
        f.instruction(&Instruction::LocalGet(s_result));
        f.instruction(&Instruction::I32Const(0)); // dst_off
        f.instruction(&Instruction::LocalGet(s0)); // src
        f.instruction(&Instruction::I32Const(0)); // src_off
        f.instruction(&Instruction::LocalGet(len_a)); // len
        f.instruction(&Instruction::ArrayCopy {
            array_type_index_dst: sty,
            array_type_index_src: sty,
        });

        // array.copy result[len_a..] = b[0..len_b]
        f.instruction(&Instruction::LocalGet(s_result));
        f.instruction(&Instruction::LocalGet(len_a)); // dst_off
        f.instruction(&Instruction::LocalGet(s1)); // src
        f.instruction(&Instruction::I32Const(0)); // src_off
        f.instruction(&Instruction::LocalGet(len_b)); // len
        f.instruction(&Instruction::ArrayCopy {
            array_type_index_dst: sty,
            array_type_index_src: sty,
        });

        // Push result
        f.instruction(&Instruction::LocalGet(s_result));
    }

    /// clone(s) → new GC string copy
    fn emit_string_clone_gc(&mut self, f: &mut Function, arg: &Operand) {
        let s0 = self.si(4);
        let len = self.si(0);
        let sty = self.string_ty;

        self.emit_operand(f, arg);
        f.instruction(&Instruction::LocalTee(s0));
        f.instruction(&Instruction::ArrayLen);
        f.instruction(&Instruction::LocalSet(len));

        // Create new array of same length
        f.instruction(&Instruction::I32Const(0)); // fill
        f.instruction(&Instruction::LocalGet(len));
        f.instruction(&Instruction::ArrayNew(sty));

        // array.copy dst dst_off src src_off len
        // But we need to keep the result ref. Use local.tee.
        let result = self.si(5);
        f.instruction(&Instruction::LocalTee(result));
        f.instruction(&Instruction::I32Const(0)); // dst_off
        f.instruction(&Instruction::LocalGet(s0)); // src
        f.instruction(&Instruction::I32Const(0)); // src_off
        f.instruction(&Instruction::LocalGet(len));
        f.instruction(&Instruction::ArrayCopy {
            array_type_index_dst: sty,
            array_type_index_src: sty,
        });
        f.instruction(&Instruction::LocalGet(result));
    }

    /// substring(s, start, end) → new GC string
    fn emit_substring_gc(&mut self, f: &mut Function, s: &Operand, start: &Operand, end: &Operand) {
        let s0 = self.si(4);
        let result = self.si(5);
        let start_idx = self.si(0);
        let sub_len = self.si(1);
        let sty = self.string_ty;

        self.emit_operand(f, s);
        f.instruction(&Instruction::LocalSet(s0));
        self.emit_operand(f, start);
        f.instruction(&Instruction::LocalSet(start_idx));
        self.emit_operand(f, end);
        f.instruction(&Instruction::LocalGet(start_idx));
        f.instruction(&Instruction::I32Sub);
        f.instruction(&Instruction::LocalSet(sub_len)); // sub_len = end - start

        // Create new array
        f.instruction(&Instruction::I32Const(0));
        f.instruction(&Instruction::LocalGet(sub_len));
        f.instruction(&Instruction::ArrayNew(sty));
        f.instruction(&Instruction::LocalTee(result));

        // array.copy result 0 s0 start sub_len
        f.instruction(&Instruction::I32Const(0));
        f.instruction(&Instruction::LocalGet(s0));
        f.instruction(&Instruction::LocalGet(start_idx));
        f.instruction(&Instruction::LocalGet(sub_len));
        f.instruction(&Instruction::ArrayCopy {
            array_type_index_dst: sty,
            array_type_index_src: sty,
        });
        f.instruction(&Instruction::LocalGet(result));
    }

    /// to_uppercase/to_lowercase: clone + byte-by-byte transform
    fn emit_case_transform_gc(&mut self, f: &mut Function, arg: &Operand, to_upper: bool) {
        let s0 = self.si(4);
        let result = self.si(5);
        let len = self.si(0);
        let i = self.si(1);
        let ch = self.si(2);
        let sty = self.string_ty;

        // Clone the string
        self.emit_operand(f, arg);
        f.instruction(&Instruction::LocalTee(s0));
        f.instruction(&Instruction::ArrayLen);
        f.instruction(&Instruction::LocalSet(len));

        f.instruction(&Instruction::I32Const(0));
        f.instruction(&Instruction::LocalGet(len));
        f.instruction(&Instruction::ArrayNew(sty));
        f.instruction(&Instruction::LocalTee(result));
        f.instruction(&Instruction::I32Const(0));
        f.instruction(&Instruction::LocalGet(s0));
        f.instruction(&Instruction::I32Const(0));
        f.instruction(&Instruction::LocalGet(len));
        f.instruction(&Instruction::ArrayCopy {
            array_type_index_dst: sty,
            array_type_index_src: sty,
        });

        // Transform each byte in-place
        f.instruction(&Instruction::I32Const(0));
        f.instruction(&Instruction::LocalSet(i));
        f.instruction(&Instruction::Block(wasm_encoder::BlockType::Empty));
        f.instruction(&Instruction::Loop(wasm_encoder::BlockType::Empty));
        f.instruction(&Instruction::LocalGet(i));
        f.instruction(&Instruction::LocalGet(len));
        f.instruction(&Instruction::I32GeU);
        f.instruction(&Instruction::BrIf(1));

        // ch = result[i]
        f.instruction(&Instruction::LocalGet(result));
        f.instruction(&Instruction::LocalGet(i));
        f.instruction(&Instruction::ArrayGetU(sty));
        f.instruction(&Instruction::LocalSet(ch));

        if to_upper {
            // if ch >= 'a' && ch <= 'z' then ch - 32
            f.instruction(&Instruction::LocalGet(ch));
            f.instruction(&Instruction::I32Const(97)); // 'a'
            f.instruction(&Instruction::I32GeU);
            f.instruction(&Instruction::LocalGet(ch));
            f.instruction(&Instruction::I32Const(122)); // 'z'
            f.instruction(&Instruction::I32LeU);
            f.instruction(&Instruction::I32And);
            f.instruction(&Instruction::If(wasm_encoder::BlockType::Empty));
            f.instruction(&Instruction::LocalGet(result));
            f.instruction(&Instruction::LocalGet(i));
            f.instruction(&Instruction::LocalGet(ch));
            f.instruction(&Instruction::I32Const(32));
            f.instruction(&Instruction::I32Sub);
            f.instruction(&Instruction::ArraySet(sty));
            f.instruction(&Instruction::End);
        } else {
            // if ch >= 'A' && ch <= 'Z' then ch + 32
            f.instruction(&Instruction::LocalGet(ch));
            f.instruction(&Instruction::I32Const(65)); // 'A'
            f.instruction(&Instruction::I32GeU);
            f.instruction(&Instruction::LocalGet(ch));
            f.instruction(&Instruction::I32Const(90)); // 'Z'
            f.instruction(&Instruction::I32LeU);
            f.instruction(&Instruction::I32And);
            f.instruction(&Instruction::If(wasm_encoder::BlockType::Empty));
            f.instruction(&Instruction::LocalGet(result));
            f.instruction(&Instruction::LocalGet(i));
            f.instruction(&Instruction::LocalGet(ch));
            f.instruction(&Instruction::I32Const(32));
            f.instruction(&Instruction::I32Add);
            f.instruction(&Instruction::ArraySet(sty));
            f.instruction(&Instruction::End);
        }

        f.instruction(&Instruction::LocalGet(i));
        f.instruction(&Instruction::I32Const(1));
        f.instruction(&Instruction::I32Add);
        f.instruction(&Instruction::LocalSet(i));
        f.instruction(&Instruction::Br(0));
        f.instruction(&Instruction::End);
        f.instruction(&Instruction::End);

        f.instruction(&Instruction::LocalGet(result));
    }

    /// trim(s) → new string with leading/trailing whitespace removed
    fn emit_trim_gc(&mut self, f: &mut Function, arg: &Operand) {
        let s0 = self.si(4);
        let len = self.si(0);
        let start = self.si(1);
        let end_idx = self.si(2);
        let sty = self.string_ty;

        self.emit_operand(f, arg);
        f.instruction(&Instruction::LocalTee(s0));
        f.instruction(&Instruction::ArrayLen);
        f.instruction(&Instruction::LocalTee(len));
        f.instruction(&Instruction::LocalSet(end_idx)); // end = len

        // Find start: skip leading spaces/tabs/newlines
        f.instruction(&Instruction::I32Const(0));
        f.instruction(&Instruction::LocalSet(start));
        f.instruction(&Instruction::Block(wasm_encoder::BlockType::Empty));
        f.instruction(&Instruction::Loop(wasm_encoder::BlockType::Empty));
        f.instruction(&Instruction::LocalGet(start));
        f.instruction(&Instruction::LocalGet(len));
        f.instruction(&Instruction::I32GeU);
        f.instruction(&Instruction::BrIf(1));
        f.instruction(&Instruction::LocalGet(s0));
        f.instruction(&Instruction::LocalGet(start));
        f.instruction(&Instruction::ArrayGetU(sty));
        f.instruction(&Instruction::I32Const(32)); // space
        f.instruction(&Instruction::I32LeU); // byte <= ' ' (covers space, tab, newline, etc.)
        f.instruction(&Instruction::I32Eqz);
        f.instruction(&Instruction::BrIf(1)); // break if not whitespace
        f.instruction(&Instruction::LocalGet(start));
        f.instruction(&Instruction::I32Const(1));
        f.instruction(&Instruction::I32Add);
        f.instruction(&Instruction::LocalSet(start));
        f.instruction(&Instruction::Br(0));
        f.instruction(&Instruction::End);
        f.instruction(&Instruction::End);

        // Find end: skip trailing whitespace
        f.instruction(&Instruction::Block(wasm_encoder::BlockType::Empty));
        f.instruction(&Instruction::Loop(wasm_encoder::BlockType::Empty));
        f.instruction(&Instruction::LocalGet(end_idx));
        f.instruction(&Instruction::LocalGet(start));
        f.instruction(&Instruction::I32LeU);
        f.instruction(&Instruction::BrIf(1));
        f.instruction(&Instruction::LocalGet(s0));
        f.instruction(&Instruction::LocalGet(end_idx));
        f.instruction(&Instruction::I32Const(1));
        f.instruction(&Instruction::I32Sub);
        f.instruction(&Instruction::ArrayGetU(sty));
        f.instruction(&Instruction::I32Const(32));
        f.instruction(&Instruction::I32LeU);
        f.instruction(&Instruction::I32Eqz);
        f.instruction(&Instruction::BrIf(1));
        f.instruction(&Instruction::LocalGet(end_idx));
        f.instruction(&Instruction::I32Const(1));
        f.instruction(&Instruction::I32Sub);
        f.instruction(&Instruction::LocalSet(end_idx));
        f.instruction(&Instruction::Br(0));
        f.instruction(&Instruction::End);
        f.instruction(&Instruction::End);

        // Create substring [start..end]
        self.emit_substring_gc_raw(f, s0, start, end_idx);
    }

    /// Helper: substring from scratch locals (src_ref_local, start_local, end_local)
    fn emit_substring_gc_raw(
        &self,
        f: &mut Function,
        src_local: u32,
        start_local: u32,
        end_local: u32,
    ) {
        let result = self.si(5);
        let sub_len = self.si(3);
        let sty = self.string_ty;

        f.instruction(&Instruction::LocalGet(end_local));
        f.instruction(&Instruction::LocalGet(start_local));
        f.instruction(&Instruction::I32Sub);
        f.instruction(&Instruction::LocalTee(sub_len));

        // Handle empty/negative length
        f.instruction(&Instruction::I32Const(0));
        f.instruction(&Instruction::I32LeS);
        f.instruction(&Instruction::If(wasm_encoder::BlockType::Result(
            ref_nullable(sty),
        )));
        f.instruction(&Instruction::I32Const(0));
        f.instruction(&Instruction::I32Const(0));
        f.instruction(&Instruction::ArrayNew(sty));
        f.instruction(&Instruction::Else);

        f.instruction(&Instruction::I32Const(0)); // fill
        f.instruction(&Instruction::LocalGet(sub_len));
        f.instruction(&Instruction::ArrayNew(sty));
        f.instruction(&Instruction::LocalTee(result));
        f.instruction(&Instruction::I32Const(0));
        f.instruction(&Instruction::LocalGet(src_local));
        f.instruction(&Instruction::LocalGet(start_local));
        f.instruction(&Instruction::LocalGet(sub_len));
        f.instruction(&Instruction::ArrayCopy {
            array_type_index_dst: sty,
            array_type_index_src: sty,
        });
        f.instruction(&Instruction::LocalGet(result));

        f.instruction(&Instruction::End);
    }

    /// starts_with(s, prefix) → i32 (0 or 1)
    fn emit_starts_with_gc(&mut self, f: &mut Function, s: &Operand, prefix: &Operand) {
        let s0 = self.si(4);
        let s1 = self.si(5);
        let s_len = self.si(0);
        let p_len = self.si(1);
        let i = self.si(2);
        let sty = self.string_ty;

        self.emit_operand(f, s);
        f.instruction(&Instruction::LocalTee(s0));
        f.instruction(&Instruction::ArrayLen);
        f.instruction(&Instruction::LocalSet(s_len));
        self.emit_operand(f, prefix);
        f.instruction(&Instruction::LocalTee(s1));
        f.instruction(&Instruction::ArrayLen);
        f.instruction(&Instruction::LocalSet(p_len));

        // If s_len < p_len, return false
        f.instruction(&Instruction::LocalGet(s_len));
        f.instruction(&Instruction::LocalGet(p_len));
        f.instruction(&Instruction::I32LtU);
        f.instruction(&Instruction::If(wasm_encoder::BlockType::Result(
            ValType::I32,
        )));
        f.instruction(&Instruction::I32Const(0));
        f.instruction(&Instruction::Else);

        // Compare bytes using scratch local for result
        f.instruction(&Instruction::I32Const(0));
        f.instruction(&Instruction::LocalSet(i));
        let result_scr = self.si(3);
        f.instruction(&Instruction::I32Const(1)); // assume true
        f.instruction(&Instruction::LocalSet(result_scr));
        f.instruction(&Instruction::Block(wasm_encoder::BlockType::Empty));
        f.instruction(&Instruction::Loop(wasm_encoder::BlockType::Empty));
        f.instruction(&Instruction::LocalGet(i));
        f.instruction(&Instruction::LocalGet(p_len));
        f.instruction(&Instruction::I32GeU);
        f.instruction(&Instruction::BrIf(1)); // done, true
        // Compare s[i] vs prefix[i]
        f.instruction(&Instruction::LocalGet(s0));
        f.instruction(&Instruction::LocalGet(i));
        f.instruction(&Instruction::ArrayGetU(sty));
        f.instruction(&Instruction::LocalGet(s1));
        f.instruction(&Instruction::LocalGet(i));
        f.instruction(&Instruction::ArrayGetU(sty));
        f.instruction(&Instruction::I32Ne);
        f.instruction(&Instruction::If(wasm_encoder::BlockType::Empty));
        f.instruction(&Instruction::I32Const(0));
        f.instruction(&Instruction::LocalSet(result_scr)); // mark false
        f.instruction(&Instruction::Br(2)); // break out
        f.instruction(&Instruction::End);
        f.instruction(&Instruction::LocalGet(i));
        f.instruction(&Instruction::I32Const(1));
        f.instruction(&Instruction::I32Add);
        f.instruction(&Instruction::LocalSet(i));
        f.instruction(&Instruction::Br(0));
        f.instruction(&Instruction::End);
        f.instruction(&Instruction::End);

        f.instruction(&Instruction::LocalGet(result_scr));

        f.instruction(&Instruction::End); // end if
    }

    /// ends_with(s, suffix) → i32
    fn emit_ends_with_gc(&mut self, f: &mut Function, s: &Operand, suffix: &Operand) {
        let s0 = self.si(4);
        let s1 = self.si(5);
        let s_len = self.si(0);
        let sf_len = self.si(1);
        let i = self.si(2);
        let offset = self.si(3);
        let sty = self.string_ty;

        self.emit_operand(f, s);
        f.instruction(&Instruction::LocalTee(s0));
        f.instruction(&Instruction::ArrayLen);
        f.instruction(&Instruction::LocalSet(s_len));
        self.emit_operand(f, suffix);
        f.instruction(&Instruction::LocalTee(s1));
        f.instruction(&Instruction::ArrayLen);
        f.instruction(&Instruction::LocalSet(sf_len));

        f.instruction(&Instruction::LocalGet(s_len));
        f.instruction(&Instruction::LocalGet(sf_len));
        f.instruction(&Instruction::I32LtU);
        f.instruction(&Instruction::If(wasm_encoder::BlockType::Result(
            ValType::I32,
        )));
        f.instruction(&Instruction::I32Const(0));
        f.instruction(&Instruction::Else);

        // offset = s_len - sf_len
        f.instruction(&Instruction::LocalGet(s_len));
        f.instruction(&Instruction::LocalGet(sf_len));
        f.instruction(&Instruction::I32Sub);
        f.instruction(&Instruction::LocalSet(offset));

        f.instruction(&Instruction::I32Const(0));
        f.instruction(&Instruction::LocalSet(i));
        // Use s_len (si(0)) as result scratch — s_len is no longer needed after offset calc
        f.instruction(&Instruction::I32Const(1));
        f.instruction(&Instruction::LocalSet(s_len));
        f.instruction(&Instruction::Block(wasm_encoder::BlockType::Empty));
        f.instruction(&Instruction::Loop(wasm_encoder::BlockType::Empty));
        f.instruction(&Instruction::LocalGet(i));
        f.instruction(&Instruction::LocalGet(sf_len));
        f.instruction(&Instruction::I32GeU);
        f.instruction(&Instruction::BrIf(1));
        f.instruction(&Instruction::LocalGet(s0));
        f.instruction(&Instruction::LocalGet(offset));
        f.instruction(&Instruction::LocalGet(i));
        f.instruction(&Instruction::I32Add);
        f.instruction(&Instruction::ArrayGetU(sty));
        f.instruction(&Instruction::LocalGet(s1));
        f.instruction(&Instruction::LocalGet(i));
        f.instruction(&Instruction::ArrayGetU(sty));
        f.instruction(&Instruction::I32Ne);
        f.instruction(&Instruction::If(wasm_encoder::BlockType::Empty));
        f.instruction(&Instruction::I32Const(0));
        f.instruction(&Instruction::LocalSet(s_len)); // mark false
        f.instruction(&Instruction::Br(2));
        f.instruction(&Instruction::End);
        f.instruction(&Instruction::LocalGet(i));
        f.instruction(&Instruction::I32Const(1));
        f.instruction(&Instruction::I32Add);
        f.instruction(&Instruction::LocalSet(i));
        f.instruction(&Instruction::Br(0));
        f.instruction(&Instruction::End);
        f.instruction(&Instruction::End);

        f.instruction(&Instruction::LocalGet(s_len)); // push result

        f.instruction(&Instruction::End);
    }

    /// String equality: [ref $string, ref $string] → i32
    /// Compares two GC string arrays byte-by-byte
    fn emit_string_eq_gc(&mut self, f: &mut Function) {
        let s0 = self.si(4);
        let s1 = self.si(5);
        let len_a = self.si(0);
        let i = self.si(1);
        let result_scr = self.si(9); // use extra scratch to avoid conflicts
        let sty = self.string_ty;

        // Store both refs
        f.instruction(&Instruction::LocalSet(s1));
        f.instruction(&Instruction::LocalSet(s0));

        // Compare lengths first
        f.instruction(&Instruction::LocalGet(s0));
        f.instruction(&Instruction::ArrayLen);
        f.instruction(&Instruction::LocalTee(len_a));
        f.instruction(&Instruction::LocalGet(s1));
        f.instruction(&Instruction::ArrayLen);
        f.instruction(&Instruction::I32Ne);
        f.instruction(&Instruction::If(wasm_encoder::BlockType::Result(
            ValType::I32,
        )));
        f.instruction(&Instruction::I32Const(0)); // different lengths → not equal
        f.instruction(&Instruction::Else);

        // Same length — compare bytes using a scratch local for result
        f.instruction(&Instruction::I32Const(0));
        f.instruction(&Instruction::LocalSet(i));
        f.instruction(&Instruction::I32Const(1)); // assume equal
        f.instruction(&Instruction::LocalSet(result_scr));

        f.instruction(&Instruction::Block(wasm_encoder::BlockType::Empty));
        f.instruction(&Instruction::Loop(wasm_encoder::BlockType::Empty));

        // if i >= len_a: break (equal)
        f.instruction(&Instruction::LocalGet(i));
        f.instruction(&Instruction::LocalGet(len_a));
        f.instruction(&Instruction::I32GeU);
        f.instruction(&Instruction::BrIf(1));

        // if s0[i] != s1[i]: not equal
        f.instruction(&Instruction::LocalGet(s0));
        f.instruction(&Instruction::LocalGet(i));
        f.instruction(&Instruction::ArrayGetU(sty));
        f.instruction(&Instruction::LocalGet(s1));
        f.instruction(&Instruction::LocalGet(i));
        f.instruction(&Instruction::ArrayGetU(sty));
        f.instruction(&Instruction::I32Ne);
        f.instruction(&Instruction::If(wasm_encoder::BlockType::Empty));
        f.instruction(&Instruction::I32Const(0));
        f.instruction(&Instruction::LocalSet(result_scr)); // mark not equal
        f.instruction(&Instruction::Br(2)); // break out of block
        f.instruction(&Instruction::End);

        // i++
        f.instruction(&Instruction::LocalGet(i));
        f.instruction(&Instruction::I32Const(1));
        f.instruction(&Instruction::I32Add);
        f.instruction(&Instruction::LocalSet(i));
        f.instruction(&Instruction::Br(0)); // continue loop
        f.instruction(&Instruction::End); // end loop
        f.instruction(&Instruction::End); // end block

        // Push result from scratch local
        f.instruction(&Instruction::LocalGet(result_scr));

        f.instruction(&Instruction::End); // end if (len mismatch)
    }

    /// contains(s, sub) → i32
    fn emit_contains_gc(&mut self, f: &mut Function, s: &Operand, sub: &Operand) {
        let s0 = self.si(4);
        let s1 = self.si(5);
        let s_len = self.si(0);
        let sub_len = self.si(1);
        let i = self.si(2);
        let j = self.si(3);
        let result = self.si(9); // extra i32 scratch for result
        let sty = self.string_ty;

        self.emit_operand(f, s);
        f.instruction(&Instruction::LocalTee(s0));
        f.instruction(&Instruction::ArrayLen);
        f.instruction(&Instruction::LocalSet(s_len));
        self.emit_operand(f, sub);
        f.instruction(&Instruction::LocalTee(s1));
        f.instruction(&Instruction::ArrayLen);
        f.instruction(&Instruction::LocalSet(sub_len));

        // Empty substring always matches
        f.instruction(&Instruction::LocalGet(sub_len));
        f.instruction(&Instruction::I32Eqz);
        f.instruction(&Instruction::If(wasm_encoder::BlockType::Result(
            ValType::I32,
        )));
        f.instruction(&Instruction::I32Const(1));
        f.instruction(&Instruction::Else);

        f.instruction(&Instruction::I32Const(0));
        f.instruction(&Instruction::LocalSet(result)); // assume not found
        f.instruction(&Instruction::I32Const(0));
        f.instruction(&Instruction::LocalSet(i));

        // outer loop: for i in 0..=(s_len - sub_len)
        f.instruction(&Instruction::Block(wasm_encoder::BlockType::Empty));
        f.instruction(&Instruction::Loop(wasm_encoder::BlockType::Empty));
        f.instruction(&Instruction::LocalGet(i));
        f.instruction(&Instruction::LocalGet(s_len));
        f.instruction(&Instruction::LocalGet(sub_len));
        f.instruction(&Instruction::I32Sub);
        f.instruction(&Instruction::I32Const(1));
        f.instruction(&Instruction::I32Add);
        f.instruction(&Instruction::I32GeU);
        f.instruction(&Instruction::BrIf(1));

        // inner check: compare sub bytes using j as match flag
        f.instruction(&Instruction::I32Const(0));
        f.instruction(&Instruction::LocalSet(j));
        f.instruction(&Instruction::Block(wasm_encoder::BlockType::Empty));
        f.instruction(&Instruction::Loop(wasm_encoder::BlockType::Empty));
        f.instruction(&Instruction::LocalGet(j));
        f.instruction(&Instruction::LocalGet(sub_len));
        f.instruction(&Instruction::I32GeU);
        f.instruction(&Instruction::BrIf(1)); // all bytes matched
        f.instruction(&Instruction::LocalGet(s0));
        f.instruction(&Instruction::LocalGet(i));
        f.instruction(&Instruction::LocalGet(j));
        f.instruction(&Instruction::I32Add);
        f.instruction(&Instruction::ArrayGetU(sty));
        f.instruction(&Instruction::LocalGet(s1));
        f.instruction(&Instruction::LocalGet(j));
        f.instruction(&Instruction::ArrayGetU(sty));
        f.instruction(&Instruction::I32Ne);
        f.instruction(&Instruction::BrIf(1)); // mismatch — break inner
        f.instruction(&Instruction::LocalGet(j));
        f.instruction(&Instruction::I32Const(1));
        f.instruction(&Instruction::I32Add);
        f.instruction(&Instruction::LocalSet(j));
        f.instruction(&Instruction::Br(0));
        f.instruction(&Instruction::End); // end inner loop
        f.instruction(&Instruction::End); // end inner block

        // After inner: if j == sub_len, all bytes matched → found
        f.instruction(&Instruction::LocalGet(j));
        f.instruction(&Instruction::LocalGet(sub_len));
        f.instruction(&Instruction::I32Eq);
        f.instruction(&Instruction::If(wasm_encoder::BlockType::Empty));
        f.instruction(&Instruction::I32Const(1));
        f.instruction(&Instruction::LocalSet(result));
        f.instruction(&Instruction::Br(1)); // break outer
        f.instruction(&Instruction::End);

        f.instruction(&Instruction::LocalGet(i));
        f.instruction(&Instruction::I32Const(1));
        f.instruction(&Instruction::I32Add);
        f.instruction(&Instruction::LocalSet(i));
        f.instruction(&Instruction::Br(0));
        f.instruction(&Instruction::End); // end outer loop
        f.instruction(&Instruction::End); // end outer block

        f.instruction(&Instruction::LocalGet(result));

        f.instruction(&Instruction::End); // end if (empty sub)
    }

    /// join(parts_vec, separator) → String
    fn emit_join_gc(&mut self, f: &mut Function, _args: &[Operand]) {
        // Stub: return empty string for now (join requires Vec<String> access)
        let sty = self.string_ty;
        f.instruction(&Instruction::I32Const(0));
        f.instruction(&Instruction::I32Const(0));
        f.instruction(&Instruction::ArrayNew(sty));
    }

    /// Emit inline filter HOF: filter(vec, predicate_fn) -> new_vec
    /// Uses scratch memory for loop state.
    fn emit_filter_hof_inline(&mut self, f: &mut Function, canonical: &str, args: &[Operand]) {
        let ma = MemArg {
            offset: 0,
            align: 2,
            memory_index: 0,
        };
        if args.len() < 2 {
            f.instruction(&Instruction::I32Const(0));
            return;
        }

        let (elem_size, is_f64, is_i64) = match canonical {
            "filter_f64" => (8i32, true, false),
            "filter_i64" => (8, false, true),
            _ => (4, false, false),
        };

        let pred_type = if is_f64 {
            self.indirect_types
                .get(&(vec![ValType::F64], vec![ValType::I32]))
                .copied()
                .unwrap_or(0)
        } else if is_i64 {
            self.indirect_types
                .get(&(vec![ValType::I64], vec![ValType::I32]))
                .copied()
                .unwrap_or(0)
        } else {
            self.indirect_types
                .get(&(vec![ValType::I32], vec![ValType::I32]))
                .copied()
                .unwrap_or(0)
        };

        // SCR_A_PTR = vec_ptr, SCR_B_PTR = fn_idx
        f.instruction(&Instruction::I32Const(SCR_A_PTR as i32));
        self.emit_operand(f, &args[0]);
        f.instruction(&Instruction::I32Store(ma));
        f.instruction(&Instruction::I32Const(SCR_B_PTR as i32));
        self.emit_operand(f, &args[1]);
        f.instruction(&Instruction::I32Store(ma));

        // SCR_A_LEN = len(vec)
        f.instruction(&Instruction::I32Const(SCR_A_LEN as i32));
        f.instruction(&Instruction::I32Const(SCR_A_PTR as i32));
        f.instruction(&Instruction::I32Load(ma));
        f.instruction(&Instruction::I32Const(4));
        f.instruction(&Instruction::I32Add);
        f.instruction(&Instruction::I32Load(ma));
        f.instruction(&Instruction::I32Store(ma));

        // Create new_vec (same capacity as original)
        self.emit_vec_new_inline(f, elem_size);
        // SCR_DST_PTR = new_vec
        f.instruction(&Instruction::I32Const(SCR_DST_PTR as i32));
        // Swap: the vec ptr is on stack
        f.instruction(&Instruction::I32Store(ma));

        // SCR_I = 0 (loop counter), SCR_J = 0 (new_len)
        f.instruction(&Instruction::I32Const(SCR_I as i32));
        f.instruction(&Instruction::I32Const(0));
        f.instruction(&Instruction::I32Store(ma));
        f.instruction(&Instruction::I32Const(SCR_J as i32));
        f.instruction(&Instruction::I32Const(0));
        f.instruction(&Instruction::I32Store(ma));

        // block { loop {
        f.instruction(&Instruction::Block(wasm_encoder::BlockType::Empty));
        f.instruction(&Instruction::Loop(wasm_encoder::BlockType::Empty));
        // if i >= n: break
        f.instruction(&Instruction::I32Const(SCR_I as i32));
        f.instruction(&Instruction::I32Load(ma));
        f.instruction(&Instruction::I32Const(SCR_A_LEN as i32));
        f.instruction(&Instruction::I32Load(ma));
        f.instruction(&Instruction::I32GeU);
        f.instruction(&Instruction::BrIf(1));

        // Store element to SCR_VAL64
        f.instruction(&Instruction::I32Const(SCR_VAL64 as i32));
        f.instruction(&Instruction::I32Const(SCR_A_PTR as i32));
        f.instruction(&Instruction::I32Load(ma));
        f.instruction(&Instruction::I32Load(ma)); // data_ptr
        f.instruction(&Instruction::I32Const(SCR_I as i32));
        f.instruction(&Instruction::I32Load(ma));
        f.instruction(&Instruction::I32Const(elem_size));
        f.instruction(&Instruction::I32Mul);
        f.instruction(&Instruction::I32Add);
        if is_f64 {
            f.instruction(&Instruction::F64Load(ma));
            f.instruction(&Instruction::F64Store(ma));
        } else if is_i64 {
            f.instruction(&Instruction::I64Load(ma));
            f.instruction(&Instruction::I64Store(ma));
        } else {
            f.instruction(&Instruction::I32Load(ma));
            f.instruction(&Instruction::I32Store(ma));
        }

        // Call predicate: pred(value) -> i32
        f.instruction(&Instruction::I32Const(SCR_VAL64 as i32));
        if is_f64 {
            f.instruction(&Instruction::F64Load(ma));
        } else if is_i64 {
            f.instruction(&Instruction::I64Load(ma));
        } else {
            f.instruction(&Instruction::I32Load(ma));
        }
        f.instruction(&Instruction::I32Const(SCR_B_PTR as i32));
        f.instruction(&Instruction::I32Load(ma)); // fn_idx
        f.instruction(&Instruction::CallIndirect {
            type_index: pred_type,
            table_index: 0,
        });

        // if predicate returned true: push value to new_vec
        f.instruction(&Instruction::If(wasm_encoder::BlockType::Empty));
        // new_data + new_len * elem_size = value
        f.instruction(&Instruction::I32Const(SCR_DST_PTR as i32));
        f.instruction(&Instruction::I32Load(ma));
        f.instruction(&Instruction::I32Load(ma)); // new_data_ptr
        f.instruction(&Instruction::I32Const(SCR_J as i32));
        f.instruction(&Instruction::I32Load(ma));
        f.instruction(&Instruction::I32Const(elem_size));
        f.instruction(&Instruction::I32Mul);
        f.instruction(&Instruction::I32Add);
        f.instruction(&Instruction::I32Const(SCR_VAL64 as i32));
        if is_f64 {
            f.instruction(&Instruction::F64Load(ma));
            f.instruction(&Instruction::F64Store(ma));
        } else if is_i64 {
            f.instruction(&Instruction::I64Load(ma));
            f.instruction(&Instruction::I64Store(ma));
        } else {
            f.instruction(&Instruction::I32Load(ma));
            f.instruction(&Instruction::I32Store(ma));
        }
        // new_len++
        f.instruction(&Instruction::I32Const(SCR_J as i32));
        f.instruction(&Instruction::I32Const(SCR_J as i32));
        f.instruction(&Instruction::I32Load(ma));
        f.instruction(&Instruction::I32Const(1));
        f.instruction(&Instruction::I32Add);
        f.instruction(&Instruction::I32Store(ma));
        f.instruction(&Instruction::End); // end if

        // i++
        f.instruction(&Instruction::I32Const(SCR_I as i32));
        f.instruction(&Instruction::I32Const(SCR_I as i32));
        f.instruction(&Instruction::I32Load(ma));
        f.instruction(&Instruction::I32Const(1));
        f.instruction(&Instruction::I32Add);
        f.instruction(&Instruction::I32Store(ma));
        f.instruction(&Instruction::Br(0));
        f.instruction(&Instruction::End); // end loop
        f.instruction(&Instruction::End); // end block

        // Set new_vec len = new_len
        f.instruction(&Instruction::I32Const(SCR_DST_PTR as i32));
        f.instruction(&Instruction::I32Load(ma));
        f.instruction(&Instruction::I32Const(4));
        f.instruction(&Instruction::I32Add);
        f.instruction(&Instruction::I32Const(SCR_J as i32));
        f.instruction(&Instruction::I32Load(ma));
        f.instruction(&Instruction::I32Store(ma));

        // Result: new_vec ptr
        f.instruction(&Instruction::I32Const(SCR_DST_PTR as i32));
        f.instruction(&Instruction::I32Load(ma));
    }

    /// Emit inline map HOF: map(vec, mapper_fn) -> new_vec
    fn emit_map_hof_inline(&mut self, f: &mut Function, canonical: &str, args: &[Operand]) {
        let ma = MemArg {
            offset: 0,
            align: 2,
            memory_index: 0,
        };
        if args.len() < 2 {
            f.instruction(&Instruction::I32Const(0));
            return;
        }

        let (in_size, out_size, in_f64, in_i64, out_f64, out_i64) = match canonical {
            "map_f64_f64" => (8i32, 8i32, true, false, true, false),
            "map_i64_i64" => (8, 8, false, true, false, true),
            _ => (4, 4, false, false, false, false), // i32->i32 or i32->String
        };

        let map_type = if in_f64 {
            self.indirect_types
                .get(&(vec![ValType::F64], vec![ValType::F64]))
                .copied()
                .unwrap_or(0)
        } else if in_i64 {
            self.indirect_types
                .get(&(vec![ValType::I64], vec![ValType::I64]))
                .copied()
                .unwrap_or(0)
        } else {
            self.indirect_types
                .get(&(vec![ValType::I32], vec![ValType::I32]))
                .copied()
                .unwrap_or(0)
        };

        // SCR_A_PTR = vec_ptr, SCR_B_PTR = fn_idx
        f.instruction(&Instruction::I32Const(SCR_A_PTR as i32));
        self.emit_operand(f, &args[0]);
        f.instruction(&Instruction::I32Store(ma));
        f.instruction(&Instruction::I32Const(SCR_B_PTR as i32));
        self.emit_operand(f, &args[1]);
        f.instruction(&Instruction::I32Store(ma));

        // SCR_A_LEN = len(vec)
        f.instruction(&Instruction::I32Const(SCR_A_LEN as i32));
        f.instruction(&Instruction::I32Const(SCR_A_PTR as i32));
        f.instruction(&Instruction::I32Load(ma));
        f.instruction(&Instruction::I32Const(4));
        f.instruction(&Instruction::I32Add);
        f.instruction(&Instruction::I32Load(ma));
        f.instruction(&Instruction::I32Store(ma));

        // Create new_vec
        self.emit_vec_new_inline(f, out_size);
        f.instruction(&Instruction::I32Const(SCR_DST_PTR as i32));
        f.instruction(&Instruction::I32Store(ma));

        // SCR_I = 0
        f.instruction(&Instruction::I32Const(SCR_I as i32));
        f.instruction(&Instruction::I32Const(0));
        f.instruction(&Instruction::I32Store(ma));

        // block { loop {
        f.instruction(&Instruction::Block(wasm_encoder::BlockType::Empty));
        f.instruction(&Instruction::Loop(wasm_encoder::BlockType::Empty));
        f.instruction(&Instruction::I32Const(SCR_I as i32));
        f.instruction(&Instruction::I32Load(ma));
        f.instruction(&Instruction::I32Const(SCR_A_LEN as i32));
        f.instruction(&Instruction::I32Load(ma));
        f.instruction(&Instruction::I32GeU);
        f.instruction(&Instruction::BrIf(1));

        // Load element
        f.instruction(&Instruction::I32Const(SCR_A_PTR as i32));
        f.instruction(&Instruction::I32Load(ma));
        f.instruction(&Instruction::I32Load(ma));
        f.instruction(&Instruction::I32Const(SCR_I as i32));
        f.instruction(&Instruction::I32Load(ma));
        f.instruction(&Instruction::I32Const(in_size));
        f.instruction(&Instruction::I32Mul);
        f.instruction(&Instruction::I32Add);
        if in_f64 {
            f.instruction(&Instruction::F64Load(ma));
        } else if in_i64 {
            f.instruction(&Instruction::I64Load(ma));
        } else {
            f.instruction(&Instruction::I32Load(ma));
        }

        // Call mapper: fn(val) -> mapped_val
        f.instruction(&Instruction::I32Const(SCR_B_PTR as i32));
        f.instruction(&Instruction::I32Load(ma));
        f.instruction(&Instruction::CallIndirect {
            type_index: map_type,
            table_index: 0,
        });

        // Store result to new_data[i]
        // First save mapped value to SCR_VAL64
        f.instruction(&Instruction::I32Const(SCR_VAL64 as i32));
        if out_f64 {
            f.instruction(&Instruction::F64Store(ma));
        } else if out_i64 {
            f.instruction(&Instruction::I64Store(ma));
        } else {
            f.instruction(&Instruction::I32Store(ma));
        }

        f.instruction(&Instruction::I32Const(SCR_DST_PTR as i32));
        f.instruction(&Instruction::I32Load(ma));
        f.instruction(&Instruction::I32Load(ma)); // new_data_ptr
        f.instruction(&Instruction::I32Const(SCR_I as i32));
        f.instruction(&Instruction::I32Load(ma));
        f.instruction(&Instruction::I32Const(out_size));
        f.instruction(&Instruction::I32Mul);
        f.instruction(&Instruction::I32Add);
        f.instruction(&Instruction::I32Const(SCR_VAL64 as i32));
        if out_f64 {
            f.instruction(&Instruction::F64Load(ma));
            f.instruction(&Instruction::F64Store(ma));
        } else if out_i64 {
            f.instruction(&Instruction::I64Load(ma));
            f.instruction(&Instruction::I64Store(ma));
        } else {
            f.instruction(&Instruction::I32Load(ma));
            f.instruction(&Instruction::I32Store(ma));
        }

        // i++
        f.instruction(&Instruction::I32Const(SCR_I as i32));
        f.instruction(&Instruction::I32Const(SCR_I as i32));
        f.instruction(&Instruction::I32Load(ma));
        f.instruction(&Instruction::I32Const(1));
        f.instruction(&Instruction::I32Add);
        f.instruction(&Instruction::I32Store(ma));
        f.instruction(&Instruction::Br(0));
        f.instruction(&Instruction::End);
        f.instruction(&Instruction::End);

        // Set new_vec len = original len
        f.instruction(&Instruction::I32Const(SCR_DST_PTR as i32));
        f.instruction(&Instruction::I32Load(ma));
        f.instruction(&Instruction::I32Const(4));
        f.instruction(&Instruction::I32Add);
        f.instruction(&Instruction::I32Const(SCR_A_LEN as i32));
        f.instruction(&Instruction::I32Load(ma));
        f.instruction(&Instruction::I32Store(ma));

        // Result: new_vec ptr
        f.instruction(&Instruction::I32Const(SCR_DST_PTR as i32));
        f.instruction(&Instruction::I32Load(ma));
    }

    /// Emit inline fold HOF: fold(vec, init, folder_fn) -> accumulated
    fn emit_fold_hof_inline(&mut self, f: &mut Function, args: &[Operand]) {
        let ma = MemArg {
            offset: 0,
            align: 2,
            memory_index: 0,
        };
        if args.len() < 3 {
            f.instruction(&Instruction::I64Const(0));
            return;
        }

        let fold_type = self
            .indirect_types
            .get(&(vec![ValType::I64, ValType::I64], vec![ValType::I64]))
            .copied()
            .unwrap_or(0);

        // SCR_A_PTR = vec_ptr
        f.instruction(&Instruction::I32Const(SCR_A_PTR as i32));
        self.emit_operand(f, &args[0]);
        f.instruction(&Instruction::I32Store(ma));

        // SCR_VAL64 = init (i64, stored as 8 bytes)
        f.instruction(&Instruction::I32Const(SCR_VAL64 as i32));
        // Coerce init to i64 if it's a ConstI32
        self.emit_operand_coerced(f, &args[1], true, false);
        f.instruction(&Instruction::I64Store(ma));

        // SCR_B_PTR = fn_idx
        f.instruction(&Instruction::I32Const(SCR_B_PTR as i32));
        self.emit_operand(f, &args[2]);
        f.instruction(&Instruction::I32Store(ma));

        // SCR_A_LEN = len
        f.instruction(&Instruction::I32Const(SCR_A_LEN as i32));
        f.instruction(&Instruction::I32Const(SCR_A_PTR as i32));
        f.instruction(&Instruction::I32Load(ma));
        f.instruction(&Instruction::I32Const(4));
        f.instruction(&Instruction::I32Add);
        f.instruction(&Instruction::I32Load(ma));
        f.instruction(&Instruction::I32Store(ma));

        // SCR_I = 0
        f.instruction(&Instruction::I32Const(SCR_I as i32));
        f.instruction(&Instruction::I32Const(0));
        f.instruction(&Instruction::I32Store(ma));

        // block { loop {
        f.instruction(&Instruction::Block(wasm_encoder::BlockType::Empty));
        f.instruction(&Instruction::Loop(wasm_encoder::BlockType::Empty));
        f.instruction(&Instruction::I32Const(SCR_I as i32));
        f.instruction(&Instruction::I32Load(ma));
        f.instruction(&Instruction::I32Const(SCR_A_LEN as i32));
        f.instruction(&Instruction::I32Load(ma));
        f.instruction(&Instruction::I32GeU);
        f.instruction(&Instruction::BrIf(1));

        // acc = folder(acc, element[i])
        // Push store destination address first (for I64Store after call)
        f.instruction(&Instruction::I32Const(SCR_VAL64 as i32));
        // Push call args: acc, element
        f.instruction(&Instruction::I32Const(SCR_VAL64 as i32));
        f.instruction(&Instruction::I64Load(ma)); // acc
        f.instruction(&Instruction::I32Const(SCR_A_PTR as i32));
        f.instruction(&Instruction::I32Load(ma));
        f.instruction(&Instruction::I32Load(ma)); // data_ptr
        f.instruction(&Instruction::I32Const(SCR_I as i32));
        f.instruction(&Instruction::I32Load(ma));
        f.instruction(&Instruction::I32Const(8));
        f.instruction(&Instruction::I32Mul);
        f.instruction(&Instruction::I32Add);
        f.instruction(&Instruction::I64Load(ma)); // element
        f.instruction(&Instruction::I32Const(SCR_B_PTR as i32));
        f.instruction(&Instruction::I32Load(ma)); // fn_idx
        f.instruction(&Instruction::CallIndirect {
            type_index: fold_type,
            table_index: 0,
        });
        // Store result: stack is [addr, i64_result]
        f.instruction(&Instruction::I64Store(ma));

        // i++
        f.instruction(&Instruction::I32Const(SCR_I as i32));
        f.instruction(&Instruction::I32Const(SCR_I as i32));
        f.instruction(&Instruction::I32Load(ma));
        f.instruction(&Instruction::I32Const(1));
        f.instruction(&Instruction::I32Add);
        f.instruction(&Instruction::I32Store(ma));
        f.instruction(&Instruction::Br(0));
        f.instruction(&Instruction::End);
        f.instruction(&Instruction::End);

        // Result: acc
        f.instruction(&Instruction::I32Const(SCR_VAL64 as i32));
        f.instruction(&Instruction::I64Load(ma));
    }

    /// Extract a direct function index from a FnRef operand (for GC-native HOF).
    fn extract_fn_idx(&self, op: &Operand) -> Option<u32> {
        match op {
            Operand::FnRef(name) => self.fn_map.get(name.as_str()).copied(),
            Operand::Place(Place::Local(_id)) => {
                // Closure: check if the local was bound to a closure → resolve to synthetic fn
                None // For now, only handle FnRef
            }
            _ => None,
        }
    }

    /// Resolve the GC array type index and vec type index for a HOF canonical name.
    fn hof_gc_types(&self, canonical: &str) -> (u32, u32) {
        if canonical.contains("String") {
            (self.arr_string_ty, self.vec_string_ty)
        } else if canonical.contains("i64") {
            (self.arr_i64_ty, self.vec_i64_ty)
        } else if canonical.contains("f64") {
            (self.arr_f64_ty, self.vec_f64_ty)
        } else {
            (self.arr_i32_ty, self.vec_i32_ty)
        }
    }

    /// Determine the output GC array/vec types for map operations.
    fn hof_map_output_types(&self, canonical: &str) -> (u32, u32) {
        match canonical {
            "map_i32_String" | "map_i64_String" | "map_f64_String" | "map_String_String" => {
                (self.arr_string_ty, self.vec_string_ty)
            }
            "map_i64_i64" => (self.arr_i64_ty, self.vec_i64_ty),
            "map_f64_f64" => (self.arr_f64_ty, self.vec_f64_ty),
            _ => (self.arr_i32_ty, self.vec_i32_ty), // map_i32_i32
        }
    }

    /// GC-native filter HOF: filter(vec, predicate) -> new_vec
    /// Uses GC struct/array ops instead of linear memory.
    fn emit_filter_hof_gc(&mut self, f: &mut Function, canonical: &str, args: &[Operand]) {
        if args.len() < 2 {
            return;
        }

        let (arr_ty, vec_ty) = self.hof_gc_types(canonical);
        let pred_fn_idx = self.extract_fn_idx(&args[1]);

        // si(0) = len, si(1) = i, si(2) = j
        // si(10) = src_arr (anyref), si(11) = dst_arr (anyref)

        // Get source array and length
        self.emit_operand(f, &args[0]);
        f.instruction(&Instruction::StructGet {
            struct_type_index: vec_ty,
            field_index: 0,
        });
        f.instruction(&Instruction::LocalSet(self.si(10))); // src_arr → anyref

        self.emit_operand(f, &args[0]);
        f.instruction(&Instruction::StructGet {
            struct_type_index: vec_ty,
            field_index: 1,
        });
        f.instruction(&Instruction::LocalSet(self.si(0))); // len

        // Create result array with same capacity
        f.instruction(&Instruction::LocalGet(self.si(0)));
        f.instruction(&Instruction::ArrayNewDefault(arr_ty));
        f.instruction(&Instruction::LocalSet(self.si(11))); // dst_arr → anyref

        // i = 0, j = 0
        f.instruction(&Instruction::I32Const(0));
        f.instruction(&Instruction::LocalSet(self.si(1)));
        f.instruction(&Instruction::I32Const(0));
        f.instruction(&Instruction::LocalSet(self.si(2)));

        // block { loop {
        f.instruction(&Instruction::Block(wasm_encoder::BlockType::Empty));
        f.instruction(&Instruction::Loop(wasm_encoder::BlockType::Empty));

        // if i >= len: break
        f.instruction(&Instruction::LocalGet(self.si(1)));
        f.instruction(&Instruction::LocalGet(self.si(0)));
        f.instruction(&Instruction::I32GeU);
        f.instruction(&Instruction::BrIf(1));

        // elem = src_arr[i]
        f.instruction(&Instruction::LocalGet(self.si(10)));
        f.instruction(&Instruction::RefCastNonNull(HeapType::Concrete(arr_ty)));
        f.instruction(&Instruction::LocalGet(self.si(1)));
        f.instruction(&Instruction::ArrayGet(arr_ty));
        // elem is on stack — save to appropriate scratch
        let elem_scratch = if canonical.contains("String") {
            self.si(4) // ref $string
        } else if canonical.contains("i64") {
            self.si(6) // i64
        } else if canonical.contains("f64") {
            self.si(7) // f64
        } else {
            self.si(3) // i32
        };
        f.instruction(&Instruction::LocalTee(elem_scratch));

        // Call predicate
        if let Some(fn_idx) = pred_fn_idx {
            f.instruction(&Instruction::Call(fn_idx));
        } else {
            // Fallback: emit operand (FnRef) and call_indirect
            self.emit_operand(f, &args[1]);
            let pred_type = self
                .indirect_types
                .get(&(vec![ValType::I32], vec![ValType::I32]))
                .copied()
                .unwrap_or(0);
            f.instruction(&Instruction::CallIndirect {
                type_index: pred_type,
                table_index: 0,
            });
        }

        // if predicate true
        f.instruction(&Instruction::If(wasm_encoder::BlockType::Empty));
        f.instruction(&Instruction::LocalGet(self.si(11)));
        f.instruction(&Instruction::RefCastNonNull(HeapType::Concrete(arr_ty)));
        f.instruction(&Instruction::LocalGet(self.si(2))); // j
        f.instruction(&Instruction::LocalGet(elem_scratch));
        f.instruction(&Instruction::ArraySet(arr_ty));
        // j++
        f.instruction(&Instruction::LocalGet(self.si(2)));
        f.instruction(&Instruction::I32Const(1));
        f.instruction(&Instruction::I32Add);
        f.instruction(&Instruction::LocalSet(self.si(2)));
        f.instruction(&Instruction::End); // end if

        // i++
        f.instruction(&Instruction::LocalGet(self.si(1)));
        f.instruction(&Instruction::I32Const(1));
        f.instruction(&Instruction::I32Add);
        f.instruction(&Instruction::LocalSet(self.si(1)));
        f.instruction(&Instruction::Br(0)); // continue loop
        f.instruction(&Instruction::End); // end loop
        f.instruction(&Instruction::End); // end block

        // Create result vec: struct.new $vec_ty (dst_arr, j)
        f.instruction(&Instruction::LocalGet(self.si(11)));
        f.instruction(&Instruction::RefCastNonNull(HeapType::Concrete(arr_ty)));
        f.instruction(&Instruction::LocalGet(self.si(2)));
        f.instruction(&Instruction::StructNew(vec_ty));
    }

    /// GC-native map HOF: map(vec, mapper) -> new_vec
    fn emit_map_hof_gc(&mut self, f: &mut Function, canonical: &str, args: &[Operand]) {
        if args.len() < 2 {
            return;
        }

        let (in_arr_ty, in_vec_ty) = self.hof_gc_types(canonical);
        let (out_arr_ty, out_vec_ty) = self.hof_map_output_types(canonical);
        let map_fn_idx = self.extract_fn_idx(&args[1]);

        // si(0) = len, si(1) = i
        // si(10) = src_arr (anyref), si(11) = dst_arr (anyref)

        // Get source array and length
        self.emit_operand(f, &args[0]);
        f.instruction(&Instruction::StructGet {
            struct_type_index: in_vec_ty,
            field_index: 0,
        });
        f.instruction(&Instruction::LocalSet(self.si(10)));

        self.emit_operand(f, &args[0]);
        f.instruction(&Instruction::StructGet {
            struct_type_index: in_vec_ty,
            field_index: 1,
        });
        f.instruction(&Instruction::LocalSet(self.si(0)));

        // Create result array with same length
        f.instruction(&Instruction::LocalGet(self.si(0)));
        f.instruction(&Instruction::ArrayNewDefault(out_arr_ty));
        f.instruction(&Instruction::LocalSet(self.si(11)));

        // i = 0
        f.instruction(&Instruction::I32Const(0));
        f.instruction(&Instruction::LocalSet(self.si(1)));

        // block { loop {
        f.instruction(&Instruction::Block(wasm_encoder::BlockType::Empty));
        f.instruction(&Instruction::Loop(wasm_encoder::BlockType::Empty));

        // if i >= len: break
        f.instruction(&Instruction::LocalGet(self.si(1)));
        f.instruction(&Instruction::LocalGet(self.si(0)));
        f.instruction(&Instruction::I32GeU);
        f.instruction(&Instruction::BrIf(1));

        // dst_arr[i] = mapper(src_arr[i])
        f.instruction(&Instruction::LocalGet(self.si(11)));
        f.instruction(&Instruction::RefCastNonNull(HeapType::Concrete(out_arr_ty)));
        f.instruction(&Instruction::LocalGet(self.si(1))); // i (index for array.set)

        // Push src_arr[i] as argument to mapper
        f.instruction(&Instruction::LocalGet(self.si(10)));
        f.instruction(&Instruction::RefCastNonNull(HeapType::Concrete(in_arr_ty)));
        f.instruction(&Instruction::LocalGet(self.si(1)));
        f.instruction(&Instruction::ArrayGet(in_arr_ty));

        // Call mapper
        if let Some(fn_idx) = map_fn_idx {
            f.instruction(&Instruction::Call(fn_idx));
        } else {
            f.instruction(&Instruction::I32Const(0));
        }

        // array.set dst_arr[i] = result
        f.instruction(&Instruction::ArraySet(out_arr_ty));

        // i++
        f.instruction(&Instruction::LocalGet(self.si(1)));
        f.instruction(&Instruction::I32Const(1));
        f.instruction(&Instruction::I32Add);
        f.instruction(&Instruction::LocalSet(self.si(1)));
        f.instruction(&Instruction::Br(0));
        f.instruction(&Instruction::End); // end loop
        f.instruction(&Instruction::End); // end block

        // Create result vec
        f.instruction(&Instruction::LocalGet(self.si(11)));
        f.instruction(&Instruction::RefCastNonNull(HeapType::Concrete(out_arr_ty)));
        f.instruction(&Instruction::LocalGet(self.si(0))); // len = same as input
        f.instruction(&Instruction::StructNew(out_vec_ty));
    }

    /// GC-native fold HOF: fold(vec, init, folder) -> acc
    fn emit_fold_hof_gc(&mut self, f: &mut Function, canonical: &str, args: &[Operand]) {
        if args.len() < 3 {
            return;
        }

        let (arr_ty, vec_ty) = self.hof_gc_types(canonical);
        let fold_fn_idx = self.extract_fn_idx(&args[2]);

        // si(0) = len, si(1) = i, si(3) = acc
        // si(10) = src_arr (anyref)

        // Get source array and length
        self.emit_operand(f, &args[0]);
        f.instruction(&Instruction::StructGet {
            struct_type_index: vec_ty,
            field_index: 0,
        });
        f.instruction(&Instruction::LocalSet(self.si(10)));

        self.emit_operand(f, &args[0]);
        f.instruction(&Instruction::StructGet {
            struct_type_index: vec_ty,
            field_index: 1,
        });
        f.instruction(&Instruction::LocalSet(self.si(0)));

        // acc = init
        self.emit_operand(f, &args[1]);
        f.instruction(&Instruction::LocalSet(self.si(3)));

        // i = 0
        f.instruction(&Instruction::I32Const(0));
        f.instruction(&Instruction::LocalSet(self.si(1)));

        // block { loop {
        f.instruction(&Instruction::Block(wasm_encoder::BlockType::Empty));
        f.instruction(&Instruction::Loop(wasm_encoder::BlockType::Empty));

        f.instruction(&Instruction::LocalGet(self.si(1)));
        f.instruction(&Instruction::LocalGet(self.si(0)));
        f.instruction(&Instruction::I32GeU);
        f.instruction(&Instruction::BrIf(1));

        // acc = folder(acc, src_arr[i])
        f.instruction(&Instruction::LocalGet(self.si(3))); // acc
        f.instruction(&Instruction::LocalGet(self.si(10)));
        f.instruction(&Instruction::RefCastNonNull(HeapType::Concrete(arr_ty)));
        f.instruction(&Instruction::LocalGet(self.si(1)));
        f.instruction(&Instruction::ArrayGet(arr_ty)); // element

        if let Some(fn_idx) = fold_fn_idx {
            f.instruction(&Instruction::Call(fn_idx));
        } else {
            f.instruction(&Instruction::Drop);
        }
        f.instruction(&Instruction::LocalSet(self.si(3)));

        // i++
        f.instruction(&Instruction::LocalGet(self.si(1)));
        f.instruction(&Instruction::I32Const(1));
        f.instruction(&Instruction::I32Add);
        f.instruction(&Instruction::LocalSet(self.si(1)));
        f.instruction(&Instruction::Br(0));
        f.instruction(&Instruction::End);
        f.instruction(&Instruction::End);

        // Result: acc
        f.instruction(&Instruction::LocalGet(self.si(3)));
    }

    /// GC-native any HOF: any(vec, pred) -> bool
    fn emit_any_hof_gc(&mut self, f: &mut Function, canonical: &str, args: &[Operand]) {
        if args.len() < 2 {
            f.instruction(&Instruction::I32Const(0));
            return;
        }

        let (arr_ty, vec_ty) = self.hof_gc_types(canonical);
        let pred_fn_idx = self.extract_fn_idx(&args[1]);

        // si(0) = len, si(1) = i, si(3) = result
        // si(10) = src_arr (anyref)

        self.emit_operand(f, &args[0]);
        f.instruction(&Instruction::StructGet {
            struct_type_index: vec_ty,
            field_index: 0,
        });
        f.instruction(&Instruction::LocalSet(self.si(10)));

        self.emit_operand(f, &args[0]);
        f.instruction(&Instruction::StructGet {
            struct_type_index: vec_ty,
            field_index: 1,
        });
        f.instruction(&Instruction::LocalSet(self.si(0)));

        f.instruction(&Instruction::I32Const(0));
        f.instruction(&Instruction::LocalSet(self.si(3))); // result = false
        f.instruction(&Instruction::I32Const(0));
        f.instruction(&Instruction::LocalSet(self.si(1))); // i = 0

        f.instruction(&Instruction::Block(wasm_encoder::BlockType::Empty));
        f.instruction(&Instruction::Loop(wasm_encoder::BlockType::Empty));

        f.instruction(&Instruction::LocalGet(self.si(1)));
        f.instruction(&Instruction::LocalGet(self.si(0)));
        f.instruction(&Instruction::I32GeU);
        f.instruction(&Instruction::BrIf(1));

        // elem = src_arr[i]
        f.instruction(&Instruction::LocalGet(self.si(10)));
        f.instruction(&Instruction::RefCastNonNull(HeapType::Concrete(arr_ty)));
        f.instruction(&Instruction::LocalGet(self.si(1)));
        f.instruction(&Instruction::ArrayGet(arr_ty));

        if let Some(fn_idx) = pred_fn_idx {
            f.instruction(&Instruction::Call(fn_idx));
        } else {
            f.instruction(&Instruction::I32Const(0));
        }

        // if pred true: result = true, break
        f.instruction(&Instruction::If(wasm_encoder::BlockType::Empty));
        f.instruction(&Instruction::I32Const(1));
        f.instruction(&Instruction::LocalSet(self.si(3)));
        f.instruction(&Instruction::Br(2)); // break outer block
        f.instruction(&Instruction::End);

        f.instruction(&Instruction::LocalGet(self.si(1)));
        f.instruction(&Instruction::I32Const(1));
        f.instruction(&Instruction::I32Add);
        f.instruction(&Instruction::LocalSet(self.si(1)));
        f.instruction(&Instruction::Br(0));
        f.instruction(&Instruction::End);
        f.instruction(&Instruction::End);

        f.instruction(&Instruction::LocalGet(self.si(3)));
    }

    /// GC-native find HOF: find(vec, pred) -> Option<T>
    fn emit_find_hof_gc(&mut self, f: &mut Function, canonical: &str, args: &[Operand]) {
        if args.len() < 2 {
            // Return None
            if let Some(&base_ty) = self.enum_base_types.get("Option") {
                f.instruction(&Instruction::StructNew(base_ty + 2)); // None variant
            } else {
                f.instruction(&Instruction::I32Const(0));
            }
            return;
        }

        let (arr_ty, vec_ty) = self.hof_gc_types(canonical);
        let pred_fn_idx = self.extract_fn_idx(&args[1]);

        // Get Option variant types
        let option_some_ty = self
            .enum_base_types
            .get("Option")
            .map(|b| b + 1)
            .unwrap_or(0);
        let option_none_ty = self
            .enum_base_types
            .get("Option")
            .map(|b| b + 2)
            .unwrap_or(0);

        // si(0) = len, si(1) = i, si(3) = found_elem, si(9) = found flag
        // si(10) = src_arr (anyref)

        self.emit_operand(f, &args[0]);
        f.instruction(&Instruction::StructGet {
            struct_type_index: vec_ty,
            field_index: 0,
        });
        f.instruction(&Instruction::LocalSet(self.si(10)));

        self.emit_operand(f, &args[0]);
        f.instruction(&Instruction::StructGet {
            struct_type_index: vec_ty,
            field_index: 1,
        });
        f.instruction(&Instruction::LocalSet(self.si(0)));

        f.instruction(&Instruction::I32Const(0));
        f.instruction(&Instruction::LocalSet(self.si(9))); // found = false
        f.instruction(&Instruction::I32Const(0));
        f.instruction(&Instruction::LocalSet(self.si(1))); // i = 0
        f.instruction(&Instruction::I32Const(0));
        f.instruction(&Instruction::LocalSet(self.si(3))); // found_elem = 0

        f.instruction(&Instruction::Block(wasm_encoder::BlockType::Empty));
        f.instruction(&Instruction::Loop(wasm_encoder::BlockType::Empty));

        f.instruction(&Instruction::LocalGet(self.si(1)));
        f.instruction(&Instruction::LocalGet(self.si(0)));
        f.instruction(&Instruction::I32GeU);
        f.instruction(&Instruction::BrIf(1));

        // elem = src_arr[i]
        f.instruction(&Instruction::LocalGet(self.si(10)));
        f.instruction(&Instruction::RefCastNonNull(HeapType::Concrete(arr_ty)));
        f.instruction(&Instruction::LocalGet(self.si(1)));
        f.instruction(&Instruction::ArrayGet(arr_ty));
        f.instruction(&Instruction::LocalTee(self.si(3)));

        if let Some(fn_idx) = pred_fn_idx {
            f.instruction(&Instruction::Call(fn_idx));
        } else {
            f.instruction(&Instruction::I32Const(0));
        }

        f.instruction(&Instruction::If(wasm_encoder::BlockType::Empty));
        f.instruction(&Instruction::I32Const(1));
        f.instruction(&Instruction::LocalSet(self.si(9))); // found = true
        f.instruction(&Instruction::Br(2)); // break outer block
        f.instruction(&Instruction::End);

        f.instruction(&Instruction::LocalGet(self.si(1)));
        f.instruction(&Instruction::I32Const(1));
        f.instruction(&Instruction::I32Add);
        f.instruction(&Instruction::LocalSet(self.si(1)));
        f.instruction(&Instruction::Br(0));
        f.instruction(&Instruction::End);
        f.instruction(&Instruction::End);

        // Return Option: if found, Some(elem), else None
        f.instruction(&Instruction::LocalGet(self.si(9)));
        f.instruction(&Instruction::If(wasm_encoder::BlockType::FunctionType(
            self.types
                .add_func(&[], &[ref_nullable(option_some_ty - 1)]),
        )));
        f.instruction(&Instruction::LocalGet(self.si(3)));
        f.instruction(&Instruction::StructNew(option_some_ty));
        f.instruction(&Instruction::Else);
        f.instruction(&Instruction::StructNew(option_none_ty));
        f.instruction(&Instruction::End);
    }

    /// GC-native fs_read_file(path) -> Result<String, String>
    /// Copy GC path to linear memory, call path_open + fd_read, build GC string result
    fn emit_fs_read_file_gc(&mut self, f: &mut Function, args: &[Operand]) {
        if args.is_empty() {
            return;
        }

        let result_base = *self.enum_base_types.get("Result_String_String").unwrap();
        let ok_variant = result_base + 1;
        let err_variant = result_base + 2;
        let ma = wasm_encoder::MemArg {
            offset: 0,
            align: 2,
            memory_index: 0,
        };

        // Pre-allocate error strings as data segments (absolute index = active segs + relative)
        let data_seg_base = self.data_segs.len() as u32;
        let err_open_seg = data_seg_base + self.alloc_string_data(b"file open error");
        let _err_read_seg = data_seg_base + self.alloc_string_data(b"file read error");

        // Step 1: Copy GC path string to linear memory at FS_SCRATCH+32
        // si(0) = path_len, si(1) = loop counter
        self.emit_operand(f, &args[0]);
        f.instruction(&Instruction::LocalSet(self.si(10))); // path ref → anyref

        // Get path length
        f.instruction(&Instruction::LocalGet(self.si(10)));
        f.instruction(&Instruction::RefCastNonNull(HeapType::Concrete(
            self.string_ty,
        )));
        f.instruction(&Instruction::ArrayLen);
        f.instruction(&Instruction::LocalSet(self.si(0))); // path_len

        // Copy path bytes to linear memory at FS_SCRATCH+32
        f.instruction(&Instruction::I32Const(0));
        f.instruction(&Instruction::LocalSet(self.si(1))); // i = 0
        f.instruction(&Instruction::Block(wasm_encoder::BlockType::Empty));
        f.instruction(&Instruction::Loop(wasm_encoder::BlockType::Empty));
        f.instruction(&Instruction::LocalGet(self.si(1)));
        f.instruction(&Instruction::LocalGet(self.si(0)));
        f.instruction(&Instruction::I32GeU);
        f.instruction(&Instruction::BrIf(1));
        // mem[FS_SCRATCH+32 + i] = path[i]
        f.instruction(&Instruction::LocalGet(self.si(1)));
        f.instruction(&Instruction::I32Const((FS_SCRATCH + 32) as i32));
        f.instruction(&Instruction::I32Add);
        f.instruction(&Instruction::LocalGet(self.si(10)));
        f.instruction(&Instruction::RefCastNonNull(HeapType::Concrete(
            self.string_ty,
        )));
        f.instruction(&Instruction::LocalGet(self.si(1)));
        f.instruction(&Instruction::ArrayGetU(self.string_ty));
        f.instruction(&Instruction::I32Store8(wasm_encoder::MemArg {
            offset: 0,
            align: 0,
            memory_index: 0,
        }));
        f.instruction(&Instruction::LocalGet(self.si(1)));
        f.instruction(&Instruction::I32Const(1));
        f.instruction(&Instruction::I32Add);
        f.instruction(&Instruction::LocalSet(self.si(1)));
        f.instruction(&Instruction::Br(0));
        f.instruction(&Instruction::End); // loop
        f.instruction(&Instruction::End); // block

        // Step 2: path_open(dirfd=3, dirflags=0, path_ptr, path_len, oflags=0,
        //                   rights=FD_READ(2), inheriting=0, fdflags=0, &opened_fd)
        // Store opened_fd at FS_SCRATCH
        f.instruction(&Instruction::I32Const(3)); // dirfd
        f.instruction(&Instruction::I32Const(0)); // dirflags
        f.instruction(&Instruction::I32Const((FS_SCRATCH + 32) as i32)); // path_ptr
        f.instruction(&Instruction::LocalGet(self.si(0))); // path_len
        f.instruction(&Instruction::I32Const(0)); // oflags
        f.instruction(&Instruction::I64Const(2)); // rights: FD_READ
        f.instruction(&Instruction::I64Const(0)); // inheriting
        f.instruction(&Instruction::I32Const(0)); // fdflags
        f.instruction(&Instruction::I32Const(FS_SCRATCH as i32)); // &opened_fd
        f.instruction(&Instruction::Call(self.wasi_path_open));

        // Step 3: Check error
        f.instruction(&Instruction::I32Const(0));
        f.instruction(&Instruction::I32Ne);
        f.instruction(&Instruction::If(wasm_encoder::BlockType::Empty));
        // Return Err("file open error")
        f.instruction(&Instruction::I32Const(0));
        f.instruction(&Instruction::I32Const(15)); // "file open error" len
        f.instruction(&Instruction::ArrayNewData {
            array_type_index: self.string_ty,
            array_data_index: err_open_seg,
        });
        f.instruction(&Instruction::StructNew(err_variant));
        f.instruction(&Instruction::LocalSet(self.si(10))); // store result
        f.instruction(&Instruction::Else);

        // Step 4: Read file in loop
        // si(2) = total_read, si(3) = fd
        f.instruction(&Instruction::I32Const(FS_SCRATCH as i32));
        f.instruction(&Instruction::I32Load(ma));
        f.instruction(&Instruction::LocalSet(self.si(3))); // fd

        f.instruction(&Instruction::I32Const(0));
        f.instruction(&Instruction::LocalSet(self.si(2))); // total_read = 0

        // Read loop
        f.instruction(&Instruction::Block(wasm_encoder::BlockType::Empty));
        f.instruction(&Instruction::Loop(wasm_encoder::BlockType::Empty));

        // Set up iovec: base = FS_SCRATCH+32 + total_read, len = BUF_SIZE
        // IOV at mem[0..7]: base(4) + len(4)
        f.instruction(&Instruction::I32Const(0)); // iov_base addr
        f.instruction(&Instruction::I32Const((FS_SCRATCH + 32) as i32));
        f.instruction(&Instruction::LocalGet(self.si(2)));
        f.instruction(&Instruction::I32Add);
        f.instruction(&Instruction::I32Store(ma));

        f.instruction(&Instruction::I32Const(4)); // iov_len addr
        f.instruction(&Instruction::I32Const(FS_BUF_SIZE as i32));
        f.instruction(&Instruction::I32Store(ma));

        // fd_read(fd, &iov, 1, &nread)
        // nread at mem[8]
        f.instruction(&Instruction::LocalGet(self.si(3))); // fd
        f.instruction(&Instruction::I32Const(0)); // iov_ptr
        f.instruction(&Instruction::I32Const(1)); // iov_count
        f.instruction(&Instruction::I32Const(8)); // &nread
        f.instruction(&Instruction::Call(self.wasi_fd_read));
        f.instruction(&Instruction::Drop); // drop errno

        // nread = mem[8]
        f.instruction(&Instruction::I32Const(8));
        f.instruction(&Instruction::I32Load(ma));

        // if nread == 0: break
        f.instruction(&Instruction::I32Eqz);
        f.instruction(&Instruction::BrIf(1)); // break out of loop

        // total_read += nread
        f.instruction(&Instruction::LocalGet(self.si(2)));
        f.instruction(&Instruction::I32Const(8));
        f.instruction(&Instruction::I32Load(ma));
        f.instruction(&Instruction::I32Add);
        f.instruction(&Instruction::LocalSet(self.si(2)));

        f.instruction(&Instruction::Br(0)); // continue loop
        f.instruction(&Instruction::End); // loop
        f.instruction(&Instruction::End); // block

        // Step 5: Close fd
        f.instruction(&Instruction::LocalGet(self.si(3)));
        f.instruction(&Instruction::Call(self.wasi_fd_close));
        f.instruction(&Instruction::Drop);

        // Step 6: Build Ok(string) — copy linear memory to GC string
        // Create GC string of total_read length
        f.instruction(&Instruction::LocalGet(self.si(2))); // total_read
        f.instruction(&Instruction::ArrayNewDefault(self.string_ty));
        f.instruction(&Instruction::LocalSet(self.si(4))); // result string ref

        // Copy loop: for i in 0..total_read { string[i] = mem[FS_SCRATCH+32+i] }
        f.instruction(&Instruction::I32Const(0));
        f.instruction(&Instruction::LocalSet(self.si(1))); // i = 0
        f.instruction(&Instruction::Block(wasm_encoder::BlockType::Empty));
        f.instruction(&Instruction::Loop(wasm_encoder::BlockType::Empty));
        f.instruction(&Instruction::LocalGet(self.si(1)));
        f.instruction(&Instruction::LocalGet(self.si(2)));
        f.instruction(&Instruction::I32GeU);
        f.instruction(&Instruction::BrIf(1));
        // string[i] = mem[FS_SCRATCH+32 + i]
        f.instruction(&Instruction::LocalGet(self.si(4)));
        f.instruction(&Instruction::LocalGet(self.si(1)));
        f.instruction(&Instruction::LocalGet(self.si(1)));
        f.instruction(&Instruction::I32Const((FS_SCRATCH + 32) as i32));
        f.instruction(&Instruction::I32Add);
        f.instruction(&Instruction::I32Load8U(wasm_encoder::MemArg {
            offset: 0,
            align: 0,
            memory_index: 0,
        }));
        f.instruction(&Instruction::ArraySet(self.string_ty));
        f.instruction(&Instruction::LocalGet(self.si(1)));
        f.instruction(&Instruction::I32Const(1));
        f.instruction(&Instruction::I32Add);
        f.instruction(&Instruction::LocalSet(self.si(1)));
        f.instruction(&Instruction::Br(0));
        f.instruction(&Instruction::End); // loop
        f.instruction(&Instruction::End); // block

        // Build Ok(string)
        f.instruction(&Instruction::LocalGet(self.si(4)));
        f.instruction(&Instruction::StructNew(ok_variant));
        f.instruction(&Instruction::LocalSet(self.si(10)));

        f.instruction(&Instruction::End); // end if/else

        // Push result
        f.instruction(&Instruction::LocalGet(self.si(10)));
        f.instruction(&Instruction::RefCastNonNull(HeapType::Concrete(
            result_base,
        )));
    }

    /// GC-native fs_write_file(path, content) -> Result<(), String>
    fn emit_fs_write_file_gc(&mut self, f: &mut Function, args: &[Operand]) {
        if args.len() < 2 {
            return;
        }

        let result_base = *self.enum_base_types.get("Result").unwrap();
        let ok_variant = result_base + 1;
        let err_variant = result_base + 2;
        let ma = wasm_encoder::MemArg {
            offset: 0,
            align: 2,
            memory_index: 0,
        };

        let err_write_seg =
            self.data_segs.len() as u32 + self.alloc_string_data(b"file write error");

        // Step 1: Copy path to linear memory at FS_SCRATCH+32
        self.emit_operand(f, &args[0]);
        f.instruction(&Instruction::LocalSet(self.si(10))); // path ref

        f.instruction(&Instruction::LocalGet(self.si(10)));
        f.instruction(&Instruction::RefCastNonNull(HeapType::Concrete(
            self.string_ty,
        )));
        f.instruction(&Instruction::ArrayLen);
        f.instruction(&Instruction::LocalSet(self.si(0))); // path_len

        // Copy path bytes
        f.instruction(&Instruction::I32Const(0));
        f.instruction(&Instruction::LocalSet(self.si(1)));
        f.instruction(&Instruction::Block(wasm_encoder::BlockType::Empty));
        f.instruction(&Instruction::Loop(wasm_encoder::BlockType::Empty));
        f.instruction(&Instruction::LocalGet(self.si(1)));
        f.instruction(&Instruction::LocalGet(self.si(0)));
        f.instruction(&Instruction::I32GeU);
        f.instruction(&Instruction::BrIf(1));
        f.instruction(&Instruction::LocalGet(self.si(1)));
        f.instruction(&Instruction::I32Const((FS_SCRATCH + 32) as i32));
        f.instruction(&Instruction::I32Add);
        f.instruction(&Instruction::LocalGet(self.si(10)));
        f.instruction(&Instruction::RefCastNonNull(HeapType::Concrete(
            self.string_ty,
        )));
        f.instruction(&Instruction::LocalGet(self.si(1)));
        f.instruction(&Instruction::ArrayGetU(self.string_ty));
        f.instruction(&Instruction::I32Store8(wasm_encoder::MemArg {
            offset: 0,
            align: 0,
            memory_index: 0,
        }));
        f.instruction(&Instruction::LocalGet(self.si(1)));
        f.instruction(&Instruction::I32Const(1));
        f.instruction(&Instruction::I32Add);
        f.instruction(&Instruction::LocalSet(self.si(1)));
        f.instruction(&Instruction::Br(0));
        f.instruction(&Instruction::End);
        f.instruction(&Instruction::End);

        // Step 2: path_open for writing
        // oflags = O_CREAT(1) | O_TRUNC(8) = 9
        // rights = FD_WRITE(64)
        f.instruction(&Instruction::I32Const(3)); // dirfd
        f.instruction(&Instruction::I32Const(0)); // dirflags
        f.instruction(&Instruction::I32Const((FS_SCRATCH + 32) as i32));
        f.instruction(&Instruction::LocalGet(self.si(0))); // path_len
        f.instruction(&Instruction::I32Const(9)); // O_CREAT | O_TRUNC
        f.instruction(&Instruction::I64Const(64)); // FD_WRITE
        f.instruction(&Instruction::I64Const(0));
        f.instruction(&Instruction::I32Const(0));
        f.instruction(&Instruction::I32Const(FS_SCRATCH as i32)); // &fd
        f.instruction(&Instruction::Call(self.wasi_path_open));

        // Check error
        f.instruction(&Instruction::I32Const(0));
        f.instruction(&Instruction::I32Ne);
        f.instruction(&Instruction::If(wasm_encoder::BlockType::Empty));
        // Err("file write error")
        f.instruction(&Instruction::I32Const(0));
        f.instruction(&Instruction::I32Const(16)); // len
        f.instruction(&Instruction::ArrayNewData {
            array_type_index: self.string_ty,
            array_data_index: err_write_seg,
        });
        f.instruction(&Instruction::StructNew(err_variant));
        f.instruction(&Instruction::LocalSet(self.si(10)));
        f.instruction(&Instruction::Else);

        // Step 3: Copy content GC string to linear memory
        self.emit_operand(f, &args[1]);
        f.instruction(&Instruction::LocalSet(self.si(11))); // content ref

        f.instruction(&Instruction::LocalGet(self.si(11)));
        f.instruction(&Instruction::RefCastNonNull(HeapType::Concrete(
            self.string_ty,
        )));
        f.instruction(&Instruction::ArrayLen);
        f.instruction(&Instruction::LocalSet(self.si(2))); // content_len

        // Copy content bytes to linear memory at FS_SCRATCH+32
        f.instruction(&Instruction::I32Const(0));
        f.instruction(&Instruction::LocalSet(self.si(1)));
        f.instruction(&Instruction::Block(wasm_encoder::BlockType::Empty));
        f.instruction(&Instruction::Loop(wasm_encoder::BlockType::Empty));
        f.instruction(&Instruction::LocalGet(self.si(1)));
        f.instruction(&Instruction::LocalGet(self.si(2)));
        f.instruction(&Instruction::I32GeU);
        f.instruction(&Instruction::BrIf(1));
        f.instruction(&Instruction::LocalGet(self.si(1)));
        f.instruction(&Instruction::I32Const((FS_SCRATCH + 32) as i32));
        f.instruction(&Instruction::I32Add);
        f.instruction(&Instruction::LocalGet(self.si(11)));
        f.instruction(&Instruction::RefCastNonNull(HeapType::Concrete(
            self.string_ty,
        )));
        f.instruction(&Instruction::LocalGet(self.si(1)));
        f.instruction(&Instruction::ArrayGetU(self.string_ty));
        f.instruction(&Instruction::I32Store8(wasm_encoder::MemArg {
            offset: 0,
            align: 0,
            memory_index: 0,
        }));
        f.instruction(&Instruction::LocalGet(self.si(1)));
        f.instruction(&Instruction::I32Const(1));
        f.instruction(&Instruction::I32Add);
        f.instruction(&Instruction::LocalSet(self.si(1)));
        f.instruction(&Instruction::Br(0));
        f.instruction(&Instruction::End);
        f.instruction(&Instruction::End);

        // Step 4: fd_write
        // Get fd
        f.instruction(&Instruction::I32Const(FS_SCRATCH as i32));
        f.instruction(&Instruction::I32Load(ma));
        f.instruction(&Instruction::LocalSet(self.si(3))); // fd

        // iov: base=FS_SCRATCH+32, len=content_len
        f.instruction(&Instruction::I32Const(0));
        f.instruction(&Instruction::I32Const((FS_SCRATCH + 32) as i32));
        f.instruction(&Instruction::I32Store(ma));
        f.instruction(&Instruction::I32Const(4));
        f.instruction(&Instruction::LocalGet(self.si(2))); // content_len
        f.instruction(&Instruction::I32Store(ma));

        // fd_write(fd, &iov, 1, &nwritten)
        f.instruction(&Instruction::LocalGet(self.si(3)));
        f.instruction(&Instruction::I32Const(0)); // iov_ptr
        f.instruction(&Instruction::I32Const(1)); // iov_count
        f.instruction(&Instruction::I32Const(8)); // &nwritten
        f.instruction(&Instruction::Call(self.wasi_fd_write));
        f.instruction(&Instruction::Drop);

        // Close fd
        f.instruction(&Instruction::LocalGet(self.si(3)));
        f.instruction(&Instruction::Call(self.wasi_fd_close));
        f.instruction(&Instruction::Drop);

        // Build Ok(()) — Ok variant with i32(0) payload
        f.instruction(&Instruction::I32Const(0));
        f.instruction(&Instruction::StructNew(ok_variant));
        f.instruction(&Instruction::LocalSet(self.si(10)));

        f.instruction(&Instruction::End); // end if/else

        // Push result
        f.instruction(&Instruction::LocalGet(self.si(10)));
        f.instruction(&Instruction::RefCastNonNull(HeapType::Concrete(
            result_base,
        )));
    }

    fn emit_vec_new(&mut self, f: &mut Function, element_size: i32, dest: Option<&Place>) {
        self.emit_vec_new_inline(f, element_size);
        if let Some(Place::Local(id)) = dest {
            f.instruction(&Instruction::LocalSet(self.local_wasm_idx(id.0)));
        } else {
            f.instruction(&Instruction::Drop);
        }
    }

    fn emit_vec_new_inline(&mut self, f: &mut Function, _element_size: i32) {
        // GC-native: default to i32 vec (HOF callers should use emit_vec_new_gc)
        self.emit_vec_new_gc(f, self.vec_i32_ty, self.arr_i32_ty);
    }

    fn emit_vec_new_gc(&mut self, f: &mut Function, vec_ty: u32, arr_ty: u32) {
        // Large initial capacity avoids needing grow logic for MVP
        let cap = 16384i32;
        // For string vecs, fill with ref.null; for scalars, fill with 0
        if arr_ty == self.arr_string_ty {
            f.instruction(&Instruction::RefNull(HeapType::Concrete(self.string_ty)));
        } else if arr_ty == self.arr_f64_ty {
            f.instruction(&Instruction::F64Const(0.0));
        } else if arr_ty == self.arr_i64_ty {
            f.instruction(&Instruction::I64Const(0));
        } else {
            // Check if this is a Vec<Struct> array — fill with ref.null of element type
            let mut is_struct_arr = false;
            for (sname, &(a_ty, _)) in &self.custom_vec_types {
                if a_ty == arr_ty {
                    if let Some(&struct_ty) = self.struct_gc_types.get(sname.as_str()) {
                        f.instruction(&Instruction::RefNull(HeapType::Concrete(struct_ty)));
                        is_struct_arr = true;
                    }
                    break;
                }
            }
            if !is_struct_arr {
                f.instruction(&Instruction::I32Const(0)); // fill value for i32 arrays
            }
        }
        f.instruction(&Instruction::I32Const(cap));
        f.instruction(&Instruction::ArrayNew(arr_ty));
        f.instruction(&Instruction::I32Const(0)); // len = 0
        f.instruction(&Instruction::StructNew(vec_ty));
    }

    /// HashMap_i32_i32_insert(map, key, value)
    /// Linear scan keys[0..count], update if found, else append.
    /// Uses scratch: si(0)=count, si(1)=i, si(2)=key, si(3)=value, si(9)=found
    fn emit_hashmap_i32_i32_insert(&mut self, f: &mut Function, args: &[Operand]) {
        if args.len() < 3 {
            return;
        }
        let hm_ty = self.hashmap_i32_i32_ty;
        let arr_ty = self.arr_i32_ty;

        // Push map ref, get count
        self.emit_operand(f, &args[0]);
        f.instruction(&Instruction::LocalSet(self.si(10))); // map → anyref scratch

        // key → si(2), value → si(3)
        self.emit_operand(f, &args[1]);
        f.instruction(&Instruction::LocalSet(self.si(2)));
        self.emit_operand(f, &args[2]);
        f.instruction(&Instruction::LocalSet(self.si(3)));

        // count = map.count
        f.instruction(&Instruction::LocalGet(self.si(10)));
        f.instruction(&Instruction::RefCastNonNull(HeapType::Concrete(hm_ty)));
        f.instruction(&Instruction::StructGet {
            struct_type_index: hm_ty,
            field_index: 2,
        });
        f.instruction(&Instruction::LocalSet(self.si(0)));

        // i = 0, found = 0
        f.instruction(&Instruction::I32Const(0));
        f.instruction(&Instruction::LocalSet(self.si(1)));
        f.instruction(&Instruction::I32Const(0));
        f.instruction(&Instruction::LocalSet(self.si(9)));

        // Search loop: for i in 0..count { if keys[i] == key { values[i] = value; found = 1; break } }
        f.instruction(&Instruction::Block(wasm_encoder::BlockType::Empty));
        f.instruction(&Instruction::Loop(wasm_encoder::BlockType::Empty));

        // if i >= count: break
        f.instruction(&Instruction::LocalGet(self.si(1)));
        f.instruction(&Instruction::LocalGet(self.si(0)));
        f.instruction(&Instruction::I32GeU);
        f.instruction(&Instruction::BrIf(1));

        // if keys[i] == key
        f.instruction(&Instruction::LocalGet(self.si(10)));
        f.instruction(&Instruction::RefCastNonNull(HeapType::Concrete(hm_ty)));
        f.instruction(&Instruction::StructGet {
            struct_type_index: hm_ty,
            field_index: 0,
        });
        f.instruction(&Instruction::RefCastNonNull(HeapType::Concrete(arr_ty)));
        f.instruction(&Instruction::LocalGet(self.si(1)));
        f.instruction(&Instruction::ArrayGet(arr_ty));
        f.instruction(&Instruction::LocalGet(self.si(2)));
        f.instruction(&Instruction::I32Eq);

        f.instruction(&Instruction::If(wasm_encoder::BlockType::Empty));
        // values[i] = value
        f.instruction(&Instruction::LocalGet(self.si(10)));
        f.instruction(&Instruction::RefCastNonNull(HeapType::Concrete(hm_ty)));
        f.instruction(&Instruction::StructGet {
            struct_type_index: hm_ty,
            field_index: 1,
        });
        f.instruction(&Instruction::RefCastNonNull(HeapType::Concrete(arr_ty)));
        f.instruction(&Instruction::LocalGet(self.si(1)));
        f.instruction(&Instruction::LocalGet(self.si(3)));
        f.instruction(&Instruction::ArraySet(arr_ty));
        // found = 1
        f.instruction(&Instruction::I32Const(1));
        f.instruction(&Instruction::LocalSet(self.si(9)));
        f.instruction(&Instruction::Br(2)); // break outer block
        f.instruction(&Instruction::End);

        // i++
        f.instruction(&Instruction::LocalGet(self.si(1)));
        f.instruction(&Instruction::I32Const(1));
        f.instruction(&Instruction::I32Add);
        f.instruction(&Instruction::LocalSet(self.si(1)));
        f.instruction(&Instruction::Br(0)); // continue loop
        f.instruction(&Instruction::End); // end loop
        f.instruction(&Instruction::End); // end block

        // If not found, append: keys[count] = key, values[count] = value, count++
        f.instruction(&Instruction::LocalGet(self.si(9)));
        f.instruction(&Instruction::I32Eqz);
        f.instruction(&Instruction::If(wasm_encoder::BlockType::Empty));

        // keys[count] = key
        f.instruction(&Instruction::LocalGet(self.si(10)));
        f.instruction(&Instruction::RefCastNonNull(HeapType::Concrete(hm_ty)));
        f.instruction(&Instruction::StructGet {
            struct_type_index: hm_ty,
            field_index: 0,
        });
        f.instruction(&Instruction::RefCastNonNull(HeapType::Concrete(arr_ty)));
        f.instruction(&Instruction::LocalGet(self.si(0)));
        f.instruction(&Instruction::LocalGet(self.si(2)));
        f.instruction(&Instruction::ArraySet(arr_ty));

        // values[count] = value
        f.instruction(&Instruction::LocalGet(self.si(10)));
        f.instruction(&Instruction::RefCastNonNull(HeapType::Concrete(hm_ty)));
        f.instruction(&Instruction::StructGet {
            struct_type_index: hm_ty,
            field_index: 1,
        });
        f.instruction(&Instruction::RefCastNonNull(HeapType::Concrete(arr_ty)));
        f.instruction(&Instruction::LocalGet(self.si(0)));
        f.instruction(&Instruction::LocalGet(self.si(3)));
        f.instruction(&Instruction::ArraySet(arr_ty));

        // map.count = count + 1
        f.instruction(&Instruction::LocalGet(self.si(10)));
        f.instruction(&Instruction::RefCastNonNull(HeapType::Concrete(hm_ty)));
        f.instruction(&Instruction::LocalGet(self.si(0)));
        f.instruction(&Instruction::I32Const(1));
        f.instruction(&Instruction::I32Add);
        f.instruction(&Instruction::StructSet {
            struct_type_index: hm_ty,
            field_index: 2,
        });

        f.instruction(&Instruction::End); // end if
    }

    /// HashMap_i32_i32_get(map, key) -> Option<i32>
    /// Linear scan, returns Some(value) or None as GC enum variants.
    fn emit_hashmap_i32_i32_get(&mut self, f: &mut Function, args: &[Operand]) {
        if args.len() < 2 {
            // Return None
            if let Some(&base_ty) = self.enum_base_types.get("Option") {
                f.instruction(&Instruction::StructNew(base_ty + 2));
            } else {
                f.instruction(&Instruction::I32Const(0));
            }
            return;
        }
        let hm_ty = self.hashmap_i32_i32_ty;
        let arr_ty = self.arr_i32_ty;
        let option_base = *self.enum_base_types.get("Option").unwrap_or(&0);
        let option_some_ty = option_base + 1;
        let option_none_ty = option_base + 2;

        // map → si(10)
        self.emit_operand(f, &args[0]);
        f.instruction(&Instruction::LocalSet(self.si(10)));

        // key → si(2)
        self.emit_operand(f, &args[1]);
        f.instruction(&Instruction::LocalSet(self.si(2)));

        // count = map.count → si(0)
        f.instruction(&Instruction::LocalGet(self.si(10)));
        f.instruction(&Instruction::RefCastNonNull(HeapType::Concrete(hm_ty)));
        f.instruction(&Instruction::StructGet {
            struct_type_index: hm_ty,
            field_index: 2,
        });
        f.instruction(&Instruction::LocalSet(self.si(0)));

        // i = 0, found = 0, result_val = 0
        f.instruction(&Instruction::I32Const(0));
        f.instruction(&Instruction::LocalSet(self.si(1)));
        f.instruction(&Instruction::I32Const(0));
        f.instruction(&Instruction::LocalSet(self.si(9))); // found flag
        f.instruction(&Instruction::I32Const(0));
        f.instruction(&Instruction::LocalSet(self.si(3))); // result value

        // Search loop
        f.instruction(&Instruction::Block(wasm_encoder::BlockType::Empty));
        f.instruction(&Instruction::Loop(wasm_encoder::BlockType::Empty));

        f.instruction(&Instruction::LocalGet(self.si(1)));
        f.instruction(&Instruction::LocalGet(self.si(0)));
        f.instruction(&Instruction::I32GeU);
        f.instruction(&Instruction::BrIf(1));

        // if keys[i] == key
        f.instruction(&Instruction::LocalGet(self.si(10)));
        f.instruction(&Instruction::RefCastNonNull(HeapType::Concrete(hm_ty)));
        f.instruction(&Instruction::StructGet {
            struct_type_index: hm_ty,
            field_index: 0,
        });
        f.instruction(&Instruction::RefCastNonNull(HeapType::Concrete(arr_ty)));
        f.instruction(&Instruction::LocalGet(self.si(1)));
        f.instruction(&Instruction::ArrayGet(arr_ty));
        f.instruction(&Instruction::LocalGet(self.si(2)));
        f.instruction(&Instruction::I32Eq);

        f.instruction(&Instruction::If(wasm_encoder::BlockType::Empty));
        // result_val = values[i]
        f.instruction(&Instruction::LocalGet(self.si(10)));
        f.instruction(&Instruction::RefCastNonNull(HeapType::Concrete(hm_ty)));
        f.instruction(&Instruction::StructGet {
            struct_type_index: hm_ty,
            field_index: 1,
        });
        f.instruction(&Instruction::RefCastNonNull(HeapType::Concrete(arr_ty)));
        f.instruction(&Instruction::LocalGet(self.si(1)));
        f.instruction(&Instruction::ArrayGet(arr_ty));
        f.instruction(&Instruction::LocalSet(self.si(3)));
        // found = 1
        f.instruction(&Instruction::I32Const(1));
        f.instruction(&Instruction::LocalSet(self.si(9)));
        f.instruction(&Instruction::Br(2)); // break outer block
        f.instruction(&Instruction::End);

        // i++
        f.instruction(&Instruction::LocalGet(self.si(1)));
        f.instruction(&Instruction::I32Const(1));
        f.instruction(&Instruction::I32Add);
        f.instruction(&Instruction::LocalSet(self.si(1)));
        f.instruction(&Instruction::Br(0));
        f.instruction(&Instruction::End); // end loop
        f.instruction(&Instruction::End); // end block

        // Return Option: if found then Some(result_val) else None
        let option_ref = ref_nullable(option_base);
        f.instruction(&Instruction::LocalGet(self.si(9)));
        f.instruction(&Instruction::If(wasm_encoder::BlockType::Result(
            option_ref,
        )));
        f.instruction(&Instruction::LocalGet(self.si(3)));
        f.instruction(&Instruction::StructNew(option_some_ty));
        f.instruction(&Instruction::Else);
        f.instruction(&Instruction::StructNew(option_none_ty));
        f.instruction(&Instruction::End);
    }

    /// HashMap_i32_i32_contains_key(map, key) -> bool (i32)
    fn emit_hashmap_i32_i32_contains_key(&mut self, f: &mut Function, args: &[Operand]) {
        if args.len() < 2 {
            f.instruction(&Instruction::I32Const(0));
            return;
        }
        let hm_ty = self.hashmap_i32_i32_ty;
        let arr_ty = self.arr_i32_ty;

        // map → si(10)
        self.emit_operand(f, &args[0]);
        f.instruction(&Instruction::LocalSet(self.si(10)));

        // key → si(2)
        self.emit_operand(f, &args[1]);
        f.instruction(&Instruction::LocalSet(self.si(2)));

        // count → si(0)
        f.instruction(&Instruction::LocalGet(self.si(10)));
        f.instruction(&Instruction::RefCastNonNull(HeapType::Concrete(hm_ty)));
        f.instruction(&Instruction::StructGet {
            struct_type_index: hm_ty,
            field_index: 2,
        });
        f.instruction(&Instruction::LocalSet(self.si(0)));

        // i = 0, found = 0
        f.instruction(&Instruction::I32Const(0));
        f.instruction(&Instruction::LocalSet(self.si(1)));
        f.instruction(&Instruction::I32Const(0));
        f.instruction(&Instruction::LocalSet(self.si(9)));

        // Search loop
        f.instruction(&Instruction::Block(wasm_encoder::BlockType::Empty));
        f.instruction(&Instruction::Loop(wasm_encoder::BlockType::Empty));

        f.instruction(&Instruction::LocalGet(self.si(1)));
        f.instruction(&Instruction::LocalGet(self.si(0)));
        f.instruction(&Instruction::I32GeU);
        f.instruction(&Instruction::BrIf(1));

        // if keys[i] == key
        f.instruction(&Instruction::LocalGet(self.si(10)));
        f.instruction(&Instruction::RefCastNonNull(HeapType::Concrete(hm_ty)));
        f.instruction(&Instruction::StructGet {
            struct_type_index: hm_ty,
            field_index: 0,
        });
        f.instruction(&Instruction::RefCastNonNull(HeapType::Concrete(arr_ty)));
        f.instruction(&Instruction::LocalGet(self.si(1)));
        f.instruction(&Instruction::ArrayGet(arr_ty));
        f.instruction(&Instruction::LocalGet(self.si(2)));
        f.instruction(&Instruction::I32Eq);

        f.instruction(&Instruction::If(wasm_encoder::BlockType::Empty));
        f.instruction(&Instruction::I32Const(1));
        f.instruction(&Instruction::LocalSet(self.si(9)));
        f.instruction(&Instruction::Br(2)); // break outer block
        f.instruction(&Instruction::End);

        // i++
        f.instruction(&Instruction::LocalGet(self.si(1)));
        f.instruction(&Instruction::I32Const(1));
        f.instruction(&Instruction::I32Add);
        f.instruction(&Instruction::LocalSet(self.si(1)));
        f.instruction(&Instruction::Br(0));
        f.instruction(&Instruction::End); // end loop
        f.instruction(&Instruction::End); // end block

        f.instruction(&Instruction::LocalGet(self.si(9)));
    }

    fn emit_push(&mut self, f: &mut Function, args: &[Operand]) {
        if args.len() < 2 {
            return;
        }

        let vec_ty = self.infer_vec_type_idx(&args[0]);
        let arr_ty = self.infer_arr_type_idx(&args[0]);
        let is_f64 = self.is_f64_vec_operand(&args[0]);
        let is_i64 = self.is_i64_vec_operand(&args[0]);

        let scr_len = self.si(0);

        // len = struct.get $vec 1
        self.emit_operand(f, &args[0]);
        f.instruction(&Instruction::StructGet {
            struct_type_index: vec_ty,
            field_index: 1,
        });
        f.instruction(&Instruction::LocalSet(scr_len));

        // array.set backing[len] = val
        self.emit_operand(f, &args[0]);
        f.instruction(&Instruction::StructGet {
            struct_type_index: vec_ty,
            field_index: 0,
        });
        f.instruction(&Instruction::LocalGet(scr_len));
        self.emit_operand_coerced(f, &args[1], is_i64, is_f64);
        f.instruction(&Instruction::ArraySet(arr_ty));

        // struct.set vec.len = len + 1
        self.emit_operand(f, &args[0]);
        f.instruction(&Instruction::LocalGet(scr_len));
        f.instruction(&Instruction::I32Const(1));
        f.instruction(&Instruction::I32Add);
        f.instruction(&Instruction::StructSet {
            struct_type_index: vec_ty,
            field_index: 1,
        });
    }

    fn emit_len_inline(&mut self, f: &mut Function, arg: Option<&Operand>) {
        if let Some(arg) = arg {
            let vt = self.infer_operand_type(arg);
            let is_ref = matches!(vt, ValType::Ref(_));
            self.emit_operand(f, arg);
            if is_ref {
                // GC-native: string → array.len, vec → struct.get $vec 1
                if self.is_string_like_operand(arg) {
                    f.instruction(&Instruction::ArrayLen);
                } else {
                    // Vec: length is field 1 of the vec struct
                    f.instruction(&Instruction::StructGet {
                        struct_type_index: self.infer_vec_type_idx(arg),
                        field_index: 1,
                    });
                }
            } else {
                let ma = MemArg {
                    offset: 0,
                    align: 2,
                    memory_index: 0,
                };
                // Bridge fallback (non-GC operand)
                f.instruction(&Instruction::I32Const(4));
                f.instruction(&Instruction::I32Add);
                f.instruction(&Instruction::I32Load(ma));
            }
        } else {
            f.instruction(&Instruction::I32Const(0));
        }
    }

    fn emit_get_unchecked_inline(&mut self, f: &mut Function, args: &[Operand]) {
        if args.len() < 2 {
            f.instruction(&Instruction::I32Const(0));
            return;
        }

        let vec_ty = self.infer_vec_type_idx(&args[0]);
        let arr_ty = self.infer_arr_type_idx(&args[0]);

        // array.get backing[index]
        self.emit_operand(f, &args[0]);
        f.instruction(&Instruction::StructGet {
            struct_type_index: vec_ty,
            field_index: 0,
        });
        self.emit_operand(f, &args[1]);
        f.instruction(&Instruction::ArrayGet(arr_ty));
    }

    fn emit_get_inline(&mut self, f: &mut Function, args: &[Operand]) {
        if args.len() < 2 {
            f.instruction(&Instruction::I32Const(0));
            return;
        }

        let vec_ty = self.infer_vec_type_idx(&args[0]);
        let arr_ty = self.infer_arr_type_idx(&args[0]);

        // get() returns Option<T>
        if let Some(&base_ty) = self.enum_base_types.get("Option") {
            let variants = self.enum_variant_types.get("Option");
            let some_ty = variants.and_then(|v| v.get("Some").copied()).unwrap_or(0);
            let none_ty = variants.and_then(|v| v.get("None").copied()).unwrap_or(0);

            let scr_idx = self.si(0);
            let option_ref = ref_nullable(base_ty);

            // Bounds check: index < len
            self.emit_operand(f, &args[1]);
            f.instruction(&Instruction::LocalSet(scr_idx));
            f.instruction(&Instruction::LocalGet(scr_idx));
            self.emit_operand(f, &args[0]);
            f.instruction(&Instruction::StructGet {
                struct_type_index: vec_ty,
                field_index: 1,
            });
            f.instruction(&Instruction::I32LtU);

            f.instruction(&Instruction::If(wasm_encoder::BlockType::Result(
                option_ref,
            )));

            // Some branch: get element, wrap in Option::Some
            self.emit_operand(f, &args[0]);
            f.instruction(&Instruction::StructGet {
                struct_type_index: vec_ty,
                field_index: 0,
            });
            f.instruction(&Instruction::LocalGet(scr_idx));
            f.instruction(&Instruction::ArrayGet(arr_ty));
            f.instruction(&Instruction::StructNew(some_ty));

            f.instruction(&Instruction::Else);

            // None branch
            f.instruction(&Instruction::StructNew(none_ty));

            f.instruction(&Instruction::End);
        } else {
            // Fallback: just do unchecked get
            self.emit_get_unchecked_inline(f, args);
        }
    }

    fn emit_set(&mut self, f: &mut Function, args: &[Operand]) {
        if args.len() < 3 {
            return;
        }

        let vec_ty = self.infer_vec_type_idx(&args[0]);
        let arr_ty = self.infer_arr_type_idx(&args[0]);
        let is_f64 = self.is_f64_vec_operand(&args[0]);
        let is_i64 = self.is_i64_vec_operand(&args[0]);

        // array.set backing[index] = val
        self.emit_operand(f, &args[0]);
        f.instruction(&Instruction::StructGet {
            struct_type_index: vec_ty,
            field_index: 0,
        });
        self.emit_operand(f, &args[1]);
        self.emit_operand_coerced(f, &args[2], is_i64, is_f64);
        f.instruction(&Instruction::ArraySet(arr_ty));
    }

    fn emit_pop_inline(&mut self, f: &mut Function, args: &[Operand]) {
        if args.is_empty() {
            f.instruction(&Instruction::I32Const(0));
            return;
        }

        let vec_ty = self.infer_vec_type_idx(&args[0]);
        let arr_ty = self.infer_arr_type_idx(&args[0]);

        // pop() returns Option<T>
        if let Some(&base_ty) = self.enum_base_types.get("Option") {
            let variants = self.enum_variant_types.get("Option");
            let some_ty = variants.and_then(|v| v.get("Some").copied()).unwrap_or(0);
            let none_ty = variants.and_then(|v| v.get("None").copied()).unwrap_or(0);
            let option_ref = ref_nullable(base_ty);

            let scr_len = self.si(0);

            // len = struct.get $vec 1
            self.emit_operand(f, &args[0]);
            f.instruction(&Instruction::StructGet {
                struct_type_index: vec_ty,
                field_index: 1,
            });
            f.instruction(&Instruction::LocalSet(scr_len));

            // if len > 0
            f.instruction(&Instruction::LocalGet(scr_len));
            f.instruction(&Instruction::I32Const(0));
            f.instruction(&Instruction::I32GtU);
            f.instruction(&Instruction::If(wasm_encoder::BlockType::Result(
                option_ref,
            )));

            // new_len = len - 1
            f.instruction(&Instruction::LocalGet(scr_len));
            f.instruction(&Instruction::I32Const(1));
            f.instruction(&Instruction::I32Sub);
            f.instruction(&Instruction::LocalSet(scr_len));

            // Get element at new_len
            self.emit_operand(f, &args[0]);
            f.instruction(&Instruction::StructGet {
                struct_type_index: vec_ty,
                field_index: 0,
            });
            f.instruction(&Instruction::LocalGet(scr_len));
            f.instruction(&Instruction::ArrayGet(arr_ty));

            // Update vec.len = new_len
            self.emit_operand(f, &args[0]);
            f.instruction(&Instruction::LocalGet(scr_len));
            f.instruction(&Instruction::StructSet {
                struct_type_index: vec_ty,
                field_index: 1,
            });

            // Wrap in Option::Some
            f.instruction(&Instruction::StructNew(some_ty));

            f.instruction(&Instruction::Else);

            // None branch
            f.instruction(&Instruction::StructNew(none_ty));

            f.instruction(&Instruction::End);
        } else {
            // Fallback: return unchecked last element
            let scr_len = self.si(0);
            self.emit_operand(f, &args[0]);
            f.instruction(&Instruction::StructGet {
                struct_type_index: vec_ty,
                field_index: 1,
            });
            f.instruction(&Instruction::I32Const(1));
            f.instruction(&Instruction::I32Sub);
            f.instruction(&Instruction::LocalSet(scr_len));

            // Decrement len
            self.emit_operand(f, &args[0]);
            f.instruction(&Instruction::LocalGet(scr_len));
            f.instruction(&Instruction::StructSet {
                struct_type_index: vec_ty,
                field_index: 1,
            });

            // Get element at new len
            self.emit_operand(f, &args[0]);
            f.instruction(&Instruction::StructGet {
                struct_type_index: vec_ty,
                field_index: 0,
            });
            f.instruction(&Instruction::LocalGet(scr_len));
            f.instruction(&Instruction::ArrayGet(arr_ty));
        }
    }

    /// contains_i32(v, x) / contains_String(v, s) → bool
    fn emit_contains_inline(&mut self, f: &mut Function, canonical: &str, args: &[Operand]) {
        if args.len() < 2 {
            f.instruction(&Instruction::I32Const(0));
            return;
        }
        let is_string = canonical.ends_with("String");
        let (vec_ty, arr_ty) = if is_string {
            (self.vec_string_ty, self.arr_string_ty)
        } else {
            let vt = self.infer_vec_type_idx(&args[0]);
            let at = self.infer_arr_type_idx(&args[0]);
            (vt, at)
        };

        // Use si(2),si(3) for our loop vars to avoid conflict with string_eq_gc (uses si(0),si(1))
        let scr_i = self.si(2);
        let scr_len = self.si(3);

        // len = vec.len
        self.emit_operand(f, &args[0]);
        f.instruction(&Instruction::StructGet {
            struct_type_index: vec_ty,
            field_index: 1,
        });
        f.instruction(&Instruction::LocalSet(scr_len));

        // i = 0
        f.instruction(&Instruction::I32Const(0));
        f.instruction(&Instruction::LocalSet(scr_i));

        // block $done (result i32)
        f.instruction(&Instruction::Block(wasm_encoder::BlockType::Result(
            ValType::I32,
        )));
        // block $not_found
        f.instruction(&Instruction::Block(wasm_encoder::BlockType::Empty));
        // loop $search
        f.instruction(&Instruction::Loop(wasm_encoder::BlockType::Empty));

        // if i >= len, br $not_found
        f.instruction(&Instruction::LocalGet(scr_i));
        f.instruction(&Instruction::LocalGet(scr_len));
        f.instruction(&Instruction::I32GeU);
        f.instruction(&Instruction::BrIf(1)); // break to $not_found

        // elem = backing[i]
        self.emit_operand(f, &args[0]);
        f.instruction(&Instruction::StructGet {
            struct_type_index: vec_ty,
            field_index: 0,
        });
        f.instruction(&Instruction::LocalGet(scr_i));
        f.instruction(&Instruction::ArrayGet(arr_ty));

        // compare with target
        if is_string {
            // String comparison: call string_eq helper or element-wise
            self.emit_operand(f, &args[1]);
            self.emit_string_eq_gc(f);
        } else {
            self.emit_operand(f, &args[1]);
            f.instruction(&Instruction::I32Eq);
        }

        f.instruction(&Instruction::If(wasm_encoder::BlockType::Empty));
        f.instruction(&Instruction::I32Const(1));
        f.instruction(&Instruction::Br(3)); // br $done with 1 (if=0, loop=1, not_found=2, done=3)
        f.instruction(&Instruction::End);

        // i++
        f.instruction(&Instruction::LocalGet(scr_i));
        f.instruction(&Instruction::I32Const(1));
        f.instruction(&Instruction::I32Add);
        f.instruction(&Instruction::LocalSet(scr_i));
        f.instruction(&Instruction::Br(0)); // br $search

        f.instruction(&Instruction::End); // end loop
        f.instruction(&Instruction::End); // end $not_found
        f.instruction(&Instruction::I32Const(0)); // not found
        f.instruction(&Instruction::End); // end $done
    }

    /// reverse_i32(v) / reverse_String(v) — in-place reversal
    fn emit_reverse_inline(&mut self, f: &mut Function, canonical: &str, args: &[Operand]) {
        if args.is_empty() {
            return;
        }
        let is_string = canonical.ends_with("String");
        let (vec_ty, arr_ty) = if is_string {
            (self.vec_string_ty, self.arr_string_ty)
        } else {
            (
                self.infer_vec_type_idx(&args[0]),
                self.infer_arr_type_idx(&args[0]),
            )
        };
        let elem_vt = if is_string {
            ref_nullable(self.string_ty)
        } else {
            ValType::I32
        };

        let scr_lo = self.si(0);
        let scr_hi = self.si(1);
        let scr_tmp_idx = self.scratch_local_for_valtype(elem_vt);

        // lo = 0
        f.instruction(&Instruction::I32Const(0));
        f.instruction(&Instruction::LocalSet(scr_lo));

        // hi = vec.len - 1
        self.emit_operand(f, &args[0]);
        f.instruction(&Instruction::StructGet {
            struct_type_index: vec_ty,
            field_index: 1,
        });
        f.instruction(&Instruction::I32Const(1));
        f.instruction(&Instruction::I32Sub);
        f.instruction(&Instruction::LocalSet(scr_hi));

        // loop while lo < hi
        f.instruction(&Instruction::Block(wasm_encoder::BlockType::Empty));
        f.instruction(&Instruction::Loop(wasm_encoder::BlockType::Empty));

        f.instruction(&Instruction::LocalGet(scr_lo));
        f.instruction(&Instruction::LocalGet(scr_hi));
        f.instruction(&Instruction::I32GeS);
        f.instruction(&Instruction::BrIf(1));

        // tmp = backing[lo]
        self.emit_operand(f, &args[0]);
        f.instruction(&Instruction::StructGet {
            struct_type_index: vec_ty,
            field_index: 0,
        });
        f.instruction(&Instruction::LocalGet(scr_lo));
        f.instruction(&Instruction::ArrayGet(arr_ty));
        f.instruction(&Instruction::LocalSet(scr_tmp_idx));

        // backing[lo] = backing[hi]
        self.emit_operand(f, &args[0]);
        f.instruction(&Instruction::StructGet {
            struct_type_index: vec_ty,
            field_index: 0,
        });
        f.instruction(&Instruction::LocalGet(scr_lo));
        self.emit_operand(f, &args[0]);
        f.instruction(&Instruction::StructGet {
            struct_type_index: vec_ty,
            field_index: 0,
        });
        f.instruction(&Instruction::LocalGet(scr_hi));
        f.instruction(&Instruction::ArrayGet(arr_ty));
        f.instruction(&Instruction::ArraySet(arr_ty));

        // backing[hi] = tmp
        self.emit_operand(f, &args[0]);
        f.instruction(&Instruction::StructGet {
            struct_type_index: vec_ty,
            field_index: 0,
        });
        f.instruction(&Instruction::LocalGet(scr_hi));
        f.instruction(&Instruction::LocalGet(scr_tmp_idx));
        f.instruction(&Instruction::ArraySet(arr_ty));

        // lo++; hi--
        f.instruction(&Instruction::LocalGet(scr_lo));
        f.instruction(&Instruction::I32Const(1));
        f.instruction(&Instruction::I32Add);
        f.instruction(&Instruction::LocalSet(scr_lo));

        f.instruction(&Instruction::LocalGet(scr_hi));
        f.instruction(&Instruction::I32Const(1));
        f.instruction(&Instruction::I32Sub);
        f.instruction(&Instruction::LocalSet(scr_hi));

        f.instruction(&Instruction::Br(0));
        f.instruction(&Instruction::End); // end loop
        f.instruction(&Instruction::End); // end block
    }

    /// remove_i32(v, idx) — shift elements left from idx+1..len, decrement len
    fn emit_remove_inline(&mut self, f: &mut Function, args: &[Operand]) {
        if args.len() < 2 {
            return;
        }
        let vec_ty = self.infer_vec_type_idx(&args[0]);
        let arr_ty = self.infer_arr_type_idx(&args[0]);

        let scr_i = self.si(0);
        let scr_len = self.si(1);

        // len = vec.len
        self.emit_operand(f, &args[0]);
        f.instruction(&Instruction::StructGet {
            struct_type_index: vec_ty,
            field_index: 1,
        });
        f.instruction(&Instruction::LocalSet(scr_len));

        // i = idx
        self.emit_operand(f, &args[1]);
        f.instruction(&Instruction::LocalSet(scr_i));

        // shift loop: for i in idx..len-1: backing[i] = backing[i+1]
        f.instruction(&Instruction::Block(wasm_encoder::BlockType::Empty));
        f.instruction(&Instruction::Loop(wasm_encoder::BlockType::Empty));

        // if i >= len - 1, break
        f.instruction(&Instruction::LocalGet(scr_i));
        f.instruction(&Instruction::LocalGet(scr_len));
        f.instruction(&Instruction::I32Const(1));
        f.instruction(&Instruction::I32Sub);
        f.instruction(&Instruction::I32GeU);
        f.instruction(&Instruction::BrIf(1));

        // backing[i] = backing[i+1]
        self.emit_operand(f, &args[0]);
        f.instruction(&Instruction::StructGet {
            struct_type_index: vec_ty,
            field_index: 0,
        });
        f.instruction(&Instruction::LocalGet(scr_i));
        self.emit_operand(f, &args[0]);
        f.instruction(&Instruction::StructGet {
            struct_type_index: vec_ty,
            field_index: 0,
        });
        f.instruction(&Instruction::LocalGet(scr_i));
        f.instruction(&Instruction::I32Const(1));
        f.instruction(&Instruction::I32Add);
        f.instruction(&Instruction::ArrayGet(arr_ty));
        f.instruction(&Instruction::ArraySet(arr_ty));

        // i++
        f.instruction(&Instruction::LocalGet(scr_i));
        f.instruction(&Instruction::I32Const(1));
        f.instruction(&Instruction::I32Add);
        f.instruction(&Instruction::LocalSet(scr_i));
        f.instruction(&Instruction::Br(0));

        f.instruction(&Instruction::End); // end loop
        f.instruction(&Instruction::End); // end block

        // vec.len = len - 1
        self.emit_operand(f, &args[0]);
        f.instruction(&Instruction::LocalGet(scr_len));
        f.instruction(&Instruction::I32Const(1));
        f.instruction(&Instruction::I32Sub);
        f.instruction(&Instruction::StructSet {
            struct_type_index: vec_ty,
            field_index: 1,
        });
    }

    /// sum_i32/i64/f64 / product_i32/i64/f64 — fold over vec
    fn emit_sum_product_inline(&mut self, f: &mut Function, canonical: &str, args: &[Operand]) {
        if args.is_empty() {
            f.instruction(&Instruction::I32Const(0));
            return;
        }
        let is_product = canonical.starts_with("product");
        let is_i64 = canonical.ends_with("i64");
        let is_f64 = canonical.ends_with("f64");

        let (vec_ty, arr_ty) = if is_f64 {
            (self.vec_f64_ty, self.arr_f64_ty)
        } else if is_i64 {
            (self.vec_i64_ty, self.arr_i64_ty)
        } else {
            (
                self.infer_vec_type_idx(&args[0]),
                self.infer_arr_type_idx(&args[0]),
            )
        };

        let scr_i = self.si(0);
        let scr_len = self.si(1);
        let scr_acc = if is_f64 {
            self.scratch_local_for_valtype(ValType::F64)
        } else if is_i64 {
            self.scratch_local_for_valtype(ValType::I64)
        } else {
            self.si(2)
        };

        // len = vec.len
        self.emit_operand(f, &args[0]);
        f.instruction(&Instruction::StructGet {
            struct_type_index: vec_ty,
            field_index: 1,
        });
        f.instruction(&Instruction::LocalSet(scr_len));

        // acc = identity (0 for sum, 1 for product)
        if is_f64 {
            f.instruction(&Instruction::F64Const(if is_product { 1.0 } else { 0.0 }));
        } else if is_i64 {
            f.instruction(&Instruction::I64Const(if is_product { 1 } else { 0 }));
        } else {
            f.instruction(&Instruction::I32Const(if is_product { 1 } else { 0 }));
        }
        f.instruction(&Instruction::LocalSet(scr_acc));

        // i = 0
        f.instruction(&Instruction::I32Const(0));
        f.instruction(&Instruction::LocalSet(scr_i));

        f.instruction(&Instruction::Block(wasm_encoder::BlockType::Empty));
        f.instruction(&Instruction::Loop(wasm_encoder::BlockType::Empty));

        // if i >= len, break
        f.instruction(&Instruction::LocalGet(scr_i));
        f.instruction(&Instruction::LocalGet(scr_len));
        f.instruction(&Instruction::I32GeU);
        f.instruction(&Instruction::BrIf(1));

        // acc = acc op backing[i]
        f.instruction(&Instruction::LocalGet(scr_acc));
        self.emit_operand(f, &args[0]);
        f.instruction(&Instruction::StructGet {
            struct_type_index: vec_ty,
            field_index: 0,
        });
        f.instruction(&Instruction::LocalGet(scr_i));
        f.instruction(&Instruction::ArrayGet(arr_ty));

        if is_product {
            if is_f64 {
                f.instruction(&Instruction::F64Mul);
            } else if is_i64 {
                f.instruction(&Instruction::I64Mul);
            } else {
                f.instruction(&Instruction::I32Mul);
            }
        } else if is_f64 {
            f.instruction(&Instruction::F64Add);
        } else if is_i64 {
            f.instruction(&Instruction::I64Add);
        } else {
            f.instruction(&Instruction::I32Add);
        }
        f.instruction(&Instruction::LocalSet(scr_acc));

        // i++
        f.instruction(&Instruction::LocalGet(scr_i));
        f.instruction(&Instruction::I32Const(1));
        f.instruction(&Instruction::I32Add);
        f.instruction(&Instruction::LocalSet(scr_i));
        f.instruction(&Instruction::Br(0));

        f.instruction(&Instruction::End); // end loop
        f.instruction(&Instruction::End); // end block

        // push acc
        f.instruction(&Instruction::LocalGet(scr_acc));
    }

    /// Allocate a scratch local for a specific ValType, returning the wasm local index.
    fn scratch_local_for_valtype(&self, vt: ValType) -> u32 {
        match vt {
            ValType::I64 => self.si(6),
            ValType::F64 => self.si(7),
            ValType::Ref(_) => self.si(8),
            _ => self.si(3), // i32 scratch
        }
    }

    /// Emit an operand, promoting i32 constants to i64/f64 when the other
    /// operand in a binary expression is i64/f64.
    fn emit_operand_coerced(
        &mut self,
        f: &mut Function,
        op: &Operand,
        need_i64: bool,
        need_f64: bool,
    ) {
        match op {
            Operand::ConstI32(v) if need_i64 => {
                f.instruction(&Instruction::I64Const(*v as i64));
            }
            Operand::ConstI32(v) if need_f64 => {
                f.instruction(&Instruction::F64Const(*v as f64));
            }
            _ => {
                self.emit_operand(f, op);
            }
        }
    }

    fn emit_binop(&self, f: &mut Function, op: BinOp, lhs_operand: Option<&Operand>) {
        // Determine operand type from LHS (not destination — comparisons return bool/i32)
        let is_f64 = lhs_operand.is_some_and(|o| self.is_f64_like_operand(o));
        let is_i64 = lhs_operand.is_some_and(|o| self.is_i64_like_operand(o));

        if is_f64 {
            match op {
                BinOp::Add => f.instruction(&Instruction::F64Add),
                BinOp::Sub => f.instruction(&Instruction::F64Sub),
                BinOp::Mul => f.instruction(&Instruction::F64Mul),
                BinOp::Div => f.instruction(&Instruction::F64Div),
                BinOp::Eq => f.instruction(&Instruction::F64Eq),
                BinOp::Ne => f.instruction(&Instruction::F64Ne),
                BinOp::Lt => f.instruction(&Instruction::F64Lt),
                BinOp::Le => f.instruction(&Instruction::F64Le),
                BinOp::Gt => f.instruction(&Instruction::F64Gt),
                BinOp::Ge => f.instruction(&Instruction::F64Ge),
                _ => f.instruction(&Instruction::F64Add),
            };
        } else if is_i64 {
            match op {
                BinOp::Add => f.instruction(&Instruction::I64Add),
                BinOp::Sub => f.instruction(&Instruction::I64Sub),
                BinOp::Mul => f.instruction(&Instruction::I64Mul),
                BinOp::Div => f.instruction(&Instruction::I64DivS),
                BinOp::Mod => f.instruction(&Instruction::I64RemS),
                BinOp::Eq => f.instruction(&Instruction::I64Eq),
                BinOp::Ne => f.instruction(&Instruction::I64Ne),
                BinOp::Lt => f.instruction(&Instruction::I64LtS),
                BinOp::Le => f.instruction(&Instruction::I64LeS),
                BinOp::Gt => f.instruction(&Instruction::I64GtS),
                BinOp::Ge => f.instruction(&Instruction::I64GeS),
                BinOp::And => f.instruction(&Instruction::I64And),
                BinOp::Or => f.instruction(&Instruction::I64Or),
                BinOp::BitAnd => f.instruction(&Instruction::I64And),
                BinOp::BitOr => f.instruction(&Instruction::I64Or),
                BinOp::BitXor => f.instruction(&Instruction::I64Xor),
                BinOp::Shl => f.instruction(&Instruction::I64Shl),
                BinOp::Shr => f.instruction(&Instruction::I64ShrS),
            };
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
            .map(|(_, fields)| {
                fields
                    .iter()
                    .map(|t| match t.as_str() {
                        "f64" | "i64" => 8u32,
                        _ => 4u32,
                    })
                    .sum::<u32>()
            })
            .max()
            .unwrap_or(4);
        4 + max_payload.max(4)
    }
}
