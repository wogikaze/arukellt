//! T1 `wasm32-wasi-p1` backend.
//!
//! Generates a WASI Preview 1 compatible Wasm module from MIR.
//! Uses linear memory with arena-only allocation.

mod helpers;
mod operands;
mod sections;
mod stdlib;
mod stmts;

use ark_diagnostics::DiagnosticSink;
use ark_mir::mir::*;
use std::collections::HashSet;
use wasm_encoder::{Function, Instruction, MemArg, TypeSection, ValType};

const IOV_BASE: u32 = 0;
const NWRITTEN: u32 = 8;
const SCRATCH: u32 = 16; // temp for i32_to_string length
const I32BUF: u32 = 48; // buffer for i32_to_string output (20 bytes max)
const STRUCT_BASE: u32 = 96; // scratch area for struct init base pointers (8 slots × 4 bytes = 96..128)
const ENUM_BASE: u32 = 128; // scratch area for enum init base pointers (8 slots × 4 bytes = 128..160)
const FS_SCRATCH: u32 = 160; // scratch for fs operations (opened_fd: 4 bytes)
const FS_NREAD: u32 = 164; // nread result from fd_read (4 bytes)
const STDIN_BUF_POS: u32 = 168; // i32: current read position in stdin buffer
const STDIN_BUF_LEN: u32 = 172; // i32: valid byte count in stdin buffer
const FS_BUF_SIZE: u32 = 4096; // read buffer chunk size for fs_read_file
const STDIN_BUF: u32 = 4096; // 65536-byte stdin buffer (4096..69631)
const STDIN_BUF_SIZE: u32 = 65536;
const HEAP_START: u32 = 69632; // = STDIN_BUF + STDIN_BUF_SIZE
const BOOL_TRUE: u32 = 80; // "true" (4 bytes)
const BOOL_FALSE: u32 = 84; // "false" (5 bytes)
const NEWLINE: u32 = 89; // "\n" (1 byte)
const DATA_START: u32 = 256;

// Function indices:
// 0 = fd_write (import)
// 1 = path_open (import)
// 2 = fd_read (import)
// 3 = fd_close (import)
// 4 = clock_time_get (import)
// 5 = random_get (import)
// 6 = __i32_to_string
// 7 = __print_i32_ln
// 8 = __print_bool_ln
// 9 = __print_str_ln
// 10 = __str_eq
// 11 = __concat
// 12 = __f64_to_str
// 13 = __i64_to_str
// 14 = __map_i32
// 15 = __filter_i32
// 16 = __fold_i32
// 17 = __map_opt_i32
// 18 = __any_i32
// 19 = __find_i32
// 20+ = user functions
const FN_FD_WRITE: u32 = 0;
const FN_PATH_OPEN: u32 = 1;
const FN_FD_READ: u32 = 2;
const FN_FD_CLOSE: u32 = 3;
const FN_CLOCK_TIME_GET: u32 = 4;
const FN_RANDOM_GET: u32 = 5;
const FN_I32_TO_STR: u32 = 6;
const FN_PRINT_I32_LN: u32 = 7;
const FN_PRINT_BOOL_LN: u32 = 8;
const FN_PRINT_STR_LN: u32 = 9;
const FN_STR_EQ: u32 = 10;
const FN_CONCAT: u32 = 11;
const FN_F64_TO_STR: u32 = 12;
const FN_I64_TO_STR: u32 = 13;
const FN_MAP_I32: u32 = 14;
const FN_FILTER_I32: u32 = 15;
const FN_FOLD_I32: u32 = 16;
const FN_MAP_OPT_I32: u32 = 17;
const FN_ANY_I32: u32 = 18;
const FN_FIND_I32: u32 = 19;
const FN_HASHMAP_I32_NEW: u32 = 20;
const FN_HASHMAP_I32_INSERT: u32 = 21;
const FN_HASHMAP_I32_GET: u32 = 22;
const FN_HASHMAP_I32_CONTAINS: u32 = 23;
const FN_HASHMAP_I32_LEN: u32 = 24;
const FN_MAP_I64: u32 = 25;
const FN_FILTER_I64: u32 = 26;
const FN_FOLD_I64: u32 = 27;
const FN_MAP_F64: u32 = 28;
const FN_FILTER_F64: u32 = 29;
const FN_GET_BYTE: u32 = 30; // buffered stdin helper
// WASI args imports (conditional)
const FN_ARGS_SIZES_GET: u32 = 31;
const FN_ARGS_GET: u32 = 32;
// Stdlib args helpers
const FN_ARG_COUNT: u32 = 33;
const FN_ARG_AT: u32 = 34;
const FN_ARGS_VEC: u32 = 35;
const FN_EPRINT_STR_LN: u32 = 36; // stderr (fd=2) string + newline printer
const FN_ENSURE_HEAP: u32 = 37; // grow memory when bump allocator exceeds current pages
const FN_USER_BASE: u32 = 38;

