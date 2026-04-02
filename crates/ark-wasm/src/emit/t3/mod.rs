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

pub(super) mod cabi_adapters;
mod const_expr;
mod helpers;
mod layout_opt;
mod operands;
mod peephole;
mod reachability;
mod stdlib;
mod stmts;
mod types;

use ark_diagnostics::DiagnosticSink;
use ark_mir::mir::*;
use ark_typecheck::types::Type;
use std::collections::{HashMap, HashSet};
use wasm_encoder::{
    ArrayType, CodeSection, CompositeInnerType, CompositeType, DataSection, DataSegment,
    ExportKind, ExportSection, FieldType, FunctionSection, GlobalSection, GlobalType, HeapType,
    ImportSection, MemorySection, MemoryType, RefType as WasmRefType, StorageType, StructType,
    SubType, TypeSection, ValType,
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

// HTTP scratch memory layout: input strings at 16KB, response at 32KB
pub(super) const HTTP_SCRATCH_IN: u32 = 16384;
pub(super) const HTTP_SCRATCH_RESP: u32 = 32768;

/// P2 canonical ABI: retptr area (12 bytes, reuses IOV_BASE region)
pub(super) const P2_RETPTR: u32 = 0;

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
    if name == "__intrinsic_replace" {
        return "__intrinsic_replace";
    }
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
            "memory_copy" => "memory_copy",
            "memory_fill" => "memory_fill",
            "args" => "args",
            "arg_count" => "arg_count",
            "arg_at" => "arg_at",
            "env_var" => "env_var",
            "f64_bits_lo" => "f64_bits_lo",
            "f64_bits_hi" => "f64_bits_hi",
            "fs_read_file" => "fs_read_file",
            "fs_write_file" => "fs_write_file",
            "fs_write_bytes" => "fs_write_bytes",
            "http_get" => "http_get",
            "http_request" => "http_request",
            other => other,
        }
    } else {
        name
    }
}

pub(super) fn nominalize_generic_type_name(name: &str) -> Option<String> {
    if !name.contains('<') {
        return None;
    }

    let mut out = String::with_capacity(name.len());
    let mut prev_was_sep = false;
    for ch in name.chars() {
        if ch.is_ascii_alphanumeric() {
            out.push(ch);
            prev_was_sep = false;
        } else if !prev_was_sep {
            out.push('_');
            prev_was_sep = true;
        }
    }
    while out.ends_with('_') {
        out.pop();
    }
    if out.is_empty() { None } else { Some(out) }
}

fn split_generic_type_args(name: &str) -> Option<(String, Vec<String>)> {
    let start = name.find('<')?;
    let end = name.rfind('>')?;
    if end <= start {
        return None;
    }

    let base = name[..start].trim().to_string();
    let mut args = Vec::new();
    let mut depth = 0usize;
    let mut current = String::new();
    for ch in name[start + 1..end].chars() {
        match ch {
            '<' => {
                depth += 1;
                current.push(ch);
            }
            '>' => {
                if depth == 0 {
                    return None;
                }
                depth -= 1;
                current.push(ch);
            }
            ',' if depth == 0 => {
                args.push(current.trim().to_string());
                current.clear();
            }
            _ => current.push(ch),
        }
    }
    if !current.trim().is_empty() {
        args.push(current.trim().to_string());
    }
    Some((base, args))
}

fn is_concrete_specialization_arg(name: &str) -> bool {
    !(name.len() == 1 && name.chars().all(|ch| ch.is_ascii_uppercase()))
}

pub(super) fn is_component_export_candidate(name: &str) -> bool {
    !matches!(
        name,
        "main"
            | "print"
            | "println"
            | "eprintln"
            | "read_to_string"
            | "write_string"
            | "write_bytes"
            | "args"
            | "arg_count"
            | "arg_at"
            | "var"
            | "has_flag"
            | "exit"
            | "abort"
            | "monotonic_now"
            | "duration_ms"
            | "duration_us"
            | "duration_ns"
            | "random_i32"
            | "random_i32_range"
            | "random_bool"
            | "seeded_random"
            | "seeded_range"
            | "shuffle_i32"
            | "panic"
            | "assert"
            | "assert_eq"
            | "assert_ne"
            | "assert_eq_i64"
            | "assert_eq_str"
            | "String_from"
    ) && !name.starts_with("__")
}

pub(super) struct TypeAlloc {
    next_idx: u32,
    names: HashMap<String, u32>,
    func_cache: HashMap<(Vec<ValType>, Vec<ValType>), u32>,
    section: TypeSection,
}

impl TypeAlloc {
    pub(super) fn new() -> Self {
        Self {
            next_idx: 0,
            names: HashMap::new(),
            func_cache: HashMap::new(),
            section: TypeSection::new(),
        }
    }