/// Normalize `__intrinsic_*` names to their canonical emit names.
pub(super) fn normalize_intrinsic_name(name: &str) -> &str {
    match name {
        "__intrinsic_println" => "println",
        "__intrinsic_print" => "print",
        "__intrinsic_eprintln" => "eprintln",
        "__intrinsic_string_from" => "String_from",
        "__intrinsic_string_new" => "String_new",
        "__intrinsic_string_eq" => "eq",
        "__intrinsic_concat" => "concat",
        "__intrinsic_string_clone" => "clone",
        "__intrinsic_starts_with" => "starts_with",
        "__intrinsic_ends_with" => "ends_with",
        "__intrinsic_to_lower" => "to_lower",
        "__intrinsic_to_upper" => "to_upper",
        "__intrinsic_string_slice" => "slice",
        "__intrinsic_string_is_empty" => "is_empty",
        "__intrinsic_i32_to_string" => "i32_to_string",
        "__intrinsic_i64_to_string" => "i64_to_string",
        "__intrinsic_f64_to_string" => "f64_to_string",
        "__intrinsic_bool_to_string" => "bool_to_string",
        "__intrinsic_char_to_string" => "char_to_string",
        "__intrinsic_parse_i32" => "parse_i32",
        "__intrinsic_parse_i64" => "parse_i64",
        "__intrinsic_parse_f64" => "parse_f64",
        "__intrinsic_sqrt" => "sqrt",
        "__intrinsic_abs" => "abs",
        "__intrinsic_min" => "min",
        "__intrinsic_max" => "max",
        "__intrinsic_panic" => "panic",
        "__intrinsic_Vec_new_i32" => "Vec_new_i32",
        "__intrinsic_Vec_new_i64" => "Vec_new_i64",
        "__intrinsic_Vec_new_f64" => "Vec_new_f64",
        "__intrinsic_Vec_new_String" => "Vec_new_String",
        "__intrinsic_Vec_new_i32_with_cap" => "Vec_new_i32_with_cap",
        "__intrinsic_Vec_new_i64_with_cap" => "Vec_new_i64_with_cap",
        "__intrinsic_Vec_new_f64_with_cap" => "Vec_new_f64_with_cap",
        "__intrinsic_sort_i32" => "sort_i32",
        "__intrinsic_sort_String" => "sort_String",
        "__intrinsic_map_i32_i32" => "map_i32_i32",
        "__intrinsic_filter_i32" => "filter_i32",
        "__intrinsic_fold_i32_i32" => "fold_i32_i32",
        "__intrinsic_map_option_i32_i32" => "map_option_i32_i32",
        "__intrinsic_any_i32" => "any_i32",
        "__intrinsic_find_i32" => "find_i32",
        "__intrinsic_split" => "split",
        "__intrinsic_join" => "join",
        "__intrinsic_push_char" => "push_char",
        "__intrinsic_read_line" => "read_line",
        "__intrinsic_read_int" => "read_int",
        "__intrinsic_trim" => "trim",
        "__intrinsic_contains" => "contains",
        "__intrinsic_char_at" => "char_at",
        "__intrinsic_substring" => "substring",
        "__intrinsic_replace" => "replace",
        "__intrinsic_fs_read_file" => "fs_read_file",
        "__intrinsic_fs_write_file" => "fs_write_file",
        "__intrinsic_fs_write_bytes" => "fs_write_bytes",
        "__intrinsic_clock_now" => "clock_now",
        "__intrinsic_random_i32" => "random_i32",
        "__intrinsic_sort_i64" => "sort_i64",
        "__intrinsic_sort_f64" => "sort_f64",
        "__intrinsic_map_String_String" => "map_String_String",
        "__intrinsic_filter_String" => "filter_String",
        "__intrinsic_assert" => "assert",
        "__intrinsic_assert_eq" => "assert_eq",
        "__intrinsic_assert_ne" => "assert_ne",
        "__intrinsic_assert_eq_i64" => "assert_eq_i64",
        "__intrinsic_assert_eq_str" => "assert_eq_str",
        "__intrinsic_map_i64_i64" => "map_i64_i64",
        "__intrinsic_filter_i64" => "filter_i64",
        "__intrinsic_fold_i64_i64" => "fold_i64_i64",
        "__intrinsic_map_f64_f64" => "map_f64_f64",
        "__intrinsic_filter_f64" => "filter_f64",
        "__intrinsic_contains_i32" => "contains_i32",
        "__intrinsic_contains_String" => "contains_String",
        "__intrinsic_reverse_i32" => "reverse_i32",
        "__intrinsic_reverse_String" => "reverse_String",
        "__intrinsic_remove_i32" => "remove_i32",
        "__intrinsic_args" => "args",
        "__intrinsic_arg_count" => "arg_count",
        "__intrinsic_arg_at" => "arg_at",
        "__intrinsic_env_var" => "env_var",
        other => other,
    }
}

pub fn emit(mir: &MirModule, _sink: &mut DiagnosticSink) -> Vec<u8> {
    // Build struct layouts and string field tracking from the type table.
    let mut struct_layouts: std::collections::HashMap<String, Vec<(String, String)>> =
        std::collections::HashMap::new();
    let mut struct_string_fields = HashSet::new();
    for (sname, fields) in &mir.type_table.struct_defs {
        for (fname, ftype) in fields {
            if ftype == "String" {
                struct_string_fields.insert((sname.clone(), fname.clone()));
            }
        }
        struct_layouts.insert(sname.clone(), fields.clone());
    }
    // TODO(MIR-01): remove checker fallback — enum_payload_types should come from type_table only
    let mut ctx = EmitCtx {
        string_literals: Vec::new(),
        data_offset: DATA_START,
        fn_names: mir.functions.iter().map(|f| f.name.clone()).collect(),
        loop_depths: Vec::new(),
        struct_init_depth: 0,
        enum_init_depth: 0,
        string_locals: HashSet::new(),
        vec_string_locals: HashSet::new(),
        vec_i64_locals: HashSet::new(),
        vec_f64_locals: HashSet::new(),
        f64_locals: HashSet::new(),
        i64_locals: HashSet::new(),
        bool_locals: HashSet::new(),
        char_locals: HashSet::new(),
        fn_return_types: mir
            .functions
            .iter()
            .map(|f| (f.name.clone(), f.return_ty.clone()))
            .collect(),
        fn_param_types: mir
            .functions
            .iter()
            .map(|f| {
                (
                    f.name.clone(),
                    f.params.iter().map(|p| p.ty.clone()).collect(),
                )
            })
            .collect(),
        struct_layouts,
        struct_string_fields,
        enum_payload_types: mir.type_table.enum_defs.clone(),
        type_registry: std::collections::HashMap::new(),
        next_type_idx: 0,
        local_struct_names: std::collections::HashMap::new(),
        fn_map: Vec::new(),
    };
    ctx.emit_module(mir)
}

struct EmitCtx {
    string_literals: Vec<(u32, Vec<u8>)>,
    data_offset: u32,
    fn_names: Vec<String>,
    /// Stack of extra block depths for break/continue inside loops.
    loop_depths: Vec<u32>,
    /// Nesting depth for struct init (for saving base pointers during nested allocations).
    struct_init_depth: u32,
    /// Nesting depth for enum init (for saving base pointers during nested allocations).
    enum_init_depth: u32,
    /// Locals known to hold string values (for println dispatch).
    string_locals: HashSet<u32>,
    /// Locals known to hold Vec<String> values (for get/get_unchecked dispatch).
    vec_string_locals: HashSet<u32>,
    /// Locals known to hold Vec<i64> values (for push/get element size).
    vec_i64_locals: HashSet<u32>,
    /// Locals known to hold Vec<f64> values (for push/get element size).
    vec_f64_locals: HashSet<u32>,
    /// Locals known to hold f64 values.
    f64_locals: HashSet<u32>,
    /// Locals known to hold i64 values.
    i64_locals: HashSet<u32>,
    /// Locals known to hold bool values.
    bool_locals: HashSet<u32>,
    /// Locals known to hold char values.
    char_locals: HashSet<u32>,
    /// Function return types (for println dispatch on user function calls).
    fn_return_types: std::collections::HashMap<String, ark_typecheck::types::Type>,
    /// Function parameter types (for type-correct argument emission).
    fn_param_types: std::collections::HashMap<String, Vec<ark_typecheck::types::Type>>,
    /// Struct layouts: struct name -> ordered field names
    struct_layouts: std::collections::HashMap<String, Vec<(String, String)>>,
    /// Set of (struct_name, field_name) pairs where field is a String
    struct_string_fields: HashSet<(String, String)>,
    /// Enum variant payload types: enum_name -> [(variant_name, [type_name])]
    enum_payload_types: std::collections::HashMap<String, Vec<(String, Vec<String>)>>,
    /// Dynamic type registry: signature -> type index
    type_registry: std::collections::HashMap<(Vec<ValType>, Vec<ValType>), u32>,
    /// Next available type index
    next_type_idx: u32,
    /// Locals known to hold a specific struct type (for field assignment dispatch).
    local_struct_names: std::collections::HashMap<u32, String>,
    /// Maps canonical function index (FN_*) to actual Wasm function index after DCE.
    fn_map: Vec<u32>,
}

impl EmitCtx {
    /// Map a language type to a Wasm ValType
    pub(super) fn type_to_valtype(ty: &ark_typecheck::types::Type) -> ValType {
        match ty {
            ark_typecheck::types::Type::F64 => ValType::F64,
            ark_typecheck::types::Type::F32 => ValType::F32,
            ark_typecheck::types::Type::I64 => ValType::I64,
            // Everything else (i32, bool, char, String, struct ptr, enum ptr, etc.) is i32
            _ => ValType::I32,
        }
    }

    /// Returns (byte_size, is_f64, is_i64) for a field type string.
    pub(super) fn field_type_info(type_str: &str) -> (u32, bool, bool) {
        match type_str {
            "f64" => (8, true, false),
            "i64" => (8, false, true),
            _ => (4, false, false), // i32, bool, char, String ptr, struct ptr, enum ptr
        }
    }

    /// Compute field offset and type info for a struct
    pub(super) fn struct_field_info(
        &self,
        struct_name: &str,
        field_name: &str,
    ) -> (u32, bool, bool) {
        if let Some(fields) = self.struct_layouts.get(struct_name) {
            let mut offset = 0u32;
            for (fname, ftype) in fields {
                let (size, is_f64, is_i64) = Self::field_type_info(ftype);
                if fname == field_name {
                    return (offset, is_f64, is_i64);
                }
                offset += size;
            }
        }
        (0, false, false)
    }

    /// Total size of a struct in bytes
    pub(super) fn struct_total_size(&self, struct_name: &str) -> u32 {
        if let Some(fields) = self.struct_layouts.get(struct_name) {
            fields
                .iter()
                .map(|(_, ftype)| Self::field_type_info(ftype).0)
                .sum()
        } else {
            0
        }
    }

    /// Get payload offset and type info for an enum variant field
    /// Returns (offset_from_tag, is_f64, is_i64) for a given field index
    pub(super) fn enum_payload_info(
        &self,
        enum_name: &str,
        variant_name: &str,
        field_index: usize,
    ) -> (u32, bool, bool) {
        if let Some(variants) = self.enum_payload_types.get(enum_name)
            && let Some((_, types)) = variants.iter().find(|(vn, _)| vn == variant_name)
        {
            let mut offset = 4u32; // skip tag
            for (i, ftype) in types.iter().enumerate() {
                let (size, is_f64, is_i64) = Self::field_type_info(ftype);
                if i == field_index {
                    return (offset, is_f64, is_i64);
                }
                offset += size;
            }
        }
        // Fallback: assume all i32
        (4 + field_index as u32 * 4, false, false)
    }

    /// Total size of an enum variant (tag + all payloads)
    pub(super) fn enum_variant_total_size(&self, enum_name: &str, variant_name: &str) -> u32 {
        if let Some(variants) = self.enum_payload_types.get(enum_name)
            && let Some((_, types)) = variants.iter().find(|(vn, _)| vn == variant_name)
        {
            let payload_size: u32 = types.iter().map(|t| Self::field_type_info(t).0).sum();
            return 4 + payload_size; // tag + payload
        }
        4 // just tag
    }

    /// Register a function type signature and return its type index.
    pub(super) fn register_type(
        &mut self,
        types: &mut TypeSection,
        params: Vec<ValType>,
        results: Vec<ValType>,
    ) -> u32 {
        let key = (params.clone(), results.clone());
        if let Some(&idx) = self.type_registry.get(&key) {
            return idx;
        }
        let idx = self.next_type_idx;
        types.ty().function(params, results);
        self.type_registry.insert(key, idx);
        self.next_type_idx += 1;
        idx
    }

    /// Look up a type index for call_indirect. Must have been registered during type section build.
    pub(super) fn lookup_or_register_indirect_type(
        &self,
        params: Vec<ValType>,
        results: Vec<ValType>,
    ) -> u32 {
        let key = (params, results);
        self.type_registry.get(&key).copied().unwrap_or(0)
    }

    pub(super) fn alloc_string(&mut self, s: &str) -> (u32, u32) {
        let bytes = s.as_bytes().to_vec();
        let len = bytes.len() as u32;
        let offset = self.data_offset;
        self.string_literals.push((offset, bytes));
        self.data_offset += len;
        (offset, len)
    }

    /// Allocate a length-prefixed string in memory.
    /// Returns the pointer to the data (after the length prefix).
    pub(super) fn alloc_length_prefixed_string(&mut self, s: &str) -> u32 {
        let bytes = s.as_bytes();
        let len = bytes.len() as u32;
        let offset = self.data_offset;
        // Write length as 4 bytes (little-endian)
        self.string_literals
            .push((offset, len.to_le_bytes().to_vec()));
        // Write data
        self.string_literals.push((offset + 4, bytes.to_vec()));
        self.data_offset += 4 + len;
        offset + 4 // pointer to data start
    }