    pub(super) fn add_func(&mut self, params: &[ValType], results: &[ValType]) -> u32 {
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

    pub(super) fn add_struct(&mut self, name: &str, fields: &[FieldType]) -> u32 {
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

    pub(super) fn add_array(&mut self, name: &str, element: FieldType) -> u32 {
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
    pub(super) fn add_sub_struct_base(&mut self, name: &str) -> u32 {
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
    pub(super) fn add_sub_struct_variant(
        &mut self,
        name: &str,
        super_idx: u32,
        fields: &[FieldType],
    ) -> u32 {
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
    pub(super) fn add_enum_rec_group(
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

pub(super) struct Ctx {
    pub(super) types: TypeAlloc,
    /// Active data segments (loaded into linear memory) for I/O scratch only.
    data_segs: Vec<(u32, Vec<u8>)>,
    data_offset: u32,
    /// Passive data segments for string literals (consumed by array.new_data).
    string_data_segs: Vec<Vec<u8>>,
    /// Deduplication cache: maps string bytes → segment index.
    string_seg_cache: HashMap<Vec<u8>, u32>,
    pub(super) fn_map: HashMap<String, u32>,
    fn_names: Vec<String>,
    next_fn: u32,
    /// Function names that are http.ark wrappers (e.g. "get", "request") which
    /// have real bodies and must be called directly rather than inlined as builtins.
    pub(super) http_wrapper_fns: HashSet<String>,
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
    pub(super) struct_gc_types: HashMap<String, u32>,
    struct_layouts: HashMap<String, Vec<(String, String)>>,
    // Enum GC type indices: subtype hierarchy
    pub(super) enum_base_types: HashMap<String, u32>,
    pub(super) enum_variant_types: HashMap<String, HashMap<String, u32>>,
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
    char_locals: std::collections::HashSet<u32>,
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
    helper_print_i32: Option<u32>,
    helper_print_bool: Option<u32>,
    helper_print_str: Option<u32>,
    helper_print_i32_ln: Option<u32>,
    helper_print_bool_ln: Option<u32>,
    helper_print_str_ln: Option<u32>,
    helper_print_newline: Option<u32>,
    helper_eprint_str_ln: Option<u32>,
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
    current_fn_ret_type_name: Option<String>,
    // P2 WASI import indices (always present)
    wasi_p2_get_stdout: u32,
    wasi_p2_write_and_flush: u32,
    wasi_p2_drop_output_stream: u32,
    // P1 WASI import indices (conditional)
    wasi_fd_write: u32,
    wasi_path_open: u32,
    wasi_fd_read: u32,
    wasi_fd_close: u32,
    wasi_needs_fd_write: bool,
    wasi_needs_fs: bool,
    wasi_clock_time_get: u32,
    wasi_needs_clock: bool,
    wasi_random_get: u32,
    wasi_needs_random: bool,
    wasi_proc_exit: u32,
    wasi_needs_proc_exit: bool,
    wasi_args_sizes_get: u32,
    wasi_args_get: u32,
    wasi_needs_args: bool,
    wasi_environ_sizes_get: u32,
    wasi_environ_get: u32,
    wasi_needs_environ: bool,
    /// Host HTTP import indices (conditional, in arukellt_host namespace)
    host_http_get: u32,
    host_http_request: u32,
    needs_http: bool,
    /// Optimization level (0 = O0, 1 = O1, 2 = O2).
    /// Tail-call emission (`return_call`) is enabled at opt_level >= 1.
    opt_level: u8,
    /// Static string interning: maps string content → global index (opt_level >= 1).
    /// Each unique string literal gets a `(global (mut (ref null $string)))` that
    /// caches the GC array after first initialization.
    string_intern_globals: HashMap<String, u32>,
    /// Number of interned-string globals emitted (used for index allocation).
    string_intern_count: u32,
    /// Struct field reorder map (opt_level >= 2): struct_name → permutation
    /// where perm[old_idx] = new_idx.
    field_remap: HashMap<String, Vec<usize>>,
}

impl Ctx {
    pub(super) fn type_to_val(&self, ty: &Type) -> ValType {
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
            Type::Option(inner) => {
                let key = if matches!(inner.as_ref(), Type::String) {
                    "Option_String"
                } else {
                    "Option"
                };
                if let Some(&base_idx) = self.enum_base_types.get(key) {
                    ref_nullable(base_idx)
                } else if let Some(&base_idx) = self.enum_base_types.get("Option") {
                    ref_nullable(base_idx)
                } else {
                    ValType::I32
                }
            }
            Type::Result(ok, err) => {
                // Result<String, String> → Result_String_String, etc.
                let key = match (ok.as_ref(), err.as_ref()) {
                    (Type::String, Type::String) => "Result_String_String".to_string(),
                    (Type::I64, Type::String) => "Result_i64_String".to_string(),
                    (Type::F64, Type::String) => "Result_f64_String".to_string(),
                    _ => "Result".to_string(),
                };
                if let Some(&base_idx) = self.enum_base_types.get(key.as_str()) {
                    ref_nullable(base_idx)
                } else if let Some(&base_idx) = self.enum_base_types.get("Result") {
                    ref_nullable(base_idx)
                } else {
                    ValType::I32
                }
            }
            _ => ValType::I32,
        }
    }

    /// Resolve a type name (from fn_sigs or struct/enum defs) to a ValType.
    pub(super) fn type_name_to_val(&self, name: &str) -> ValType {
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
                if let Some(specialized_name) = nominalize_generic_type_name(name) {
                    if let Some(&base_idx) = self.enum_base_types.get(specialized_name.as_str()) {
                        return ref_nullable(base_idx);
                    }
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
                if (name.starts_with("Option<") || name == "Option")
                    && let Some(&base_idx) = self.enum_base_types.get("Option")
                {
                    return ref_nullable(base_idx);
                }
                // Result<T, E> → use "Result" base enum type
                if (name.starts_with("Result<") || name == "Result")
                    && let Some(&base_idx) = self.enum_base_types.get("Result")
                {
                    return ref_nullable(base_idx);
                }
                // Tuple type names: "(i32, i32)" etc — map to __tupleN struct
                if name.starts_with('(') && name.ends_with(')') {
                    let inner = &name[1..name.len() - 1];
                    let arity = inner.split(',').count();
                    let tuple_name = format!("__tuple{}", arity);
                    if let Some(&ty_idx) = self.struct_gc_types.get(&tuple_name) {
                        return ref_nullable(ty_idx);
                    }
                }
                ValType::I32
            }
        }
    }

    pub(super) fn result_enum_name_for_type_name(&self, type_name: &str) -> Option<String> {
        if self.enum_base_types.contains_key(type_name) {
            return Some(type_name.to_string());
        }
        let (base, args) = split_generic_type_args(type_name)?;
        if base != "Result" || args.len() != 2 {
            return None;
        }
        if let Some(specialized_name) = nominalize_generic_type_name(type_name)
            && self.enum_base_types.contains_key(specialized_name.as_str())
        {
            return Some(specialized_name);
        }
        if self.enum_base_types.contains_key("Result") {
            Some("Result".to_string())
        } else {
            None
        }
    }

    pub(super) fn current_result_enum_name(&self) -> Option<String> {
        self.current_fn_ret_type_name
            .as_deref()
            .and_then(|type_name| self.result_enum_name_for_type_name(type_name))
    }

    pub(super) fn ensure_specialized_result_enums(&mut self) {
        let mut pending = Vec::new();
        let mut type_names: Vec<String> = self.fn_ret_type_names.values().cloned().collect();
        type_names.extend(
            self.fn_param_type_names
                .values()
                .flat_map(|params| params.iter().cloned()),
        );
        type_names.sort();

        for type_name in type_names {
            let Some((base, args)) = split_generic_type_args(type_name.as_str()) else {
                continue;
            };
            if base != "Result" || args.len() != 2 {
                continue;
            }
            if !args
                .iter()
                .all(|arg| is_concrete_specialization_arg(arg.as_str()))
            {
                continue;
            }
            let Some(enum_name) = nominalize_generic_type_name(type_name.as_str()) else {
                continue;
            };
            if !matches!(
                enum_name.as_str(),
                "Result_i64_String" | "Result_f64_String" | "Result_String_String"
            ) {
                continue;
            }
            if enum_name == "Result" || self.enum_defs.contains_key(enum_name.as_str()) {
                continue;
            }
            pending.push((
                enum_name,
                vec![
                    ("Ok".to_string(), vec![args[0].clone()]),
                    ("Err".to_string(), vec![args[1].clone()]),
                ],
            ));
        }

        for (enum_name, variants) in pending {
            self.enum_defs.insert(enum_name, variants);
        }
    }

    /// Resolve the Wasm ValType for a MIR local, using struct/enum typed-local
    /// side-channel maps to return GC ref types instead of i32.
    #[allow(clippy::type_complexity)]
    pub(super) fn local_val_type(
        &self,
        local: &MirLocal,
        struct_typed_locals: &HashMap<u32, String>,
        enum_typed_locals: &HashMap<u32, String>,
        vec_sets: Option<(&HashSet<u32>, &HashSet<u32>, &HashSet<u32>, &HashSet<u32>)>,
    ) -> ValType {
        // Check struct side-channel first
        if let Some(sname) = struct_typed_locals.get(&local.id.0)
            && let Some(&ty_idx) = self.struct_gc_types.get(sname)
        {
            return ref_nullable(ty_idx);
        }
        // Check enum side-channel
        if let Some(ename) = enum_typed_locals.get(&local.id.0)
            && let Some(&base_idx) = self.enum_base_types.get(ename)
        {
            return ref_nullable(base_idx);
        }
        // Check Vec<Struct> side-channel
        if let Some(sname) = self.struct_vec_locals.get(&local.id.0)
            && let Some(&(_, vec_ty)) = self.custom_vec_types.get(sname.as_str())
        {
            return ref_nullable(vec_ty);
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

    pub(super) fn alloc_data(&mut self, data: &[u8]) -> u32 {
        let offset = self.data_offset;
        self.data_segs.push((offset, data.to_vec()));
        self.data_offset += data.len() as u32;
        // Align to 4 bytes
        while !self.data_offset.is_multiple_of(4) {
            self.data_offset += 1;
        }
        offset
    }

    /// Allocate a passive data segment for a string literal.
    /// Returns the segment index (used by array.new_data).
    /// Deduplicates identical byte sequences.
    pub(super) fn alloc_string_data(&mut self, data: &[u8]) -> u32 {
        if let Some(&idx) = self.string_seg_cache.get(data) {
            return idx;
        }
        let idx = self.string_data_segs.len() as u32;
        self.string_data_segs.push(data.to_vec());
        self.string_seg_cache.insert(data.to_vec(), idx);
        idx
    }

    pub(super) fn field_valtype(&self, ty_name: &str) -> ValType {
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
    pub(super) fn scan_operands_for_vec_struct(
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
                if let Some(sname) = name.strip_prefix("Vec_new_")
                    && struct_defs.contains_key(sname)
                {
                    out.insert(sname.to_string());
                }
                for a in args {
                    self.scan_op_for_vec_struct(a, struct_defs, out);
                }
            }
            _ => {}
        }
    }

    pub(super) fn scan_op_for_vec_struct(
        &self,
        op: &Operand,
        struct_defs: &HashMap<String, Vec<(String, String)>>,
        out: &mut HashSet<String>,
    ) {
        if let Operand::Call(name, _) = op
            && let Some(sname) = name.strip_prefix("Vec_new_")
            && struct_defs.contains_key(sname)
        {
            out.insert(sname.to_string());
        }
    }
}

// ── Public entry point ───────────────────────────────────────────

/// Returns true if the MIR function body wraps `__intrinsic_http_get` or
/// `__intrinsic_http_request` — i.e., it's an HTTP module wrapper that must
/// be fully emitted even when its unqualified name (e.g. "get") collides
/// with a Vec builtin name.
fn func_body_wraps_http_intrinsic(func: &MirFunction) -> bool {
    let is_http_intrinsic = |name: &str| {
        name == "__intrinsic_http_get"
            || name == "__intrinsic_http_request"
            || normalize_intrinsic(name) == "http_get"
            || normalize_intrinsic(name) == "http_request"
    };
    for block in &func.blocks {
        for stmt in &block.stmts {
            if let MirStmt::CallBuiltin { name, .. } = stmt {
                if is_http_intrinsic(name) {
                    return true;
                }
            }
        }
        if let Terminator::Return(Some(Operand::Call(name, _))) = &block.terminator {
            if is_http_intrinsic(name) {
                return true;
            }
        }
    }
    false
}

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
        http_wrapper_fns: mir
            .functions
            .iter()
            .filter(|f| func_body_wraps_http_intrinsic(f))
            .map(|f| f.name.clone())
            .collect(),
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
        char_locals: Default::default(),
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
        helper_print_i32: None,
        helper_print_bool: None,
        helper_print_str: None,
        helper_print_i32_ln: None,
        helper_print_bool_ln: None,
        helper_print_str_ln: None,
        helper_print_newline: None,
        helper_eprint_str_ln: None,
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
        current_fn_ret_type_name: None,
        wasi_p2_get_stdout: 0,
        wasi_p2_write_and_flush: 0,
        wasi_p2_drop_output_stream: 0,
        wasi_fd_write: 0,
        wasi_path_open: 0,
        wasi_fd_read: 0,
        wasi_fd_close: 0,
        wasi_needs_fd_write: false,
        wasi_needs_fs: false,
        wasi_clock_time_get: 0,
        wasi_needs_clock: false,
        wasi_random_get: 0,
        wasi_needs_random: false,
        wasi_proc_exit: 0,
        wasi_needs_proc_exit: false,
        wasi_args_sizes_get: 0,
        wasi_args_get: 0,
        wasi_needs_args: false,
        wasi_environ_sizes_get: 0,
        wasi_environ_get: 0,
        wasi_needs_environ: false,
        host_http_get: 0,
        host_http_request: 0,
        needs_http: false,
        opt_level,
        string_intern_globals: HashMap::new(),
        string_intern_count: 0,
        field_remap: HashMap::new(),
    };
    ctx.emit_module(mir)
}

// ── Module emission ──────────────────────────────────────────────

impl Ctx {
    pub(super) fn emit_module(&mut self, mir: &MirModule) -> Vec<u8> {
        let reachable_user_indices = self.reachable_function_indices(mir);

        // Struct field layout optimization (opt_level >= 2): reorder fields
        // by access frequency so hot fields get lower Wasm GC indices.
        if self.opt_level >= 2 {
            let remap = layout_opt::compute_field_reorder(
                mir,
                &reachable_user_indices,
                &mir.type_table.struct_defs,
            );
            for (sname, perm) in &remap {
                if let Some(layout) = self.struct_layouts.get(sname) {
                    let reordered = layout_opt::reorder_layout(layout, perm);
                    self.struct_layouts.insert(sname.clone(), reordered);
                }
            }
            self.field_remap = remap;
        }

        // Scan MIR to determine which WASI imports are needed
        let needs_fs = Self::mir_uses_fs(mir, &reachable_user_indices);
        self.wasi_needs_fs = needs_fs;
        let needs_clock = Self::mir_uses_clock(mir, &reachable_user_indices);
        self.wasi_needs_clock = needs_clock;
        let needs_random = Self::mir_uses_random(mir, &reachable_user_indices);
        self.wasi_needs_random = needs_random;
        let needs_proc_exit = Self::mir_uses_proc_exit(mir, &reachable_user_indices);
        self.wasi_needs_proc_exit = needs_proc_exit;
        let needs_args = Self::mir_uses_args(mir, &reachable_user_indices);
        self.wasi_needs_args = needs_args;
        let needs_environ = Self::mir_uses_environ(mir, &reachable_user_indices);
        self.wasi_needs_environ = needs_environ;
        let needs_http = Self::mir_uses_http(mir, &reachable_user_indices);
        self.needs_http = needs_http;

        self.ensure_specialized_result_enums();

        // Phase 1: Register GC types
        self.register_gc_types(mir);

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
        // clock_time_get: (i32, i64, i32) -> i32
        let clock_time_get_ty = self
            .types
            .add_func(&[ValType::I32, ValType::I64, ValType::I32], &[ValType::I32]);
        // random_get: (i32, i32) -> i32
        let random_get_ty = self
            .types
            .add_func(&[ValType::I32, ValType::I32], &[ValType::I32]);
        // proc_exit: (i32) -> ()
        let proc_exit_ty = self.types.add_func(&[ValType::I32], &[]);
        // args_sizes_get: (i32, i32) -> i32
        let args_sizes_get_ty = self
            .types
            .add_func(&[ValType::I32, ValType::I32], &[ValType::I32]);
        // args_get: (i32, i32) -> i32
        let args_get_ty = args_sizes_get_ty; // same signature
        // http_get: (url_ptr: i32, url_len: i32, resp_ptr: i32) -> i32
        let http_get_ty = self
            .types
            .add_func(&[ValType::I32, ValType::I32, ValType::I32], &[ValType::I32]);
        // http_request: (method_ptr, method_len, url_ptr, url_len, body_ptr, body_len, resp_ptr) -> i32
        let http_request_ty = self.types.add_func(
            &[
                ValType::I32,
                ValType::I32,
                ValType::I32,
                ValType::I32,
                ValType::I32,
                ValType::I32,
                ValType::I32,
            ],
            &[ValType::I32],
        );

        // Scan MIR to determine which stdlib helpers are actually needed
        // (must happen before import counting so we know if fd_write is needed)
        let needed = Self::scan_needed_helpers(mir, &reachable_user_indices);

        // fd_write is only needed when any print/eprint helper is used
        let needs_fd_write = needed.print_str
            || needed.print_i32
            || needed.print_bool
            || needed.print_str_ln
            || needed.print_i32_ln
            || needed.print_bool_ln
            || needed.print_newline
            || needed.eprint_str_ln;
        self.wasi_needs_fd_write = needs_fd_write;

        // Dynamic WASI import count: fd_write only when print is used; others conditional
        let mut num_imports = 0u32;
        if needs_fd_write {
            self.wasi_fd_write = num_imports;
            num_imports += 1;
        }
        if needs_fs {
            self.wasi_path_open = num_imports;
            num_imports += 1;
            self.wasi_fd_read = num_imports;
            num_imports += 1;
            self.wasi_fd_close = num_imports;
            num_imports += 1;
        }
        if needs_clock {
            self.wasi_clock_time_get = num_imports;
            num_imports += 1;
        }
        if needs_random {
            self.wasi_random_get = num_imports;
            num_imports += 1;
        }
        if needs_proc_exit {
            self.wasi_proc_exit = num_imports;
            num_imports += 1;
        }
        if needs_args {
            self.wasi_args_sizes_get = num_imports;
            num_imports += 1;
            self.wasi_args_get = num_imports;
            num_imports += 1;
        }
        if needs_environ {
            self.wasi_environ_sizes_get = num_imports;
            num_imports += 1;
            self.wasi_environ_get = num_imports;
            num_imports += 1;
        }
        if needs_http {
            self.host_http_get = num_imports;
            num_imports += 1;
            self.host_http_request = num_imports;
            num_imports += 1;
        }

        // GC-native helper function signatures — only include needed ones
        let str_ref = ref_nullable(self.string_ty);
        let mut helper_fns: Vec<(String, Vec<ValType>, Vec<ValType>)> = Vec::new();

        // Track position of each helper in the dynamic list
        let mut p2_get_stdout_pos: Option<usize> = None;
        let mut p2_write_and_flush_pos: Option<usize> = None;
        let mut p2_drop_output_stream_pos: Option<usize> = None;
        let mut print_str_pos: Option<usize> = None;
        let mut print_i32_pos: Option<usize> = None;
        let mut print_bool_pos: Option<usize> = None;
        let mut print_str_ln_pos: Option<usize> = None;
        let mut print_i32_ln_pos: Option<usize> = None;
        let mut print_bool_ln_pos: Option<usize> = None;
        let mut i32_to_str_pos: Option<usize> = None;
        let mut print_newline_pos: Option<usize> = None;
        let mut i64_to_str_pos: Option<usize> = None;
        let mut f64_to_str_pos: Option<usize> = None;
        let mut p2_get_stderr_pos: Option<usize> = None;
        let mut eprint_str_ln_pos: Option<usize> = None;
        let needs_p2_stdio_shims = needed.print_str
            || needed.print_i32
            || needed.print_bool
            || needed.print_str_ln
            || needed.print_i32_ln
            || needed.print_bool_ln
            || needed.print_newline
            || needed.eprint_str_ln;

        if needs_p2_stdio_shims {
            p2_get_stdout_pos = Some(helper_fns.len());
            helper_fns.push(("__wasi_p2_get_stdout".into(), vec![], vec![ValType::I32]));
            p2_write_and_flush_pos = Some(helper_fns.len());
            helper_fns.push((
                "__wasi_p2_write_and_flush".into(),
                vec![ValType::I32, ValType::I32, ValType::I32, ValType::I32],
                vec![],
            ));
            p2_drop_output_stream_pos = Some(helper_fns.len());
            helper_fns.push((
                "__wasi_p2_drop_output_stream".into(),
                vec![ValType::I32],
                vec![],
            ));
        }

        if needed.print_str {
            print_str_pos = Some(helper_fns.len());
            helper_fns.push(("__print_str".into(), vec![str_ref], vec![]));
        }
        if needed.print_i32 {
            print_i32_pos = Some(helper_fns.len());
            helper_fns.push(("__print_i32".into(), vec![ValType::I32], vec![]));
        }
        if needed.print_bool {
            print_bool_pos = Some(helper_fns.len());
            helper_fns.push(("__print_bool".into(), vec![ValType::I32], vec![]));
        }
        // Always include print_str_ln — it's the most basic output primitive
        if needed.print_str_ln || needed.print_i32_ln || needed.print_bool_ln {
            print_str_ln_pos = Some(helper_fns.len());
            helper_fns.push(("__print_str_ln".into(), vec![str_ref], vec![]));
        }
        if needed.print_i32_ln {
            print_i32_ln_pos = Some(helper_fns.len());
            helper_fns.push(("__print_i32_ln".into(), vec![ValType::I32], vec![]));
        }
        if needed.print_bool_ln {
            print_bool_ln_pos = Some(helper_fns.len());
            helper_fns.push(("__print_bool_ln".into(), vec![ValType::I32], vec![]));
        }
        if needed.i32_to_str {
            i32_to_str_pos = Some(helper_fns.len());
            helper_fns.push(("__i32_to_str".into(), vec![ValType::I32], vec![str_ref]));
        }
        if needed.print_newline {
            print_newline_pos = Some(helper_fns.len());
            helper_fns.push(("__print_newline".into(), vec![], vec![]));
        }
        if needed.eprint_str_ln {
            p2_get_stderr_pos = Some(helper_fns.len());
            helper_fns.push(("__wasi_p2_get_stderr".into(), vec![], vec![ValType::I32]));
            eprint_str_ln_pos = Some(helper_fns.len());
            helper_fns.push(("__eprint_str_ln".into(), vec![str_ref], vec![]));
        }
        if needed.i64_to_str {
            i64_to_str_pos = Some(helper_fns.len());
            helper_fns.push(("__i64_to_str".into(), vec![ValType::I64], vec![str_ref]));
        }
        if needed.f64_to_str {
            f64_to_str_pos = Some(helper_fns.len());
            helper_fns.push(("__f64_to_str".into(), vec![ValType::F64], vec![str_ref]));
        }

        // Pre-allocate error strings only when parse helpers are needed
        if needed.parse_i32 || needed.parse_i64 {
            let seg = self.alloc_string_data(b"parse error: invalid integer");
            self.err_string_seg = Some(seg);
        }
        if needed.parse_f64 {
            let seg_f = self.alloc_string_data(b"parse error: invalid float");
            self.err_float_string_seg = Some(seg_f);
        }

        // Conditionally add parse helpers if needed AND the relevant Result enum types exist
        let parse_i32_helper_idx =
            if needed.parse_i32 && self.enum_base_types.contains_key("Result") {
                let result_ref = ref_nullable(*self.enum_base_types.get("Result").unwrap());
                let idx = helper_fns.len();
                helper_fns.push(("__parse_i32".into(), vec![str_ref], vec![result_ref]));
                Some(idx)
            } else {
                None
            };
        let parse_i64_helper_idx = if needed.parse_i64
            && self.enum_base_types.contains_key("Result_i64_String")
        {
            let result_ref = ref_nullable(*self.enum_base_types.get("Result_i64_String").unwrap());
            let idx = helper_fns.len();
            helper_fns.push(("__parse_i64".into(), vec![str_ref], vec![result_ref]));
            Some(idx)
        } else {
            None
        };
        let parse_f64_helper_idx = if needed.parse_f64
            && self.enum_base_types.contains_key("Result_f64_String")
        {
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
        // Set helper indices based on dynamic positions
        self.wasi_p2_get_stdout = p2_get_stdout_pos.map_or(0, |p| helper_base + p as u32);
        self.wasi_p2_write_and_flush = p2_write_and_flush_pos.map_or(0, |p| helper_base + p as u32);
        self.wasi_p2_drop_output_stream =
            p2_drop_output_stream_pos.map_or(0, |p| helper_base + p as u32);
        self.helper_print_str = print_str_pos.map(|p| helper_base + p as u32);
        self.helper_print_i32 = print_i32_pos.map(|p| helper_base + p as u32);
        self.helper_print_bool = print_bool_pos.map(|p| helper_base + p as u32);
        self.helper_print_str_ln = print_str_ln_pos.map(|p| helper_base + p as u32);
        self.helper_print_i32_ln = print_i32_ln_pos.map(|p| helper_base + p as u32);
        self.helper_print_bool_ln = print_bool_ln_pos.map(|p| helper_base + p as u32);
        self.helper_i32_to_str = i32_to_str_pos.map(|p| helper_base + p as u32);
        self.helper_print_newline = print_newline_pos.map(|p| helper_base + p as u32);
        self.helper_eprint_str_ln = eprint_str_ln_pos.map(|p| helper_base + p as u32);
        let wasi_p2_get_stderr = p2_get_stderr_pos.map_or(0, |p| helper_base + p as u32);
        self.helper_i64_to_str = i64_to_str_pos.map(|p| helper_base + p as u32);
        self.helper_f64_to_str = f64_to_str_pos.map(|p| helper_base + p as u32);
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

        // Compute canonical ABI adapters for component-exported functions
        // that use GC reference types (enums, structs).
        let adapter_base = user_base + reachable_user_indices.len() as u32;
        let cabi_adapters = self.compute_cabi_adapters(mir, adapter_base);

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
        if needs_fd_write {
            imports.import(
                "wasi_snapshot_preview1",
                "fd_write",
                wasm_encoder::EntityType::Function(fd_write_ty),
            );
        }
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
        if needs_clock {
            imports.import(
                "wasi_snapshot_preview1",
                "clock_time_get",
                wasm_encoder::EntityType::Function(clock_time_get_ty),
            );
        }
        if needs_random {
            imports.import(
                "wasi_snapshot_preview1",
                "random_get",
                wasm_encoder::EntityType::Function(random_get_ty),
            );
        }
        if needs_proc_exit {
            imports.import(
                "wasi_snapshot_preview1",
                "proc_exit",
                wasm_encoder::EntityType::Function(proc_exit_ty),
            );
        }
        if needs_args {
            imports.import(
                "wasi_snapshot_preview1",
                "args_sizes_get",
                wasm_encoder::EntityType::Function(args_sizes_get_ty),
            );
            imports.import(
                "wasi_snapshot_preview1",
                "args_get",
                wasm_encoder::EntityType::Function(args_get_ty),
            );
        }
        if needs_environ {
            imports.import(
                "wasi_snapshot_preview1",
                "environ_sizes_get",
                wasm_encoder::EntityType::Function(args_sizes_get_ty),
            );
            imports.import(
                "wasi_snapshot_preview1",
                "environ_get",
                wasm_encoder::EntityType::Function(args_get_ty),
            );
        }
        if needs_http {
            imports.import(
                "arukellt_host",
                "http_get",
                wasm_encoder::EntityType::Function(http_get_ty),
            );
            imports.import(
                "arukellt_host",
                "http_request",
                wasm_encoder::EntityType::Function(http_request_ty),
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
        // Add canonical ABI adapter functions
        for adapter in &cabi_adapters {
            functions.function(adapter.adapter_type_idx);
        }

        // Add cabi_realloc if any string or list adapter exists (required by canonical ABI)
        let has_realloc_adapter = cabi_adapters.iter().any(|a| {
            a.param_adaptations.iter().any(|p| {
                matches!(
                    p,
                    cabi_adapters::ParamAdaptation::String { .. }
                        | cabi_adapters::ParamAdaptation::List { .. }
                )
            }) || matches!(
                &a.return_adaptation,
                Some(cabi_adapters::ReturnAdaptation::String { .. })
                    | Some(cabi_adapters::ReturnAdaptation::List { .. })
            )
        });
        let cabi_realloc_idx = if has_realloc_adapter {
            let realloc_type = self.types.add_func(
                &[ValType::I32, ValType::I32, ValType::I32, ValType::I32],
                &[ValType::I32],
            );
            let idx = adapter_base + cabi_adapters.len() as u32;
            functions.function(realloc_type);
            Some(idx)
        } else {
            None
        };

        // Linear memory: kept at 4-10 pages for WASI fd_write/fd_read I/O buffer.
        // GC-native codegen is complete; linear memory is only used for I/O syscalls.
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
        // If a canonical ABI adapter exists for a function, export the adapter instead.
        let adapter_export_map: std::collections::HashMap<String, u32> = cabi_adapters
            .iter()
            .map(|a| (a.export_name.clone(), a.adapter_fn_idx))
            .collect();
        for func in &mir.functions {
            if func.is_exported
                && is_component_export_candidate(&func.name)
                && let Some(&idx) = self.fn_map.get(func.name.as_str())
            {
                let export_name = func.name.replace('_', "-");
                let export_idx = adapter_export_map.get(&export_name).copied().unwrap_or(idx);
                exports.export(&export_name, ExportKind::Func, export_idx);
            }
        }

        // Export cabi_realloc for canonical ABI string/list support
        if let Some(realloc_idx) = cabi_realloc_idx {
            exports.export("cabi_realloc", ExportKind::Func, realloc_idx);
        }

        // Data section: static string literals and constants
        // Pre-allocate "true", "false", "\n" for print helpers
        let true_offset = self.alloc_data(b"true");
        let false_offset = self.alloc_data(b"false");
        let newline_offset = self.alloc_data(b"\n");

        // Code section: emit helper + user functions
        let mut codes = CodeSection::new();

        // Emit only the helpers that were registered (in order of registration)
        if p2_get_stdout_pos.is_some() {
            self.emit_wasi_p2_get_stdout_shim(&mut codes);
        }
        if p2_write_and_flush_pos.is_some() {
            self.emit_wasi_p2_write_and_flush_shim(&mut codes);
        }
        if p2_drop_output_stream_pos.is_some() {
            self.emit_wasi_p2_drop_output_stream_shim(&mut codes);
        }
        if print_str_pos.is_some() {
            self.emit_print_str_helper(&mut codes);
        }
        if print_i32_pos.is_some() {
            self.emit_print_i32_helper(&mut codes);
        }
        if print_bool_pos.is_some() {
            self.emit_print_bool_helper(&mut codes, true_offset, false_offset);
        }
        if print_str_ln_pos.is_some() {
            self.emit_print_str_ln_helper(&mut codes, newline_offset);
        }
        if print_i32_ln_pos.is_some() {
            self.emit_print_i32_ln_helper(&mut codes, newline_offset);
        }
        if print_bool_ln_pos.is_some() {
            self.emit_print_bool_ln_helper(&mut codes, true_offset, false_offset, newline_offset);
        }
        if i32_to_str_pos.is_some() {
            self.emit_i32_to_str_helper(&mut codes);
        }
        if print_newline_pos.is_some() {
            self.emit_print_newline_helper(&mut codes, newline_offset);
        }
        if eprint_str_ln_pos.is_some() {
            self.emit_wasi_p2_get_stderr_shim(&mut codes);
            self.emit_eprint_str_ln_helper(&mut codes, newline_offset, wasi_p2_get_stderr);
        }
        if i64_to_str_pos.is_some() {
            self.emit_i64_to_str_helper(&mut codes);
        }
        if f64_to_str_pos.is_some() {
            self.emit_f64_to_str_helper(&mut codes);
        }
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
            let lookup = func.name.rsplit("::").next().unwrap_or(&func.name);
            if (self.is_builtin_name(canonical) || self.is_builtin_name(lookup))
                && !self.http_wrapper_fns.contains(&func.name)
            {
                // Builtin functions are inlined at call sites — emit a stub body
                // that just returns a default value to satisfy validation.
                self.emit_builtin_stub(&mut codes, func);
            } else {
                self.emit_function(&mut codes, func);
            }
        }

        // Emit canonical ABI adapter function bodies
        for adapter in &cabi_adapters {
            self.emit_cabi_adapter_code(&mut codes, adapter);
        }

        // Emit cabi_realloc body: bump allocator
        // cabi_realloc(old_ptr, old_size, align, new_size) -> new_ptr
        // Simply returns global 0 (heap_ptr) and advances it by new_size.
        if cabi_realloc_idx.is_some() {
            let mut f = wasm_encoder::Function::new(vec![]);
            f.instruction(&wasm_encoder::Instruction::GlobalGet(0));
            f.instruction(&wasm_encoder::Instruction::GlobalGet(0));
            f.instruction(&wasm_encoder::Instruction::LocalGet(3));
            f.instruction(&wasm_encoder::Instruction::I32Add);
            f.instruction(&wasm_encoder::Instruction::GlobalSet(0));
            f.instruction(&wasm_encoder::Instruction::End);
            codes.function(&f);
        }

        // Global: heap_ptr for legacy I/O buffer allocation and VecLiteral fallback.
        // Retained for backward compatibility with call_indirect-based HOF dispatch.
        // At opt_level >= 2, uses extended const: (i32.add (i32.const DATA_START) (i32.const size))
        let mut globals = GlobalSection::new();
        let heap_init = const_expr::heap_ptr_init(DATA_START, self.data_offset, self.opt_level);
        globals.global(
            GlobalType {
                val_type: ValType::I32,
                mutable: true,
                shared: false,
            },
            &heap_init,
        );

        // Interned string globals: (global (mut (ref null $string)) (ref.null $string))
        for _ in 0..self.string_intern_count {
            globals.global(
                GlobalType {
                    val_type: ref_nullable(self.string_ty),
                    mutable: true,
                    shared: false,
                },
                &wasm_encoder::ConstExpr::ref_null(HeapType::Concrete(self.string_ty)),
            );
        }

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
        let realloc_count = if cabi_realloc_idx.is_some() { 1u32 } else { 0 };
        let total_funcs = num_imports
            + helper_fns.len() as u32
            + reachable_user_indices.len() as u32
            + cabi_adapters.len() as u32
            + realloc_count;
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
        if needs_fd_write {
            func_names.append(self.wasi_fd_write, "wasi:fd_write");
        }
        if needs_fs {
            func_names.append(self.wasi_path_open, "wasi:path_open");
            func_names.append(self.wasi_fd_read, "wasi:fd_read");
            func_names.append(self.wasi_fd_close, "wasi:fd_close");
        }
        if needs_clock {
            func_names.append(self.wasi_clock_time_get, "wasi:clock_time_get");
        }
        if needs_random {
            func_names.append(self.wasi_random_get, "wasi:random_get");
        }
        if needs_proc_exit {
            func_names.append(self.wasi_proc_exit, "wasi:proc_exit");
        }
        if needs_args {
            func_names.append(self.wasi_args_sizes_get, "wasi:args_sizes_get");
            func_names.append(self.wasi_args_get, "wasi:args_get");
        }
        if needs_environ {
            func_names.append(self.wasi_environ_sizes_get, "wasi:environ_sizes_get");
            func_names.append(self.wasi_environ_get, "wasi:environ_get");
        }
        if needs_http {
            func_names.append(self.host_http_get, "arukellt_host:http_get");
            func_names.append(self.host_http_request, "arukellt_host:http_request");
        }
        // Helper function names (sorted by index for NameMap)
        let mut helpers: Vec<(u32, &str)> = self
            .fn_map
            .iter()
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
}