    /// Emit inline panic with a static message: write message to stderr, then unreachable.
    pub(super) fn emit_static_panic(&mut self, f: &mut Function, msg: &str) {
        let ma = MemArg {
            offset: 0,
            align: 2,
            memory_index: 0,
        };
        let msg_ptr = self.alloc_length_prefixed_string(msg);
        let msg_len = msg.len() as i32;
        // Write message to stderr
        f.instruction(&Instruction::I32Const(IOV_BASE as i32));
        f.instruction(&Instruction::I32Const(msg_ptr as i32));
        f.instruction(&Instruction::I32Store(ma));
        f.instruction(&Instruction::I32Const(IOV_BASE as i32 + 4));
        f.instruction(&Instruction::I32Const(msg_len));
        f.instruction(&Instruction::I32Store(ma));
        f.instruction(&Instruction::I32Const(2)); // fd=stderr
        f.instruction(&Instruction::I32Const(IOV_BASE as i32));
        f.instruction(&Instruction::I32Const(1));
        f.instruction(&Instruction::I32Const(NWRITTEN as i32));
        self.call_fn(f, FN_FD_WRITE);
        f.instruction(&Instruction::Drop);
        // Write newline to stderr
        f.instruction(&Instruction::I32Const(IOV_BASE as i32));
        f.instruction(&Instruction::I32Const(NEWLINE as i32));
        f.instruction(&Instruction::I32Store(ma));
        f.instruction(&Instruction::I32Const(IOV_BASE as i32 + 4));
        f.instruction(&Instruction::I32Const(1));
        f.instruction(&Instruction::I32Store(ma));
        f.instruction(&Instruction::I32Const(2)); // fd=stderr
        f.instruction(&Instruction::I32Const(IOV_BASE as i32));
        f.instruction(&Instruction::I32Const(1));
        f.instruction(&Instruction::I32Const(NWRITTEN as i32));
        self.call_fn(f, FN_FD_WRITE);
        f.instruction(&Instruction::Drop);
        f.instruction(&Instruction::Unreachable);
    }

    pub(super) fn call_fn(&self, f: &mut Function, canonical: u32) {
        let idx = self.fn_map[canonical as usize];
        f.instruction(&Instruction::Call(idx));
    }

    pub(super) fn resolve_fn(&self, name: &str) -> Option<u32> {
        self.fn_names
            .iter()
            .position(|n| n == name)
            .map(|i| self.fn_map[FN_USER_BASE as usize + i])
    }

    pub(super) fn emit_bump_alloc(&self, f: &mut Function, size: i32) {
        // ptr = heap_ptr
        f.instruction(&Instruction::GlobalGet(0));
        // heap_ptr += size
        f.instruction(&Instruction::GlobalGet(0));
        f.instruction(&Instruction::I32Const(size));
        f.instruction(&Instruction::I32Add);
        f.instruction(&Instruction::GlobalSet(0));
        self.emit_heap_grow_check(f);
        // stack: [ptr]
    }

    /// Emit a call to __ensure_heap to grow memory if the bump pointer
    /// has reached or exceeded the current memory boundary.
    pub(super) fn emit_heap_grow_check(&self, f: &mut Function) {
        let idx = self.fn_map[FN_ENSURE_HEAP as usize];
        if idx != u32::MAX {
            f.instruction(&Instruction::Call(idx));
        }
    }

    /// Build the __ensure_heap runtime function.
    /// Checks if global 0 (heap_ptr) >= memory.size * 65536, and if so
    /// grows memory by enough pages to accommodate.
    pub(super) fn build_ensure_heap(&self) -> Function {
        let mut f = Function::new(vec![]);
        // if heap_ptr >= memory_size_in_bytes
        f.instruction(&Instruction::GlobalGet(0));
        f.instruction(&Instruction::MemorySize(0));
        f.instruction(&Instruction::I32Const(16));
        f.instruction(&Instruction::I32Shl);
        f.instruction(&Instruction::I32GeU);
        f.instruction(&Instruction::If(wasm_encoder::BlockType::Empty));
        // pages_needed = ((heap_ptr - mem_bytes) >> 16) + 2
        f.instruction(&Instruction::GlobalGet(0));
        f.instruction(&Instruction::MemorySize(0));
        f.instruction(&Instruction::I32Const(16));
        f.instruction(&Instruction::I32Shl);
        f.instruction(&Instruction::I32Sub);
        f.instruction(&Instruction::I32Const(16));
        f.instruction(&Instruction::I32ShrU);
        f.instruction(&Instruction::I32Const(2));
        f.instruction(&Instruction::I32Add);
        f.instruction(&Instruction::MemoryGrow(0));
        f.instruction(&Instruction::Drop);
        f.instruction(&Instruction::End);
        f.instruction(&Instruction::End);
        f
    }

    pub(super) fn build_user_fn(&mut self, func: &MirFunction) -> Function {
        let num_params = func.params.len() as u32;
        let num_locals = func.locals.len() as u32;

        // Populate struct-typed locals from MIR metadata
        self.local_struct_names = func.struct_typed_locals.clone();

        // Build locals list with proper types for non-parameter locals
        let mut locals = Vec::new();
        if num_locals > num_params {
            for local in &func.locals[num_params as usize..] {
                locals.push((1, Self::type_to_valtype(&local.ty)));
            }
        }

        let mut f = Function::new(locals);

        // Identify string and f64 locals from MIR type info and operand scanning
        self.string_locals.clear();
        self.vec_string_locals.clear();
        self.vec_i64_locals.clear();
        self.vec_f64_locals.clear();
        self.f64_locals.clear();
        self.i64_locals.clear();
        self.bool_locals.clear();
        self.char_locals.clear();
        for local in func.params.iter().chain(func.locals.iter()) {
            match &local.ty {
                ark_typecheck::types::Type::String => {
                    self.string_locals.insert(local.id.0);
                }
                ark_typecheck::types::Type::F64 => {
                    self.f64_locals.insert(local.id.0);
                }
                ark_typecheck::types::Type::I64 | ark_typecheck::types::Type::U64 => {
                    self.i64_locals.insert(local.id.0);
                }
                ark_typecheck::types::Type::Bool => {
                    self.bool_locals.insert(local.id.0);
                }
                ark_typecheck::types::Type::Char => {
                    self.char_locals.insert(local.id.0);
                }
                ark_typecheck::types::Type::Vec(inner) => match inner.as_ref() {
                    ark_typecheck::types::Type::String => {
                        self.vec_string_locals.insert(local.id.0);
                    }
                    ark_typecheck::types::Type::I64 => {
                        self.vec_i64_locals.insert(local.id.0);
                    }
                    ark_typecheck::types::Type::F64 => {
                        self.vec_f64_locals.insert(local.id.0);
                    }
                    _ => {}
                },
                _ => {}
            }
        }
        for block in &func.blocks {
            self.scan_string_locals(&block.stmts);
        }

        for block in &func.blocks {
            for stmt in &block.stmts {
                self.emit_stmt(&mut f, stmt);
            }
            if let Terminator::Return(Some(op)) = &block.terminator {
                self.emit_operand(&mut f, op);
            }
        }

        f.instruction(&Instruction::End);
        f
    }

    /// Recursively scan statements to identify string-typed locals.
    pub(super) fn scan_string_locals(&mut self, stmts: &[MirStmt]) {
        for stmt in stmts {
            match stmt {
                MirStmt::Assign(Place::Local(id), Rvalue::Use(op)) => {
                    if self.is_string_operand(op) {
                        self.string_locals.insert(id.0);
                    }
                }
                MirStmt::IfStmt {
                    then_body,
                    else_body,
                    ..
                } => {
                    self.scan_string_locals(then_body);
                    self.scan_string_locals(else_body);
                }
                MirStmt::WhileStmt { body, .. } => {
                    self.scan_string_locals(body);
                }
                _ => {}
            }
        }
    }

    pub(super) fn is_string_operand(&self, op: &Operand) -> bool {
        match op {
            Operand::ConstString(_) => true,
            Operand::Call(name, args) => {
                let name = normalize_intrinsic_name(name.as_str());
                if matches!(
                    name,
                    "String_from"
                        | "concat"
                        | "to_string"
                        | "i32_to_string"
                        | "f64_to_string"
                        | "f32_to_string"
                        | "i64_to_string"
                        | "bool_to_string"
                        | "char_to_string"
                        | "join"
                        | "slice"
                        | "substring"
                        | "trim"
                        | "replace"
                        | "read_line"
                        | "to_lower"
                        | "to_upper"
                        | "clone"
                        | "String_new"
                ) {
                    return true;
                }
                // Display trait impl: TypeName__to_string returns String
                if name.ends_with("__to_string") {
                    return true;
                }
                // get/get_unchecked on Vec<String> returns String
                if matches!(name, "get" | "get_unchecked")
                    && let Some(Operand::Place(Place::Local(id))) = args.first()
                    && self.vec_string_locals.contains(&id.0)
                {
                    return true;
                }
                // Check if function returns String
                if self
                    .fn_return_types
                    .get(name)
                    .is_some_and(|t| matches!(t, ark_typecheck::types::Type::String))
                {
                    return true;
                }
                // If the function has a known return type that is NOT String (and not
                // Option<String>), return false early to prevent the arg-heuristic below
                // from misclassifying it.  E.g. tokenize(String)->Vec<Token> must NOT be
                // treated as a String-returning function just because its argument is String.
                if self.fn_return_types.get(name).is_some_and(|t| {
                    !matches!(
                        t,
                        ark_typecheck::types::Type::String | ark_typecheck::types::Type::Option(_)
                    )
                }) {
                    return false;
                }
                // Check if function returns Option<String>: the result local is used
                // as the object in EnumPayload extraction, so tracking it here lets the
                // EnumPayload fallback (string_locals check) recognise the extracted String.
                if self.fn_return_types.get(name).is_some_and(|t| {
                    matches!(
                        t,
                        ark_typecheck::types::Type::Option(inner)
                        if matches!(inner.as_ref(), ark_typecheck::types::Type::String)
                    )
                }) {
                    return true;
                }
                // Heuristic: if any arg is a string, generic function might return string
                // Exclude functions known to NOT return String
                if matches!(
                    name,
                    "split"
                        | "push"
                        | "set"
                        | "len"
                        | "is_empty"
                        | "starts_with"
                        | "ends_with"
                        | "eq"
                        | "println"
                        | "print"
                        | "eprintln"
                        | "Vec_new_String"
                        | "Vec_new_i32"
                        | "sort_i32"
                        | "sort_String"
                        | "sort_i64"
                        | "sort_f64"
                        | "reverse_i32"
                        | "reverse_String"
                        | "remove_i32"
                        | "contains_i32"
                        | "contains_String"
                        | "parse_i32"
                        | "parse_i64"
                        | "parse_f64"
                        | "fs_read_file"
                        | "fs_write_file"
                        | "fs_write_bytes"
                        | "map_i32_i32"
                        | "filter_i32"
                        | "map_String_String"
                        | "filter_String"
                        | "fold_i32_i32"
                        | "map_option_i32_i32"
                        | "any_i32"
                        | "find_i32"
                        | "HashMap_i32_i32_new"
                        | "HashMap_i32_i32_insert"
                        | "HashMap_i32_i32_get"
                        | "HashMap_i32_i32_contains_key"
                        | "HashMap_i32_i32_len"
                        | "push_char"
                        | "panic"
                        | "assert"
                        | "assert_eq"
                        | "assert_ne"
                        | "assert_eq_str"
                        | "assert_eq_i64"
                        | "clock_now"
                        | "random_i32"
                        | "sqrt"
                        | "abs"
                        | "min"
                        | "max"
                ) || name.starts_with("Vec_new_")
                {
                    return false;
                }
                args.iter().any(|a| self.is_string_operand(a))
            }
            Operand::IfExpr {
                then_result,
                else_result,
                ..
            } => {
                let t = then_result
                    .as_ref()
                    .is_some_and(|r| self.is_string_operand(r));
                let e = else_result
                    .as_ref()
                    .is_some_and(|r| self.is_string_operand(r));
                t || e
            }
            Operand::FieldAccess {
                struct_name, field, ..
            } => self
                .struct_string_fields
                .contains(&(struct_name.clone(), field.clone())),
            Operand::Place(Place::Local(id)) => self.string_locals.contains(&id.0),
            Operand::EnumPayload {
                object,
                enum_name,
                variant_name,
                index,
                ..
            } => {
                // Check if this payload field is a String type
                if let Some(variants) = self.enum_payload_types.get(enum_name.as_str())
                    && let Some((_, types)) = variants.iter().find(|(vn, _)| vn == variant_name)
                    && let Some(t) = types.get(*index as usize)
                    && t == "String"
                {
                    return true;
                }
                // Fallback: check if the variable is known to hold strings
                if let Operand::Place(Place::Local(id)) = object.as_ref() {
                    self.string_locals.contains(&id.0)
                } else {
                    false
                }
            }
            _ => false,
        }
    }
}

pub(super) fn collect_needed_fns(mir: &MirModule) -> std::collections::HashSet<u32> {
    let mut needed = std::collections::HashSet::new();
    needed.insert(FN_FD_WRITE);
    needed.insert(FN_ENSURE_HEAP); // always needed — every allocation calls grow check
    for func in &mir.functions {
        for block in &func.blocks {
            for stmt in &block.stmts {
                cfn_visit_stmt(stmt, func, mir, &mut needed);
            }
            match &block.terminator {
                ark_mir::mir::Terminator::If { cond, .. } => {
                    cfn_visit_operand(cond, func, mir, &mut needed);
                }
                ark_mir::mir::Terminator::Switch { scrutinee, .. } => {
                    cfn_visit_operand(scrutinee, func, mir, &mut needed);
                }
                ark_mir::mir::Terminator::Return(Some(op)) => {
                    cfn_visit_operand(op, func, mir, &mut needed);
                }
                _ => {}
            }
        }
    }
    // Transitive deps: stdlib functions that call other stdlib functions
    cfn_add_transitive_deps(&mut needed);
    needed
}

fn cfn_add_transitive_deps(needed: &mut std::collections::HashSet<u32>) {
    loop {
        let before = needed.len();
        // FN_PRINT_I32_LN calls FN_I32_TO_STR and FN_FD_WRITE
        if needed.contains(&FN_PRINT_I32_LN) {
            needed.insert(FN_I32_TO_STR);
            needed.insert(FN_FD_WRITE);
        }
        // FN_PRINT_BOOL_LN calls FN_FD_WRITE
        if needed.contains(&FN_PRINT_BOOL_LN) {
            needed.insert(FN_FD_WRITE);
        }
        // FN_PRINT_STR_LN calls FN_FD_WRITE
        if needed.contains(&FN_PRINT_STR_LN) {
            needed.insert(FN_FD_WRITE);
        }
        // FN_EPRINT_STR_LN calls FN_FD_WRITE
        if needed.contains(&FN_EPRINT_STR_LN) {
            needed.insert(FN_FD_WRITE);
        }
        // FN_GET_BYTE (buffered stdin) calls FN_FD_READ
        if needed.contains(&FN_GET_BYTE) {
            needed.insert(FN_FD_READ);
        }
        if needed.len() == before {
            break;
        }
    }
}

fn cfn_visit_stmt(
    stmt: &MirStmt,
    func: &MirFunction,
    mir: &MirModule,
    needed: &mut std::collections::HashSet<u32>,
) {
    match stmt {
        MirStmt::CallBuiltin { name, args, .. } => {
            let n = normalize_intrinsic_name(name.as_str());
            cfn_handle_builtin(n, args, func, mir, needed);
            for arg in args {
                cfn_visit_operand(arg, func, mir, needed);
            }
        }
        MirStmt::Call { args, .. } => {
            for arg in args {
                cfn_visit_operand(arg, func, mir, needed);
            }
        }
        MirStmt::Assign(_, rvalue) => {
            cfn_visit_rvalue(rvalue, func, mir, needed);
        }
        MirStmt::IfStmt {
            cond,
            then_body,
            else_body,
        } => {
            cfn_visit_operand(cond, func, mir, needed);
            for s in then_body {
                cfn_visit_stmt(s, func, mir, needed);
            }
            for s in else_body {
                cfn_visit_stmt(s, func, mir, needed);
            }
        }
        MirStmt::WhileStmt { cond, body } => {
            cfn_visit_operand(cond, func, mir, needed);
            for s in body {
                cfn_visit_stmt(s, func, mir, needed);
            }
        }
        MirStmt::Return(Some(op)) => {
            cfn_visit_operand(op, func, mir, needed);
        }
        _ => {}
    }
}

fn cfn_visit_rvalue(
    rvalue: &Rvalue,
    func: &MirFunction,
    mir: &MirModule,
    needed: &mut std::collections::HashSet<u32>,
) {
    match rvalue {
        Rvalue::Use(op) => cfn_visit_operand(op, func, mir, needed),
        Rvalue::BinaryOp(_, l, r) => {
            cfn_visit_operand(l, func, mir, needed);
            cfn_visit_operand(r, func, mir, needed);
        }
        Rvalue::UnaryOp(_, o) => cfn_visit_operand(o, func, mir, needed),
        Rvalue::Aggregate(_, ops) => {
            for op in ops {
                cfn_visit_operand(op, func, mir, needed);
            }
        }
        Rvalue::Ref(_) => {}
    }
}

fn cfn_visit_operand(
    op: &Operand,
    func: &MirFunction,
    mir: &MirModule,
    needed: &mut std::collections::HashSet<u32>,
) {
    match op {
        Operand::Call(name, args) => {
            let n = normalize_intrinsic_name(name.as_str());
            cfn_handle_builtin(n, args, func, mir, needed);
            for arg in args {
                cfn_visit_operand(arg, func, mir, needed);
            }
        }
        Operand::BinOp(_, l, r) => {
            cfn_visit_operand(l, func, mir, needed);
            cfn_visit_operand(r, func, mir, needed);
        }
        Operand::UnaryOp(_, o) => cfn_visit_operand(o, func, mir, needed),
        Operand::IfExpr {
            cond,
            then_body,
            then_result,
            else_body,
            else_result,
        } => {
            cfn_visit_operand(cond, func, mir, needed);
            for s in then_body {
                cfn_visit_stmt(s, func, mir, needed);
            }
            if let Some(r) = then_result {
                cfn_visit_operand(r, func, mir, needed);
            }
            for s in else_body {
                cfn_visit_stmt(s, func, mir, needed);
            }
            if let Some(r) = else_result {
                cfn_visit_operand(r, func, mir, needed);
            }
        }
        Operand::StructInit { fields, .. } => {
            for (_, op) in fields {
                cfn_visit_operand(op, func, mir, needed);
            }
        }
        Operand::FieldAccess { object, .. } => cfn_visit_operand(object, func, mir, needed),
        Operand::EnumInit { payload, .. } => {
            for op in payload {
                cfn_visit_operand(op, func, mir, needed);
            }
        }
        Operand::EnumTag(o) => cfn_visit_operand(o, func, mir, needed),
        Operand::EnumPayload { object, .. } => cfn_visit_operand(object, func, mir, needed),
        Operand::LoopExpr { init, body, result } => {
            cfn_visit_operand(init, func, mir, needed);
            for s in body {
                cfn_visit_stmt(s, func, mir, needed);
            }
            cfn_visit_operand(result, func, mir, needed);
        }
        Operand::TryExpr { expr, .. } => cfn_visit_operand(expr, func, mir, needed),
        _ => {}
    }
}

fn cfn_handle_builtin(
    n: &str,
    args: &[Operand],
    func: &MirFunction,
    mir: &MirModule,
    needed: &mut std::collections::HashSet<u32>,
) {
    match n {
        "println" | "print" => {
            needed.insert(FN_FD_WRITE);
            if let Some(arg) = args.first() {
                cfn_add_needed_for_print(arg, func, mir, needed);
            }
        }
        "eprintln" | "eprint" => {
            needed.insert(FN_FD_WRITE);
            needed.insert(FN_EPRINT_STR_LN);
            if let Some(arg) = args.first() {
                cfn_add_needed_for_eprint(arg, func, needed);
            }
        }
        "print_i32_ln" => {
            needed.insert(FN_FD_WRITE);
            needed.insert(FN_PRINT_I32_LN);
        }
        "print_bool_ln" => {
            needed.insert(FN_FD_WRITE);
            needed.insert(FN_PRINT_BOOL_LN);
        }
        "print_str_ln" => {
            needed.insert(FN_FD_WRITE);
            needed.insert(FN_PRINT_STR_LN);
        }
        "str_eq" | "eq" => {
            needed.insert(FN_STR_EQ);
        }
        "concat" => {
            needed.insert(FN_CONCAT);
        }
        "i32_to_string" | "int_to_string" => {
            needed.insert(FN_I32_TO_STR);
        }
        "f64_to_string" => {
            needed.insert(FN_F64_TO_STR);
        }
        "i64_to_string" => {
            needed.insert(FN_I64_TO_STR);
        }
        "to_string" => {
            // Polymorphic dispatch: conservatively add all numeric to_string helpers
            needed.insert(FN_I32_TO_STR);
            needed.insert(FN_F64_TO_STR);
            needed.insert(FN_I64_TO_STR);
        }
        "read_line" => {
            needed.insert(FN_FD_READ);
        }
        "read_int" => {
            needed.insert(FN_GET_BYTE);
        }
        "clock_now" | "clock_time_get" => {
            needed.insert(FN_CLOCK_TIME_GET);
        }
        "random_i32" | "random_f64" => {
            needed.insert(FN_RANDOM_GET);
        }
        "fs_read_file" => {
            needed.insert(FN_PATH_OPEN);
            needed.insert(FN_FD_READ);
            needed.insert(FN_FD_CLOSE);
        }
        "fs_write_file" | "fs_write_bytes" => {
            needed.insert(FN_PATH_OPEN);
            needed.insert(FN_FD_WRITE);
            needed.insert(FN_FD_CLOSE);
        }
        "map" | "map_i32_i32" | "map_i64_i64" | "map_f64_f64" | "map_String_String" => {
            needed.insert(FN_MAP_I32);
            needed.insert(FN_MAP_I64);
            needed.insert(FN_MAP_F64);
        }
        "filter" | "filter_i32" | "filter_i64" | "filter_f64" | "filter_String" => {
            needed.insert(FN_FILTER_I32);
            needed.insert(FN_FILTER_I64);
            needed.insert(FN_FILTER_F64);
        }
        "fold" | "reduce" | "fold_i32_i32" | "fold_i64_i64" => {
            needed.insert(FN_FOLD_I32);
            needed.insert(FN_FOLD_I64);
        }
        "map_option" | "map_option_i32_i32" => {
            needed.insert(FN_MAP_OPT_I32);
        }
        "any" | "any_i32" => {
            needed.insert(FN_ANY_I32);
        }
        "find" | "find_i32" => {
            needed.insert(FN_FIND_I32);
        }
        "panic" | "assert" | "assert_eq" | "assert_ne" | "assert_eq_str" | "assert_eq_i64" => {
            needed.insert(FN_FD_WRITE);
        }
        "arg_count" => {
            needed.insert(FN_ARGS_SIZES_GET);
            needed.insert(FN_ARG_COUNT);
        }
        "args" => {
            needed.insert(FN_ARGS_SIZES_GET);
            needed.insert(FN_ARGS_GET);
            needed.insert(FN_ARGS_VEC);
        }
        other
            if (other.contains("HashMap") || other.contains("hashmap"))
                && other.ends_with("_new") =>
        {
            needed.insert(FN_HASHMAP_I32_NEW);
        }
        other
            if (other.contains("HashMap") || other.contains("hashmap"))
                && other.contains("_insert") =>
        {
            needed.insert(FN_HASHMAP_I32_INSERT);
        }
        other
            if (other.contains("HashMap") || other.contains("hashmap"))
                && other.contains("_get")
                && !other.contains("_contains") =>
        {
            needed.insert(FN_HASHMAP_I32_GET);
        }
        other
            if (other.contains("HashMap") || other.contains("hashmap"))
                && other.contains("_contains") =>
        {
            needed.insert(FN_HASHMAP_I32_CONTAINS);
        }
        other
            if (other.contains("HashMap") || other.contains("hashmap"))
                && other.contains("_len") =>
        {
            needed.insert(FN_HASHMAP_I32_LEN);
        }
        _ => {}
    }
}

fn cfn_add_needed_for_print(
    arg: &Operand,
    func: &MirFunction,
    mir: &MirModule,
    needed: &mut std::collections::HashSet<u32>,
) {
    match arg {
        Operand::ConstString(_) => {
            // emit_fd_write inline -> only FN_FD_WRITE (already added)
        }
        Operand::ConstBool(_) => {
            needed.insert(FN_PRINT_BOOL_LN);
        }
        Operand::ConstI32(_)
        | Operand::ConstI8(_)
        | Operand::ConstI16(_)
        | Operand::ConstU8(_)
        | Operand::ConstU16(_)
        | Operand::ConstU32(_)
        | Operand::ConstChar(_) => {
            needed.insert(FN_PRINT_I32_LN);
        }
        Operand::ConstF64(_) | Operand::ConstF32(_) => {
            needed.insert(FN_F64_TO_STR);
            needed.insert(FN_PRINT_STR_LN);
        }
        Operand::ConstI64(_) | Operand::ConstU64(_) => {
            needed.insert(FN_I64_TO_STR);
            needed.insert(FN_PRINT_STR_LN);
        }
        Operand::Call(name, _) => {
            let n = normalize_intrinsic_name(name.as_str());
            match n {
                "i32_to_string" | "int_to_string" => {
                    needed.insert(FN_PRINT_I32_LN);
                }
                "bool_to_string" => {
                    needed.insert(FN_PRINT_BOOL_LN);
                }
                "f64_to_string" => {
                    needed.insert(FN_F64_TO_STR);
                    needed.insert(FN_PRINT_STR_LN);
                }
                "i64_to_string" => {
                    needed.insert(FN_I64_TO_STR);
                    needed.insert(FN_PRINT_STR_LN);
                }
                "concat" => {
                    needed.insert(FN_CONCAT);
                    needed.insert(FN_PRINT_STR_LN);
                }
                "String_from" | "String_new" | "char_to_string" | "clone" => {
                    needed.insert(FN_PRINT_STR_LN);
                }
                _ => {
                    let ret_ty = mir
                        .functions
                        .iter()
                        .find(|f| f.name == n)
                        .map(|f| &f.return_ty);
                    match ret_ty {
                        Some(ark_typecheck::types::Type::String) => {
                            needed.insert(FN_PRINT_STR_LN);
                        }
                        Some(ark_typecheck::types::Type::Bool) => {
                            needed.insert(FN_PRINT_BOOL_LN);
                        }
                        Some(ark_typecheck::types::Type::F64)
                        | Some(ark_typecheck::types::Type::F32) => {
                            needed.insert(FN_F64_TO_STR);
                            needed.insert(FN_PRINT_STR_LN);
                        }
                        Some(ark_typecheck::types::Type::I64) => {
                            needed.insert(FN_I64_TO_STR);
                            needed.insert(FN_PRINT_STR_LN);
                        }
                        _ => {
                            needed.insert(FN_PRINT_I32_LN);
                            needed.insert(FN_PRINT_BOOL_LN);
                            needed.insert(FN_PRINT_STR_LN);
                            needed.insert(FN_I32_TO_STR);
                            needed.insert(FN_F64_TO_STR);
                            needed.insert(FN_I64_TO_STR);
                            needed.insert(FN_CONCAT);
                        }
                    }
                }
            }
        }
        Operand::Place(Place::Local(lid)) => {
            let params_and_locals: Vec<_> = func.params.iter().chain(func.locals.iter()).collect();
            if let Some(local) = params_and_locals.iter().find(|l| l.id.0 == lid.0) {
                match &local.ty {
                    ark_typecheck::types::Type::String => {
                        needed.insert(FN_PRINT_STR_LN);
                    }
                    ark_typecheck::types::Type::Bool => {
                        needed.insert(FN_PRINT_BOOL_LN);
                    }
                    ark_typecheck::types::Type::F64 | ark_typecheck::types::Type::F32 => {
                        needed.insert(FN_F64_TO_STR);
                        needed.insert(FN_PRINT_STR_LN);
                    }
                    ark_typecheck::types::Type::I64 | ark_typecheck::types::Type::U64 => {
                        needed.insert(FN_I64_TO_STR);
                        needed.insert(FN_PRINT_STR_LN);
                    }
                    _ => {
                        needed.insert(FN_PRINT_I32_LN);
                    }
                }
            } else {
                needed.insert(FN_PRINT_I32_LN);
                needed.insert(FN_PRINT_BOOL_LN);
                needed.insert(FN_PRINT_STR_LN);
                needed.insert(FN_I32_TO_STR);
                needed.insert(FN_F64_TO_STR);
                needed.insert(FN_I64_TO_STR);
                needed.insert(FN_CONCAT);
            }
        }
        _ => {
            needed.insert(FN_PRINT_I32_LN);
            needed.insert(FN_PRINT_BOOL_LN);
            needed.insert(FN_PRINT_STR_LN);
            needed.insert(FN_I32_TO_STR);
            needed.insert(FN_F64_TO_STR);
            needed.insert(FN_I64_TO_STR);
            needed.insert(FN_CONCAT);
        }
    }
}

fn cfn_add_needed_for_eprint(
    arg: &Operand,
    func: &MirFunction,
    needed: &mut std::collections::HashSet<u32>,
) {
    // Same as cfn_add_needed_for_print but uses FN_EPRINT_STR_LN instead of FN_PRINT_STR_LN
    match arg {
        Operand::ConstString(_) => {}
        Operand::ConstBool(_) => {
            needed.insert(FN_EPRINT_STR_LN);
        }
        Operand::ConstI32(_)
        | Operand::ConstI8(_)
        | Operand::ConstI16(_)
        | Operand::ConstU8(_)
        | Operand::ConstU16(_)
        | Operand::ConstU32(_)
        | Operand::ConstChar(_) => {
            needed.insert(FN_I32_TO_STR);
            needed.insert(FN_EPRINT_STR_LN);
        }
        Operand::ConstF64(_) | Operand::ConstF32(_) => {
            needed.insert(FN_F64_TO_STR);
            needed.insert(FN_EPRINT_STR_LN);
        }
        Operand::ConstI64(_) | Operand::ConstU64(_) => {
            needed.insert(FN_I64_TO_STR);
            needed.insert(FN_EPRINT_STR_LN);
        }
        Operand::Call(name, inner_args) => {
            let n = normalize_intrinsic_name(name.as_str());
            match n {
                "i32_to_string" | "int_to_string" => {
                    needed.insert(FN_I32_TO_STR);
                    needed.insert(FN_EPRINT_STR_LN);
                }
                "bool_to_string" => {
                    needed.insert(FN_EPRINT_STR_LN);
                }
                "f64_to_string" => {
                    needed.insert(FN_F64_TO_STR);
                    needed.insert(FN_EPRINT_STR_LN);
                }
                "i64_to_string" => {
                    needed.insert(FN_I64_TO_STR);
                    needed.insert(FN_EPRINT_STR_LN);
                }
                "concat" => {
                    needed.insert(FN_CONCAT);
                    needed.insert(FN_EPRINT_STR_LN);
                    for a in inner_args {
                        cfn_add_needed_for_eprint(a, func, needed);
                    }
                }
                "String_from" | "String_new" | "char_to_string" | "clone" => {
                    needed.insert(FN_EPRINT_STR_LN);
                }
                _ => {
                    needed.insert(FN_EPRINT_STR_LN);
                    needed.insert(FN_I32_TO_STR);
                    needed.insert(FN_CONCAT);
                }
            }
        }
        Operand::Place(Place::Local(lid)) => {
            let params_and_locals: Vec<_> = func.params.iter().chain(func.locals.iter()).collect();
            if let Some(local) = params_and_locals.iter().find(|l| l.id.0 == lid.0) {
                match &local.ty {
                    ark_typecheck::types::Type::String => {
                        needed.insert(FN_EPRINT_STR_LN);
                    }
                    ark_typecheck::types::Type::F64 | ark_typecheck::types::Type::F32 => {
                        needed.insert(FN_F64_TO_STR);
                        needed.insert(FN_EPRINT_STR_LN);
                    }
                    ark_typecheck::types::Type::I64 | ark_typecheck::types::Type::U64 => {
                        needed.insert(FN_I64_TO_STR);
                        needed.insert(FN_EPRINT_STR_LN);
                    }
                    _ => {
                        needed.insert(FN_I32_TO_STR);
                        needed.insert(FN_EPRINT_STR_LN);
                    }
                }
            } else {
                needed.insert(FN_EPRINT_STR_LN);
                needed.insert(FN_I32_TO_STR);
                needed.insert(FN_CONCAT);
            }
        }
        _ => {
            needed.insert(FN_EPRINT_STR_LN);
            needed.insert(FN_I32_TO_STR);
            needed.insert(FN_CONCAT);
        }
    }
}
