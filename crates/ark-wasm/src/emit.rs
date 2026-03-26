//! Wasm module emission using wasm-encoder.
//!
//! Generates a WASI Preview 1 compatible Wasm module from MIR.
//! Uses linear memory. Direct recursive emission (no planning phase).

use ark_diagnostics::DiagnosticSink;
use ark_mir::mir::*;
use std::collections::HashSet;
use wasm_encoder::{
    CodeSection, DataSection, ExportKind, ExportSection, Function, FunctionSection, ImportSection,
    Instruction, MemArg, MemorySection, MemoryType, Module, TypeSection, ValType,
};

const IOV_BASE: u32 = 0;
const NWRITTEN: u32 = 8;
const SCRATCH: u32 = 16; // temp for i32_to_string length
const I32BUF: u32 = 48; // buffer for i32_to_string output (20 bytes max)
const STRUCT_BASE: u32 = 96; // scratch area for struct init base pointers (8 slots × 4 bytes = 96..128)
const ENUM_BASE: u32 = 128; // scratch area for enum init base pointers (8 slots × 4 bytes = 128..160)
const FS_SCRATCH: u32 = 160; // scratch for fs operations (opened_fd: 4 bytes)
const FS_NREAD: u32 = 164; // nread result from fd_read (4 bytes)
const FS_BUF_SIZE: u32 = 4096; // read buffer chunk size
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
const FN_USER_BASE: u32 = 20;

/// Normalize `__intrinsic_*` names to their canonical emit names.
fn normalize_intrinsic_name(name: &str) -> &str {
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
        "__intrinsic_sqrt" => "sqrt",
        "__intrinsic_abs" => "abs",
        "__intrinsic_min" => "min",
        "__intrinsic_max" => "max",
        "__intrinsic_panic" => "panic",
        "__intrinsic_Vec_new_i32" => "Vec_new_i32",
        "__intrinsic_Vec_new_i64" => "Vec_new_i64",
        "__intrinsic_Vec_new_f64" => "Vec_new_f64",
        "__intrinsic_Vec_new_String" => "Vec_new_String",
        "__intrinsic_sort_i32" => "sort_i32",
        "__intrinsic_sort_String" => "sort_String",
        "__intrinsic_map_i32_i32" => "map_i32_i32",
        "__intrinsic_filter_i32" => "filter_i32",
        "__intrinsic_fold_i32_i32" => "fold_i32_i32",
        "__intrinsic_map_option_i32_i32" => "map_option_i32_i32",
        "__intrinsic_any_i32" => "any_i32",
        "__intrinsic_find_i32" => "find_i32",
        other => other,
    }
}

pub fn emit(mir: &MirModule, _sink: &mut DiagnosticSink) -> Vec<u8> {
    // Build struct layouts and string field tracking
    let mut struct_layouts: std::collections::HashMap<String, Vec<(String, String)>> =
        std::collections::HashMap::new();
    let mut struct_string_fields = HashSet::new();
    for (sname, fields) in &mir.struct_defs {
        for (fname, ftype) in fields {
            if ftype == "String" {
                struct_string_fields.insert((sname.clone(), fname.clone()));
            }
        }
        struct_layouts.insert(sname.clone(), fields.clone());
    }
    let mut ctx = EmitCtx {
        string_literals: Vec::new(),
        data_offset: DATA_START,
        fn_names: mir.functions.iter().map(|f| f.name.clone()).collect(),
        loop_depths: Vec::new(),
        struct_init_depth: 0,
        enum_init_depth: 0,
        string_locals: HashSet::new(),
        vec_string_locals: HashSet::new(),
        f64_locals: HashSet::new(),
        i64_locals: HashSet::new(),
        bool_locals: HashSet::new(),
        fn_return_types: mir
            .functions
            .iter()
            .map(|f| (f.name.clone(), f.return_ty.clone()))
            .collect(),
        struct_layouts,
        struct_string_fields,
        enum_payload_types: mir.enum_defs.clone(),
        type_registry: std::collections::HashMap::new(),
        next_type_idx: 0,
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
    /// Locals known to hold f64 values.
    f64_locals: HashSet<u32>,
    /// Locals known to hold i64 values.
    i64_locals: HashSet<u32>,
    /// Locals known to hold bool values.
    bool_locals: HashSet<u32>,
    /// Function return types (for println dispatch on user function calls).
    fn_return_types: std::collections::HashMap<String, ark_typecheck::types::Type>,
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
}

impl EmitCtx {
    /// Map a language type to a Wasm ValType
    fn type_to_valtype(ty: &ark_typecheck::types::Type) -> ValType {
        match ty {
            ark_typecheck::types::Type::F64 => ValType::F64,
            ark_typecheck::types::Type::F32 => ValType::F32,
            ark_typecheck::types::Type::I64 => ValType::I64,
            // Everything else (i32, bool, char, String, struct ptr, enum ptr, etc.) is i32
            _ => ValType::I32,
        }
    }

    /// Returns byte size and whether a field type is f64 (needs F64Store/F64Load)
    fn field_type_info(type_str: &str) -> (u32, bool) {
        match type_str {
            "f64" => (8, true),
            _ => (4, false), // i32, bool, char, String ptr, struct ptr, enum ptr
        }
    }

    /// Compute field offset and type info for a struct
    fn struct_field_info(&self, struct_name: &str, field_name: &str) -> (u32, bool) {
        if let Some(fields) = self.struct_layouts.get(struct_name) {
            let mut offset = 0u32;
            for (fname, ftype) in fields {
                let (size, is_f64) = Self::field_type_info(ftype);
                if fname == field_name {
                    return (offset, is_f64);
                }
                offset += size;
            }
        }
        (0, false)
    }

    /// Total size of a struct in bytes
    fn struct_total_size(&self, struct_name: &str) -> u32 {
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
    /// Returns (offset_from_tag, is_f64) for a given field index
    fn enum_payload_info(
        &self,
        enum_name: &str,
        variant_name: &str,
        field_index: usize,
    ) -> (u32, bool) {
        if let Some(variants) = self.enum_payload_types.get(enum_name) {
            if let Some((_, types)) = variants.iter().find(|(vn, _)| vn == variant_name) {
                let mut offset = 4u32; // skip tag
                for (i, ftype) in types.iter().enumerate() {
                    let (size, is_f64) = Self::field_type_info(ftype);
                    if i == field_index {
                        return (offset, is_f64);
                    }
                    offset += size;
                }
            }
        }
        // Fallback: assume all i32
        (4 + field_index as u32 * 4, false)
    }

    /// Total size of an enum variant (tag + all payloads)
    fn enum_variant_total_size(&self, enum_name: &str, variant_name: &str) -> u32 {
        if let Some(variants) = self.enum_payload_types.get(enum_name) {
            if let Some((_, types)) = variants.iter().find(|(vn, _)| vn == variant_name) {
                let payload_size: u32 = types.iter().map(|t| Self::field_type_info(t).0).sum();
                return 4 + payload_size; // tag + payload
            }
        }
        4 // just tag
    }

    /// Register a function type signature and return its type index.
    fn register_type(
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
    fn lookup_or_register_indirect_type(&self, params: Vec<ValType>, results: Vec<ValType>) -> u32 {
        let key = (params, results);
        self.type_registry.get(&key).copied().unwrap_or(0)
    }

    fn alloc_string(&mut self, s: &str) -> (u32, u32) {
        let bytes = s.as_bytes().to_vec();
        let len = bytes.len() as u32;
        let offset = self.data_offset;
        self.string_literals.push((offset, bytes));
        self.data_offset += len;
        (offset, len)
    }

    /// Allocate a length-prefixed string in memory.
    /// Returns the pointer to the data (after the length prefix).
    fn alloc_length_prefixed_string(&mut self, s: &str) -> u32 {
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
    fn emit_static_panic(&mut self, f: &mut Function, msg: &str) {
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
        f.instruction(&Instruction::Call(FN_FD_WRITE));
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
        f.instruction(&Instruction::Call(FN_FD_WRITE));
        f.instruction(&Instruction::Drop);
        f.instruction(&Instruction::Unreachable);
    }

    fn resolve_fn(&self, name: &str) -> Option<u32> {
        self.fn_names
            .iter()
            .position(|n| n == name)
            .map(|i| FN_USER_BASE + i as u32)
    }

    fn emit_module(&mut self, mir: &MirModule) -> Vec<u8> {
        let mut module = Module::new();

        // Type section — register types dynamically
        let mut types = TypeSection::new();
        // Pre-register known helper types
        let ty_fd_write = self.register_type(&mut types, vec![ValType::I32; 4], vec![ValType::I32]);
        let ty_void_void = self.register_type(&mut types, vec![], vec![]);
        let _ty_void_i32 = self.register_type(&mut types, vec![], vec![ValType::I32]);
        let ty_i32_void = self.register_type(&mut types, vec![ValType::I32], vec![]);
        let ty_i32_i32_i32 = self.register_type(
            &mut types,
            vec![ValType::I32, ValType::I32],
            vec![ValType::I32],
        );
        let ty_i32_i32 = self.register_type(&mut types, vec![ValType::I32], vec![ValType::I32]);
        // Register f64-related types
        let ty_f64_i32 = self.register_type(&mut types, vec![ValType::F64], vec![ValType::I32]);
        // Register i64-related types
        let ty_i64_i32 = self.register_type(&mut types, vec![ValType::I64], vec![ValType::I32]);
        // Register HOF helper types
        let ty_i32x3_i32 =
            self.register_type(&mut types, vec![ValType::I32; 3], vec![ValType::I32]);
        // Register WASI path_open type: (i32,i32,i32,i32,i32,i64,i64,i32,i32) -> i32
        let ty_path_open = self.register_type(
            &mut types,
            vec![
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
            vec![ValType::I32],
        );
        // Register WASI clock_time_get type: (i32, i64, i32) -> i32
        let ty_clock_time_get = self.register_type(
            &mut types,
            vec![ValType::I32, ValType::I64, ValType::I32],
            vec![ValType::I32],
        );
        // Pre-register user function types
        let mut user_func_type_indices = Vec::new();
        for func in &mir.functions {
            let params: Vec<ValType> = func
                .params
                .iter()
                .map(|p| Self::type_to_valtype(&p.ty))
                .collect();
            let results: Vec<ValType> =
                if matches!(func.return_ty, ark_typecheck::types::Type::Unit) {
                    vec![]
                } else {
                    vec![Self::type_to_valtype(&func.return_ty)]
                };
            let idx = self.register_type(&mut types, params, results);
            user_func_type_indices.push(idx);
        }
        module.section(&types);

        // Import section
        let mut imports = ImportSection::new();
        imports.import(
            "wasi_snapshot_preview1",
            "fd_write",
            wasm_encoder::EntityType::Function(ty_fd_write),
        );
        imports.import(
            "wasi_snapshot_preview1",
            "path_open",
            wasm_encoder::EntityType::Function(ty_path_open),
        );
        imports.import(
            "wasi_snapshot_preview1",
            "fd_read",
            wasm_encoder::EntityType::Function(ty_fd_write), // same signature: (i32,i32,i32,i32)->i32
        );
        imports.import(
            "wasi_snapshot_preview1",
            "fd_close",
            wasm_encoder::EntityType::Function(ty_i32_i32), // (i32)->i32
        );
        imports.import(
            "wasi_snapshot_preview1",
            "clock_time_get",
            wasm_encoder::EntityType::Function(ty_clock_time_get), // (i32,i64,i32)->i32
        );
        imports.import(
            "wasi_snapshot_preview1",
            "random_get",
            wasm_encoder::EntityType::Function(ty_i32_i32_i32), // (i32,i32)->i32
        );
        module.section(&imports);

        // Function section (declare types for helpers + user funcs)
        let mut functions = FunctionSection::new();
        functions.function(ty_i32_void); // __i32_to_string: (i32)->()
        functions.function(ty_i32_void); // __print_i32_ln: (i32)->()
        functions.function(ty_i32_void); // __print_bool_ln: (i32)->()
        functions.function(ty_i32_void); // __print_str_ln: (i32)->()
        functions.function(ty_i32_i32_i32); // __str_eq: (i32,i32)->i32
        functions.function(ty_i32_i32_i32); // __concat: (i32,i32)->i32
        functions.function(ty_f64_i32); // __f64_to_str: (f64)->i32
        functions.function(ty_i64_i32); // __i64_to_str: (i64)->i32
        functions.function(ty_i32_i32_i32); // __map_i32: (vec,fn)->vec
        functions.function(ty_i32_i32_i32); // __filter_i32: (vec,fn)->vec
        functions.function(ty_i32x3_i32); // __fold_i32: (vec,init,fn)->i32
        functions.function(ty_i32_i32_i32); // __map_opt_i32: (opt,fn)->opt
        functions.function(ty_i32_i32_i32); // __any_i32: (vec,fn)->i32(bool)
        functions.function(ty_i32_i32_i32); // __find_i32: (vec,fn)->i32(option_ptr)
        let mut needs_start_wrapper = false;
        for (i, func) in mir.functions.iter().enumerate() {
            functions.function(user_func_type_indices[i]);
            if func.name == "main" && !matches!(func.return_ty, ark_typecheck::types::Type::Unit) {
                needs_start_wrapper = true;
            }
        }
        let start_wrapper_idx = if needs_start_wrapper {
            let idx = FN_USER_BASE + mir.functions.len() as u32;
            functions.function(ty_void_void); // ()->()
            Some(idx)
        } else {
            None
        };
        module.section(&functions);

        // Table section — for indirect calls (higher-order functions)
        let total_funcs =
            4 + 14 + mir.functions.len() as u64 + if needs_start_wrapper { 1 } else { 0 };
        let mut tables = wasm_encoder::TableSection::new();
        tables.table(wasm_encoder::TableType {
            element_type: wasm_encoder::RefType::FUNCREF,
            minimum: total_funcs,
            maximum: Some(total_funcs),
            table64: false,
            shared: false,
        });
        module.section(&tables);

        // Memory section
        let mut memory = MemorySection::new();
        memory.memory(MemoryType {
            minimum: 1,
            maximum: None,
            memory64: false,
            shared: false,
            page_size_log2: None,
        });
        module.section(&memory);

        // Global section: heap pointer for struct/runtime allocation
        // Heap starts at 4096 (after static data region 256-4095)
        let mut globals = wasm_encoder::GlobalSection::new();
        globals.global(
            wasm_encoder::GlobalType {
                val_type: ValType::I32,
                mutable: true,
                shared: false,
            },
            &wasm_encoder::ConstExpr::i32_const(4096),
        );
        module.section(&globals);

        // Export section
        let mut exports = ExportSection::new();
        exports.export("memory", ExportKind::Memory, 0);
        if let Some(wrapper_idx) = start_wrapper_idx {
            exports.export("_start", ExportKind::Func, wrapper_idx);
        } else {
            for (idx, func) in mir.functions.iter().enumerate() {
                if func.name == "main" {
                    exports.export("_start", ExportKind::Func, FN_USER_BASE + idx as u32);
                }
            }
        }
        module.section(&exports);

        // Element section — populate table with all function refs
        let mut elements = wasm_encoder::ElementSection::new();
        let func_indices: Vec<u32> = (0..total_funcs as u32).collect();
        elements.active(
            Some(0),
            &wasm_encoder::ConstExpr::i32_const(0),
            wasm_encoder::Elements::Functions(std::borrow::Cow::Borrowed(&func_indices)),
        );
        module.section(&elements);

        // Code section
        let mut code = CodeSection::new();
        code.function(&self.build_i32_to_string());
        code.function(&self.build_print_i32_ln());
        code.function(&self.build_print_bool_ln());
        code.function(&self.build_print_str_ln());
        code.function(&self.build_str_eq());
        code.function(&self.build_concat());
        code.function(&self.build_f64_to_string());
        code.function(&self.build_i64_to_string());
        code.function(&self.build_map_i32());
        code.function(&self.build_filter_i32());
        code.function(&self.build_fold_i32());
        code.function(&self.build_map_option_i32());
        code.function(&self.build_any_i32());
        code.function(&self.build_find_i32());
        for func in &mir.functions {
            let f = self.build_user_fn(func);
            code.function(&f);
        }
        if needs_start_wrapper {
            // _start wrapper: call main, drop result
            let main_idx = mir
                .functions
                .iter()
                .position(|f| f.name == "main")
                .map(|i| FN_USER_BASE + i as u32)
                .unwrap();
            let mut wrapper = Function::new(vec![]);
            wrapper.instruction(&Instruction::Call(main_idx));
            wrapper.instruction(&Instruction::Drop);
            wrapper.instruction(&Instruction::End);
            code.function(&wrapper);
        }
        module.section(&code);

        // Data section — include static bool strings
        let mut data = DataSection::new();
        // "true" at BOOL_TRUE (80)
        data.active(
            0,
            &wasm_encoder::ConstExpr::i32_const(BOOL_TRUE as i32),
            b"true".iter().copied(),
        );
        // "false" at BOOL_FALSE (84)
        data.active(
            0,
            &wasm_encoder::ConstExpr::i32_const(BOOL_FALSE as i32),
            b"false".iter().copied(),
        );
        // "\n" at NEWLINE (89)
        data.active(
            0,
            &wasm_encoder::ConstExpr::i32_const(NEWLINE as i32),
            b"\n".iter().copied(),
        );

        for (offset, bytes) in &self.string_literals {
            data.active(
                0,
                &wasm_encoder::ConstExpr::i32_const(*offset as i32),
                bytes.iter().copied(),
            );
        }
        module.section(&data);

        module.finish()
    }

    /// __i32_to_string(value: i32) -> void
    /// Writes decimal string to I32BUF, stores offset at SCRATCH, length at SCRATCH+4
    fn build_i32_to_string(&self) -> Function {
        let ma = MemArg {
            offset: 0,
            align: 0,
            memory_index: 0,
        };
        let ma2 = MemArg {
            offset: 0,
            align: 2,
            memory_index: 0,
        };
        let buf_end = I32BUF + 20; // write digits right-to-left
        // Locals: 0=value(param), 1=is_neg, 2=pos, 3=abs_val
        let mut f = Function::new(vec![(3, ValType::I32)]);

        // pos = buf_end
        f.instruction(&Instruction::I32Const(buf_end as i32));
        f.instruction(&Instruction::LocalSet(2));

        // Zero check
        f.instruction(&Instruction::LocalGet(0));
        f.instruction(&Instruction::I32Eqz);
        f.instruction(&Instruction::If(wasm_encoder::BlockType::Empty));
        {
            f.instruction(&Instruction::I32Const(IOV_BASE as i32)); // not needed but let's go
            // Store '0' at buf_end-1
            f.instruction(&Instruction::I32Const((buf_end - 1) as i32));
            f.instruction(&Instruction::I32Const(48));
            f.instruction(&Instruction::I32Store8(ma));
            // SCRATCH = buf_end - 1
            f.instruction(&Instruction::I32Const(SCRATCH as i32));
            f.instruction(&Instruction::I32Const((buf_end - 1) as i32));
            f.instruction(&Instruction::I32Store(ma2));
            // SCRATCH+4 = 1
            f.instruction(&Instruction::I32Const((SCRATCH + 4) as i32));
            f.instruction(&Instruction::I32Const(1));
            f.instruction(&Instruction::I32Store(ma2));
            f.instruction(&Instruction::Return);
        }
        f.instruction(&Instruction::End);

        // Check negative
        f.instruction(&Instruction::LocalGet(0));
        f.instruction(&Instruction::I32Const(0));
        f.instruction(&Instruction::I32LtS);
        f.instruction(&Instruction::LocalSet(1));

        // abs_val
        f.instruction(&Instruction::LocalGet(1));
        f.instruction(&Instruction::If(wasm_encoder::BlockType::Empty));
        f.instruction(&Instruction::I32Const(0));
        f.instruction(&Instruction::LocalGet(0));
        f.instruction(&Instruction::I32Sub);
        f.instruction(&Instruction::LocalSet(3));
        f.instruction(&Instruction::Else);
        f.instruction(&Instruction::LocalGet(0));
        f.instruction(&Instruction::LocalSet(3));
        f.instruction(&Instruction::End);

        // Digit extraction loop
        f.instruction(&Instruction::Block(wasm_encoder::BlockType::Empty));
        f.instruction(&Instruction::Loop(wasm_encoder::BlockType::Empty));
        f.instruction(&Instruction::LocalGet(3));
        f.instruction(&Instruction::I32Eqz);
        f.instruction(&Instruction::BrIf(1));
        // pos -= 1
        f.instruction(&Instruction::LocalGet(2));
        f.instruction(&Instruction::I32Const(1));
        f.instruction(&Instruction::I32Sub);
        f.instruction(&Instruction::LocalSet(2));
        // mem[pos] = (abs_val % 10) + '0'
        f.instruction(&Instruction::LocalGet(2));
        f.instruction(&Instruction::LocalGet(3));
        f.instruction(&Instruction::I32Const(10));
        f.instruction(&Instruction::I32RemU);
        f.instruction(&Instruction::I32Const(48));
        f.instruction(&Instruction::I32Add);
        f.instruction(&Instruction::I32Store8(ma));
        // abs_val /= 10
        f.instruction(&Instruction::LocalGet(3));
        f.instruction(&Instruction::I32Const(10));
        f.instruction(&Instruction::I32DivU);
        f.instruction(&Instruction::LocalSet(3));
        f.instruction(&Instruction::Br(0));
        f.instruction(&Instruction::End); // loop
        f.instruction(&Instruction::End); // block

        // Prepend '-' if negative
        f.instruction(&Instruction::LocalGet(1));
        f.instruction(&Instruction::If(wasm_encoder::BlockType::Empty));
        f.instruction(&Instruction::LocalGet(2));
        f.instruction(&Instruction::I32Const(1));
        f.instruction(&Instruction::I32Sub);
        f.instruction(&Instruction::LocalSet(2));
        f.instruction(&Instruction::LocalGet(2));
        f.instruction(&Instruction::I32Const(45)); // '-'
        f.instruction(&Instruction::I32Store8(ma));
        f.instruction(&Instruction::End);

        // Store result: SCRATCH = pos (offset), SCRATCH+4 = buf_end - pos (len)
        f.instruction(&Instruction::I32Const(SCRATCH as i32));
        f.instruction(&Instruction::LocalGet(2));
        f.instruction(&Instruction::I32Store(ma2));
        f.instruction(&Instruction::I32Const((SCRATCH + 4) as i32));
        f.instruction(&Instruction::I32Const(buf_end as i32));
        f.instruction(&Instruction::LocalGet(2));
        f.instruction(&Instruction::I32Sub);
        f.instruction(&Instruction::I32Store(ma2));

        f.instruction(&Instruction::End);
        f
    }

    /// __print_i32_ln(value: i32) -> void
    /// Prints value as decimal string + newline to stdout
    fn build_print_i32_ln(&self) -> Function {
        let ma2 = MemArg {
            offset: 0,
            align: 2,
            memory_index: 0,
        };
        let mut f = Function::new(vec![]);

        // Call __i32_to_string(value)
        f.instruction(&Instruction::LocalGet(0));
        f.instruction(&Instruction::Call(FN_I32_TO_STR));

        // Set up iov for the number string
        f.instruction(&Instruction::I32Const(IOV_BASE as i32));
        f.instruction(&Instruction::I32Const(SCRATCH as i32));
        f.instruction(&Instruction::I32Load(ma2));
        f.instruction(&Instruction::I32Store(ma2));

        f.instruction(&Instruction::I32Const((IOV_BASE + 4) as i32));
        f.instruction(&Instruction::I32Const((SCRATCH + 4) as i32));
        f.instruction(&Instruction::I32Load(ma2));
        f.instruction(&Instruction::I32Store(ma2));

        // fd_write(1, iov, 1, nwritten)
        f.instruction(&Instruction::I32Const(1));
        f.instruction(&Instruction::I32Const(IOV_BASE as i32));
        f.instruction(&Instruction::I32Const(1));
        f.instruction(&Instruction::I32Const(NWRITTEN as i32));
        f.instruction(&Instruction::Call(FN_FD_WRITE));
        f.instruction(&Instruction::Drop);

        // Print newline
        self.emit_static_print(&mut f, NEWLINE, 1);

        f.instruction(&Instruction::End);
        f
    }

    /// __print_bool_ln(value: i32) -> void
    /// Prints "true\n" or "false\n" to stdout
    fn build_print_bool_ln(&self) -> Function {
        let ma2 = MemArg {
            offset: 0,
            align: 2,
            memory_index: 0,
        };
        // Locals: 0=value(param), 1=ptr, 2=len
        let mut f = Function::new(vec![(2, ValType::I32)]);

        f.instruction(&Instruction::LocalGet(0));
        f.instruction(&Instruction::If(wasm_encoder::BlockType::Empty));
        {
            // true
            f.instruction(&Instruction::I32Const(BOOL_TRUE as i32));
            f.instruction(&Instruction::LocalSet(1));
            f.instruction(&Instruction::I32Const(4));
            f.instruction(&Instruction::LocalSet(2));
        }
        f.instruction(&Instruction::Else);
        {
            // false
            f.instruction(&Instruction::I32Const(BOOL_FALSE as i32));
            f.instruction(&Instruction::LocalSet(1));
            f.instruction(&Instruction::I32Const(5));
            f.instruction(&Instruction::LocalSet(2));
        }
        f.instruction(&Instruction::End);

        // Setup iov and write
        f.instruction(&Instruction::I32Const(IOV_BASE as i32));
        f.instruction(&Instruction::LocalGet(1));
        f.instruction(&Instruction::I32Store(ma2));
        f.instruction(&Instruction::I32Const((IOV_BASE + 4) as i32));
        f.instruction(&Instruction::LocalGet(2));
        f.instruction(&Instruction::I32Store(ma2));
        f.instruction(&Instruction::I32Const(1));
        f.instruction(&Instruction::I32Const(IOV_BASE as i32));
        f.instruction(&Instruction::I32Const(1));
        f.instruction(&Instruction::I32Const(NWRITTEN as i32));
        f.instruction(&Instruction::Call(FN_FD_WRITE));
        f.instruction(&Instruction::Drop);

        // Print newline
        self.emit_static_print(&mut f, NEWLINE, 1);

        f.instruction(&Instruction::End);
        f
    }

    /// __print_str_ln(ptr: i32) -> void
    /// ptr points to string data; length is at (ptr - 4) as i32.
    /// Prints the string + newline to stdout.
    fn build_print_str_ln(&self) -> Function {
        let ma2 = MemArg {
            offset: 0,
            align: 2,
            memory_index: 0,
        };
        let mut f = Function::new(vec![]);

        // iov.base = ptr
        f.instruction(&Instruction::I32Const(IOV_BASE as i32));
        f.instruction(&Instruction::LocalGet(0));
        f.instruction(&Instruction::I32Store(ma2));

        // iov.len = i32.load(ptr - 4)
        f.instruction(&Instruction::I32Const((IOV_BASE + 4) as i32));
        f.instruction(&Instruction::LocalGet(0));
        f.instruction(&Instruction::I32Const(4));
        f.instruction(&Instruction::I32Sub);
        f.instruction(&Instruction::I32Load(ma2));
        f.instruction(&Instruction::I32Store(ma2));

        // fd_write(1, iov, 1, nwritten)
        f.instruction(&Instruction::I32Const(1));
        f.instruction(&Instruction::I32Const(IOV_BASE as i32));
        f.instruction(&Instruction::I32Const(1));
        f.instruction(&Instruction::I32Const(NWRITTEN as i32));
        f.instruction(&Instruction::Call(FN_FD_WRITE));
        f.instruction(&Instruction::Drop);

        // Print newline
        self.emit_static_print(&mut f, NEWLINE, 1);

        f.instruction(&Instruction::End);
        f
    }

    /// Emit fd_write for a static data segment at known offset/length.
    fn emit_static_print(&self, f: &mut Function, offset: u32, len: u32) {
        let ma2 = MemArg {
            offset: 0,
            align: 2,
            memory_index: 0,
        };
        f.instruction(&Instruction::I32Const(IOV_BASE as i32));
        f.instruction(&Instruction::I32Const(offset as i32));
        f.instruction(&Instruction::I32Store(ma2));
        f.instruction(&Instruction::I32Const((IOV_BASE + 4) as i32));
        f.instruction(&Instruction::I32Const(len as i32));
        f.instruction(&Instruction::I32Store(ma2));
        f.instruction(&Instruction::I32Const(1));
        f.instruction(&Instruction::I32Const(IOV_BASE as i32));
        f.instruction(&Instruction::I32Const(1));
        f.instruction(&Instruction::I32Const(NWRITTEN as i32));
        f.instruction(&Instruction::Call(FN_FD_WRITE));
        f.instruction(&Instruction::Drop);
    }

    /// __str_eq(ptr1: i32, ptr2: i32) -> i32 (0 or 1)
    /// Compares two length-prefixed strings for equality.
    /// Length is at ptr-4, data at ptr.
    fn build_str_eq(&self) -> Function {
        let ma2 = MemArg {
            offset: 0,
            align: 2,
            memory_index: 0,
        };
        let ma0 = MemArg {
            offset: 0,
            align: 0,
            memory_index: 0,
        };
        // params: 0=ptr1, 1=ptr2; locals: 2=len1, 3=len2, 4=i
        let mut f = Function::new(vec![(3, ValType::I32)]);

        // len1 = mem[ptr1 - 4]
        f.instruction(&Instruction::LocalGet(0));
        f.instruction(&Instruction::I32Const(4));
        f.instruction(&Instruction::I32Sub);
        f.instruction(&Instruction::I32Load(ma2));
        f.instruction(&Instruction::LocalSet(2));

        // len2 = mem[ptr2 - 4]
        f.instruction(&Instruction::LocalGet(1));
        f.instruction(&Instruction::I32Const(4));
        f.instruction(&Instruction::I32Sub);
        f.instruction(&Instruction::I32Load(ma2));
        f.instruction(&Instruction::LocalSet(3));

        // if len1 != len2, return 0
        f.instruction(&Instruction::LocalGet(2));
        f.instruction(&Instruction::LocalGet(3));
        f.instruction(&Instruction::I32Ne);
        f.instruction(&Instruction::If(wasm_encoder::BlockType::Empty));
        f.instruction(&Instruction::I32Const(0));
        f.instruction(&Instruction::Return);
        f.instruction(&Instruction::End);

        // i = 0
        f.instruction(&Instruction::I32Const(0));
        f.instruction(&Instruction::LocalSet(4));

        // loop: compare bytes
        f.instruction(&Instruction::Block(wasm_encoder::BlockType::Empty));
        f.instruction(&Instruction::Loop(wasm_encoder::BlockType::Empty));

        // if i >= len1, break (all equal)
        f.instruction(&Instruction::LocalGet(4));
        f.instruction(&Instruction::LocalGet(2));
        f.instruction(&Instruction::I32GeU);
        f.instruction(&Instruction::BrIf(1));

        // if mem[ptr1+i] != mem[ptr2+i], return 0
        f.instruction(&Instruction::LocalGet(0));
        f.instruction(&Instruction::LocalGet(4));
        f.instruction(&Instruction::I32Add);
        f.instruction(&Instruction::I32Load8U(ma0));

        f.instruction(&Instruction::LocalGet(1));
        f.instruction(&Instruction::LocalGet(4));
        f.instruction(&Instruction::I32Add);
        f.instruction(&Instruction::I32Load8U(ma0));

        f.instruction(&Instruction::I32Ne);
        f.instruction(&Instruction::If(wasm_encoder::BlockType::Empty));
        f.instruction(&Instruction::I32Const(0));
        f.instruction(&Instruction::Return);
        f.instruction(&Instruction::End);

        // i += 1
        f.instruction(&Instruction::LocalGet(4));
        f.instruction(&Instruction::I32Const(1));
        f.instruction(&Instruction::I32Add);
        f.instruction(&Instruction::LocalSet(4));

        f.instruction(&Instruction::Br(0)); // continue loop
        f.instruction(&Instruction::End); // end loop
        f.instruction(&Instruction::End); // end block

        // All bytes match
        f.instruction(&Instruction::I32Const(1));
        f.instruction(&Instruction::End);
        f
    }

    /// __concat(ptr1: i32, ptr2: i32) -> i32 (pointer to new length-prefixed string)
    /// Allocates new string = len1 + len2, copies data from both, returns data pointer.
    fn build_concat(&self) -> Function {
        let ma2 = MemArg {
            offset: 0,
            align: 2,
            memory_index: 0,
        };
        let ma0 = MemArg {
            offset: 0,
            align: 0,
            memory_index: 0,
        };
        // params: 0=ptr1, 1=ptr2; locals: 2=len1, 3=len2, 4=total_len, 5=dst, 6=i
        let mut f = Function::new(vec![(5, ValType::I32)]);

        // len1 = mem[ptr1 - 4]
        f.instruction(&Instruction::LocalGet(0));
        f.instruction(&Instruction::I32Const(4));
        f.instruction(&Instruction::I32Sub);
        f.instruction(&Instruction::I32Load(ma2));
        f.instruction(&Instruction::LocalSet(2));

        // len2 = mem[ptr2 - 4]
        f.instruction(&Instruction::LocalGet(1));
        f.instruction(&Instruction::I32Const(4));
        f.instruction(&Instruction::I32Sub);
        f.instruction(&Instruction::I32Load(ma2));
        f.instruction(&Instruction::LocalSet(3));

        // total_len = len1 + len2
        f.instruction(&Instruction::LocalGet(2));
        f.instruction(&Instruction::LocalGet(3));
        f.instruction(&Instruction::I32Add);
        f.instruction(&Instruction::LocalSet(4));

        // Write total_len at heap_ptr (length prefix)
        f.instruction(&Instruction::GlobalGet(0));
        f.instruction(&Instruction::LocalGet(4));
        f.instruction(&Instruction::I32Store(ma2));

        // dst = heap_ptr + 4 (data start)
        f.instruction(&Instruction::GlobalGet(0));
        f.instruction(&Instruction::I32Const(4));
        f.instruction(&Instruction::I32Add);
        f.instruction(&Instruction::LocalSet(5));

        // Copy bytes from ptr1 (len1 bytes)
        // i = 0
        f.instruction(&Instruction::I32Const(0));
        f.instruction(&Instruction::LocalSet(6));
        f.instruction(&Instruction::Block(wasm_encoder::BlockType::Empty));
        f.instruction(&Instruction::Loop(wasm_encoder::BlockType::Empty));
        // if i >= len1, break
        f.instruction(&Instruction::LocalGet(6));
        f.instruction(&Instruction::LocalGet(2));
        f.instruction(&Instruction::I32GeU);
        f.instruction(&Instruction::BrIf(1));
        // mem[dst + i] = mem[ptr1 + i]
        f.instruction(&Instruction::LocalGet(5));
        f.instruction(&Instruction::LocalGet(6));
        f.instruction(&Instruction::I32Add);
        f.instruction(&Instruction::LocalGet(0));
        f.instruction(&Instruction::LocalGet(6));
        f.instruction(&Instruction::I32Add);
        f.instruction(&Instruction::I32Load8U(ma0));
        f.instruction(&Instruction::I32Store8(ma0));
        // i += 1
        f.instruction(&Instruction::LocalGet(6));
        f.instruction(&Instruction::I32Const(1));
        f.instruction(&Instruction::I32Add);
        f.instruction(&Instruction::LocalSet(6));
        f.instruction(&Instruction::Br(0));
        f.instruction(&Instruction::End); // end loop
        f.instruction(&Instruction::End); // end block

        // Copy bytes from ptr2 (len2 bytes)
        // i = 0
        f.instruction(&Instruction::I32Const(0));
        f.instruction(&Instruction::LocalSet(6));
        f.instruction(&Instruction::Block(wasm_encoder::BlockType::Empty));
        f.instruction(&Instruction::Loop(wasm_encoder::BlockType::Empty));
        // if i >= len2, break
        f.instruction(&Instruction::LocalGet(6));
        f.instruction(&Instruction::LocalGet(3));
        f.instruction(&Instruction::I32GeU);
        f.instruction(&Instruction::BrIf(1));
        // mem[dst + len1 + i] = mem[ptr2 + i]
        f.instruction(&Instruction::LocalGet(5));
        f.instruction(&Instruction::LocalGet(2));
        f.instruction(&Instruction::I32Add);
        f.instruction(&Instruction::LocalGet(6));
        f.instruction(&Instruction::I32Add);
        f.instruction(&Instruction::LocalGet(1));
        f.instruction(&Instruction::LocalGet(6));
        f.instruction(&Instruction::I32Add);
        f.instruction(&Instruction::I32Load8U(ma0));
        f.instruction(&Instruction::I32Store8(ma0));
        // i += 1
        f.instruction(&Instruction::LocalGet(6));
        f.instruction(&Instruction::I32Const(1));
        f.instruction(&Instruction::I32Add);
        f.instruction(&Instruction::LocalSet(6));
        f.instruction(&Instruction::Br(0));
        f.instruction(&Instruction::End); // end loop
        f.instruction(&Instruction::End); // end block

        // Bump heap pointer: heap_ptr += 4 + total_len
        f.instruction(&Instruction::GlobalGet(0));
        f.instruction(&Instruction::I32Const(4));
        f.instruction(&Instruction::I32Add);
        f.instruction(&Instruction::LocalGet(4));
        f.instruction(&Instruction::I32Add);
        f.instruction(&Instruction::GlobalSet(0));

        // Return dst (data pointer)
        f.instruction(&Instruction::LocalGet(5));
        f.instruction(&Instruction::End);
        f
    }

    /// __f64_to_str(value: f64) -> i32 (string ptr)
    /// Converts f64 to a length-prefixed string on the heap.
    /// Uses I32BUF (scratch) area for building the string.
    /// Locals: 0=value(f64 param), 1=is_neg(i32), 2=int_part(i32), 3=frac_digits(i32),
    ///         4=pos(i32), 5=digit(i32), 6=len(i32), 7=dst(i32)
    fn build_f64_to_string(&self) -> Function {
        let ma0 = MemArg {
            offset: 0,
            align: 0,
            memory_index: 0,
        };
        let ma2 = MemArg {
            offset: 0,
            align: 2,
            memory_index: 0,
        };
        let buf = I32BUF;
        // Locals: 7 i32 locals
        let mut f = Function::new(vec![(7, ValType::I32)]);

        // pos = buf (write position)
        f.instruction(&Instruction::I32Const(buf as i32));
        f.instruction(&Instruction::LocalSet(4));

        // is_neg = value < 0.0
        f.instruction(&Instruction::LocalGet(0));
        f.instruction(&Instruction::F64Const(0.0));
        f.instruction(&Instruction::F64Lt);
        f.instruction(&Instruction::LocalSet(1));

        // if negative, negate value
        f.instruction(&Instruction::LocalGet(1));
        f.instruction(&Instruction::If(wasm_encoder::BlockType::Empty));
        f.instruction(&Instruction::LocalGet(0));
        f.instruction(&Instruction::F64Neg);
        f.instruction(&Instruction::LocalSet(0));
        // write '-' at pos
        f.instruction(&Instruction::LocalGet(4));
        f.instruction(&Instruction::I32Const(b'-' as i32));
        f.instruction(&Instruction::I32Store8(ma0));
        f.instruction(&Instruction::LocalGet(4));
        f.instruction(&Instruction::I32Const(1));
        f.instruction(&Instruction::I32Add);
        f.instruction(&Instruction::LocalSet(4));
        f.instruction(&Instruction::End);

        // int_part = trunc(value) as i32
        f.instruction(&Instruction::LocalGet(0));
        f.instruction(&Instruction::F64Floor);
        f.instruction(&Instruction::I32TruncF64S);
        f.instruction(&Instruction::LocalSet(2));

        // Convert integer part to digits (right to left into temp, then copy)
        // Handle 0 case
        f.instruction(&Instruction::LocalGet(2));
        f.instruction(&Instruction::I32Eqz);
        f.instruction(&Instruction::If(wasm_encoder::BlockType::Empty));
        {
            // Write '0' at pos
            f.instruction(&Instruction::LocalGet(4));
            f.instruction(&Instruction::I32Const(b'0' as i32));
            f.instruction(&Instruction::I32Store8(ma0));
            f.instruction(&Instruction::LocalGet(4));
            f.instruction(&Instruction::I32Const(1));
            f.instruction(&Instruction::I32Add);
            f.instruction(&Instruction::LocalSet(4));
        }
        f.instruction(&Instruction::Else);
        {
            // Write integer digits: use SCRATCH area temporarily
            // digit_pos = SCRATCH+20 (write from right)
            f.instruction(&Instruction::I32Const(SCRATCH as i32 + 20));
            f.instruction(&Instruction::LocalSet(5)); // reuse local 5 as digit_pos
            // digit_count = 0 in local 6
            f.instruction(&Instruction::I32Const(0));
            f.instruction(&Instruction::LocalSet(6));
            // loop: extract digits
            f.instruction(&Instruction::Block(wasm_encoder::BlockType::Empty));
            f.instruction(&Instruction::Loop(wasm_encoder::BlockType::Empty));
            // if int_part == 0, break
            f.instruction(&Instruction::LocalGet(2));
            f.instruction(&Instruction::I32Eqz);
            f.instruction(&Instruction::BrIf(1));
            // digit = int_part % 10
            f.instruction(&Instruction::LocalGet(5));
            f.instruction(&Instruction::I32Const(1));
            f.instruction(&Instruction::I32Sub);
            f.instruction(&Instruction::LocalTee(5));
            // store digit char
            f.instruction(&Instruction::LocalGet(2));
            f.instruction(&Instruction::I32Const(10));
            f.instruction(&Instruction::I32RemU);
            f.instruction(&Instruction::I32Const(b'0' as i32));
            f.instruction(&Instruction::I32Add);
            f.instruction(&Instruction::I32Store8(ma0));
            // int_part /= 10
            f.instruction(&Instruction::LocalGet(2));
            f.instruction(&Instruction::I32Const(10));
            f.instruction(&Instruction::I32DivU);
            f.instruction(&Instruction::LocalSet(2));
            // digit_count += 1
            f.instruction(&Instruction::LocalGet(6));
            f.instruction(&Instruction::I32Const(1));
            f.instruction(&Instruction::I32Add);
            f.instruction(&Instruction::LocalSet(6));
            f.instruction(&Instruction::Br(0));
            f.instruction(&Instruction::End); // end loop
            f.instruction(&Instruction::End); // end block
            // Copy digits from SCRATCH area to output buffer at pos
            // digit_pos points to first digit, digit_count has count
            f.instruction(&Instruction::I32Const(0));
            f.instruction(&Instruction::LocalSet(2)); // reuse as counter
            f.instruction(&Instruction::Block(wasm_encoder::BlockType::Empty));
            f.instruction(&Instruction::Loop(wasm_encoder::BlockType::Empty));
            f.instruction(&Instruction::LocalGet(2));
            f.instruction(&Instruction::LocalGet(6));
            f.instruction(&Instruction::I32GeU);
            f.instruction(&Instruction::BrIf(1));
            // buf[pos + i] = scratch[digit_pos + i]
            f.instruction(&Instruction::LocalGet(4));
            f.instruction(&Instruction::LocalGet(2));
            f.instruction(&Instruction::I32Add);
            f.instruction(&Instruction::LocalGet(5));
            f.instruction(&Instruction::LocalGet(2));
            f.instruction(&Instruction::I32Add);
            f.instruction(&Instruction::I32Load8U(ma0));
            f.instruction(&Instruction::I32Store8(ma0));
            f.instruction(&Instruction::LocalGet(2));
            f.instruction(&Instruction::I32Const(1));
            f.instruction(&Instruction::I32Add);
            f.instruction(&Instruction::LocalSet(2));
            f.instruction(&Instruction::Br(0));
            f.instruction(&Instruction::End);
            f.instruction(&Instruction::End);
            // pos += digit_count
            f.instruction(&Instruction::LocalGet(4));
            f.instruction(&Instruction::LocalGet(6));
            f.instruction(&Instruction::I32Add);
            f.instruction(&Instruction::LocalSet(4));
        }
        f.instruction(&Instruction::End);

        // Fractional part: frac = value - floor(value)
        // frac_digits = round(frac * 100000000) as i32  (8 decimal digits)
        f.instruction(&Instruction::LocalGet(0));
        f.instruction(&Instruction::LocalGet(0));
        f.instruction(&Instruction::F64Floor);
        f.instruction(&Instruction::F64Sub);
        f.instruction(&Instruction::F64Const(100000000.0)); // 10^8
        f.instruction(&Instruction::F64Mul);
        f.instruction(&Instruction::F64Nearest);
        f.instruction(&Instruction::I32TruncF64S);
        f.instruction(&Instruction::LocalSet(3)); // frac_digits

        // If frac_digits > 0, add decimal point and digits
        f.instruction(&Instruction::LocalGet(3));
        f.instruction(&Instruction::I32Const(0));
        f.instruction(&Instruction::I32GtS);
        f.instruction(&Instruction::If(wasm_encoder::BlockType::Empty));
        {
            // Write '.'
            f.instruction(&Instruction::LocalGet(4));
            f.instruction(&Instruction::I32Const(b'.' as i32));
            f.instruction(&Instruction::I32Store8(ma0));
            f.instruction(&Instruction::LocalGet(4));
            f.instruction(&Instruction::I32Const(1));
            f.instruction(&Instruction::I32Add);
            f.instruction(&Instruction::LocalSet(4));

            // Write fractional digits (up to 8, removing trailing zeros)
            // Find how many trailing zeros to remove
            // Start with 8 digits: pad with leading zeros, remove trailing zeros
            // digit_count = 8
            f.instruction(&Instruction::I32Const(8));
            f.instruction(&Instruction::LocalSet(6));

            // Remove trailing zeros from frac_digits
            f.instruction(&Instruction::Block(wasm_encoder::BlockType::Empty));
            f.instruction(&Instruction::Loop(wasm_encoder::BlockType::Empty));
            // if frac_digits % 10 != 0, break
            f.instruction(&Instruction::LocalGet(3));
            f.instruction(&Instruction::I32Const(10));
            f.instruction(&Instruction::I32RemU);
            f.instruction(&Instruction::I32Const(0));
            f.instruction(&Instruction::I32Ne);
            f.instruction(&Instruction::BrIf(1));
            // frac_digits /= 10, digit_count -= 1
            f.instruction(&Instruction::LocalGet(3));
            f.instruction(&Instruction::I32Const(10));
            f.instruction(&Instruction::I32DivU);
            f.instruction(&Instruction::LocalSet(3));
            f.instruction(&Instruction::LocalGet(6));
            f.instruction(&Instruction::I32Const(1));
            f.instruction(&Instruction::I32Sub);
            f.instruction(&Instruction::LocalSet(6));
            // if digit_count == 0, break
            f.instruction(&Instruction::LocalGet(6));
            f.instruction(&Instruction::I32Eqz);
            f.instruction(&Instruction::BrIf(1));
            f.instruction(&Instruction::Br(0));
            f.instruction(&Instruction::End);
            f.instruction(&Instruction::End);

            // Now write digit_count digits from frac_digits (right to left)
            // Write digits at pos + digit_count - 1 .. pos
            f.instruction(&Instruction::LocalGet(6));
            f.instruction(&Instruction::LocalSet(5)); // remaining = digit_count
            f.instruction(&Instruction::Block(wasm_encoder::BlockType::Empty));
            f.instruction(&Instruction::Loop(wasm_encoder::BlockType::Empty));
            f.instruction(&Instruction::LocalGet(5));
            f.instruction(&Instruction::I32Eqz);
            f.instruction(&Instruction::BrIf(1));
            // buf[pos + remaining - 1] = '0' + frac_digits % 10
            f.instruction(&Instruction::LocalGet(4));
            f.instruction(&Instruction::LocalGet(5));
            f.instruction(&Instruction::I32Add);
            f.instruction(&Instruction::I32Const(1));
            f.instruction(&Instruction::I32Sub);
            f.instruction(&Instruction::LocalGet(3));
            f.instruction(&Instruction::I32Const(10));
            f.instruction(&Instruction::I32RemU);
            f.instruction(&Instruction::I32Const(b'0' as i32));
            f.instruction(&Instruction::I32Add);
            f.instruction(&Instruction::I32Store8(ma0));
            // frac_digits /= 10
            f.instruction(&Instruction::LocalGet(3));
            f.instruction(&Instruction::I32Const(10));
            f.instruction(&Instruction::I32DivU);
            f.instruction(&Instruction::LocalSet(3));
            // remaining -= 1
            f.instruction(&Instruction::LocalGet(5));
            f.instruction(&Instruction::I32Const(1));
            f.instruction(&Instruction::I32Sub);
            f.instruction(&Instruction::LocalSet(5));
            f.instruction(&Instruction::Br(0));
            f.instruction(&Instruction::End);
            f.instruction(&Instruction::End);

            // pos += digit_count
            f.instruction(&Instruction::LocalGet(4));
            f.instruction(&Instruction::LocalGet(6));
            f.instruction(&Instruction::I32Add);
            f.instruction(&Instruction::LocalSet(4));
        }
        f.instruction(&Instruction::Else);
        {
            // If frac_digits == 0, write ".0"
            f.instruction(&Instruction::LocalGet(4));
            f.instruction(&Instruction::I32Const(b'.' as i32));
            f.instruction(&Instruction::I32Store8(ma0));
            f.instruction(&Instruction::LocalGet(4));
            f.instruction(&Instruction::I32Const(1));
            f.instruction(&Instruction::I32Add);
            f.instruction(&Instruction::LocalTee(4));
            f.instruction(&Instruction::I32Const(b'0' as i32));
            f.instruction(&Instruction::I32Store8(ma0));
            f.instruction(&Instruction::LocalGet(4));
            f.instruction(&Instruction::I32Const(1));
            f.instruction(&Instruction::I32Add);
            f.instruction(&Instruction::LocalSet(4));
        }
        f.instruction(&Instruction::End);

        // Now build length-prefixed string on heap
        // len = pos - buf
        f.instruction(&Instruction::LocalGet(4));
        f.instruction(&Instruction::I32Const(buf as i32));
        f.instruction(&Instruction::I32Sub);
        f.instruction(&Instruction::LocalSet(6)); // len

        // Write length at heap_ptr
        f.instruction(&Instruction::GlobalGet(0));
        f.instruction(&Instruction::LocalGet(6));
        f.instruction(&Instruction::I32Store(ma2));

        // Copy data from buf to heap_ptr+4
        f.instruction(&Instruction::I32Const(0));
        f.instruction(&Instruction::LocalSet(5)); // i = 0
        f.instruction(&Instruction::Block(wasm_encoder::BlockType::Empty));
        f.instruction(&Instruction::Loop(wasm_encoder::BlockType::Empty));
        f.instruction(&Instruction::LocalGet(5));
        f.instruction(&Instruction::LocalGet(6));
        f.instruction(&Instruction::I32GeU);
        f.instruction(&Instruction::BrIf(1));
        // heap[heap_ptr + 4 + i] = buf[buf + i]
        f.instruction(&Instruction::GlobalGet(0));
        f.instruction(&Instruction::I32Const(4));
        f.instruction(&Instruction::I32Add);
        f.instruction(&Instruction::LocalGet(5));
        f.instruction(&Instruction::I32Add);
        f.instruction(&Instruction::I32Const(buf as i32));
        f.instruction(&Instruction::LocalGet(5));
        f.instruction(&Instruction::I32Add);
        f.instruction(&Instruction::I32Load8U(ma0));
        f.instruction(&Instruction::I32Store8(ma0));
        f.instruction(&Instruction::LocalGet(5));
        f.instruction(&Instruction::I32Const(1));
        f.instruction(&Instruction::I32Add);
        f.instruction(&Instruction::LocalSet(5));
        f.instruction(&Instruction::Br(0));
        f.instruction(&Instruction::End);
        f.instruction(&Instruction::End);

        // dst = heap_ptr + 4
        f.instruction(&Instruction::GlobalGet(0));
        f.instruction(&Instruction::I32Const(4));
        f.instruction(&Instruction::I32Add);
        f.instruction(&Instruction::LocalSet(7));

        // Bump heap: heap_ptr += 4 + len
        f.instruction(&Instruction::GlobalGet(0));
        f.instruction(&Instruction::I32Const(4));
        f.instruction(&Instruction::I32Add);
        f.instruction(&Instruction::LocalGet(6));
        f.instruction(&Instruction::I32Add);
        f.instruction(&Instruction::GlobalSet(0));

        // Return dst
        f.instruction(&Instruction::LocalGet(7));
        f.instruction(&Instruction::End);
        f
    }

    /// __i64_to_str(value: i64) -> i32 (string ptr)
    /// Similar to i32_to_string but handles 64-bit integers.
    /// Locals: 0=value(i64 param), 1=is_neg(i32), 2=pos(i32), 3=digit_count(i32),
    ///         4=digit(i32), 5=dst(i32), 6=len(i32)
    fn build_i64_to_string(&self) -> Function {
        let ma0 = MemArg {
            offset: 0,
            align: 0,
            memory_index: 0,
        };
        let ma2 = MemArg {
            offset: 0,
            align: 2,
            memory_index: 0,
        };
        let buf = I32BUF;
        let buf_end = buf + 22; // max 20 digits for i64 + sign
        let mut f = Function::new(vec![(6, ValType::I32)]);

        // is_neg = value < 0
        f.instruction(&Instruction::LocalGet(0));
        f.instruction(&Instruction::I64Const(0));
        f.instruction(&Instruction::I64LtS);
        f.instruction(&Instruction::LocalSet(1));

        // if negative, negate value
        f.instruction(&Instruction::LocalGet(1));
        f.instruction(&Instruction::If(wasm_encoder::BlockType::Empty));
        f.instruction(&Instruction::I64Const(0));
        f.instruction(&Instruction::LocalGet(0));
        f.instruction(&Instruction::I64Sub);
        f.instruction(&Instruction::LocalSet(0));
        f.instruction(&Instruction::End);

        // pos = buf_end (write digits right-to-left)
        f.instruction(&Instruction::I32Const(buf_end as i32));
        f.instruction(&Instruction::LocalSet(2));
        // digit_count = 0
        f.instruction(&Instruction::I32Const(0));
        f.instruction(&Instruction::LocalSet(3));

        // Handle 0 case
        f.instruction(&Instruction::LocalGet(0));
        f.instruction(&Instruction::I64Eqz);
        f.instruction(&Instruction::If(wasm_encoder::BlockType::Empty));
        {
            f.instruction(&Instruction::LocalGet(2));
            f.instruction(&Instruction::I32Const(1));
            f.instruction(&Instruction::I32Sub);
            f.instruction(&Instruction::LocalTee(2));
            f.instruction(&Instruction::I32Const(b'0' as i32));
            f.instruction(&Instruction::I32Store8(ma0));
            f.instruction(&Instruction::I32Const(1));
            f.instruction(&Instruction::LocalSet(3));
        }
        f.instruction(&Instruction::Else);
        {
            // Loop: extract digits
            f.instruction(&Instruction::Block(wasm_encoder::BlockType::Empty));
            f.instruction(&Instruction::Loop(wasm_encoder::BlockType::Empty));
            f.instruction(&Instruction::LocalGet(0));
            f.instruction(&Instruction::I64Eqz);
            f.instruction(&Instruction::BrIf(1));
            // digit = (value % 10) as i32
            f.instruction(&Instruction::LocalGet(0));
            f.instruction(&Instruction::I64Const(10));
            f.instruction(&Instruction::I64RemU);
            f.instruction(&Instruction::I32WrapI64);
            f.instruction(&Instruction::LocalSet(4));
            // pos -= 1; store digit char
            f.instruction(&Instruction::LocalGet(2));
            f.instruction(&Instruction::I32Const(1));
            f.instruction(&Instruction::I32Sub);
            f.instruction(&Instruction::LocalTee(2));
            f.instruction(&Instruction::LocalGet(4));
            f.instruction(&Instruction::I32Const(b'0' as i32));
            f.instruction(&Instruction::I32Add);
            f.instruction(&Instruction::I32Store8(ma0));
            // value /= 10
            f.instruction(&Instruction::LocalGet(0));
            f.instruction(&Instruction::I64Const(10));
            f.instruction(&Instruction::I64DivU);
            f.instruction(&Instruction::LocalSet(0));
            // digit_count += 1
            f.instruction(&Instruction::LocalGet(3));
            f.instruction(&Instruction::I32Const(1));
            f.instruction(&Instruction::I32Add);
            f.instruction(&Instruction::LocalSet(3));
            f.instruction(&Instruction::Br(0));
            f.instruction(&Instruction::End); // end loop
            f.instruction(&Instruction::End); // end block
        }
        f.instruction(&Instruction::End);

        // If negative, prepend '-'
        f.instruction(&Instruction::LocalGet(1));
        f.instruction(&Instruction::If(wasm_encoder::BlockType::Empty));
        f.instruction(&Instruction::LocalGet(2));
        f.instruction(&Instruction::I32Const(1));
        f.instruction(&Instruction::I32Sub);
        f.instruction(&Instruction::LocalTee(2));
        f.instruction(&Instruction::I32Const(b'-' as i32));
        f.instruction(&Instruction::I32Store8(ma0));
        f.instruction(&Instruction::LocalGet(3));
        f.instruction(&Instruction::I32Const(1));
        f.instruction(&Instruction::I32Add);
        f.instruction(&Instruction::LocalSet(3));
        f.instruction(&Instruction::End);

        // len = digit_count (including sign)
        f.instruction(&Instruction::LocalGet(3));
        f.instruction(&Instruction::LocalSet(6));

        // Allocate length-prefixed string on heap: [len:i32][data:bytes]
        f.instruction(&Instruction::GlobalGet(0));
        f.instruction(&Instruction::LocalGet(6));
        f.instruction(&Instruction::I32Store(ma2));

        // Copy digits from buf area to heap+4
        f.instruction(&Instruction::I32Const(0));
        f.instruction(&Instruction::LocalSet(4)); // i = 0
        f.instruction(&Instruction::Block(wasm_encoder::BlockType::Empty));
        f.instruction(&Instruction::Loop(wasm_encoder::BlockType::Empty));
        f.instruction(&Instruction::LocalGet(4));
        f.instruction(&Instruction::LocalGet(6));
        f.instruction(&Instruction::I32GeU);
        f.instruction(&Instruction::BrIf(1));
        // heap[heap_ptr + 4 + i] = buf[pos + i]
        f.instruction(&Instruction::GlobalGet(0));
        f.instruction(&Instruction::I32Const(4));
        f.instruction(&Instruction::I32Add);
        f.instruction(&Instruction::LocalGet(4));
        f.instruction(&Instruction::I32Add);
        f.instruction(&Instruction::LocalGet(2));
        f.instruction(&Instruction::LocalGet(4));
        f.instruction(&Instruction::I32Add);
        f.instruction(&Instruction::I32Load8U(ma0));
        f.instruction(&Instruction::I32Store8(ma0));
        f.instruction(&Instruction::LocalGet(4));
        f.instruction(&Instruction::I32Const(1));
        f.instruction(&Instruction::I32Add);
        f.instruction(&Instruction::LocalSet(4));
        f.instruction(&Instruction::Br(0));
        f.instruction(&Instruction::End);
        f.instruction(&Instruction::End);

        // dst = heap_ptr + 4
        f.instruction(&Instruction::GlobalGet(0));
        f.instruction(&Instruction::I32Const(4));
        f.instruction(&Instruction::I32Add);
        f.instruction(&Instruction::LocalSet(5));

        // Bump heap
        f.instruction(&Instruction::GlobalGet(0));
        f.instruction(&Instruction::I32Const(4));
        f.instruction(&Instruction::I32Add);
        f.instruction(&Instruction::LocalGet(6));
        f.instruction(&Instruction::I32Add);
        f.instruction(&Instruction::GlobalSet(0));

        // Return dst
        f.instruction(&Instruction::LocalGet(5));
        f.instruction(&Instruction::End);
        f
    }

    /// __map_i32(vec_ptr: i32, fn_idx: i32) -> i32 (new vec ptr)
    /// Allocates a new Vec, iterates src, applies fn via call_indirect, pushes result.
    fn build_map_i32(&self) -> Function {
        let ma = MemArg {
            offset: 0,
            align: 2,
            memory_index: 0,
        };
        // params: 0=vec_ptr, 1=fn_idx
        // locals: 2=i, 3=n, 4=src_data, 5=new_vec, 6=new_data
        let mut f = Function::new(vec![(5, ValType::I32)]);
        // n = vec.len
        f.instruction(&Instruction::LocalGet(0));
        f.instruction(&Instruction::I32Load(ma));
        f.instruction(&Instruction::LocalSet(3));
        // src_data = vec.data_ptr
        f.instruction(&Instruction::LocalGet(0));
        f.instruction(&Instruction::I32Const(8));
        f.instruction(&Instruction::I32Add);
        f.instruction(&Instruction::I32Load(ma));
        f.instruction(&Instruction::LocalSet(4));
        // Allocate new vec: [len, cap, data_ptr]
        f.instruction(&Instruction::GlobalGet(0));
        f.instruction(&Instruction::LocalSet(5)); // new_vec = heap
        // new_vec.len = n
        f.instruction(&Instruction::LocalGet(5));
        f.instruction(&Instruction::LocalGet(3));
        f.instruction(&Instruction::I32Store(ma));
        // new_vec.cap = n
        f.instruction(&Instruction::LocalGet(5));
        f.instruction(&Instruction::I32Const(4));
        f.instruction(&Instruction::I32Add);
        f.instruction(&Instruction::LocalGet(3));
        f.instruction(&Instruction::I32Store(ma));
        // new_data = heap + 12
        f.instruction(&Instruction::GlobalGet(0));
        f.instruction(&Instruction::I32Const(12));
        f.instruction(&Instruction::I32Add);
        f.instruction(&Instruction::LocalSet(6));
        // new_vec.data_ptr = new_data
        f.instruction(&Instruction::LocalGet(5));
        f.instruction(&Instruction::I32Const(8));
        f.instruction(&Instruction::I32Add);
        f.instruction(&Instruction::LocalGet(6));
        f.instruction(&Instruction::I32Store(ma));
        // bump heap: heap += 12 + n*4
        f.instruction(&Instruction::GlobalGet(0));
        f.instruction(&Instruction::I32Const(12));
        f.instruction(&Instruction::I32Add);
        f.instruction(&Instruction::LocalGet(3));
        f.instruction(&Instruction::I32Const(4));
        f.instruction(&Instruction::I32Mul);
        f.instruction(&Instruction::I32Add);
        f.instruction(&Instruction::GlobalSet(0));
        // i = 0
        f.instruction(&Instruction::I32Const(0));
        f.instruction(&Instruction::LocalSet(2));
        // Loop
        f.instruction(&Instruction::Block(wasm_encoder::BlockType::Empty));
        f.instruction(&Instruction::Loop(wasm_encoder::BlockType::Empty));
        // if i >= n, break
        f.instruction(&Instruction::LocalGet(2));
        f.instruction(&Instruction::LocalGet(3));
        f.instruction(&Instruction::I32GeU);
        f.instruction(&Instruction::BrIf(1));
        // new_data[i] = call_indirect(fn_idx, src_data[i])
        f.instruction(&Instruction::LocalGet(6));
        f.instruction(&Instruction::LocalGet(2));
        f.instruction(&Instruction::I32Const(4));
        f.instruction(&Instruction::I32Mul);
        f.instruction(&Instruction::I32Add); // &new_data[i]
        // call_indirect(fn_idx, src_data[i])
        f.instruction(&Instruction::LocalGet(4));
        f.instruction(&Instruction::LocalGet(2));
        f.instruction(&Instruction::I32Const(4));
        f.instruction(&Instruction::I32Mul);
        f.instruction(&Instruction::I32Add);
        f.instruction(&Instruction::I32Load(ma)); // src_data[i]
        f.instruction(&Instruction::LocalGet(1)); // fn table index
        let ty_i32_i32 =
            self.lookup_or_register_indirect_type(vec![ValType::I32], vec![ValType::I32]);
        f.instruction(&Instruction::CallIndirect {
            type_index: ty_i32_i32,
            table_index: 0,
        });
        f.instruction(&Instruction::I32Store(ma)); // store result
        // i++
        f.instruction(&Instruction::LocalGet(2));
        f.instruction(&Instruction::I32Const(1));
        f.instruction(&Instruction::I32Add);
        f.instruction(&Instruction::LocalSet(2));
        f.instruction(&Instruction::Br(0));
        f.instruction(&Instruction::End); // end loop
        f.instruction(&Instruction::End); // end block
        // return new_vec
        f.instruction(&Instruction::LocalGet(5));
        f.instruction(&Instruction::End);
        f
    }

    /// __filter_i32(vec_ptr: i32, fn_idx: i32) -> i32 (new vec ptr)
    fn build_filter_i32(&self) -> Function {
        let ma = MemArg {
            offset: 0,
            align: 2,
            memory_index: 0,
        };
        // params: 0=vec_ptr, 1=fn_idx
        // locals: 2=i, 3=n, 4=src_data, 5=new_vec, 6=new_data, 7=new_len
        let mut f = Function::new(vec![(6, ValType::I32)]);
        // n = vec.len
        f.instruction(&Instruction::LocalGet(0));
        f.instruction(&Instruction::I32Load(ma));
        f.instruction(&Instruction::LocalSet(3));
        // src_data = vec.data_ptr
        f.instruction(&Instruction::LocalGet(0));
        f.instruction(&Instruction::I32Const(8));
        f.instruction(&Instruction::I32Add);
        f.instruction(&Instruction::I32Load(ma));
        f.instruction(&Instruction::LocalSet(4));
        // Allocate new vec with cap = n
        f.instruction(&Instruction::GlobalGet(0));
        f.instruction(&Instruction::LocalSet(5));
        // new_vec.len = 0 (will be updated)
        f.instruction(&Instruction::LocalGet(5));
        f.instruction(&Instruction::I32Const(0));
        f.instruction(&Instruction::I32Store(ma));
        // new_vec.cap = n
        f.instruction(&Instruction::LocalGet(5));
        f.instruction(&Instruction::I32Const(4));
        f.instruction(&Instruction::I32Add);
        f.instruction(&Instruction::LocalGet(3));
        f.instruction(&Instruction::I32Store(ma));
        // new_data = heap + 12
        f.instruction(&Instruction::GlobalGet(0));
        f.instruction(&Instruction::I32Const(12));
        f.instruction(&Instruction::I32Add);
        f.instruction(&Instruction::LocalSet(6));
        // new_vec.data_ptr = new_data
        f.instruction(&Instruction::LocalGet(5));
        f.instruction(&Instruction::I32Const(8));
        f.instruction(&Instruction::I32Add);
        f.instruction(&Instruction::LocalGet(6));
        f.instruction(&Instruction::I32Store(ma));
        // bump heap: heap += 12 + n*4
        f.instruction(&Instruction::GlobalGet(0));
        f.instruction(&Instruction::I32Const(12));
        f.instruction(&Instruction::I32Add);
        f.instruction(&Instruction::LocalGet(3));
        f.instruction(&Instruction::I32Const(4));
        f.instruction(&Instruction::I32Mul);
        f.instruction(&Instruction::I32Add);
        f.instruction(&Instruction::GlobalSet(0));
        // new_len = 0
        f.instruction(&Instruction::I32Const(0));
        f.instruction(&Instruction::LocalSet(7));
        // i = 0
        f.instruction(&Instruction::I32Const(0));
        f.instruction(&Instruction::LocalSet(2));
        // Loop
        f.instruction(&Instruction::Block(wasm_encoder::BlockType::Empty));
        f.instruction(&Instruction::Loop(wasm_encoder::BlockType::Empty));
        f.instruction(&Instruction::LocalGet(2));
        f.instruction(&Instruction::LocalGet(3));
        f.instruction(&Instruction::I32GeU);
        f.instruction(&Instruction::BrIf(1));
        // val = src_data[i]
        f.instruction(&Instruction::LocalGet(4));
        f.instruction(&Instruction::LocalGet(2));
        f.instruction(&Instruction::I32Const(4));
        f.instruction(&Instruction::I32Mul);
        f.instruction(&Instruction::I32Add);
        f.instruction(&Instruction::I32Load(ma)); // val
        // if fn(val) != 0, push
        f.instruction(&Instruction::LocalGet(4));
        f.instruction(&Instruction::LocalGet(2));
        f.instruction(&Instruction::I32Const(4));
        f.instruction(&Instruction::I32Mul);
        f.instruction(&Instruction::I32Add);
        f.instruction(&Instruction::I32Load(ma)); // val again for fn call
        f.instruction(&Instruction::LocalGet(1)); // fn table index
        let ty_i32_i32 =
            self.lookup_or_register_indirect_type(vec![ValType::I32], vec![ValType::I32]);
        f.instruction(&Instruction::CallIndirect {
            type_index: ty_i32_i32,
            table_index: 0,
        });
        f.instruction(&Instruction::If(wasm_encoder::BlockType::Empty));
        // new_data[new_len] = val (val is on stack from earlier load)
        f.instruction(&Instruction::LocalGet(6));
        f.instruction(&Instruction::LocalGet(7));
        f.instruction(&Instruction::I32Const(4));
        f.instruction(&Instruction::I32Mul);
        f.instruction(&Instruction::I32Add);
        // reload val
        f.instruction(&Instruction::LocalGet(4));
        f.instruction(&Instruction::LocalGet(2));
        f.instruction(&Instruction::I32Const(4));
        f.instruction(&Instruction::I32Mul);
        f.instruction(&Instruction::I32Add);
        f.instruction(&Instruction::I32Load(ma));
        f.instruction(&Instruction::I32Store(ma));
        // new_len++
        f.instruction(&Instruction::LocalGet(7));
        f.instruction(&Instruction::I32Const(1));
        f.instruction(&Instruction::I32Add);
        f.instruction(&Instruction::LocalSet(7));
        f.instruction(&Instruction::End); // end if
        // drop the val that was loaded before the if
        f.instruction(&Instruction::Drop);
        // i++
        f.instruction(&Instruction::LocalGet(2));
        f.instruction(&Instruction::I32Const(1));
        f.instruction(&Instruction::I32Add);
        f.instruction(&Instruction::LocalSet(2));
        f.instruction(&Instruction::Br(0));
        f.instruction(&Instruction::End); // end loop
        f.instruction(&Instruction::End); // end block
        // update new_vec.len = new_len
        f.instruction(&Instruction::LocalGet(5));
        f.instruction(&Instruction::LocalGet(7));
        f.instruction(&Instruction::I32Store(ma));
        // return new_vec
        f.instruction(&Instruction::LocalGet(5));
        f.instruction(&Instruction::End);
        f
    }

    /// __fold_i32(vec_ptr: i32, init: i32, fn_idx: i32) -> i32
    fn build_fold_i32(&self) -> Function {
        let ma = MemArg {
            offset: 0,
            align: 2,
            memory_index: 0,
        };
        // params: 0=vec_ptr, 1=init, 2=fn_idx
        // locals: 3=i, 4=n, 5=src_data, 6=acc
        let mut f = Function::new(vec![(4, ValType::I32)]);
        // n = vec.len
        f.instruction(&Instruction::LocalGet(0));
        f.instruction(&Instruction::I32Load(ma));
        f.instruction(&Instruction::LocalSet(4));
        // src_data = vec.data_ptr
        f.instruction(&Instruction::LocalGet(0));
        f.instruction(&Instruction::I32Const(8));
        f.instruction(&Instruction::I32Add);
        f.instruction(&Instruction::I32Load(ma));
        f.instruction(&Instruction::LocalSet(5));
        // acc = init
        f.instruction(&Instruction::LocalGet(1));
        f.instruction(&Instruction::LocalSet(6));
        // i = 0
        f.instruction(&Instruction::I32Const(0));
        f.instruction(&Instruction::LocalSet(3));
        // Loop
        f.instruction(&Instruction::Block(wasm_encoder::BlockType::Empty));
        f.instruction(&Instruction::Loop(wasm_encoder::BlockType::Empty));
        f.instruction(&Instruction::LocalGet(3));
        f.instruction(&Instruction::LocalGet(4));
        f.instruction(&Instruction::I32GeU);
        f.instruction(&Instruction::BrIf(1));
        // acc = call_indirect(fn_idx, acc, src_data[i])
        f.instruction(&Instruction::LocalGet(6)); // acc
        f.instruction(&Instruction::LocalGet(5));
        f.instruction(&Instruction::LocalGet(3));
        f.instruction(&Instruction::I32Const(4));
        f.instruction(&Instruction::I32Mul);
        f.instruction(&Instruction::I32Add);
        f.instruction(&Instruction::I32Load(ma)); // src_data[i]
        f.instruction(&Instruction::LocalGet(2)); // fn table index
        let ty_i32x2_i32 =
            self.lookup_or_register_indirect_type(vec![ValType::I32; 2], vec![ValType::I32]);
        f.instruction(&Instruction::CallIndirect {
            type_index: ty_i32x2_i32,
            table_index: 0,
        });
        f.instruction(&Instruction::LocalSet(6));
        // i++
        f.instruction(&Instruction::LocalGet(3));
        f.instruction(&Instruction::I32Const(1));
        f.instruction(&Instruction::I32Add);
        f.instruction(&Instruction::LocalSet(3));
        f.instruction(&Instruction::Br(0));
        f.instruction(&Instruction::End);
        f.instruction(&Instruction::End);
        // return acc
        f.instruction(&Instruction::LocalGet(6));
        f.instruction(&Instruction::End);
        f
    }

    /// __map_option_i32(opt_ptr: i32, fn_idx: i32) -> i32 (new option ptr)
    /// Option layout: [tag: i32, payload: i32] — tag 0 = Some, tag 1 = None
    fn build_map_option_i32(&self) -> Function {
        let ma = MemArg {
            offset: 0,
            align: 2,
            memory_index: 0,
        };
        // params: 0=opt_ptr, 1=fn_idx
        // locals: 2=new_opt
        let mut f = Function::new(vec![(1, ValType::I32)]);
        // Allocate new option: 8 bytes
        f.instruction(&Instruction::GlobalGet(0));
        f.instruction(&Instruction::LocalSet(2));
        f.instruction(&Instruction::GlobalGet(0));
        f.instruction(&Instruction::I32Const(8));
        f.instruction(&Instruction::I32Add);
        f.instruction(&Instruction::GlobalSet(0));
        // Read tag
        f.instruction(&Instruction::LocalGet(0));
        f.instruction(&Instruction::I32Load(ma)); // tag
        f.instruction(&Instruction::If(wasm_encoder::BlockType::Empty));
        // tag != 0 → None: copy tag
        f.instruction(&Instruction::LocalGet(2));
        f.instruction(&Instruction::I32Const(1));
        f.instruction(&Instruction::I32Store(ma));
        f.instruction(&Instruction::Else);
        // tag == 0 → Some: new_opt.tag = 0, new_opt.payload = fn(old_payload)
        f.instruction(&Instruction::LocalGet(2));
        f.instruction(&Instruction::I32Const(0));
        f.instruction(&Instruction::I32Store(ma));
        f.instruction(&Instruction::LocalGet(2));
        f.instruction(&Instruction::I32Const(4));
        f.instruction(&Instruction::I32Add);
        // call fn(payload)
        f.instruction(&Instruction::LocalGet(0));
        f.instruction(&Instruction::I32Const(4));
        f.instruction(&Instruction::I32Add);
        f.instruction(&Instruction::I32Load(ma)); // payload
        f.instruction(&Instruction::LocalGet(1)); // fn table index
        let ty_i32_i32 =
            self.lookup_or_register_indirect_type(vec![ValType::I32], vec![ValType::I32]);
        f.instruction(&Instruction::CallIndirect {
            type_index: ty_i32_i32,
            table_index: 0,
        });
        f.instruction(&Instruction::I32Store(ma));
        f.instruction(&Instruction::End);
        // return new_opt
        f.instruction(&Instruction::LocalGet(2));
        f.instruction(&Instruction::End);
        f
    }

    fn build_any_i32(&self) -> Function {
        // any_i32(vec_ptr: i32, fn_idx: i32) -> i32 (0 or 1)
        // params: 0=vec_ptr, 1=fn_idx
        // locals: 2=i, 3=n, 4=src_data
        let ma = MemArg {
            offset: 0,
            align: 2,
            memory_index: 0,
        };
        let mut f = Function::new(vec![(3, ValType::I32)]);

        // n = mem[vec_ptr] (vec length)
        f.instruction(&Instruction::LocalGet(0));
        f.instruction(&Instruction::I32Load(ma));
        f.instruction(&Instruction::LocalSet(3));

        // src_data = mem[vec_ptr + 8] (data pointer)
        f.instruction(&Instruction::LocalGet(0));
        f.instruction(&Instruction::I32Const(8));
        f.instruction(&Instruction::I32Add);
        f.instruction(&Instruction::I32Load(ma));
        f.instruction(&Instruction::LocalSet(4));

        // i = 0
        f.instruction(&Instruction::I32Const(0));
        f.instruction(&Instruction::LocalSet(2));

        // loop
        f.instruction(&Instruction::Block(wasm_encoder::BlockType::Empty));
        f.instruction(&Instruction::Loop(wasm_encoder::BlockType::Empty));

        // if i >= n, break (return false)
        f.instruction(&Instruction::LocalGet(2));
        f.instruction(&Instruction::LocalGet(3));
        f.instruction(&Instruction::I32GeU);
        f.instruction(&Instruction::BrIf(1));

        // elem = src_data[i*4]
        f.instruction(&Instruction::LocalGet(4));
        f.instruction(&Instruction::LocalGet(2));
        f.instruction(&Instruction::I32Const(4));
        f.instruction(&Instruction::I32Mul);
        f.instruction(&Instruction::I32Add);
        f.instruction(&Instruction::I32Load(ma));

        // call_indirect pred(elem) -> bool
        f.instruction(&Instruction::LocalGet(1)); // fn_idx
        let ty_i32_i32 =
            self.lookup_or_register_indirect_type(vec![ValType::I32], vec![ValType::I32]);
        f.instruction(&Instruction::CallIndirect {
            type_index: ty_i32_i32,
            table_index: 0,
        });

        // if result != 0, return 1
        f.instruction(&Instruction::If(wasm_encoder::BlockType::Empty));
        f.instruction(&Instruction::I32Const(1));
        f.instruction(&Instruction::Return);
        f.instruction(&Instruction::End);

        // i++
        f.instruction(&Instruction::LocalGet(2));
        f.instruction(&Instruction::I32Const(1));
        f.instruction(&Instruction::I32Add);
        f.instruction(&Instruction::LocalSet(2));

        f.instruction(&Instruction::Br(0)); // continue loop
        f.instruction(&Instruction::End); // end loop
        f.instruction(&Instruction::End); // end block

        // return 0 (false)
        f.instruction(&Instruction::I32Const(0));
        f.instruction(&Instruction::End);
        f
    }

    fn build_find_i32(&self) -> Function {
        // find_i32(vec_ptr: i32, fn_idx: i32) -> i32 (Option ptr)
        // Returns heap-allocated Option: [tag=0(Some), payload=value] or [tag=1(None)]
        // params: 0=vec_ptr, 1=fn_idx
        // locals: 2=i, 3=n, 4=src_data
        let ma = MemArg {
            offset: 0,
            align: 2,
            memory_index: 0,
        };
        let mut f = Function::new(vec![(3, ValType::I32)]);

        // n = mem[vec_ptr]
        f.instruction(&Instruction::LocalGet(0));
        f.instruction(&Instruction::I32Load(ma));
        f.instruction(&Instruction::LocalSet(3));

        // src_data = mem[vec_ptr + 8]
        f.instruction(&Instruction::LocalGet(0));
        f.instruction(&Instruction::I32Const(8));
        f.instruction(&Instruction::I32Add);
        f.instruction(&Instruction::I32Load(ma));
        f.instruction(&Instruction::LocalSet(4));

        // i = 0
        f.instruction(&Instruction::I32Const(0));
        f.instruction(&Instruction::LocalSet(2));

        // loop
        f.instruction(&Instruction::Block(wasm_encoder::BlockType::Empty));
        f.instruction(&Instruction::Loop(wasm_encoder::BlockType::Empty));

        // if i >= n, break
        f.instruction(&Instruction::LocalGet(2));
        f.instruction(&Instruction::LocalGet(3));
        f.instruction(&Instruction::I32GeU);
        f.instruction(&Instruction::BrIf(1));

        // call_indirect pred(elem) where elem = src_data[i*4]
        f.instruction(&Instruction::LocalGet(4));
        f.instruction(&Instruction::LocalGet(2));
        f.instruction(&Instruction::I32Const(4));
        f.instruction(&Instruction::I32Mul);
        f.instruction(&Instruction::I32Add);
        f.instruction(&Instruction::I32Load(ma));
        f.instruction(&Instruction::LocalGet(1)); // fn_idx
        let ty_i32_i32 =
            self.lookup_or_register_indirect_type(vec![ValType::I32], vec![ValType::I32]);
        f.instruction(&Instruction::CallIndirect {
            type_index: ty_i32_i32,
            table_index: 0,
        });

        // if pred returned true, build Some(elem) and return
        f.instruction(&Instruction::If(wasm_encoder::BlockType::Empty));
        {
            // Allocate Some(elem) on heap: [tag=0][payload=elem]
            f.instruction(&Instruction::GlobalGet(0)); // base ptr (will be result)
            f.instruction(&Instruction::GlobalGet(0));
            f.instruction(&Instruction::I32Const(0)); // tag = Some
            f.instruction(&Instruction::I32Store(ma));
            // payload = elem (reload)
            f.instruction(&Instruction::GlobalGet(0));
            f.instruction(&Instruction::I32Const(4));
            f.instruction(&Instruction::I32Add);
            f.instruction(&Instruction::LocalGet(4));
            f.instruction(&Instruction::LocalGet(2));
            f.instruction(&Instruction::I32Const(4));
            f.instruction(&Instruction::I32Mul);
            f.instruction(&Instruction::I32Add);
            f.instruction(&Instruction::I32Load(ma));
            f.instruction(&Instruction::I32Store(ma));
            // bump heap
            f.instruction(&Instruction::GlobalGet(0));
            f.instruction(&Instruction::I32Const(8));
            f.instruction(&Instruction::I32Add);
            f.instruction(&Instruction::GlobalSet(0));
            // return base ptr
            f.instruction(&Instruction::Return);
        }
        f.instruction(&Instruction::End);

        // i++
        f.instruction(&Instruction::LocalGet(2));
        f.instruction(&Instruction::I32Const(1));
        f.instruction(&Instruction::I32Add);
        f.instruction(&Instruction::LocalSet(2));

        f.instruction(&Instruction::Br(0));
        f.instruction(&Instruction::End); // end loop
        f.instruction(&Instruction::End); // end block

        // Not found: allocate None: [tag=1]
        f.instruction(&Instruction::GlobalGet(0));
        f.instruction(&Instruction::GlobalGet(0));
        f.instruction(&Instruction::I32Const(1)); // tag = None
        f.instruction(&Instruction::I32Store(ma));
        f.instruction(&Instruction::GlobalGet(0));
        f.instruction(&Instruction::I32Const(4));
        f.instruction(&Instruction::I32Add);
        f.instruction(&Instruction::I32Const(0)); // padding
        f.instruction(&Instruction::I32Store(ma));
        f.instruction(&Instruction::GlobalGet(0));
        f.instruction(&Instruction::I32Const(8));
        f.instruction(&Instruction::I32Add);
        f.instruction(&Instruction::GlobalSet(0));
        // base ptr on stack
        f.instruction(&Instruction::End);
        f
    }

    fn build_user_fn(&mut self, func: &MirFunction) -> Function {
        let num_params = func.params.len() as u32;
        let num_locals = func.locals.len() as u32;

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
        self.f64_locals.clear();
        self.i64_locals.clear();
        self.bool_locals.clear();
        for local in &func.locals {
            match &local.ty {
                ark_typecheck::types::Type::String => {
                    self.string_locals.insert(local.id.0);
                }
                ark_typecheck::types::Type::F64 => {
                    self.f64_locals.insert(local.id.0);
                }
                ark_typecheck::types::Type::I64 => {
                    self.i64_locals.insert(local.id.0);
                }
                ark_typecheck::types::Type::Bool => {
                    self.bool_locals.insert(local.id.0);
                }
                ark_typecheck::types::Type::Vec(inner) => {
                    if matches!(inner.as_ref(), ark_typecheck::types::Type::String) {
                        self.vec_string_locals.insert(local.id.0);
                    }
                }
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
    fn scan_string_locals(&mut self, stmts: &[MirStmt]) {
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

    fn is_string_operand(&self, op: &Operand) -> bool {
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
                if matches!(name, "get" | "get_unchecked") {
                    if let Some(Operand::Place(Place::Local(id))) = args.first() {
                        if self.vec_string_locals.contains(&id.0) {
                            return true;
                        }
                    }
                }
                // Check if function returns String
                if self
                    .fn_return_types
                    .get(name)
                    .is_some_and(|t| matches!(t, ark_typecheck::types::Type::String))
                {
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
                        | "parse_i32"
                        | "parse_i64"
                        | "parse_f64"
                        | "fs_read_file"
                        | "fs_write_file"
                        | "map_i32_i32"
                        | "filter_i32"
                        | "fold_i32_i32"
                        | "map_option_i32_i32"
                        | "any_i32"
                        | "find_i32"
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
                if let Some(variants) = self.enum_payload_types.get(enum_name.as_str()) {
                    if let Some((_, types)) = variants.iter().find(|(vn, _)| vn == variant_name) {
                        if let Some(t) = types.get(*index as usize) {
                            if t == "String" {
                                return true;
                            }
                        }
                    }
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

    fn emit_stmt(&mut self, f: &mut Function, stmt: &MirStmt) {
        match stmt {
            MirStmt::Assign(Place::Local(id), Rvalue::Use(op)) => {
                self.emit_operand(f, op);
                f.instruction(&Instruction::LocalSet(id.0));
            }
            MirStmt::CallBuiltin { name, args, .. } => {
                let name = normalize_intrinsic_name(name.as_str());
                match name {
                    "println" => self.emit_println(f, args),
                    "print" => self.emit_print(f, args),
                    "eprintln" => {
                        self.emit_eprintln(f, args);
                    }
                    "print_i32_ln" => {
                        if let Some(arg) = args.first() {
                            self.emit_operand(f, arg);
                            f.instruction(&Instruction::Call(FN_PRINT_I32_LN));
                        }
                    }
                    "print_bool_ln" => {
                        if let Some(arg) = args.first() {
                            self.emit_operand(f, arg);
                            f.instruction(&Instruction::Call(FN_PRINT_BOOL_LN));
                        }
                    }
                    "print_str_ln" => {
                        if let Some(arg) = args.first() {
                            self.emit_operand(f, arg);
                            f.instruction(&Instruction::Call(FN_PRINT_STR_LN));
                        }
                    }
                    "i32_to_string" => {
                        // As statement, result is discarded
                        if let Some(arg) = args.first() {
                            self.emit_operand(f, arg);
                            f.instruction(&Instruction::Call(FN_I32_TO_STR));
                        }
                    }
                    "push" | "set" | "sort_i32" | "sort_String" => {
                        // Void Vec operations — emit inline via Operand::Call path
                        let call_op = Operand::Call(name.to_string(), args.clone());
                        self.emit_operand(f, &call_op);
                    }
                    "panic" | "assert" | "assert_eq" | "assert_ne" | "assert_eq_str"
                    | "assert_eq_i64" => {
                        // void builtins — emit inline via Operand::Call path
                        let call_op = Operand::Call(name.to_string(), args.clone());
                        self.emit_operand(f, &call_op);
                    }
                    "push_char" => {
                        // push_char is mutating — emit call and update the local
                        let call_op = Operand::Call(name.to_string(), args.clone());
                        self.emit_operand(f, &call_op);
                        // push_char returns new str ptr; update the variable
                        if let Some(Operand::Place(Place::Local(lid))) = args.first() {
                            f.instruction(&Instruction::LocalSet(lid.0));
                        } else {
                            f.instruction(&Instruction::Drop);
                        }
                    }
                    "pop" | "get" | "Vec_new_i32" | "Vec_new_String" | "len" | "get_unchecked"
                    | "fs_read_file" | "fs_write_file" | "any_i32" | "find_i32" | "clock_now"
                    | "random_i32" => {
                        // Value-returning Vec operations called as statement — emit and drop result
                        let call_op = Operand::Call(name.to_string(), args.clone());
                        self.emit_operand(f, &call_op);
                        f.instruction(&Instruction::Drop);
                    }
                    other if other.starts_with("Vec_new_") => {
                        // Dynamic Vec_new_<Type> — same as Vec_new_i32
                        let call_op = Operand::Call(other.to_string(), args.clone());
                        self.emit_operand(f, &call_op);
                        f.instruction(&Instruction::Drop);
                    }
                    other => {
                        // User function call
                        for arg in args {
                            self.emit_operand(f, arg);
                        }
                        if let Some(idx) = self.resolve_fn(other) {
                            f.instruction(&Instruction::Call(idx));
                            let returns_value = self
                                .fn_return_types
                                .get(other)
                                .is_some_and(|t| !matches!(t, ark_typecheck::types::Type::Unit));
                            if returns_value {
                                f.instruction(&Instruction::Drop);
                            }
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
                // Track block depth for break/continue
                if let Some(d) = self.loop_depths.last_mut() {
                    *d += 1;
                }
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
                if let Some(d) = self.loop_depths.last_mut() {
                    *d -= 1;
                }
            }
            MirStmt::WhileStmt { cond, body } => {
                f.instruction(&Instruction::Block(wasm_encoder::BlockType::Empty)); // break target
                f.instruction(&Instruction::Loop(wasm_encoder::BlockType::Empty)); // continue target
                self.emit_operand(f, cond);
                f.instruction(&Instruction::I32Eqz);
                f.instruction(&Instruction::BrIf(1)); // exit if cond is false
                self.loop_depths.push(0);
                for s in body {
                    self.emit_stmt(f, s);
                }
                self.loop_depths.pop();
                f.instruction(&Instruction::Br(0)); // loop back
                f.instruction(&Instruction::End); // end loop
                f.instruction(&Instruction::End); // end block
            }
            MirStmt::Break => {
                // break: jump to the outer block of the enclosing while
                // br(extra_depth + 1): +1 because loop label is between us and block
                let depth = self.loop_depths.last().copied().unwrap_or(0) + 1;
                f.instruction(&Instruction::Br(depth));
            }
            MirStmt::Continue => {
                // continue: jump to the loop label
                let depth = self.loop_depths.last().copied().unwrap_or(0);
                f.instruction(&Instruction::Br(depth));
            }
            MirStmt::Return(op) => {
                if let Some(val) = op {
                    self.emit_operand(f, val);
                }
                f.instruction(&Instruction::Return);
            }
            other => {
                eprintln!(
                    "ICE: unhandled statement in emit_stmt: {:?}",
                    std::mem::discriminant(other)
                );
            }
        }
    }

    fn emit_println(&mut self, f: &mut Function, args: &[Operand]) {
        if let Some(arg) = args.first() {
            match arg {
                Operand::ConstString(s) => {
                    let msg = format!("{}\n", s);
                    let (offset, len) = self.alloc_string(&msg);
                    self.emit_fd_write(f, 1, offset, len);
                }
                Operand::Call(name, inner_args) => {
                    match normalize_intrinsic_name(name.as_str()) {
                        "i32_to_string" => {
                            if let Some(inner) = inner_args.first() {
                                self.emit_operand(f, inner);
                                f.instruction(&Instruction::Call(FN_PRINT_I32_LN));
                            }
                        }
                        "bool_to_string" => {
                            if let Some(inner) = inner_args.first() {
                                self.emit_operand(f, inner);
                                f.instruction(&Instruction::Call(FN_PRINT_BOOL_LN));
                            }
                        }
                        "f64_to_string" => {
                            // Convert f64 to string, then print as string
                            if let Some(inner) = inner_args.first() {
                                self.emit_operand(f, inner);
                                f.instruction(&Instruction::Call(FN_F64_TO_STR));
                                f.instruction(&Instruction::Call(FN_PRINT_STR_LN));
                            }
                        }
                        "i64_to_string" => {
                            // Convert i64 to string, then print as string
                            if let Some(inner) = inner_args.first() {
                                self.emit_operand(f, inner);
                                f.instruction(&Instruction::Call(FN_I64_TO_STR));
                                f.instruction(&Instruction::Call(FN_PRINT_STR_LN));
                            }
                        }
                        "concat" => {
                            // Concatenate strings, then print result
                            for a in inner_args {
                                self.emit_operand(f, a);
                            }
                            f.instruction(&Instruction::Call(FN_CONCAT));
                            f.instruction(&Instruction::Call(FN_PRINT_STR_LN));
                        }
                        "char_to_string" => {
                            // Write char byte to scratch, print it + newline
                            if let Some(inner) = inner_args.first() {
                                let ma0 = MemArg {
                                    offset: 0,
                                    align: 0,
                                    memory_index: 0,
                                };
                                let ma2 = MemArg {
                                    offset: 0,
                                    align: 2,
                                    memory_index: 0,
                                };
                                let char_addr = SCRATCH + 12;
                                f.instruction(&Instruction::I32Const(char_addr as i32));
                                self.emit_operand(f, inner);
                                f.instruction(&Instruction::I32Store8(ma0));
                                // Print char
                                f.instruction(&Instruction::I32Const(IOV_BASE as i32));
                                f.instruction(&Instruction::I32Const(char_addr as i32));
                                f.instruction(&Instruction::I32Store(ma2));
                                f.instruction(&Instruction::I32Const((IOV_BASE + 4) as i32));
                                f.instruction(&Instruction::I32Const(1));
                                f.instruction(&Instruction::I32Store(ma2));
                                f.instruction(&Instruction::I32Const(1));
                                f.instruction(&Instruction::I32Const(IOV_BASE as i32));
                                f.instruction(&Instruction::I32Const(1));
                                f.instruction(&Instruction::I32Const(NWRITTEN as i32));
                                f.instruction(&Instruction::Call(FN_FD_WRITE));
                                f.instruction(&Instruction::Drop);
                                // Print newline
                                self.emit_static_print(f, NEWLINE, 1);
                            }
                        }
                        "String_from" => {
                            // String_from("literal") — just print the literal
                            if let Some(Operand::ConstString(s)) = inner_args.first() {
                                let msg = format!("{}\n", s);
                                let (offset, len) = self.alloc_string(&msg);
                                self.emit_fd_write(f, 1, offset, len);
                            } else if let Some(inner) = inner_args.first() {
                                self.emit_operand(f, inner);
                                f.instruction(&Instruction::Call(FN_PRINT_STR_LN));
                            }
                        }
                        other => {
                            // Check if this is a builtin inline function (not a real fn index)
                            if self.resolve_fn(other).is_some() {
                                // Real user function call — emit args, call, then print result
                                for a in inner_args {
                                    self.emit_operand(f, a);
                                }
                                let idx = self.resolve_fn(other).unwrap();
                                f.instruction(&Instruction::Call(idx));
                                let is_str = self.fn_return_types.get(other).is_some_and(|t| {
                                    matches!(t, ark_typecheck::types::Type::String)
                                });
                                let args_suggest_str =
                                    inner_args.iter().any(|a| self.is_string_operand(a));
                                if is_str || args_suggest_str {
                                    f.instruction(&Instruction::Call(FN_PRINT_STR_LN));
                                } else {
                                    f.instruction(&Instruction::Call(FN_PRINT_I32_LN));
                                }
                            } else {
                                // Inline builtin (len, get, etc.) — emit full operand
                                self.emit_operand(f, arg);
                                if self.is_string_operand(arg) {
                                    f.instruction(&Instruction::Call(FN_PRINT_STR_LN));
                                } else {
                                    f.instruction(&Instruction::Call(FN_PRINT_I32_LN));
                                }
                            }
                        }
                    }
                }
                Operand::Place(Place::Local(id)) => {
                    f.instruction(&Instruction::LocalGet(id.0));
                    if self.string_locals.contains(&id.0) {
                        f.instruction(&Instruction::Call(FN_PRINT_STR_LN));
                    } else {
                        f.instruction(&Instruction::Call(FN_PRINT_I32_LN));
                    }
                }
                _ => {
                    // Generic: emit operand and dispatch based on type
                    self.emit_operand(f, arg);
                    if self.is_string_operand(arg) {
                        f.instruction(&Instruction::Call(FN_PRINT_STR_LN));
                    } else {
                        f.instruction(&Instruction::Call(FN_PRINT_I32_LN));
                    }
                }
            }
        }
    }

    fn emit_eprintln(&mut self, f: &mut Function, args: &[Operand]) {
        // Write to stderr (fd=2)
        if let Some(arg) = args.first() {
            match arg {
                Operand::ConstString(s) => {
                    let msg = format!("{}\n", s);
                    let (offset, len) = self.alloc_string(&msg);
                    self.emit_fd_write(f, 2, offset, len);
                }
                _ => {
                    // For now, print as string literal to stderr
                    self.emit_operand(f, arg);
                    f.instruction(&Instruction::Drop);
                }
            }
        }
    }

    fn emit_print(&mut self, f: &mut Function, args: &[Operand]) {
        if let Some(arg) = args.first() {
            match arg {
                Operand::ConstString(s) => {
                    let (offset, len) = self.alloc_string(s);
                    self.emit_fd_write(f, 1, offset, len);
                }
                _ => {
                    // Print i32 without newline — call i32_to_string then fd_write
                    self.emit_operand(f, arg);
                    f.instruction(&Instruction::Call(FN_I32_TO_STR));
                    let ma2 = MemArg {
                        offset: 0,
                        align: 2,
                        memory_index: 0,
                    };
                    // iov setup from SCRATCH
                    f.instruction(&Instruction::I32Const(IOV_BASE as i32));
                    f.instruction(&Instruction::I32Const(SCRATCH as i32));
                    f.instruction(&Instruction::I32Load(ma2));
                    f.instruction(&Instruction::I32Store(ma2));
                    f.instruction(&Instruction::I32Const((IOV_BASE + 4) as i32));
                    f.instruction(&Instruction::I32Const((SCRATCH + 4) as i32));
                    f.instruction(&Instruction::I32Load(ma2));
                    f.instruction(&Instruction::I32Store(ma2));
                    f.instruction(&Instruction::I32Const(1));
                    f.instruction(&Instruction::I32Const(IOV_BASE as i32));
                    f.instruction(&Instruction::I32Const(1));
                    f.instruction(&Instruction::I32Const(NWRITTEN as i32));
                    f.instruction(&Instruction::Call(FN_FD_WRITE));
                    f.instruction(&Instruction::Drop);
                }
            }
        }
    }

    fn emit_fd_write(&mut self, f: &mut Function, fd: u32, str_offset: u32, str_len: u32) {
        let ma2 = MemArg {
            offset: 0,
            align: 2,
            memory_index: 0,
        };
        f.instruction(&Instruction::I32Const(IOV_BASE as i32));
        f.instruction(&Instruction::I32Const(str_offset as i32));
        f.instruction(&Instruction::I32Store(ma2));
        f.instruction(&Instruction::I32Const((IOV_BASE + 4) as i32));
        f.instruction(&Instruction::I32Const(str_len as i32));
        f.instruction(&Instruction::I32Store(ma2));
        f.instruction(&Instruction::I32Const(fd as i32));
        f.instruction(&Instruction::I32Const(IOV_BASE as i32));
        f.instruction(&Instruction::I32Const(1));
        f.instruction(&Instruction::I32Const(NWRITTEN as i32));
        f.instruction(&Instruction::Call(FN_FD_WRITE));
        f.instruction(&Instruction::Drop);
    }

    fn emit_operand(&mut self, f: &mut Function, op: &Operand) {
        match op {
            Operand::ConstI32(v) => {
                f.instruction(&Instruction::I32Const(*v));
            }
            Operand::ConstI64(v) => {
                f.instruction(&Instruction::I64Const(*v));
            }
            Operand::ConstF64(v) => {
                f.instruction(&Instruction::F64Const(*v));
            }
            Operand::ConstF32(v) => {
                f.instruction(&Instruction::F32Const(*v));
            }
            Operand::ConstBool(v) => {
                f.instruction(&Instruction::I32Const(if *v { 1 } else { 0 }));
            }
            Operand::ConstChar(c) => {
                f.instruction(&Instruction::I32Const(*c as i32));
            }
            Operand::ConstString(s) => {
                // Allocate as length-prefixed string, return pointer
                let ptr = self.alloc_length_prefixed_string(s);
                f.instruction(&Instruction::I32Const(ptr as i32));
            }
            Operand::Place(Place::Local(id)) => {
                f.instruction(&Instruction::LocalGet(id.0));
            }
            Operand::BinOp(op, left, right) => {
                let is_f64 = self.is_f64_operand(left) || self.is_f64_operand(right);
                let is_i64 = !is_f64 && (self.is_i64_operand(left) || self.is_i64_operand(right));
                if is_f64 {
                    // Promote both operands to f64 if needed
                    self.emit_f64_operand(f, left);
                    self.emit_f64_operand(f, right);
                    self.emit_binop_f64(f, op);
                } else if is_i64 {
                    // Promote both operands to i64 if needed
                    self.emit_i64_operand(f, left);
                    self.emit_i64_operand(f, right);
                    self.emit_binop_i64(f, op);
                } else {
                    self.emit_operand(f, left);
                    self.emit_operand(f, right);
                    self.emit_binop(f, op);
                }
            }
            Operand::UnaryOp(op, inner) => {
                self.emit_unaryop(f, op, inner);
            }
            Operand::Call(name, args) => {
                let name = normalize_intrinsic_name(name.as_str());
                match name {
                    "to_string" => {
                        // Polymorphic to_string: dispatch based on argument type
                        if let Some(arg) = args.first() {
                            if self.is_string_operand(arg) {
                                // Already a string — just emit it
                                self.emit_operand(f, arg);
                            } else if self.is_f64_operand(arg) {
                                let converted =
                                    Operand::Call("f64_to_string".to_string(), args.clone());
                                self.emit_operand(f, &converted);
                            } else if self.is_i64_operand(arg) {
                                let converted =
                                    Operand::Call("i64_to_string".to_string(), args.clone());
                                self.emit_operand(f, &converted);
                            } else if matches!(arg, Operand::ConstBool(_))
                                || self.is_bool_operand(arg)
                            {
                                let converted =
                                    Operand::Call("bool_to_string".to_string(), args.clone());
                                self.emit_operand(f, &converted);
                            } else {
                                // Default: i32_to_string
                                let converted =
                                    Operand::Call("i32_to_string".to_string(), args.clone());
                                self.emit_operand(f, &converted);
                            }
                        }
                    }
                    "i32_to_string" => {
                        // Convert i32 to length-prefixed string on heap
                        if let Some(a) = args.first() {
                            self.emit_operand(f, a);
                        }
                        f.instruction(&Instruction::Call(FN_I32_TO_STR));
                        // After __i32_to_string: SCRATCH has offset, SCRATCH+4 has len
                        // Allocate length-prefixed string on heap
                        let ma2 = MemArg {
                            offset: 0,
                            align: 2,
                            memory_index: 0,
                        };
                        let ma0 = MemArg {
                            offset: 0,
                            align: 0,
                            memory_index: 0,
                        };
                        // Write length at heap_ptr
                        f.instruction(&Instruction::GlobalGet(0));
                        f.instruction(&Instruction::I32Const(SCRATCH as i32 + 4));
                        f.instruction(&Instruction::I32Load(ma2)); // len
                        f.instruction(&Instruction::I32Store(ma2));
                        // Copy digits from I32BUF area to heap_ptr+4
                        // Use a byte-by-byte copy loop
                        // local for loop counter - we don't have a dedicated local,
                        // so use NWRITTEN as temp storage
                        f.instruction(&Instruction::I32Const(NWRITTEN as i32));
                        f.instruction(&Instruction::I32Const(0));
                        f.instruction(&Instruction::I32Store(ma2)); // i = 0
                        f.instruction(&Instruction::Block(wasm_encoder::BlockType::Empty));
                        f.instruction(&Instruction::Loop(wasm_encoder::BlockType::Empty));
                        // if i >= len, break
                        f.instruction(&Instruction::I32Const(NWRITTEN as i32));
                        f.instruction(&Instruction::I32Load(ma2)); // i
                        f.instruction(&Instruction::I32Const(SCRATCH as i32 + 4));
                        f.instruction(&Instruction::I32Load(ma2)); // len
                        f.instruction(&Instruction::I32GeU);
                        f.instruction(&Instruction::BrIf(1));
                        // mem[heap_ptr + 4 + i] = mem[scratch_offset + i]
                        f.instruction(&Instruction::GlobalGet(0));
                        f.instruction(&Instruction::I32Const(4));
                        f.instruction(&Instruction::I32Add);
                        f.instruction(&Instruction::I32Const(NWRITTEN as i32));
                        f.instruction(&Instruction::I32Load(ma2)); // i
                        f.instruction(&Instruction::I32Add); // dst = heap+4+i
                        f.instruction(&Instruction::I32Const(SCRATCH as i32));
                        f.instruction(&Instruction::I32Load(ma2)); // offset
                        f.instruction(&Instruction::I32Const(NWRITTEN as i32));
                        f.instruction(&Instruction::I32Load(ma2)); // i
                        f.instruction(&Instruction::I32Add); // src = offset+i
                        f.instruction(&Instruction::I32Load8U(ma0));
                        f.instruction(&Instruction::I32Store8(ma0));
                        // i += 1
                        f.instruction(&Instruction::I32Const(NWRITTEN as i32));
                        f.instruction(&Instruction::I32Const(NWRITTEN as i32));
                        f.instruction(&Instruction::I32Load(ma2));
                        f.instruction(&Instruction::I32Const(1));
                        f.instruction(&Instruction::I32Add);
                        f.instruction(&Instruction::I32Store(ma2));
                        f.instruction(&Instruction::Br(0));
                        f.instruction(&Instruction::End); // end loop
                        f.instruction(&Instruction::End); // end block
                        // Result = heap_ptr + 4 (data pointer)
                        f.instruction(&Instruction::GlobalGet(0));
                        f.instruction(&Instruction::I32Const(4));
                        f.instruction(&Instruction::I32Add);
                        // Bump heap: heap_ptr += 4 + len
                        f.instruction(&Instruction::GlobalGet(0));
                        f.instruction(&Instruction::I32Const(4));
                        f.instruction(&Instruction::I32Add);
                        f.instruction(&Instruction::I32Const(SCRATCH as i32 + 4));
                        f.instruction(&Instruction::I32Load(ma2));
                        f.instruction(&Instruction::I32Add);
                        f.instruction(&Instruction::GlobalSet(0));
                    }
                    "bool_to_string" => {
                        // Convert bool (i32) to "true" or "false" string
                        // Pre-allocate both string literals
                        let true_ptr = self.alloc_length_prefixed_string("true");
                        let false_ptr = self.alloc_length_prefixed_string("false");
                        if let Some(a) = args.first() {
                            self.emit_operand(f, a);
                        }
                        f.instruction(&Instruction::If(wasm_encoder::BlockType::Result(
                            wasm_encoder::ValType::I32,
                        )));
                        f.instruction(&Instruction::I32Const(true_ptr as i32));
                        f.instruction(&Instruction::Else);
                        f.instruction(&Instruction::I32Const(false_ptr as i32));
                        f.instruction(&Instruction::End);
                    }
                    "String_from" => {
                        // String_from("literal") → allocate length-prefixed string, return ptr
                        if let Some(Operand::ConstString(s)) = args.first() {
                            let ptr = self.alloc_length_prefixed_string(s);
                            f.instruction(&Instruction::I32Const(ptr as i32));
                        } else if let Some(a) = args.first() {
                            // Pass through the inner operand
                            self.emit_operand(f, a);
                        } else {
                            f.instruction(&Instruction::I32Const(0));
                        }
                    }
                    "eq" => {
                        // String equality: eq(a, b) -> bool (i32)
                        for a in args {
                            self.emit_operand(f, a);
                        }
                        f.instruction(&Instruction::Call(FN_STR_EQ));
                    }
                    "concat" => {
                        // String concatenation: concat(a, b) -> new string ptr
                        for a in args {
                            self.emit_operand(f, a);
                        }
                        f.instruction(&Instruction::Call(FN_CONCAT));
                    }
                    "f64_to_string" => {
                        // Convert f64 to length-prefixed string
                        if let Some(a) = args.first() {
                            self.emit_operand(f, a);
                        }
                        f.instruction(&Instruction::Call(FN_F64_TO_STR));
                    }
                    "i64_to_string" => {
                        // Convert i64 to length-prefixed string
                        if let Some(a) = args.first() {
                            self.emit_operand(f, a);
                        }
                        f.instruction(&Instruction::Call(FN_I64_TO_STR));
                    }
                    name if name == "Vec_new_i32"
                        || name == "Vec_new_String"
                        || name.starts_with("Vec_new_") =>
                    {
                        // Allocate Vec header: {len:0, cap:8, data_ptr}
                        let ma = MemArg {
                            offset: 0,
                            align: 2,
                            memory_index: 0,
                        };
                        // len = 0
                        f.instruction(&Instruction::GlobalGet(0));
                        f.instruction(&Instruction::I32Const(0));
                        f.instruction(&Instruction::I32Store(ma));
                        // cap = 8
                        f.instruction(&Instruction::GlobalGet(0));
                        f.instruction(&Instruction::I32Const(4));
                        f.instruction(&Instruction::I32Add);
                        f.instruction(&Instruction::I32Const(8));
                        f.instruction(&Instruction::I32Store(ma));
                        // bump heap past header (12 bytes)
                        f.instruction(&Instruction::GlobalGet(0));
                        f.instruction(&Instruction::I32Const(12));
                        f.instruction(&Instruction::I32Add);
                        f.instruction(&Instruction::GlobalSet(0));
                        // Store data_ptr at header+8
                        f.instruction(&Instruction::GlobalGet(0));
                        f.instruction(&Instruction::I32Const(12));
                        f.instruction(&Instruction::I32Sub);
                        f.instruction(&Instruction::I32Const(8));
                        f.instruction(&Instruction::I32Add);
                        f.instruction(&Instruction::GlobalGet(0)); // data_ptr = current heap
                        f.instruction(&Instruction::I32Store(ma));
                        // bump heap past data (8 * 4 = 32 bytes for initial cap=8)
                        f.instruction(&Instruction::GlobalGet(0));
                        f.instruction(&Instruction::I32Const(32));
                        f.instruction(&Instruction::I32Add);
                        f.instruction(&Instruction::GlobalSet(0));
                        // result: header ptr (heap_ptr - 12 - 32 = heap_ptr - 44)
                        f.instruction(&Instruction::GlobalGet(0));
                        f.instruction(&Instruction::I32Const(44));
                        f.instruction(&Instruction::I32Sub);
                    }
                    "push" => {
                        // push(v, x): grow if needed, store x at data[len], increment len
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

                        // Check if len >= cap, if so grow
                        if let Some(v) = args.first() {
                            self.emit_operand(f, v);
                        }
                        f.instruction(&Instruction::I32Load(ma)); // len
                        if let Some(v) = args.first() {
                            self.emit_operand(f, v);
                        }
                        f.instruction(&Instruction::I32Const(4));
                        f.instruction(&Instruction::I32Add);
                        f.instruction(&Instruction::I32Load(ma)); // cap
                        f.instruction(&Instruction::I32GeU); // len >= cap?
                        f.instruction(&Instruction::If(wasm_encoder::BlockType::Empty));
                        {
                            // Save new_data_ptr = heap_ptr to SCRATCH+8
                            f.instruction(&Instruction::I32Const((SCRATCH + 8) as i32));
                            f.instruction(&Instruction::GlobalGet(0));
                            f.instruction(&Instruction::I32Store(ma));

                            // Bump heap: heap_ptr += old_cap * 2 * 4
                            f.instruction(&Instruction::GlobalGet(0));
                            if let Some(v) = args.first() {
                                self.emit_operand(f, v);
                            }
                            f.instruction(&Instruction::I32Const(4));
                            f.instruction(&Instruction::I32Add);
                            f.instruction(&Instruction::I32Load(ma)); // old_cap
                            f.instruction(&Instruction::I32Const(8)); // * 2 * 4
                            f.instruction(&Instruction::I32Mul);
                            f.instruction(&Instruction::I32Add);
                            f.instruction(&Instruction::GlobalSet(0));

                            // Copy old data byte-by-byte: i=0..len*4
                            f.instruction(&Instruction::I32Const(NWRITTEN as i32));
                            f.instruction(&Instruction::I32Const(0));
                            f.instruction(&Instruction::I32Store(ma)); // i = 0
                            f.instruction(&Instruction::Block(wasm_encoder::BlockType::Empty));
                            f.instruction(&Instruction::Loop(wasm_encoder::BlockType::Empty));
                            // if i >= len*4, break
                            f.instruction(&Instruction::I32Const(NWRITTEN as i32));
                            f.instruction(&Instruction::I32Load(ma)); // i
                            if let Some(v) = args.first() {
                                self.emit_operand(f, v);
                            }
                            f.instruction(&Instruction::I32Load(ma)); // len
                            f.instruction(&Instruction::I32Const(4));
                            f.instruction(&Instruction::I32Mul); // len*4
                            f.instruction(&Instruction::I32GeU);
                            f.instruction(&Instruction::BrIf(1));
                            // new_data[i] = old_data[i]
                            f.instruction(&Instruction::I32Const((SCRATCH + 8) as i32));
                            f.instruction(&Instruction::I32Load(ma)); // new_data
                            f.instruction(&Instruction::I32Const(NWRITTEN as i32));
                            f.instruction(&Instruction::I32Load(ma)); // i
                            f.instruction(&Instruction::I32Add); // dst
                            if let Some(v) = args.first() {
                                self.emit_operand(f, v);
                            }
                            f.instruction(&Instruction::I32Const(8));
                            f.instruction(&Instruction::I32Add);
                            f.instruction(&Instruction::I32Load(ma)); // old_data
                            f.instruction(&Instruction::I32Const(NWRITTEN as i32));
                            f.instruction(&Instruction::I32Load(ma)); // i
                            f.instruction(&Instruction::I32Add); // src
                            f.instruction(&Instruction::I32Load8U(ma0));
                            f.instruction(&Instruction::I32Store8(ma0));
                            // i += 1
                            f.instruction(&Instruction::I32Const(NWRITTEN as i32));
                            f.instruction(&Instruction::I32Const(NWRITTEN as i32));
                            f.instruction(&Instruction::I32Load(ma));
                            f.instruction(&Instruction::I32Const(1));
                            f.instruction(&Instruction::I32Add);
                            f.instruction(&Instruction::I32Store(ma));
                            f.instruction(&Instruction::Br(0));
                            f.instruction(&Instruction::End); // end loop
                            f.instruction(&Instruction::End); // end block

                            // Update v.data_ptr = new_data
                            if let Some(v) = args.first() {
                                self.emit_operand(f, v);
                            }
                            f.instruction(&Instruction::I32Const(8));
                            f.instruction(&Instruction::I32Add);
                            f.instruction(&Instruction::I32Const((SCRATCH + 8) as i32));
                            f.instruction(&Instruction::I32Load(ma));
                            f.instruction(&Instruction::I32Store(ma));

                            // Update v.cap = old_cap * 2
                            if let Some(v) = args.first() {
                                self.emit_operand(f, v);
                            }
                            f.instruction(&Instruction::I32Const(4));
                            f.instruction(&Instruction::I32Add);
                            if let Some(v) = args.first() {
                                self.emit_operand(f, v);
                            }
                            f.instruction(&Instruction::I32Const(4));
                            f.instruction(&Instruction::I32Add);
                            f.instruction(&Instruction::I32Load(ma)); // old_cap
                            f.instruction(&Instruction::I32Const(2));
                            f.instruction(&Instruction::I32Mul); // new_cap
                            f.instruction(&Instruction::I32Store(ma));
                        }
                        f.instruction(&Instruction::End); // end if

                        // Now do the actual push: data[len] = value
                        if let Some(v) = args.first() {
                            self.emit_operand(f, v);
                        }
                        f.instruction(&Instruction::I32Const(8));
                        f.instruction(&Instruction::I32Add);
                        f.instruction(&Instruction::I32Load(ma)); // data_ptr
                        if let Some(v) = args.first() {
                            self.emit_operand(f, v);
                        }
                        f.instruction(&Instruction::I32Load(ma)); // len
                        f.instruction(&Instruction::I32Const(4));
                        f.instruction(&Instruction::I32Mul);
                        f.instruction(&Instruction::I32Add); // data_ptr + len*4
                        if let Some(x) = args.get(1) {
                            self.emit_operand(f, x);
                        }
                        f.instruction(&Instruction::I32Store(ma));
                        // Increment len
                        if let Some(v) = args.first() {
                            self.emit_operand(f, v);
                        }
                        if let Some(v) = args.first() {
                            self.emit_operand(f, v);
                        }
                        f.instruction(&Instruction::I32Load(ma));
                        f.instruction(&Instruction::I32Const(1));
                        f.instruction(&Instruction::I32Add);
                        f.instruction(&Instruction::I32Store(ma));
                    }
                    "len" => {
                        let ma = MemArg {
                            offset: 0,
                            align: 2,
                            memory_index: 0,
                        };
                        if let Some(v) = args.first() {
                            if self.is_string_operand(v) {
                                // String len: load from ptr - 4
                                self.emit_operand(f, v);
                                f.instruction(&Instruction::I32Const(4));
                                f.instruction(&Instruction::I32Sub);
                                f.instruction(&Instruction::I32Load(ma));
                            } else {
                                // Vec len: load from v+0
                                self.emit_operand(f, v);
                                f.instruction(&Instruction::I32Load(ma));
                            }
                        }
                    }
                    "get_unchecked" => {
                        // get_unchecked(v, i): load data[i]
                        let ma = MemArg {
                            offset: 0,
                            align: 2,
                            memory_index: 0,
                        };
                        // Load data_ptr
                        if let Some(v) = args.first() {
                            self.emit_operand(f, v);
                        }
                        f.instruction(&Instruction::I32Const(8));
                        f.instruction(&Instruction::I32Add);
                        f.instruction(&Instruction::I32Load(ma));
                        // Add i*4
                        if let Some(i) = args.get(1) {
                            self.emit_operand(f, i);
                        }
                        f.instruction(&Instruction::I32Const(4));
                        f.instruction(&Instruction::I32Mul);
                        f.instruction(&Instruction::I32Add);
                        f.instruction(&Instruction::I32Load(ma));
                    }
                    "get" => {
                        // get(v, i) -> Option<i32>: bounds check, return Some(data[i]) or None
                        let ma = MemArg {
                            offset: 0,
                            align: 2,
                            memory_index: 0,
                        };
                        // Check i < len
                        if let Some(i_arg) = args.get(1) {
                            self.emit_operand(f, i_arg);
                        }
                        if let Some(v) = args.first() {
                            self.emit_operand(f, v);
                        }
                        f.instruction(&Instruction::I32Load(ma)); // len
                        f.instruction(&Instruction::I32LtU);
                        f.instruction(&Instruction::If(wasm_encoder::BlockType::Result(
                            ValType::I32,
                        )));
                        if let Some(d) = self.loop_depths.last_mut() {
                            *d += 1;
                        }
                        // In bounds: construct Some(data[i])
                        // Allocate enum: tag=0 (Some), payload=data[i]
                        f.instruction(&Instruction::GlobalGet(0));
                        f.instruction(&Instruction::I32Const(0)); // tag = Some
                        f.instruction(&Instruction::I32Store(ma));
                        f.instruction(&Instruction::GlobalGet(0));
                        f.instruction(&Instruction::I32Const(4));
                        f.instruction(&Instruction::I32Add);
                        // Load data[i]
                        if let Some(v) = args.first() {
                            self.emit_operand(f, v);
                        }
                        f.instruction(&Instruction::I32Const(8));
                        f.instruction(&Instruction::I32Add);
                        f.instruction(&Instruction::I32Load(ma)); // data_ptr
                        if let Some(i_arg) = args.get(1) {
                            self.emit_operand(f, i_arg);
                        }
                        f.instruction(&Instruction::I32Const(4));
                        f.instruction(&Instruction::I32Mul);
                        f.instruction(&Instruction::I32Add);
                        f.instruction(&Instruction::I32Load(ma)); // data[i]
                        f.instruction(&Instruction::I32Store(ma)); // store payload
                        f.instruction(&Instruction::GlobalGet(0)); // result ptr
                        f.instruction(&Instruction::GlobalGet(0));
                        f.instruction(&Instruction::I32Const(8));
                        f.instruction(&Instruction::I32Add);
                        f.instruction(&Instruction::GlobalSet(0));
                        f.instruction(&Instruction::Else);
                        // Out of bounds: construct None
                        f.instruction(&Instruction::GlobalGet(0));
                        f.instruction(&Instruction::I32Const(1)); // tag = None
                        f.instruction(&Instruction::I32Store(ma));
                        f.instruction(&Instruction::GlobalGet(0)); // result ptr
                        f.instruction(&Instruction::GlobalGet(0));
                        f.instruction(&Instruction::I32Const(4));
                        f.instruction(&Instruction::I32Add);
                        f.instruction(&Instruction::GlobalSet(0));
                        f.instruction(&Instruction::End);
                        if let Some(d) = self.loop_depths.last_mut() {
                            *d -= 1;
                        }
                    }
                    "set" => {
                        // set(v, i, x): store x at data[i]
                        let ma = MemArg {
                            offset: 0,
                            align: 2,
                            memory_index: 0,
                        };
                        if let Some(v) = args.first() {
                            self.emit_operand(f, v);
                        }
                        f.instruction(&Instruction::I32Const(8));
                        f.instruction(&Instruction::I32Add);
                        f.instruction(&Instruction::I32Load(ma)); // data_ptr
                        if let Some(i_arg) = args.get(1) {
                            self.emit_operand(f, i_arg);
                        }
                        f.instruction(&Instruction::I32Const(4));
                        f.instruction(&Instruction::I32Mul);
                        f.instruction(&Instruction::I32Add);
                        if let Some(x) = args.get(2) {
                            self.emit_operand(f, x);
                        }
                        f.instruction(&Instruction::I32Store(ma));
                    }
                    "pop" => {
                        // pop(v) -> Option<i32>: if len > 0, decrement len, return Some(data[len-1]); else None
                        let ma = MemArg {
                            offset: 0,
                            align: 2,
                            memory_index: 0,
                        };
                        // Check len > 0
                        if let Some(v) = args.first() {
                            self.emit_operand(f, v);
                        }
                        f.instruction(&Instruction::I32Load(ma)); // len
                        f.instruction(&Instruction::I32Const(0));
                        f.instruction(&Instruction::I32GtU);
                        f.instruction(&Instruction::If(wasm_encoder::BlockType::Result(
                            ValType::I32,
                        )));
                        if let Some(d) = self.loop_depths.last_mut() {
                            *d += 1;
                        }
                        // Decrement len
                        if let Some(v) = args.first() {
                            self.emit_operand(f, v);
                        }
                        if let Some(v) = args.first() {
                            self.emit_operand(f, v);
                        }
                        f.instruction(&Instruction::I32Load(ma)); // current len
                        f.instruction(&Instruction::I32Const(1));
                        f.instruction(&Instruction::I32Sub);
                        f.instruction(&Instruction::I32Store(ma)); // store new len
                        // Construct Some(data[new_len])
                        f.instruction(&Instruction::GlobalGet(0));
                        f.instruction(&Instruction::I32Const(0)); // tag = Some
                        f.instruction(&Instruction::I32Store(ma));
                        f.instruction(&Instruction::GlobalGet(0));
                        f.instruction(&Instruction::I32Const(4));
                        f.instruction(&Instruction::I32Add);
                        // Load data[new_len]: data_ptr + new_len * 4
                        if let Some(v) = args.first() {
                            self.emit_operand(f, v);
                        }
                        f.instruction(&Instruction::I32Const(8));
                        f.instruction(&Instruction::I32Add);
                        f.instruction(&Instruction::I32Load(ma)); // data_ptr
                        if let Some(v) = args.first() {
                            self.emit_operand(f, v);
                        }
                        f.instruction(&Instruction::I32Load(ma)); // new len
                        f.instruction(&Instruction::I32Const(4));
                        f.instruction(&Instruction::I32Mul);
                        f.instruction(&Instruction::I32Add);
                        f.instruction(&Instruction::I32Load(ma)); // data[new_len]
                        f.instruction(&Instruction::I32Store(ma)); // store payload
                        f.instruction(&Instruction::GlobalGet(0)); // result ptr
                        f.instruction(&Instruction::GlobalGet(0));
                        f.instruction(&Instruction::I32Const(8));
                        f.instruction(&Instruction::I32Add);
                        f.instruction(&Instruction::GlobalSet(0));
                        f.instruction(&Instruction::Else);
                        // Empty: construct None
                        f.instruction(&Instruction::GlobalGet(0));
                        f.instruction(&Instruction::I32Const(1)); // tag = None
                        f.instruction(&Instruction::I32Store(ma));
                        f.instruction(&Instruction::GlobalGet(0)); // result ptr
                        f.instruction(&Instruction::GlobalGet(0));
                        f.instruction(&Instruction::I32Const(4));
                        f.instruction(&Instruction::I32Add);
                        f.instruction(&Instruction::GlobalSet(0));
                        f.instruction(&Instruction::End);
                        if let Some(d) = self.loop_depths.last_mut() {
                            *d -= 1;
                        }
                    }
                    "sort_i32" => {
                        // Bubble sort in-place
                        let ma = MemArg {
                            offset: 0,
                            align: 2,
                            memory_index: 0,
                        };
                        // We need locals for the sort loop. Since we can't easily declare
                        // new locals mid-function in wasm-encoder, we'll use scratch memory.
                        // scratch[16..32]: sort temporaries
                        // SCRATCH+0 (16): outer i
                        // SCRATCH+4 (20): inner j
                        // SCRATCH+8 (24): n (len)
                        // SCRATCH+12 (28): data_ptr
                        // SCRATCH+16 (32): temp for swap
                        let s_i = SCRATCH;
                        let s_j = SCRATCH + 4;
                        let s_n = SCRATCH + 8;
                        let s_data = SCRATCH + 12;
                        let s_tmp = SCRATCH + 16;
                        // Load n = len(v)
                        f.instruction(&Instruction::I32Const(s_n as i32));
                        if let Some(v) = args.first() {
                            self.emit_operand(f, v);
                        }
                        f.instruction(&Instruction::I32Load(ma));
                        f.instruction(&Instruction::I32Store(ma));
                        // Load data_ptr
                        f.instruction(&Instruction::I32Const(s_data as i32));
                        if let Some(v) = args.first() {
                            self.emit_operand(f, v);
                        }
                        f.instruction(&Instruction::I32Const(8));
                        f.instruction(&Instruction::I32Add);
                        f.instruction(&Instruction::I32Load(ma));
                        f.instruction(&Instruction::I32Store(ma));
                        // i = 0
                        f.instruction(&Instruction::I32Const(s_i as i32));
                        f.instruction(&Instruction::I32Const(0));
                        f.instruction(&Instruction::I32Store(ma));
                        // Outer loop: while i < n-1
                        f.instruction(&Instruction::Block(wasm_encoder::BlockType::Empty));
                        f.instruction(&Instruction::Loop(wasm_encoder::BlockType::Empty));
                        // Check i < n-1
                        f.instruction(&Instruction::I32Const(s_i as i32));
                        f.instruction(&Instruction::I32Load(ma));
                        f.instruction(&Instruction::I32Const(s_n as i32));
                        f.instruction(&Instruction::I32Load(ma));
                        f.instruction(&Instruction::I32Const(1));
                        f.instruction(&Instruction::I32Sub);
                        f.instruction(&Instruction::I32GeU);
                        f.instruction(&Instruction::BrIf(1)); // break if i >= n-1
                        // j = 0
                        f.instruction(&Instruction::I32Const(s_j as i32));
                        f.instruction(&Instruction::I32Const(0));
                        f.instruction(&Instruction::I32Store(ma));
                        // Inner loop: while j < n-i-1
                        f.instruction(&Instruction::Block(wasm_encoder::BlockType::Empty));
                        f.instruction(&Instruction::Loop(wasm_encoder::BlockType::Empty));
                        // Check j < n-i-1
                        f.instruction(&Instruction::I32Const(s_j as i32));
                        f.instruction(&Instruction::I32Load(ma));
                        f.instruction(&Instruction::I32Const(s_n as i32));
                        f.instruction(&Instruction::I32Load(ma));
                        f.instruction(&Instruction::I32Const(s_i as i32));
                        f.instruction(&Instruction::I32Load(ma));
                        f.instruction(&Instruction::I32Sub);
                        f.instruction(&Instruction::I32Const(1));
                        f.instruction(&Instruction::I32Sub);
                        f.instruction(&Instruction::I32GeU);
                        f.instruction(&Instruction::BrIf(1)); // break if j >= n-i-1
                        // Compare data[j] > data[j+1]
                        // addr_j = data_ptr + j*4
                        f.instruction(&Instruction::I32Const(s_data as i32));
                        f.instruction(&Instruction::I32Load(ma));
                        f.instruction(&Instruction::I32Const(s_j as i32));
                        f.instruction(&Instruction::I32Load(ma));
                        f.instruction(&Instruction::I32Const(4));
                        f.instruction(&Instruction::I32Mul);
                        f.instruction(&Instruction::I32Add);
                        f.instruction(&Instruction::I32Load(ma)); // data[j]
                        // addr_j1 = data_ptr + (j+1)*4
                        f.instruction(&Instruction::I32Const(s_data as i32));
                        f.instruction(&Instruction::I32Load(ma));
                        f.instruction(&Instruction::I32Const(s_j as i32));
                        f.instruction(&Instruction::I32Load(ma));
                        f.instruction(&Instruction::I32Const(1));
                        f.instruction(&Instruction::I32Add);
                        f.instruction(&Instruction::I32Const(4));
                        f.instruction(&Instruction::I32Mul);
                        f.instruction(&Instruction::I32Add);
                        f.instruction(&Instruction::I32Load(ma)); // data[j+1]
                        f.instruction(&Instruction::I32GtS);
                        f.instruction(&Instruction::If(wasm_encoder::BlockType::Empty));
                        // Swap data[j] and data[j+1]
                        // tmp = data[j]
                        f.instruction(&Instruction::I32Const(s_tmp as i32));
                        f.instruction(&Instruction::I32Const(s_data as i32));
                        f.instruction(&Instruction::I32Load(ma));
                        f.instruction(&Instruction::I32Const(s_j as i32));
                        f.instruction(&Instruction::I32Load(ma));
                        f.instruction(&Instruction::I32Const(4));
                        f.instruction(&Instruction::I32Mul);
                        f.instruction(&Instruction::I32Add);
                        f.instruction(&Instruction::I32Load(ma));
                        f.instruction(&Instruction::I32Store(ma));
                        // data[j] = data[j+1]
                        f.instruction(&Instruction::I32Const(s_data as i32));
                        f.instruction(&Instruction::I32Load(ma));
                        f.instruction(&Instruction::I32Const(s_j as i32));
                        f.instruction(&Instruction::I32Load(ma));
                        f.instruction(&Instruction::I32Const(4));
                        f.instruction(&Instruction::I32Mul);
                        f.instruction(&Instruction::I32Add);
                        // data[j+1] value
                        f.instruction(&Instruction::I32Const(s_data as i32));
                        f.instruction(&Instruction::I32Load(ma));
                        f.instruction(&Instruction::I32Const(s_j as i32));
                        f.instruction(&Instruction::I32Load(ma));
                        f.instruction(&Instruction::I32Const(1));
                        f.instruction(&Instruction::I32Add);
                        f.instruction(&Instruction::I32Const(4));
                        f.instruction(&Instruction::I32Mul);
                        f.instruction(&Instruction::I32Add);
                        f.instruction(&Instruction::I32Load(ma));
                        f.instruction(&Instruction::I32Store(ma));
                        // data[j+1] = tmp
                        f.instruction(&Instruction::I32Const(s_data as i32));
                        f.instruction(&Instruction::I32Load(ma));
                        f.instruction(&Instruction::I32Const(s_j as i32));
                        f.instruction(&Instruction::I32Load(ma));
                        f.instruction(&Instruction::I32Const(1));
                        f.instruction(&Instruction::I32Add);
                        f.instruction(&Instruction::I32Const(4));
                        f.instruction(&Instruction::I32Mul);
                        f.instruction(&Instruction::I32Add);
                        f.instruction(&Instruction::I32Const(s_tmp as i32));
                        f.instruction(&Instruction::I32Load(ma));
                        f.instruction(&Instruction::I32Store(ma));
                        f.instruction(&Instruction::End); // end if
                        // j++
                        f.instruction(&Instruction::I32Const(s_j as i32));
                        f.instruction(&Instruction::I32Const(s_j as i32));
                        f.instruction(&Instruction::I32Load(ma));
                        f.instruction(&Instruction::I32Const(1));
                        f.instruction(&Instruction::I32Add);
                        f.instruction(&Instruction::I32Store(ma));
                        f.instruction(&Instruction::Br(0)); // continue inner
                        f.instruction(&Instruction::End); // end inner loop
                        f.instruction(&Instruction::End); // end inner block
                        // i++
                        f.instruction(&Instruction::I32Const(s_i as i32));
                        f.instruction(&Instruction::I32Const(s_i as i32));
                        f.instruction(&Instruction::I32Load(ma));
                        f.instruction(&Instruction::I32Const(1));
                        f.instruction(&Instruction::I32Add);
                        f.instruction(&Instruction::I32Store(ma));
                        f.instruction(&Instruction::Br(0)); // continue outer
                        f.instruction(&Instruction::End); // end outer loop
                        f.instruction(&Instruction::End); // end outer block
                    }
                    "string_len" => {
                        // string_len(s) -> i32: load length from length-prefixed string
                        let ma = MemArg {
                            offset: 0,
                            align: 2,
                            memory_index: 0,
                        };
                        if let Some(s) = args.first() {
                            self.emit_operand(f, s);
                        }
                        f.instruction(&Instruction::I32Const(4));
                        f.instruction(&Instruction::I32Sub);
                        f.instruction(&Instruction::I32Load(ma));
                    }
                    "string_eq" => {
                        // string_eq(a, b) -> bool
                        for a in args {
                            self.emit_operand(f, a);
                        }
                        f.instruction(&Instruction::Call(FN_STR_EQ));
                    }
                    "String_new" => {
                        // Empty string: allocate [len=0] on heap, return data ptr
                        let ma = MemArg {
                            offset: 0,
                            align: 2,
                            memory_index: 0,
                        };
                        f.instruction(&Instruction::GlobalGet(0));
                        f.instruction(&Instruction::I32Const(0));
                        f.instruction(&Instruction::I32Store(ma));
                        f.instruction(&Instruction::GlobalGet(0));
                        f.instruction(&Instruction::I32Const(4));
                        f.instruction(&Instruction::I32Add);
                        // bump heap
                        f.instruction(&Instruction::GlobalGet(0));
                        f.instruction(&Instruction::I32Const(4));
                        f.instruction(&Instruction::I32Add);
                        f.instruction(&Instruction::GlobalSet(0));
                    }
                    "is_empty" => {
                        // is_empty(s): len(s) == 0
                        let ma = MemArg {
                            offset: 0,
                            align: 2,
                            memory_index: 0,
                        };
                        if let Some(s) = args.first() {
                            self.emit_operand(f, s);
                        }
                        f.instruction(&Instruction::I32Const(4));
                        f.instruction(&Instruction::I32Sub);
                        f.instruction(&Instruction::I32Load(ma));
                        f.instruction(&Instruction::I32Eqz);
                    }
                    "slice" => {
                        // slice(s, start, end) -> String: copy bytes [start..end] to new string
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
                        // new_len = end - start
                        // Write new_len at heap[0]
                        f.instruction(&Instruction::GlobalGet(0));
                        if let Some(end_arg) = args.get(2) {
                            self.emit_operand(f, end_arg);
                        }
                        if let Some(start_arg) = args.get(1) {
                            self.emit_operand(f, start_arg);
                        }
                        f.instruction(&Instruction::I32Sub);
                        f.instruction(&Instruction::I32Store(ma));
                        // Copy bytes: mem[heap+4+i] = mem[s+start+i] for i in 0..new_len
                        // Use NWRITTEN as loop counter
                        f.instruction(&Instruction::I32Const(NWRITTEN as i32));
                        f.instruction(&Instruction::I32Const(0));
                        f.instruction(&Instruction::I32Store(ma));
                        f.instruction(&Instruction::Block(wasm_encoder::BlockType::Empty));
                        f.instruction(&Instruction::Loop(wasm_encoder::BlockType::Empty));
                        // if i >= new_len, break
                        f.instruction(&Instruction::I32Const(NWRITTEN as i32));
                        f.instruction(&Instruction::I32Load(ma));
                        f.instruction(&Instruction::GlobalGet(0));
                        f.instruction(&Instruction::I32Load(ma)); // new_len
                        f.instruction(&Instruction::I32GeU);
                        f.instruction(&Instruction::BrIf(1));
                        // dst = heap + 4 + i
                        f.instruction(&Instruction::GlobalGet(0));
                        f.instruction(&Instruction::I32Const(4));
                        f.instruction(&Instruction::I32Add);
                        f.instruction(&Instruction::I32Const(NWRITTEN as i32));
                        f.instruction(&Instruction::I32Load(ma));
                        f.instruction(&Instruction::I32Add);
                        // src = s + start + i
                        if let Some(s) = args.first() {
                            self.emit_operand(f, s);
                        }
                        if let Some(start_arg) = args.get(1) {
                            self.emit_operand(f, start_arg);
                        }
                        f.instruction(&Instruction::I32Add);
                        f.instruction(&Instruction::I32Const(NWRITTEN as i32));
                        f.instruction(&Instruction::I32Load(ma));
                        f.instruction(&Instruction::I32Add);
                        f.instruction(&Instruction::I32Load8U(ma0));
                        f.instruction(&Instruction::I32Store8(ma0));
                        // i++
                        f.instruction(&Instruction::I32Const(NWRITTEN as i32));
                        f.instruction(&Instruction::I32Const(NWRITTEN as i32));
                        f.instruction(&Instruction::I32Load(ma));
                        f.instruction(&Instruction::I32Const(1));
                        f.instruction(&Instruction::I32Add);
                        f.instruction(&Instruction::I32Store(ma));
                        f.instruction(&Instruction::Br(0));
                        f.instruction(&Instruction::End);
                        f.instruction(&Instruction::End);
                        // result = heap + 4 (data ptr)
                        f.instruction(&Instruction::GlobalGet(0));
                        f.instruction(&Instruction::I32Const(4));
                        f.instruction(&Instruction::I32Add);
                        // bump heap past len + data
                        f.instruction(&Instruction::GlobalGet(0));
                        f.instruction(&Instruction::GlobalGet(0));
                        f.instruction(&Instruction::I32Load(ma)); // new_len
                        f.instruction(&Instruction::I32Const(4));
                        f.instruction(&Instruction::I32Add);
                        f.instruction(&Instruction::I32Add);
                        f.instruction(&Instruction::GlobalSet(0));
                    }
                    "starts_with" => {
                        // starts_with(s, prefix) -> bool
                        // Compare first prefix_len bytes of s with prefix
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
                        // Check s_len >= prefix_len
                        if let Some(s) = args.first() {
                            self.emit_operand(f, s);
                        }
                        f.instruction(&Instruction::I32Const(4));
                        f.instruction(&Instruction::I32Sub);
                        f.instruction(&Instruction::I32Load(ma)); // s_len
                        if let Some(p) = args.get(1) {
                            self.emit_operand(f, p);
                        }
                        f.instruction(&Instruction::I32Const(4));
                        f.instruction(&Instruction::I32Sub);
                        f.instruction(&Instruction::I32Load(ma)); // p_len
                        f.instruction(&Instruction::I32GeU);
                        f.instruction(&Instruction::If(wasm_encoder::BlockType::Result(
                            ValType::I32,
                        )));
                        if let Some(d) = self.loop_depths.last_mut() {
                            *d += 1;
                        }
                        // Compare byte by byte using NWRITTEN as counter
                        // Store prefix_len in SCRATCH
                        f.instruction(&Instruction::I32Const(SCRATCH as i32));
                        if let Some(p) = args.get(1) {
                            self.emit_operand(f, p);
                        }
                        f.instruction(&Instruction::I32Const(4));
                        f.instruction(&Instruction::I32Sub);
                        f.instruction(&Instruction::I32Load(ma));
                        f.instruction(&Instruction::I32Store(ma));
                        // i = 0
                        f.instruction(&Instruction::I32Const(NWRITTEN as i32));
                        f.instruction(&Instruction::I32Const(0));
                        f.instruction(&Instruction::I32Store(ma));
                        // result = 1 (assume true)
                        f.instruction(&Instruction::I32Const(SCRATCH as i32 + 4));
                        f.instruction(&Instruction::I32Const(1));
                        f.instruction(&Instruction::I32Store(ma));
                        f.instruction(&Instruction::Block(wasm_encoder::BlockType::Empty));
                        f.instruction(&Instruction::Loop(wasm_encoder::BlockType::Empty));
                        // if i >= prefix_len, break
                        f.instruction(&Instruction::I32Const(NWRITTEN as i32));
                        f.instruction(&Instruction::I32Load(ma));
                        f.instruction(&Instruction::I32Const(SCRATCH as i32));
                        f.instruction(&Instruction::I32Load(ma));
                        f.instruction(&Instruction::I32GeU);
                        f.instruction(&Instruction::BrIf(1));
                        // if s[i] != prefix[i], set result=0 and break
                        if let Some(s) = args.first() {
                            self.emit_operand(f, s);
                        }
                        f.instruction(&Instruction::I32Const(NWRITTEN as i32));
                        f.instruction(&Instruction::I32Load(ma));
                        f.instruction(&Instruction::I32Add);
                        f.instruction(&Instruction::I32Load8U(ma0));
                        if let Some(p) = args.get(1) {
                            self.emit_operand(f, p);
                        }
                        f.instruction(&Instruction::I32Const(NWRITTEN as i32));
                        f.instruction(&Instruction::I32Load(ma));
                        f.instruction(&Instruction::I32Add);
                        f.instruction(&Instruction::I32Load8U(ma0));
                        f.instruction(&Instruction::I32Ne);
                        f.instruction(&Instruction::If(wasm_encoder::BlockType::Empty));
                        f.instruction(&Instruction::I32Const(SCRATCH as i32 + 4));
                        f.instruction(&Instruction::I32Const(0));
                        f.instruction(&Instruction::I32Store(ma));
                        f.instruction(&Instruction::Br(2)); // break to Block (exit loop)
                        f.instruction(&Instruction::End);
                        // i++
                        f.instruction(&Instruction::I32Const(NWRITTEN as i32));
                        f.instruction(&Instruction::I32Const(NWRITTEN as i32));
                        f.instruction(&Instruction::I32Load(ma));
                        f.instruction(&Instruction::I32Const(1));
                        f.instruction(&Instruction::I32Add);
                        f.instruction(&Instruction::I32Store(ma));
                        f.instruction(&Instruction::Br(0));
                        f.instruction(&Instruction::End); // end loop
                        f.instruction(&Instruction::End); // end block
                        // Load result
                        f.instruction(&Instruction::I32Const(SCRATCH as i32 + 4));
                        f.instruction(&Instruction::I32Load(ma));
                        f.instruction(&Instruction::Else);
                        // s_len < prefix_len → false
                        f.instruction(&Instruction::I32Const(0));
                        f.instruction(&Instruction::End);
                        if let Some(d) = self.loop_depths.last_mut() {
                            *d -= 1;
                        }
                    }
                    "ends_with" => {
                        // ends_with(s, suffix) -> bool
                        // Compare last suffix_len bytes of s with suffix
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
                        // Check s_len >= suffix_len
                        if let Some(s) = args.first() {
                            self.emit_operand(f, s);
                        }
                        f.instruction(&Instruction::I32Const(4));
                        f.instruction(&Instruction::I32Sub);
                        f.instruction(&Instruction::I32Load(ma)); // s_len
                        if let Some(p) = args.get(1) {
                            self.emit_operand(f, p);
                        }
                        f.instruction(&Instruction::I32Const(4));
                        f.instruction(&Instruction::I32Sub);
                        f.instruction(&Instruction::I32Load(ma)); // suffix_len
                        f.instruction(&Instruction::I32GeU);
                        f.instruction(&Instruction::If(wasm_encoder::BlockType::Result(
                            ValType::I32,
                        )));
                        if let Some(d) = self.loop_depths.last_mut() {
                            *d += 1;
                        }
                        // offset = s_len - suffix_len
                        f.instruction(&Instruction::I32Const(SCRATCH as i32));
                        if let Some(s) = args.first() {
                            self.emit_operand(f, s);
                        }
                        f.instruction(&Instruction::I32Const(4));
                        f.instruction(&Instruction::I32Sub);
                        f.instruction(&Instruction::I32Load(ma));
                        if let Some(p) = args.get(1) {
                            self.emit_operand(f, p);
                        }
                        f.instruction(&Instruction::I32Const(4));
                        f.instruction(&Instruction::I32Sub);
                        f.instruction(&Instruction::I32Load(ma));
                        f.instruction(&Instruction::I32Sub);
                        f.instruction(&Instruction::I32Store(ma)); // scratch[0] = offset
                        // suffix_len
                        f.instruction(&Instruction::I32Const(SCRATCH as i32 + 4));
                        if let Some(p) = args.get(1) {
                            self.emit_operand(f, p);
                        }
                        f.instruction(&Instruction::I32Const(4));
                        f.instruction(&Instruction::I32Sub);
                        f.instruction(&Instruction::I32Load(ma));
                        f.instruction(&Instruction::I32Store(ma)); // scratch[4] = suffix_len
                        // i = 0, result = 1
                        f.instruction(&Instruction::I32Const(NWRITTEN as i32));
                        f.instruction(&Instruction::I32Const(0));
                        f.instruction(&Instruction::I32Store(ma));
                        f.instruction(&Instruction::I32Const(SCRATCH as i32 + 8));
                        f.instruction(&Instruction::I32Const(1));
                        f.instruction(&Instruction::I32Store(ma));
                        f.instruction(&Instruction::Block(wasm_encoder::BlockType::Empty));
                        f.instruction(&Instruction::Loop(wasm_encoder::BlockType::Empty));
                        // if i >= suffix_len, break
                        f.instruction(&Instruction::I32Const(NWRITTEN as i32));
                        f.instruction(&Instruction::I32Load(ma));
                        f.instruction(&Instruction::I32Const(SCRATCH as i32 + 4));
                        f.instruction(&Instruction::I32Load(ma));
                        f.instruction(&Instruction::I32GeU);
                        f.instruction(&Instruction::BrIf(1));
                        // if s[offset+i] != suffix[i], set result=0 and break
                        if let Some(s) = args.first() {
                            self.emit_operand(f, s);
                        }
                        f.instruction(&Instruction::I32Const(SCRATCH as i32));
                        f.instruction(&Instruction::I32Load(ma)); // offset
                        f.instruction(&Instruction::I32Add);
                        f.instruction(&Instruction::I32Const(NWRITTEN as i32));
                        f.instruction(&Instruction::I32Load(ma));
                        f.instruction(&Instruction::I32Add);
                        f.instruction(&Instruction::I32Load8U(ma0));
                        if let Some(p) = args.get(1) {
                            self.emit_operand(f, p);
                        }
                        f.instruction(&Instruction::I32Const(NWRITTEN as i32));
                        f.instruction(&Instruction::I32Load(ma));
                        f.instruction(&Instruction::I32Add);
                        f.instruction(&Instruction::I32Load8U(ma0));
                        f.instruction(&Instruction::I32Ne);
                        f.instruction(&Instruction::If(wasm_encoder::BlockType::Empty));
                        f.instruction(&Instruction::I32Const(SCRATCH as i32 + 8));
                        f.instruction(&Instruction::I32Const(0));
                        f.instruction(&Instruction::I32Store(ma));
                        f.instruction(&Instruction::Br(2)); // break to Block (exit loop)
                        f.instruction(&Instruction::End);
                        // i++
                        f.instruction(&Instruction::I32Const(NWRITTEN as i32));
                        f.instruction(&Instruction::I32Const(NWRITTEN as i32));
                        f.instruction(&Instruction::I32Load(ma));
                        f.instruction(&Instruction::I32Const(1));
                        f.instruction(&Instruction::I32Add);
                        f.instruction(&Instruction::I32Store(ma));
                        f.instruction(&Instruction::Br(0));
                        f.instruction(&Instruction::End); // end loop
                        f.instruction(&Instruction::End); // end block
                        f.instruction(&Instruction::I32Const(SCRATCH as i32 + 8));
                        f.instruction(&Instruction::I32Load(ma));
                        f.instruction(&Instruction::Else);
                        f.instruction(&Instruction::I32Const(0));
                        f.instruction(&Instruction::End);
                        if let Some(d) = self.loop_depths.last_mut() {
                            *d -= 1;
                        }
                    }
                    "join" => {
                        // join(parts: Vec<String>, sep: String) -> String
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
                        let s_i = SCRATCH;
                        let s_n = SCRATCH + 4;
                        let s_out_start = SCRATCH + 8;
                        let s_out_pos = SCRATCH + 12;
                        // n = len(parts)
                        f.instruction(&Instruction::I32Const(s_n as i32));
                        if let Some(v) = args.first() {
                            self.emit_operand(f, v);
                        }
                        f.instruction(&Instruction::I32Load(ma));
                        f.instruction(&Instruction::I32Store(ma));
                        // out_start = heap
                        f.instruction(&Instruction::I32Const(s_out_start as i32));
                        f.instruction(&Instruction::GlobalGet(0));
                        f.instruction(&Instruction::I32Store(ma));
                        // out_pos = heap + 4  (leave room for length prefix)
                        f.instruction(&Instruction::I32Const(s_out_pos as i32));
                        f.instruction(&Instruction::GlobalGet(0));
                        f.instruction(&Instruction::I32Const(4));
                        f.instruction(&Instruction::I32Add);
                        f.instruction(&Instruction::I32Store(ma));
                        // i = 0
                        f.instruction(&Instruction::I32Const(s_i as i32));
                        f.instruction(&Instruction::I32Const(0));
                        f.instruction(&Instruction::I32Store(ma));
                        f.instruction(&Instruction::Block(wasm_encoder::BlockType::Empty));
                        f.instruction(&Instruction::Loop(wasm_encoder::BlockType::Empty));
                        // if i >= n, break
                        f.instruction(&Instruction::I32Const(s_i as i32));
                        f.instruction(&Instruction::I32Load(ma));
                        f.instruction(&Instruction::I32Const(s_n as i32));
                        f.instruction(&Instruction::I32Load(ma));
                        f.instruction(&Instruction::I32GeU);
                        f.instruction(&Instruction::BrIf(1));
                        // If i > 0, copy separator
                        f.instruction(&Instruction::I32Const(s_i as i32));
                        f.instruction(&Instruction::I32Load(ma));
                        f.instruction(&Instruction::I32Const(0));
                        f.instruction(&Instruction::I32GtU);
                        f.instruction(&Instruction::If(wasm_encoder::BlockType::Empty));
                        {
                            // sep_len
                            f.instruction(&Instruction::I32Const(NWRITTEN as i32));
                            f.instruction(&Instruction::I32Const(0));
                            f.instruction(&Instruction::I32Store(ma));
                            f.instruction(&Instruction::Block(wasm_encoder::BlockType::Empty));
                            f.instruction(&Instruction::Loop(wasm_encoder::BlockType::Empty));
                            // if j >= sep_len, break
                            f.instruction(&Instruction::I32Const(NWRITTEN as i32));
                            f.instruction(&Instruction::I32Load(ma));
                            if let Some(sep) = args.get(1) {
                                self.emit_operand(f, sep);
                            }
                            f.instruction(&Instruction::I32Const(4));
                            f.instruction(&Instruction::I32Sub);
                            f.instruction(&Instruction::I32Load(ma));
                            f.instruction(&Instruction::I32GeU);
                            f.instruction(&Instruction::BrIf(1));
                            // out_pos[j] = sep[j]
                            f.instruction(&Instruction::I32Const(s_out_pos as i32));
                            f.instruction(&Instruction::I32Load(ma));
                            f.instruction(&Instruction::I32Const(NWRITTEN as i32));
                            f.instruction(&Instruction::I32Load(ma));
                            f.instruction(&Instruction::I32Add);
                            if let Some(sep) = args.get(1) {
                                self.emit_operand(f, sep);
                            }
                            f.instruction(&Instruction::I32Const(NWRITTEN as i32));
                            f.instruction(&Instruction::I32Load(ma));
                            f.instruction(&Instruction::I32Add);
                            f.instruction(&Instruction::I32Load8U(ma0));
                            f.instruction(&Instruction::I32Store8(ma0));
                            // j++
                            f.instruction(&Instruction::I32Const(NWRITTEN as i32));
                            f.instruction(&Instruction::I32Const(NWRITTEN as i32));
                            f.instruction(&Instruction::I32Load(ma));
                            f.instruction(&Instruction::I32Const(1));
                            f.instruction(&Instruction::I32Add);
                            f.instruction(&Instruction::I32Store(ma));
                            f.instruction(&Instruction::Br(0));
                            f.instruction(&Instruction::End);
                            f.instruction(&Instruction::End);
                            // out_pos += sep_len
                            f.instruction(&Instruction::I32Const(s_out_pos as i32));
                            f.instruction(&Instruction::I32Const(s_out_pos as i32));
                            f.instruction(&Instruction::I32Load(ma));
                            if let Some(sep) = args.get(1) {
                                self.emit_operand(f, sep);
                            }
                            f.instruction(&Instruction::I32Const(4));
                            f.instruction(&Instruction::I32Sub);
                            f.instruction(&Instruction::I32Load(ma));
                            f.instruction(&Instruction::I32Add);
                            f.instruction(&Instruction::I32Store(ma));
                        }
                        f.instruction(&Instruction::End); // end if i > 0
                        // Copy current string: str_ptr = parts.data[i]
                        f.instruction(&Instruction::I32Const(NWRITTEN as i32));
                        f.instruction(&Instruction::I32Const(0));
                        f.instruction(&Instruction::I32Store(ma));
                        f.instruction(&Instruction::Block(wasm_encoder::BlockType::Empty));
                        f.instruction(&Instruction::Loop(wasm_encoder::BlockType::Empty));
                        // if j >= str_len, break
                        f.instruction(&Instruction::I32Const(NWRITTEN as i32));
                        f.instruction(&Instruction::I32Load(ma));
                        // str_ptr = parts.data_ptr + i*4, deref to get string ptr
                        if let Some(v) = args.first() {
                            self.emit_operand(f, v);
                        }
                        f.instruction(&Instruction::I32Const(8));
                        f.instruction(&Instruction::I32Add);
                        f.instruction(&Instruction::I32Load(ma)); // data_ptr
                        f.instruction(&Instruction::I32Const(s_i as i32));
                        f.instruction(&Instruction::I32Load(ma));
                        f.instruction(&Instruction::I32Const(4));
                        f.instruction(&Instruction::I32Mul);
                        f.instruction(&Instruction::I32Add);
                        f.instruction(&Instruction::I32Load(ma)); // str_ptr
                        f.instruction(&Instruction::I32Const(4));
                        f.instruction(&Instruction::I32Sub);
                        f.instruction(&Instruction::I32Load(ma)); // str_len
                        f.instruction(&Instruction::I32GeU);
                        f.instruction(&Instruction::BrIf(1));
                        // out_pos[j] = str[j]
                        f.instruction(&Instruction::I32Const(s_out_pos as i32));
                        f.instruction(&Instruction::I32Load(ma));
                        f.instruction(&Instruction::I32Const(NWRITTEN as i32));
                        f.instruction(&Instruction::I32Load(ma));
                        f.instruction(&Instruction::I32Add);
                        // str_ptr again
                        if let Some(v) = args.first() {
                            self.emit_operand(f, v);
                        }
                        f.instruction(&Instruction::I32Const(8));
                        f.instruction(&Instruction::I32Add);
                        f.instruction(&Instruction::I32Load(ma));
                        f.instruction(&Instruction::I32Const(s_i as i32));
                        f.instruction(&Instruction::I32Load(ma));
                        f.instruction(&Instruction::I32Const(4));
                        f.instruction(&Instruction::I32Mul);
                        f.instruction(&Instruction::I32Add);
                        f.instruction(&Instruction::I32Load(ma)); // str_ptr
                        f.instruction(&Instruction::I32Const(NWRITTEN as i32));
                        f.instruction(&Instruction::I32Load(ma));
                        f.instruction(&Instruction::I32Add);
                        f.instruction(&Instruction::I32Load8U(ma0));
                        f.instruction(&Instruction::I32Store8(ma0));
                        // j++
                        f.instruction(&Instruction::I32Const(NWRITTEN as i32));
                        f.instruction(&Instruction::I32Const(NWRITTEN as i32));
                        f.instruction(&Instruction::I32Load(ma));
                        f.instruction(&Instruction::I32Const(1));
                        f.instruction(&Instruction::I32Add);
                        f.instruction(&Instruction::I32Store(ma));
                        f.instruction(&Instruction::Br(0));
                        f.instruction(&Instruction::End);
                        f.instruction(&Instruction::End);
                        // out_pos += str_len
                        f.instruction(&Instruction::I32Const(s_out_pos as i32));
                        f.instruction(&Instruction::I32Const(s_out_pos as i32));
                        f.instruction(&Instruction::I32Load(ma));
                        if let Some(v) = args.first() {
                            self.emit_operand(f, v);
                        }
                        f.instruction(&Instruction::I32Const(8));
                        f.instruction(&Instruction::I32Add);
                        f.instruction(&Instruction::I32Load(ma));
                        f.instruction(&Instruction::I32Const(s_i as i32));
                        f.instruction(&Instruction::I32Load(ma));
                        f.instruction(&Instruction::I32Const(4));
                        f.instruction(&Instruction::I32Mul);
                        f.instruction(&Instruction::I32Add);
                        f.instruction(&Instruction::I32Load(ma)); // str_ptr
                        f.instruction(&Instruction::I32Const(4));
                        f.instruction(&Instruction::I32Sub);
                        f.instruction(&Instruction::I32Load(ma)); // str_len
                        f.instruction(&Instruction::I32Add);
                        f.instruction(&Instruction::I32Store(ma));
                        // i++
                        f.instruction(&Instruction::I32Const(s_i as i32));
                        f.instruction(&Instruction::I32Const(s_i as i32));
                        f.instruction(&Instruction::I32Load(ma));
                        f.instruction(&Instruction::I32Const(1));
                        f.instruction(&Instruction::I32Add);
                        f.instruction(&Instruction::I32Store(ma));
                        f.instruction(&Instruction::Br(0));
                        f.instruction(&Instruction::End); // end loop
                        f.instruction(&Instruction::End); // end block
                        // Write total length at out_start
                        f.instruction(&Instruction::I32Const(s_out_start as i32));
                        f.instruction(&Instruction::I32Load(ma));
                        f.instruction(&Instruction::I32Const(s_out_pos as i32));
                        f.instruction(&Instruction::I32Load(ma));
                        f.instruction(&Instruction::I32Const(s_out_start as i32));
                        f.instruction(&Instruction::I32Load(ma));
                        f.instruction(&Instruction::I32Sub);
                        f.instruction(&Instruction::I32Const(4));
                        f.instruction(&Instruction::I32Sub);
                        f.instruction(&Instruction::I32Store(ma));
                        // Result: out_start + 4 (pointer to data, length-prefixed)
                        f.instruction(&Instruction::I32Const(s_out_start as i32));
                        f.instruction(&Instruction::I32Load(ma));
                        f.instruction(&Instruction::I32Const(4));
                        f.instruction(&Instruction::I32Add);
                        // Bump heap
                        f.instruction(&Instruction::I32Const(s_out_pos as i32));
                        f.instruction(&Instruction::I32Load(ma));
                        f.instruction(&Instruction::GlobalSet(0));
                    }
                    "split" => {
                        // split(s: String, delim: String) -> Vec<String>
                        // Note: single-char delim only
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
                        let s_i = SCRATCH;
                        let s_slen = SCRATCH + 4;
                        let s_seg = SCRATCH + 8;
                        let s_vec = SCRATCH + 12;
                        // Create Vec header: len=0, cap=8, data_ptr=heap+12
                        f.instruction(&Instruction::I32Const(s_vec as i32));
                        f.instruction(&Instruction::GlobalGet(0));
                        f.instruction(&Instruction::I32Store(ma));
                        // vec.len = 0
                        f.instruction(&Instruction::GlobalGet(0));
                        f.instruction(&Instruction::I32Const(0));
                        f.instruction(&Instruction::I32Store(ma));
                        // vec.cap = 8
                        f.instruction(&Instruction::GlobalGet(0));
                        f.instruction(&Instruction::I32Const(4));
                        f.instruction(&Instruction::I32Add);
                        f.instruction(&Instruction::I32Const(8));
                        f.instruction(&Instruction::I32Store(ma));
                        // vec.data_ptr = heap + 12
                        f.instruction(&Instruction::GlobalGet(0));
                        f.instruction(&Instruction::I32Const(8));
                        f.instruction(&Instruction::I32Add);
                        f.instruction(&Instruction::GlobalGet(0));
                        f.instruction(&Instruction::I32Const(12));
                        f.instruction(&Instruction::I32Add);
                        f.instruction(&Instruction::I32Store(ma));
                        // Bump heap past header + data (12 + 32 = 44)
                        f.instruction(&Instruction::GlobalGet(0));
                        f.instruction(&Instruction::I32Const(44));
                        f.instruction(&Instruction::I32Add);
                        f.instruction(&Instruction::GlobalSet(0));
                        // Init: slen = len(s), i = 0, seg_start = 0
                        f.instruction(&Instruction::I32Const(s_slen as i32));
                        if let Some(s) = args.first() {
                            self.emit_operand(f, s);
                        }
                        f.instruction(&Instruction::I32Const(4));
                        f.instruction(&Instruction::I32Sub);
                        f.instruction(&Instruction::I32Load(ma));
                        f.instruction(&Instruction::I32Store(ma));
                        f.instruction(&Instruction::I32Const(s_i as i32));
                        f.instruction(&Instruction::I32Const(0));
                        f.instruction(&Instruction::I32Store(ma));
                        f.instruction(&Instruction::I32Const(s_seg as i32));
                        f.instruction(&Instruction::I32Const(0));
                        f.instruction(&Instruction::I32Store(ma));
                        // Store delim char in I32BUF
                        f.instruction(&Instruction::I32Const(I32BUF as i32));
                        if let Some(d) = args.get(1) {
                            self.emit_operand(f, d);
                        }
                        f.instruction(&Instruction::I32Load8U(ma0));
                        f.instruction(&Instruction::I32Store(ma));
                        // Scan loop
                        f.instruction(&Instruction::Block(wasm_encoder::BlockType::Empty));
                        f.instruction(&Instruction::Loop(wasm_encoder::BlockType::Empty));
                        // if i >= slen, break
                        f.instruction(&Instruction::I32Const(s_i as i32));
                        f.instruction(&Instruction::I32Load(ma));
                        f.instruction(&Instruction::I32Const(s_slen as i32));
                        f.instruction(&Instruction::I32Load(ma));
                        f.instruction(&Instruction::I32GeU);
                        f.instruction(&Instruction::BrIf(1));
                        // if s[i] == delim_char
                        if let Some(s) = args.first() {
                            self.emit_operand(f, s);
                        }
                        f.instruction(&Instruction::I32Const(s_i as i32));
                        f.instruction(&Instruction::I32Load(ma));
                        f.instruction(&Instruction::I32Add);
                        f.instruction(&Instruction::I32Load8U(ma0));
                        f.instruction(&Instruction::I32Const(I32BUF as i32));
                        f.instruction(&Instruction::I32Load(ma));
                        f.instruction(&Instruction::I32Eq);
                        f.instruction(&Instruction::If(wasm_encoder::BlockType::Empty));
                        {
                            // Allocate substring [seg_start..i]
                            // Write length prefix at heap
                            f.instruction(&Instruction::GlobalGet(0));
                            f.instruction(&Instruction::I32Const(s_i as i32));
                            f.instruction(&Instruction::I32Load(ma));
                            f.instruction(&Instruction::I32Const(s_seg as i32));
                            f.instruction(&Instruction::I32Load(ma));
                            f.instruction(&Instruction::I32Sub);
                            f.instruction(&Instruction::I32Store(ma)); // heap[0] = seg_len
                            // Copy bytes
                            f.instruction(&Instruction::I32Const(NWRITTEN as i32));
                            f.instruction(&Instruction::I32Const(0));
                            f.instruction(&Instruction::I32Store(ma));
                            f.instruction(&Instruction::Block(wasm_encoder::BlockType::Empty));
                            f.instruction(&Instruction::Loop(wasm_encoder::BlockType::Empty));
                            f.instruction(&Instruction::I32Const(NWRITTEN as i32));
                            f.instruction(&Instruction::I32Load(ma));
                            f.instruction(&Instruction::I32Const(s_i as i32));
                            f.instruction(&Instruction::I32Load(ma));
                            f.instruction(&Instruction::I32Const(s_seg as i32));
                            f.instruction(&Instruction::I32Load(ma));
                            f.instruction(&Instruction::I32Sub);
                            f.instruction(&Instruction::I32GeU);
                            f.instruction(&Instruction::BrIf(1));
                            // heap[4 + j] = s[seg_start + j]
                            f.instruction(&Instruction::GlobalGet(0));
                            f.instruction(&Instruction::I32Const(4));
                            f.instruction(&Instruction::I32Add);
                            f.instruction(&Instruction::I32Const(NWRITTEN as i32));
                            f.instruction(&Instruction::I32Load(ma));
                            f.instruction(&Instruction::I32Add);
                            if let Some(s) = args.first() {
                                self.emit_operand(f, s);
                            }
                            f.instruction(&Instruction::I32Const(s_seg as i32));
                            f.instruction(&Instruction::I32Load(ma));
                            f.instruction(&Instruction::I32Add);
                            f.instruction(&Instruction::I32Const(NWRITTEN as i32));
                            f.instruction(&Instruction::I32Load(ma));
                            f.instruction(&Instruction::I32Add);
                            f.instruction(&Instruction::I32Load8U(ma0));
                            f.instruction(&Instruction::I32Store8(ma0));
                            // j++
                            f.instruction(&Instruction::I32Const(NWRITTEN as i32));
                            f.instruction(&Instruction::I32Const(NWRITTEN as i32));
                            f.instruction(&Instruction::I32Load(ma));
                            f.instruction(&Instruction::I32Const(1));
                            f.instruction(&Instruction::I32Add);
                            f.instruction(&Instruction::I32Store(ma));
                            f.instruction(&Instruction::Br(0));
                            f.instruction(&Instruction::End);
                            f.instruction(&Instruction::End);
                            // Push string ptr (heap+4) into vec
                            f.instruction(&Instruction::I32Const(s_vec as i32));
                            f.instruction(&Instruction::I32Load(ma));
                            f.instruction(&Instruction::I32Const(8));
                            f.instruction(&Instruction::I32Add);
                            f.instruction(&Instruction::I32Load(ma)); // data_ptr
                            f.instruction(&Instruction::I32Const(s_vec as i32));
                            f.instruction(&Instruction::I32Load(ma));
                            f.instruction(&Instruction::I32Load(ma)); // vec.len
                            f.instruction(&Instruction::I32Const(4));
                            f.instruction(&Instruction::I32Mul);
                            f.instruction(&Instruction::I32Add);
                            f.instruction(&Instruction::GlobalGet(0));
                            f.instruction(&Instruction::I32Const(4));
                            f.instruction(&Instruction::I32Add);
                            f.instruction(&Instruction::I32Store(ma));
                            // vec.len++
                            f.instruction(&Instruction::I32Const(s_vec as i32));
                            f.instruction(&Instruction::I32Load(ma));
                            f.instruction(&Instruction::I32Const(s_vec as i32));
                            f.instruction(&Instruction::I32Load(ma));
                            f.instruction(&Instruction::I32Load(ma));
                            f.instruction(&Instruction::I32Const(1));
                            f.instruction(&Instruction::I32Add);
                            f.instruction(&Instruction::I32Store(ma));
                            // Bump heap past string (4 + seg_len)
                            f.instruction(&Instruction::GlobalGet(0));
                            f.instruction(&Instruction::I32Const(4));
                            f.instruction(&Instruction::I32Add);
                            f.instruction(&Instruction::GlobalGet(0));
                            f.instruction(&Instruction::I32Load(ma)); // seg_len
                            f.instruction(&Instruction::I32Add);
                            f.instruction(&Instruction::GlobalSet(0));
                            // seg_start = i + 1
                            f.instruction(&Instruction::I32Const(s_seg as i32));
                            f.instruction(&Instruction::I32Const(s_i as i32));
                            f.instruction(&Instruction::I32Load(ma));
                            f.instruction(&Instruction::I32Const(1));
                            f.instruction(&Instruction::I32Add);
                            f.instruction(&Instruction::I32Store(ma));
                        }
                        f.instruction(&Instruction::End); // end if delim
                        // i++
                        f.instruction(&Instruction::I32Const(s_i as i32));
                        f.instruction(&Instruction::I32Const(s_i as i32));
                        f.instruction(&Instruction::I32Load(ma));
                        f.instruction(&Instruction::I32Const(1));
                        f.instruction(&Instruction::I32Add);
                        f.instruction(&Instruction::I32Store(ma));
                        f.instruction(&Instruction::Br(0));
                        f.instruction(&Instruction::End); // end loop
                        f.instruction(&Instruction::End); // end block
                        // Push final segment [seg_start..slen]
                        f.instruction(&Instruction::GlobalGet(0));
                        f.instruction(&Instruction::I32Const(s_slen as i32));
                        f.instruction(&Instruction::I32Load(ma));
                        f.instruction(&Instruction::I32Const(s_seg as i32));
                        f.instruction(&Instruction::I32Load(ma));
                        f.instruction(&Instruction::I32Sub);
                        f.instruction(&Instruction::I32Store(ma)); // heap[0] = final_seg_len
                        // Copy final segment bytes
                        f.instruction(&Instruction::I32Const(NWRITTEN as i32));
                        f.instruction(&Instruction::I32Const(0));
                        f.instruction(&Instruction::I32Store(ma));
                        f.instruction(&Instruction::Block(wasm_encoder::BlockType::Empty));
                        f.instruction(&Instruction::Loop(wasm_encoder::BlockType::Empty));
                        f.instruction(&Instruction::I32Const(NWRITTEN as i32));
                        f.instruction(&Instruction::I32Load(ma));
                        f.instruction(&Instruction::I32Const(s_slen as i32));
                        f.instruction(&Instruction::I32Load(ma));
                        f.instruction(&Instruction::I32Const(s_seg as i32));
                        f.instruction(&Instruction::I32Load(ma));
                        f.instruction(&Instruction::I32Sub);
                        f.instruction(&Instruction::I32GeU);
                        f.instruction(&Instruction::BrIf(1));
                        f.instruction(&Instruction::GlobalGet(0));
                        f.instruction(&Instruction::I32Const(4));
                        f.instruction(&Instruction::I32Add);
                        f.instruction(&Instruction::I32Const(NWRITTEN as i32));
                        f.instruction(&Instruction::I32Load(ma));
                        f.instruction(&Instruction::I32Add);
                        if let Some(s) = args.first() {
                            self.emit_operand(f, s);
                        }
                        f.instruction(&Instruction::I32Const(s_seg as i32));
                        f.instruction(&Instruction::I32Load(ma));
                        f.instruction(&Instruction::I32Add);
                        f.instruction(&Instruction::I32Const(NWRITTEN as i32));
                        f.instruction(&Instruction::I32Load(ma));
                        f.instruction(&Instruction::I32Add);
                        f.instruction(&Instruction::I32Load8U(ma0));
                        f.instruction(&Instruction::I32Store8(ma0));
                        f.instruction(&Instruction::I32Const(NWRITTEN as i32));
                        f.instruction(&Instruction::I32Const(NWRITTEN as i32));
                        f.instruction(&Instruction::I32Load(ma));
                        f.instruction(&Instruction::I32Const(1));
                        f.instruction(&Instruction::I32Add);
                        f.instruction(&Instruction::I32Store(ma));
                        f.instruction(&Instruction::Br(0));
                        f.instruction(&Instruction::End);
                        f.instruction(&Instruction::End);
                        // Push final string into vec
                        f.instruction(&Instruction::I32Const(s_vec as i32));
                        f.instruction(&Instruction::I32Load(ma));
                        f.instruction(&Instruction::I32Const(8));
                        f.instruction(&Instruction::I32Add);
                        f.instruction(&Instruction::I32Load(ma));
                        f.instruction(&Instruction::I32Const(s_vec as i32));
                        f.instruction(&Instruction::I32Load(ma));
                        f.instruction(&Instruction::I32Load(ma));
                        f.instruction(&Instruction::I32Const(4));
                        f.instruction(&Instruction::I32Mul);
                        f.instruction(&Instruction::I32Add);
                        f.instruction(&Instruction::GlobalGet(0));
                        f.instruction(&Instruction::I32Const(4));
                        f.instruction(&Instruction::I32Add);
                        f.instruction(&Instruction::I32Store(ma));
                        // vec.len++
                        f.instruction(&Instruction::I32Const(s_vec as i32));
                        f.instruction(&Instruction::I32Load(ma));
                        f.instruction(&Instruction::I32Const(s_vec as i32));
                        f.instruction(&Instruction::I32Load(ma));
                        f.instruction(&Instruction::I32Load(ma));
                        f.instruction(&Instruction::I32Const(1));
                        f.instruction(&Instruction::I32Add);
                        f.instruction(&Instruction::I32Store(ma));
                        // Bump heap
                        f.instruction(&Instruction::GlobalGet(0));
                        f.instruction(&Instruction::I32Const(4));
                        f.instruction(&Instruction::I32Add);
                        f.instruction(&Instruction::GlobalGet(0));
                        f.instruction(&Instruction::I32Load(ma));
                        f.instruction(&Instruction::I32Add);
                        f.instruction(&Instruction::GlobalSet(0));
                        // Result: vec_ptr
                        f.instruction(&Instruction::I32Const(s_vec as i32));
                        f.instruction(&Instruction::I32Load(ma));
                    }
                    "parse_i32" => {
                        // parse_i32(s: String) -> Result<i32, String>
                        // Returns enum ptr: tag=0 (Ok) + payload=value, or tag=1 (Err) + payload=err_string
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
                        // Save string ptr to SCRATCH+8
                        f.instruction(&Instruction::I32Const((SCRATCH + 8) as i32));
                        if let Some(a) = args.first() {
                            self.emit_operand(f, a);
                        }
                        f.instruction(&Instruction::I32Store(ma));
                        // Get string len
                        f.instruction(&Instruction::I32Const((SCRATCH + 12) as i32));
                        f.instruction(&Instruction::I32Const((SCRATCH + 8) as i32));
                        f.instruction(&Instruction::I32Load(ma)); // str_ptr
                        f.instruction(&Instruction::I32Const(4));
                        f.instruction(&Instruction::I32Sub);
                        f.instruction(&Instruction::I32Load(ma)); // len
                        f.instruction(&Instruction::I32Store(ma));
                        // Parse: accumulate digits, handle optional leading '-'
                        // result = 0, i = 0, is_neg = 0, is_err = 0
                        f.instruction(&Instruction::I32Const(NWRITTEN as i32));
                        f.instruction(&Instruction::I32Const(0));
                        f.instruction(&Instruction::I32Store(ma)); // result = 0
                        f.instruction(&Instruction::I32Const((SCRATCH + 16) as i32));
                        f.instruction(&Instruction::I32Const(0));
                        f.instruction(&Instruction::I32Store(ma)); // i = 0
                        f.instruction(&Instruction::I32Const((SCRATCH + 20) as i32));
                        f.instruction(&Instruction::I32Const(0));
                        f.instruction(&Instruction::I32Store(ma)); // is_neg = 0
                        f.instruction(&Instruction::I32Const((SCRATCH + 28) as i32));
                        f.instruction(&Instruction::I32Const(0));
                        f.instruction(&Instruction::I32Store(ma)); // is_err = 0

                        // Check for empty string → error
                        f.instruction(&Instruction::I32Const((SCRATCH + 12) as i32));
                        f.instruction(&Instruction::I32Load(ma)); // len
                        f.instruction(&Instruction::I32Eqz);
                        f.instruction(&Instruction::If(wasm_encoder::BlockType::Empty));
                        f.instruction(&Instruction::I32Const((SCRATCH + 28) as i32));
                        f.instruction(&Instruction::I32Const(1));
                        f.instruction(&Instruction::I32Store(ma)); // is_err = 1
                        f.instruction(&Instruction::End);

                        // Check if first char is '-'
                        f.instruction(&Instruction::I32Const((SCRATCH + 28) as i32));
                        f.instruction(&Instruction::I32Load(ma)); // is_err
                        f.instruction(&Instruction::I32Eqz); // !is_err
                        f.instruction(&Instruction::If(wasm_encoder::BlockType::Empty));
                        f.instruction(&Instruction::I32Const((SCRATCH + 8) as i32));
                        f.instruction(&Instruction::I32Load(ma)); // str_ptr
                        f.instruction(&Instruction::I32Load8U(ma0)); // first byte
                        f.instruction(&Instruction::I32Const(45)); // '-'
                        f.instruction(&Instruction::I32Eq);
                        f.instruction(&Instruction::If(wasm_encoder::BlockType::Empty));
                        f.instruction(&Instruction::I32Const((SCRATCH + 20) as i32));
                        f.instruction(&Instruction::I32Const(1));
                        f.instruction(&Instruction::I32Store(ma)); // is_neg = 1
                        f.instruction(&Instruction::I32Const((SCRATCH + 16) as i32));
                        f.instruction(&Instruction::I32Const(1));
                        f.instruction(&Instruction::I32Store(ma)); // i = 1
                        // Check that string isn't just "-"
                        f.instruction(&Instruction::I32Const((SCRATCH + 12) as i32));
                        f.instruction(&Instruction::I32Load(ma)); // len
                        f.instruction(&Instruction::I32Const(1));
                        f.instruction(&Instruction::I32LeU);
                        f.instruction(&Instruction::If(wasm_encoder::BlockType::Empty));
                        f.instruction(&Instruction::I32Const((SCRATCH + 28) as i32));
                        f.instruction(&Instruction::I32Const(1));
                        f.instruction(&Instruction::I32Store(ma)); // is_err = 1
                        f.instruction(&Instruction::End);
                        f.instruction(&Instruction::End);
                        f.instruction(&Instruction::End);

                        // Digit loop
                        f.instruction(&Instruction::Block(wasm_encoder::BlockType::Empty));
                        f.instruction(&Instruction::Loop(wasm_encoder::BlockType::Empty));
                        // if is_err, break
                        f.instruction(&Instruction::I32Const((SCRATCH + 28) as i32));
                        f.instruction(&Instruction::I32Load(ma));
                        f.instruction(&Instruction::BrIf(1));
                        // if i >= len, break
                        f.instruction(&Instruction::I32Const((SCRATCH + 16) as i32));
                        f.instruction(&Instruction::I32Load(ma)); // i
                        f.instruction(&Instruction::I32Const((SCRATCH + 12) as i32));
                        f.instruction(&Instruction::I32Load(ma)); // len
                        f.instruction(&Instruction::I32GeU);
                        f.instruction(&Instruction::BrIf(1));
                        // Load byte at str[i] and store to SCRATCH+32
                        f.instruction(&Instruction::I32Const((SCRATCH + 32) as i32));
                        f.instruction(&Instruction::I32Const((SCRATCH + 8) as i32));
                        f.instruction(&Instruction::I32Load(ma)); // str_ptr
                        f.instruction(&Instruction::I32Const((SCRATCH + 16) as i32));
                        f.instruction(&Instruction::I32Load(ma)); // i
                        f.instruction(&Instruction::I32Add);
                        f.instruction(&Instruction::I32Load8U(ma0)); // byte
                        f.instruction(&Instruction::I32Store(ma)); // mem[SCRATCH+32] = byte
                        // Check byte < '0' || byte > '9'
                        f.instruction(&Instruction::I32Const((SCRATCH + 32) as i32));
                        f.instruction(&Instruction::I32Load(ma));
                        f.instruction(&Instruction::I32Const(48));
                        f.instruction(&Instruction::I32LtU);
                        f.instruction(&Instruction::I32Const((SCRATCH + 32) as i32));
                        f.instruction(&Instruction::I32Load(ma));
                        f.instruction(&Instruction::I32Const(57));
                        f.instruction(&Instruction::I32GtU);
                        f.instruction(&Instruction::I32Or);
                        f.instruction(&Instruction::If(wasm_encoder::BlockType::Empty));
                        f.instruction(&Instruction::I32Const((SCRATCH + 28) as i32));
                        f.instruction(&Instruction::I32Const(1));
                        f.instruction(&Instruction::I32Store(ma)); // is_err = 1
                        f.instruction(&Instruction::Br(2)); // break outer block
                        f.instruction(&Instruction::End);
                        // result = result * 10 + (byte - '0')
                        f.instruction(&Instruction::I32Const(NWRITTEN as i32)); // addr for store
                        f.instruction(&Instruction::I32Const(NWRITTEN as i32));
                        f.instruction(&Instruction::I32Load(ma)); // old_result
                        f.instruction(&Instruction::I32Const(10));
                        f.instruction(&Instruction::I32Mul);
                        f.instruction(&Instruction::I32Const((SCRATCH + 32) as i32));
                        f.instruction(&Instruction::I32Load(ma)); // byte
                        f.instruction(&Instruction::I32Const(48));
                        f.instruction(&Instruction::I32Sub); // digit
                        f.instruction(&Instruction::I32Add); // result*10 + digit
                        f.instruction(&Instruction::I32Store(ma));
                        // i += 1
                        f.instruction(&Instruction::I32Const((SCRATCH + 16) as i32));
                        f.instruction(&Instruction::I32Const((SCRATCH + 16) as i32));
                        f.instruction(&Instruction::I32Load(ma));
                        f.instruction(&Instruction::I32Const(1));
                        f.instruction(&Instruction::I32Add);
                        f.instruction(&Instruction::I32Store(ma));
                        f.instruction(&Instruction::Br(0));
                        f.instruction(&Instruction::End); // end loop
                        f.instruction(&Instruction::End); // end block

                        // Check is_err
                        f.instruction(&Instruction::I32Const((SCRATCH + 28) as i32));
                        f.instruction(&Instruction::I32Load(ma));
                        f.instruction(&Instruction::If(wasm_encoder::BlockType::Result(
                            ValType::I32,
                        )));
                        {
                            // Build Err Result: tag=1, payload=error string ptr
                            // First, create the error string "parse error: invalid integer"
                            let err_msg = b"parse error: invalid integer";
                            let err_len = err_msg.len() as i32;
                            // Allocate string on heap: [len:4][data:N]
                            f.instruction(&Instruction::GlobalGet(0));
                            f.instruction(&Instruction::I32Const(err_len));
                            f.instruction(&Instruction::I32Store(ma)); // store len at heap_ptr
                            // Copy bytes
                            for (j, &b) in err_msg.iter().enumerate() {
                                f.instruction(&Instruction::GlobalGet(0));
                                f.instruction(&Instruction::I32Const(4 + j as i32));
                                f.instruction(&Instruction::I32Add);
                                f.instruction(&Instruction::I32Const(b as i32));
                                f.instruction(&Instruction::I32Store8(ma0));
                            }
                            // str_ptr = heap_ptr + 4 (points to data)
                            // Save str_ptr to SCRATCH+32
                            f.instruction(&Instruction::I32Const((SCRATCH + 32) as i32));
                            f.instruction(&Instruction::GlobalGet(0));
                            f.instruction(&Instruction::I32Const(4));
                            f.instruction(&Instruction::I32Add);
                            f.instruction(&Instruction::I32Store(ma));
                            // Bump heap past string
                            f.instruction(&Instruction::GlobalGet(0));
                            f.instruction(&Instruction::I32Const(4 + err_len));
                            f.instruction(&Instruction::I32Add);
                            f.instruction(&Instruction::GlobalSet(0));
                            // Build enum: [tag=1, payload=str_ptr]
                            f.instruction(&Instruction::GlobalGet(0)); // enum_base (save for result)
                            f.instruction(&Instruction::GlobalGet(0));
                            f.instruction(&Instruction::I32Const(1)); // tag = Err
                            f.instruction(&Instruction::I32Store(ma));
                            f.instruction(&Instruction::GlobalGet(0));
                            f.instruction(&Instruction::I32Const(4));
                            f.instruction(&Instruction::I32Add);
                            f.instruction(&Instruction::I32Const((SCRATCH + 32) as i32));
                            f.instruction(&Instruction::I32Load(ma)); // str_ptr
                            f.instruction(&Instruction::I32Store(ma));
                            // Bump heap past enum (8 bytes)
                            f.instruction(&Instruction::GlobalGet(0));
                            f.instruction(&Instruction::I32Const(8));
                            f.instruction(&Instruction::I32Add);
                            f.instruction(&Instruction::GlobalSet(0));
                            // enum_base is on stack — this will be left for the outer if/else
                        }
                        f.instruction(&Instruction::Else);
                        {
                            // Apply negation if is_neg
                            f.instruction(&Instruction::I32Const((SCRATCH + 20) as i32));
                            f.instruction(&Instruction::I32Load(ma)); // is_neg
                            f.instruction(&Instruction::If(wasm_encoder::BlockType::Empty));
                            f.instruction(&Instruction::I32Const(NWRITTEN as i32));
                            f.instruction(&Instruction::I32Const(0));
                            f.instruction(&Instruction::I32Const(NWRITTEN as i32));
                            f.instruction(&Instruction::I32Load(ma));
                            f.instruction(&Instruction::I32Sub);
                            f.instruction(&Instruction::I32Store(ma)); // result = -result
                            f.instruction(&Instruction::End);
                            // Build Result::Ok(value) enum on heap
                            f.instruction(&Instruction::GlobalGet(0)); // save base
                            f.instruction(&Instruction::GlobalGet(0));
                            f.instruction(&Instruction::I32Const(0)); // tag = Ok
                            f.instruction(&Instruction::I32Store(ma));
                            f.instruction(&Instruction::GlobalGet(0));
                            f.instruction(&Instruction::I32Const(4));
                            f.instruction(&Instruction::I32Add);
                            f.instruction(&Instruction::I32Const(NWRITTEN as i32));
                            f.instruction(&Instruction::I32Load(ma)); // result value
                            f.instruction(&Instruction::I32Store(ma));
                            // bump heap
                            f.instruction(&Instruction::GlobalGet(0));
                            f.instruction(&Instruction::I32Const(8));
                            f.instruction(&Instruction::I32Add);
                            f.instruction(&Instruction::GlobalSet(0));
                            // enum_base on stack
                        }
                        f.instruction(&Instruction::End); // end if/else
                    }
                    "parse_i64" => {
                        // parse_i64(s: String) -> i64
                        // Returns parsed i64, or 0 on error
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
                        let ma8 = MemArg {
                            offset: 0,
                            align: 2,
                            memory_index: 0,
                        };
                        // Save string ptr to SCRATCH+8
                        f.instruction(&Instruction::I32Const((SCRATCH + 8) as i32));
                        if let Some(a) = args.first() {
                            self.emit_operand(f, a);
                        }
                        f.instruction(&Instruction::I32Store(ma));
                        // Get string len
                        f.instruction(&Instruction::I32Const((SCRATCH + 12) as i32));
                        f.instruction(&Instruction::I32Const((SCRATCH + 8) as i32));
                        f.instruction(&Instruction::I32Load(ma)); // str_ptr
                        f.instruction(&Instruction::I32Const(4));
                        f.instruction(&Instruction::I32Sub);
                        f.instruction(&Instruction::I32Load(ma)); // len
                        f.instruction(&Instruction::I32Store(ma));
                        // Initialize: result=0i64, i=0, is_neg=0, is_err=0
                        f.instruction(&Instruction::I32Const(NWRITTEN as i32));
                        f.instruction(&Instruction::I64Const(0));
                        f.instruction(&Instruction::I64Store(ma8)); // result = 0i64
                        f.instruction(&Instruction::I32Const((SCRATCH + 16) as i32));
                        f.instruction(&Instruction::I32Const(0));
                        f.instruction(&Instruction::I32Store(ma)); // i = 0
                        f.instruction(&Instruction::I32Const((SCRATCH + 20) as i32));
                        f.instruction(&Instruction::I32Const(0));
                        f.instruction(&Instruction::I32Store(ma)); // is_neg = 0
                        f.instruction(&Instruction::I32Const((SCRATCH + 28) as i32));
                        f.instruction(&Instruction::I32Const(0));
                        f.instruction(&Instruction::I32Store(ma)); // is_err = 0

                        // Check for empty string → error
                        f.instruction(&Instruction::I32Const((SCRATCH + 12) as i32));
                        f.instruction(&Instruction::I32Load(ma)); // len
                        f.instruction(&Instruction::I32Eqz);
                        f.instruction(&Instruction::If(wasm_encoder::BlockType::Empty));
                        f.instruction(&Instruction::I32Const((SCRATCH + 28) as i32));
                        f.instruction(&Instruction::I32Const(1));
                        f.instruction(&Instruction::I32Store(ma)); // is_err = 1
                        f.instruction(&Instruction::End);

                        // Check if first char is '-'
                        f.instruction(&Instruction::I32Const((SCRATCH + 28) as i32));
                        f.instruction(&Instruction::I32Load(ma)); // is_err
                        f.instruction(&Instruction::I32Eqz); // !is_err
                        f.instruction(&Instruction::If(wasm_encoder::BlockType::Empty));
                        f.instruction(&Instruction::I32Const((SCRATCH + 8) as i32));
                        f.instruction(&Instruction::I32Load(ma)); // str_ptr
                        f.instruction(&Instruction::I32Load8U(ma0)); // first byte
                        f.instruction(&Instruction::I32Const(45)); // '-'
                        f.instruction(&Instruction::I32Eq);
                        f.instruction(&Instruction::If(wasm_encoder::BlockType::Empty));
                        f.instruction(&Instruction::I32Const((SCRATCH + 20) as i32));
                        f.instruction(&Instruction::I32Const(1));
                        f.instruction(&Instruction::I32Store(ma)); // is_neg = 1
                        f.instruction(&Instruction::I32Const((SCRATCH + 16) as i32));
                        f.instruction(&Instruction::I32Const(1));
                        f.instruction(&Instruction::I32Store(ma)); // i = 1
                        // Check that string isn't just "-"
                        f.instruction(&Instruction::I32Const((SCRATCH + 12) as i32));
                        f.instruction(&Instruction::I32Load(ma)); // len
                        f.instruction(&Instruction::I32Const(1));
                        f.instruction(&Instruction::I32LeU);
                        f.instruction(&Instruction::If(wasm_encoder::BlockType::Empty));
                        f.instruction(&Instruction::I32Const((SCRATCH + 28) as i32));
                        f.instruction(&Instruction::I32Const(1));
                        f.instruction(&Instruction::I32Store(ma)); // is_err = 1
                        f.instruction(&Instruction::End);
                        f.instruction(&Instruction::End);
                        f.instruction(&Instruction::End);

                        // Digit loop
                        f.instruction(&Instruction::Block(wasm_encoder::BlockType::Empty));
                        f.instruction(&Instruction::Loop(wasm_encoder::BlockType::Empty));
                        // if is_err, break
                        f.instruction(&Instruction::I32Const((SCRATCH + 28) as i32));
                        f.instruction(&Instruction::I32Load(ma));
                        f.instruction(&Instruction::BrIf(1));
                        // if i >= len, break
                        f.instruction(&Instruction::I32Const((SCRATCH + 16) as i32));
                        f.instruction(&Instruction::I32Load(ma)); // i
                        f.instruction(&Instruction::I32Const((SCRATCH + 12) as i32));
                        f.instruction(&Instruction::I32Load(ma)); // len
                        f.instruction(&Instruction::I32GeU);
                        f.instruction(&Instruction::BrIf(1));
                        // Load byte at str[i] and store to SCRATCH+32
                        f.instruction(&Instruction::I32Const((SCRATCH + 32) as i32));
                        f.instruction(&Instruction::I32Const((SCRATCH + 8) as i32));
                        f.instruction(&Instruction::I32Load(ma)); // str_ptr
                        f.instruction(&Instruction::I32Const((SCRATCH + 16) as i32));
                        f.instruction(&Instruction::I32Load(ma)); // i
                        f.instruction(&Instruction::I32Add);
                        f.instruction(&Instruction::I32Load8U(ma0)); // byte
                        f.instruction(&Instruction::I32Store(ma)); // mem[SCRATCH+32] = byte
                        // Check byte < '0' || byte > '9'
                        f.instruction(&Instruction::I32Const((SCRATCH + 32) as i32));
                        f.instruction(&Instruction::I32Load(ma));
                        f.instruction(&Instruction::I32Const(48));
                        f.instruction(&Instruction::I32LtU);
                        f.instruction(&Instruction::I32Const((SCRATCH + 32) as i32));
                        f.instruction(&Instruction::I32Load(ma));
                        f.instruction(&Instruction::I32Const(57));
                        f.instruction(&Instruction::I32GtU);
                        f.instruction(&Instruction::I32Or);
                        f.instruction(&Instruction::If(wasm_encoder::BlockType::Empty));
                        f.instruction(&Instruction::I32Const((SCRATCH + 28) as i32));
                        f.instruction(&Instruction::I32Const(1));
                        f.instruction(&Instruction::I32Store(ma)); // is_err = 1
                        f.instruction(&Instruction::Br(2)); // break outer block
                        f.instruction(&Instruction::End);
                        // result = result * 10i64 + i64.extend_i32_u(byte - '0')
                        f.instruction(&Instruction::I32Const(NWRITTEN as i32)); // addr for store
                        f.instruction(&Instruction::I32Const(NWRITTEN as i32));
                        f.instruction(&Instruction::I64Load(ma8)); // old_result
                        f.instruction(&Instruction::I64Const(10));
                        f.instruction(&Instruction::I64Mul);
                        f.instruction(&Instruction::I32Const((SCRATCH + 32) as i32));
                        f.instruction(&Instruction::I32Load(ma)); // byte
                        f.instruction(&Instruction::I32Const(48));
                        f.instruction(&Instruction::I32Sub); // digit (i32)
                        f.instruction(&Instruction::I64ExtendI32U); // digit as i64
                        f.instruction(&Instruction::I64Add); // result*10 + digit
                        f.instruction(&Instruction::I64Store(ma8));
                        // i += 1
                        f.instruction(&Instruction::I32Const((SCRATCH + 16) as i32));
                        f.instruction(&Instruction::I32Const((SCRATCH + 16) as i32));
                        f.instruction(&Instruction::I32Load(ma));
                        f.instruction(&Instruction::I32Const(1));
                        f.instruction(&Instruction::I32Add);
                        f.instruction(&Instruction::I32Store(ma));
                        f.instruction(&Instruction::Br(0));
                        f.instruction(&Instruction::End); // end loop
                        f.instruction(&Instruction::End); // end block

                        // Produce result: if is_err → 0i64, else → value (negated if needed)
                        f.instruction(&Instruction::I32Const((SCRATCH + 28) as i32));
                        f.instruction(&Instruction::I32Load(ma));
                        f.instruction(&Instruction::If(wasm_encoder::BlockType::Result(
                            ValType::I64,
                        )));
                        {
                            f.instruction(&Instruction::I64Const(0));
                        }
                        f.instruction(&Instruction::Else);
                        {
                            // Apply negation if is_neg
                            f.instruction(&Instruction::I32Const((SCRATCH + 20) as i32));
                            f.instruction(&Instruction::I32Load(ma)); // is_neg
                            f.instruction(&Instruction::If(wasm_encoder::BlockType::Empty));
                            f.instruction(&Instruction::I32Const(NWRITTEN as i32));
                            f.instruction(&Instruction::I64Const(0));
                            f.instruction(&Instruction::I32Const(NWRITTEN as i32));
                            f.instruction(&Instruction::I64Load(ma8));
                            f.instruction(&Instruction::I64Sub);
                            f.instruction(&Instruction::I64Store(ma8)); // result = -result
                            f.instruction(&Instruction::End);
                            // Push result
                            f.instruction(&Instruction::I32Const(NWRITTEN as i32));
                            f.instruction(&Instruction::I64Load(ma8));
                        }
                        f.instruction(&Instruction::End); // end if/else
                    }
                    "clock_now" => {
                        // clock_time_get(clock_id=0 (realtime), precision=0, result_ptr=SCRATCH)
                        f.instruction(&Instruction::I32Const(0)); // clock_id = REALTIME
                        f.instruction(&Instruction::I64Const(0)); // precision
                        f.instruction(&Instruction::I32Const(SCRATCH as i32)); // result buffer
                        f.instruction(&Instruction::Call(FN_CLOCK_TIME_GET));
                        f.instruction(&Instruction::Drop); // drop errno
                        // Load i64 result from SCRATCH
                        f.instruction(&Instruction::I32Const(SCRATCH as i32));
                        f.instruction(&Instruction::I64Load(MemArg {
                            offset: 0,
                            align: 3,
                            memory_index: 0,
                        }));
                    }
                    "random_i32" => {
                        // random_get(buf_ptr=SCRATCH, buf_len=4)
                        f.instruction(&Instruction::I32Const(SCRATCH as i32));
                        f.instruction(&Instruction::I32Const(4));
                        f.instruction(&Instruction::Call(FN_RANDOM_GET));
                        f.instruction(&Instruction::Drop); // drop errno
                        // Load i32 result from SCRATCH
                        f.instruction(&Instruction::I32Const(SCRATCH as i32));
                        f.instruction(&Instruction::I32Load(MemArg {
                            offset: 0,
                            align: 2,
                            memory_index: 0,
                        }));
                    }
                    "parse_f64" => {
                        // parse_f64(s: String) -> f64
                        // Returns parsed f64, or 0.0 on error
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
                        let ma_f64 = MemArg {
                            offset: 0,
                            align: 2,
                            memory_index: 0,
                        };
                        // Save string ptr to SCRATCH+8
                        f.instruction(&Instruction::I32Const((SCRATCH + 8) as i32));
                        if let Some(a) = args.first() {
                            self.emit_operand(f, a);
                        }
                        f.instruction(&Instruction::I32Store(ma));
                        // Get string len
                        f.instruction(&Instruction::I32Const((SCRATCH + 12) as i32));
                        f.instruction(&Instruction::I32Const((SCRATCH + 8) as i32));
                        f.instruction(&Instruction::I32Load(ma)); // str_ptr
                        f.instruction(&Instruction::I32Const(4));
                        f.instruction(&Instruction::I32Sub);
                        f.instruction(&Instruction::I32Load(ma)); // len
                        f.instruction(&Instruction::I32Store(ma));
                        // Initialize
                        f.instruction(&Instruction::I32Const(NWRITTEN as i32));
                        f.instruction(&Instruction::F64Const(0.0));
                        f.instruction(&Instruction::F64Store(ma_f64)); // result = 0.0
                        f.instruction(&Instruction::I32Const((SCRATCH + 16) as i32));
                        f.instruction(&Instruction::I32Const(0));
                        f.instruction(&Instruction::I32Store(ma)); // i = 0
                        f.instruction(&Instruction::I32Const((SCRATCH + 20) as i32));
                        f.instruction(&Instruction::I32Const(0));
                        f.instruction(&Instruction::I32Store(ma)); // is_neg = 0
                        f.instruction(&Instruction::I32Const((SCRATCH + 28) as i32));
                        f.instruction(&Instruction::I32Const(0));
                        f.instruction(&Instruction::I32Store(ma)); // is_err = 0
                        f.instruction(&Instruction::I32Const((SCRATCH + 36) as i32));
                        f.instruction(&Instruction::I32Const(0));
                        f.instruction(&Instruction::I32Store(ma)); // saw_dot = 0
                        f.instruction(&Instruction::I32Const((SCRATCH + 40) as i32));
                        f.instruction(&Instruction::I32Const(0));
                        f.instruction(&Instruction::I32Store(ma)); // decimal_count = 0

                        // Check for empty string → error
                        f.instruction(&Instruction::I32Const((SCRATCH + 12) as i32));
                        f.instruction(&Instruction::I32Load(ma)); // len
                        f.instruction(&Instruction::I32Eqz);
                        f.instruction(&Instruction::If(wasm_encoder::BlockType::Empty));
                        f.instruction(&Instruction::I32Const((SCRATCH + 28) as i32));
                        f.instruction(&Instruction::I32Const(1));
                        f.instruction(&Instruction::I32Store(ma)); // is_err = 1
                        f.instruction(&Instruction::End);

                        // Check if first char is '-'
                        f.instruction(&Instruction::I32Const((SCRATCH + 28) as i32));
                        f.instruction(&Instruction::I32Load(ma)); // is_err
                        f.instruction(&Instruction::I32Eqz); // !is_err
                        f.instruction(&Instruction::If(wasm_encoder::BlockType::Empty));
                        f.instruction(&Instruction::I32Const((SCRATCH + 8) as i32));
                        f.instruction(&Instruction::I32Load(ma)); // str_ptr
                        f.instruction(&Instruction::I32Load8U(ma0)); // first byte
                        f.instruction(&Instruction::I32Const(45)); // '-'
                        f.instruction(&Instruction::I32Eq);
                        f.instruction(&Instruction::If(wasm_encoder::BlockType::Empty));
                        f.instruction(&Instruction::I32Const((SCRATCH + 20) as i32));
                        f.instruction(&Instruction::I32Const(1));
                        f.instruction(&Instruction::I32Store(ma)); // is_neg = 1
                        f.instruction(&Instruction::I32Const((SCRATCH + 16) as i32));
                        f.instruction(&Instruction::I32Const(1));
                        f.instruction(&Instruction::I32Store(ma)); // i = 1
                        // Check that string isn't just "-"
                        f.instruction(&Instruction::I32Const((SCRATCH + 12) as i32));
                        f.instruction(&Instruction::I32Load(ma)); // len
                        f.instruction(&Instruction::I32Const(1));
                        f.instruction(&Instruction::I32LeU);
                        f.instruction(&Instruction::If(wasm_encoder::BlockType::Empty));
                        f.instruction(&Instruction::I32Const((SCRATCH + 28) as i32));
                        f.instruction(&Instruction::I32Const(1));
                        f.instruction(&Instruction::I32Store(ma)); // is_err = 1
                        f.instruction(&Instruction::End);
                        f.instruction(&Instruction::End);
                        f.instruction(&Instruction::End);

                        // Digit loop (handles digits and '.')
                        f.instruction(&Instruction::Block(wasm_encoder::BlockType::Empty));
                        f.instruction(&Instruction::Loop(wasm_encoder::BlockType::Empty));
                        // if is_err, break
                        f.instruction(&Instruction::I32Const((SCRATCH + 28) as i32));
                        f.instruction(&Instruction::I32Load(ma));
                        f.instruction(&Instruction::BrIf(1));
                        // if i >= len, break
                        f.instruction(&Instruction::I32Const((SCRATCH + 16) as i32));
                        f.instruction(&Instruction::I32Load(ma)); // i
                        f.instruction(&Instruction::I32Const((SCRATCH + 12) as i32));
                        f.instruction(&Instruction::I32Load(ma)); // len
                        f.instruction(&Instruction::I32GeU);
                        f.instruction(&Instruction::BrIf(1));
                        // Load byte at str[i] and store to SCRATCH+32
                        f.instruction(&Instruction::I32Const((SCRATCH + 32) as i32));
                        f.instruction(&Instruction::I32Const((SCRATCH + 8) as i32));
                        f.instruction(&Instruction::I32Load(ma)); // str_ptr
                        f.instruction(&Instruction::I32Const((SCRATCH + 16) as i32));
                        f.instruction(&Instruction::I32Load(ma)); // i
                        f.instruction(&Instruction::I32Add);
                        f.instruction(&Instruction::I32Load8U(ma0)); // byte
                        f.instruction(&Instruction::I32Store(ma)); // mem[SCRATCH+32] = byte
                        // Check if byte == '.' (46)
                        f.instruction(&Instruction::I32Const((SCRATCH + 32) as i32));
                        f.instruction(&Instruction::I32Load(ma));
                        f.instruction(&Instruction::I32Const(46)); // '.'
                        f.instruction(&Instruction::I32Eq);
                        f.instruction(&Instruction::If(wasm_encoder::BlockType::Empty));
                        {
                            // Check if we already saw a dot → error
                            f.instruction(&Instruction::I32Const((SCRATCH + 36) as i32));
                            f.instruction(&Instruction::I32Load(ma));
                            f.instruction(&Instruction::If(wasm_encoder::BlockType::Empty));
                            f.instruction(&Instruction::I32Const((SCRATCH + 28) as i32));
                            f.instruction(&Instruction::I32Const(1));
                            f.instruction(&Instruction::I32Store(ma)); // is_err = 1
                            f.instruction(&Instruction::Br(3)); // break outer block
                            f.instruction(&Instruction::End);
                            // Set saw_dot = 1
                            f.instruction(&Instruction::I32Const((SCRATCH + 36) as i32));
                            f.instruction(&Instruction::I32Const(1));
                            f.instruction(&Instruction::I32Store(ma));
                            // i += 1
                            f.instruction(&Instruction::I32Const((SCRATCH + 16) as i32));
                            f.instruction(&Instruction::I32Const((SCRATCH + 16) as i32));
                            f.instruction(&Instruction::I32Load(ma));
                            f.instruction(&Instruction::I32Const(1));
                            f.instruction(&Instruction::I32Add);
                            f.instruction(&Instruction::I32Store(ma));
                            f.instruction(&Instruction::Br(1)); // continue loop
                        }
                        f.instruction(&Instruction::End); // end dot check
                        // Check byte < '0' || byte > '9'
                        f.instruction(&Instruction::I32Const((SCRATCH + 32) as i32));
                        f.instruction(&Instruction::I32Load(ma));
                        f.instruction(&Instruction::I32Const(48));
                        f.instruction(&Instruction::I32LtU);
                        f.instruction(&Instruction::I32Const((SCRATCH + 32) as i32));
                        f.instruction(&Instruction::I32Load(ma));
                        f.instruction(&Instruction::I32Const(57));
                        f.instruction(&Instruction::I32GtU);
                        f.instruction(&Instruction::I32Or);
                        f.instruction(&Instruction::If(wasm_encoder::BlockType::Empty));
                        f.instruction(&Instruction::I32Const((SCRATCH + 28) as i32));
                        f.instruction(&Instruction::I32Const(1));
                        f.instruction(&Instruction::I32Store(ma)); // is_err = 1
                        f.instruction(&Instruction::Br(2)); // break outer block
                        f.instruction(&Instruction::End);
                        // result = result * 10.0 + f64.convert_i32_u(byte - '0')
                        f.instruction(&Instruction::I32Const(NWRITTEN as i32)); // addr for store
                        f.instruction(&Instruction::I32Const(NWRITTEN as i32));
                        f.instruction(&Instruction::F64Load(ma_f64)); // old_result
                        f.instruction(&Instruction::F64Const(10.0));
                        f.instruction(&Instruction::F64Mul);
                        f.instruction(&Instruction::I32Const((SCRATCH + 32) as i32));
                        f.instruction(&Instruction::I32Load(ma)); // byte
                        f.instruction(&Instruction::I32Const(48));
                        f.instruction(&Instruction::I32Sub); // digit (i32)
                        f.instruction(&Instruction::F64ConvertI32U); // digit as f64
                        f.instruction(&Instruction::F64Add); // result * 10.0 + digit
                        f.instruction(&Instruction::F64Store(ma_f64));
                        // if saw_dot: decimal_count += 1
                        f.instruction(&Instruction::I32Const((SCRATCH + 36) as i32));
                        f.instruction(&Instruction::I32Load(ma)); // saw_dot
                        f.instruction(&Instruction::If(wasm_encoder::BlockType::Empty));
                        f.instruction(&Instruction::I32Const((SCRATCH + 40) as i32));
                        f.instruction(&Instruction::I32Const((SCRATCH + 40) as i32));
                        f.instruction(&Instruction::I32Load(ma));
                        f.instruction(&Instruction::I32Const(1));
                        f.instruction(&Instruction::I32Add);
                        f.instruction(&Instruction::I32Store(ma)); // decimal_count++
                        f.instruction(&Instruction::End);
                        // i += 1
                        f.instruction(&Instruction::I32Const((SCRATCH + 16) as i32));
                        f.instruction(&Instruction::I32Const((SCRATCH + 16) as i32));
                        f.instruction(&Instruction::I32Load(ma));
                        f.instruction(&Instruction::I32Const(1));
                        f.instruction(&Instruction::I32Add);
                        f.instruction(&Instruction::I32Store(ma));
                        f.instruction(&Instruction::Br(0));
                        f.instruction(&Instruction::End); // end loop
                        f.instruction(&Instruction::End); // end block

                        // Produce result
                        f.instruction(&Instruction::I32Const((SCRATCH + 28) as i32));
                        f.instruction(&Instruction::I32Load(ma));
                        f.instruction(&Instruction::If(wasm_encoder::BlockType::Result(
                            ValType::F64,
                        )));
                        {
                            f.instruction(&Instruction::F64Const(0.0));
                        }
                        f.instruction(&Instruction::Else);
                        {
                            // Compute divisor = 10^decimal_count via loop
                            // Store divisor at SCRATCH+44 (f64, 8 bytes)
                            f.instruction(&Instruction::I32Const((SCRATCH + 44) as i32));
                            f.instruction(&Instruction::F64Const(1.0));
                            f.instruction(&Instruction::F64Store(ma_f64)); // divisor = 1.0
                            // Loop: while decimal_count > 0, divisor *= 10.0
                            f.instruction(&Instruction::Block(wasm_encoder::BlockType::Empty));
                            f.instruction(&Instruction::Loop(wasm_encoder::BlockType::Empty));
                            f.instruction(&Instruction::I32Const((SCRATCH + 40) as i32));
                            f.instruction(&Instruction::I32Load(ma)); // decimal_count
                            f.instruction(&Instruction::I32Eqz);
                            f.instruction(&Instruction::BrIf(1)); // break if 0
                            // divisor *= 10.0
                            f.instruction(&Instruction::I32Const((SCRATCH + 44) as i32));
                            f.instruction(&Instruction::I32Const((SCRATCH + 44) as i32));
                            f.instruction(&Instruction::F64Load(ma_f64));
                            f.instruction(&Instruction::F64Const(10.0));
                            f.instruction(&Instruction::F64Mul);
                            f.instruction(&Instruction::F64Store(ma_f64));
                            // decimal_count -= 1
                            f.instruction(&Instruction::I32Const((SCRATCH + 40) as i32));
                            f.instruction(&Instruction::I32Const((SCRATCH + 40) as i32));
                            f.instruction(&Instruction::I32Load(ma));
                            f.instruction(&Instruction::I32Const(1));
                            f.instruction(&Instruction::I32Sub);
                            f.instruction(&Instruction::I32Store(ma));
                            f.instruction(&Instruction::Br(0)); // continue
                            f.instruction(&Instruction::End); // end loop
                            f.instruction(&Instruction::End); // end block

                            // Divide result by divisor (only if saw_dot)
                            f.instruction(&Instruction::I32Const((SCRATCH + 36) as i32));
                            f.instruction(&Instruction::I32Load(ma)); // saw_dot
                            f.instruction(&Instruction::If(wasm_encoder::BlockType::Empty));
                            f.instruction(&Instruction::I32Const(NWRITTEN as i32));
                            f.instruction(&Instruction::I32Const(NWRITTEN as i32));
                            f.instruction(&Instruction::F64Load(ma_f64));
                            f.instruction(&Instruction::I32Const((SCRATCH + 44) as i32));
                            f.instruction(&Instruction::F64Load(ma_f64));
                            f.instruction(&Instruction::F64Div);
                            f.instruction(&Instruction::F64Store(ma_f64));
                            f.instruction(&Instruction::End);

                            // Apply negation if is_neg
                            f.instruction(&Instruction::I32Const((SCRATCH + 20) as i32));
                            f.instruction(&Instruction::I32Load(ma)); // is_neg
                            f.instruction(&Instruction::If(wasm_encoder::BlockType::Empty));
                            f.instruction(&Instruction::I32Const(NWRITTEN as i32));
                            f.instruction(&Instruction::I32Const(NWRITTEN as i32));
                            f.instruction(&Instruction::F64Load(ma_f64));
                            f.instruction(&Instruction::F64Neg);
                            f.instruction(&Instruction::F64Store(ma_f64));
                            f.instruction(&Instruction::End);

                            // Push result
                            f.instruction(&Instruction::I32Const(NWRITTEN as i32));
                            f.instruction(&Instruction::F64Load(ma_f64));
                        }
                        f.instruction(&Instruction::End); // end if/else
                    }
                    "fs_read_file" => {
                        // fs_read_file(path: String) -> Result<String, String>
                        // path is a length-prefixed string pointer
                        // Uses WASI path_open + fd_read + fd_close
                        // Returns heap-allocated enum: tag=0(Ok)+str_ptr or tag=1(Err)+err_str_ptr
                        let ma = MemArg {
                            offset: 0,
                            align: 2,
                            memory_index: 0,
                        };

                        // Save path_ptr to FS_SCRATCH+8
                        f.instruction(&Instruction::I32Const((FS_SCRATCH + 8) as i32));
                        self.emit_operand(f, &args[0]);
                        f.instruction(&Instruction::I32Store(ma));

                        // Get path_len from length prefix (ptr - 4)
                        f.instruction(&Instruction::I32Const((FS_SCRATCH + 12) as i32));
                        f.instruction(&Instruction::I32Const((FS_SCRATCH + 8) as i32));
                        f.instruction(&Instruction::I32Load(ma)); // path_ptr
                        f.instruction(&Instruction::I32Const(4));
                        f.instruction(&Instruction::I32Sub);
                        f.instruction(&Instruction::I32Load(ma)); // path_len
                        f.instruction(&Instruction::I32Store(ma));

                        // Call path_open(dirfd=3, dirflags=0, path, path_len, oflags=0,
                        //   fs_rights_base=FD_READ(2), fs_rights_inheriting=0, fdflags=0, &opened_fd)
                        f.instruction(&Instruction::I32Const(3)); // dirfd = 3 (first preopened dir)
                        f.instruction(&Instruction::I32Const(0)); // dirflags
                        f.instruction(&Instruction::I32Const((FS_SCRATCH + 8) as i32));
                        f.instruction(&Instruction::I32Load(ma)); // path_ptr
                        f.instruction(&Instruction::I32Const((FS_SCRATCH + 12) as i32));
                        f.instruction(&Instruction::I32Load(ma)); // path_len
                        f.instruction(&Instruction::I32Const(0)); // oflags = 0
                        f.instruction(&Instruction::I64Const(2)); // fs_rights_base = FD_READ
                        f.instruction(&Instruction::I64Const(0)); // fs_rights_inheriting
                        f.instruction(&Instruction::I32Const(0)); // fdflags
                        f.instruction(&Instruction::I32Const(FS_SCRATCH as i32)); // &opened_fd
                        f.instruction(&Instruction::Call(FN_PATH_OPEN));

                        // Check errno: 0 = success
                        f.instruction(&Instruction::I32Const(0));
                        f.instruction(&Instruction::I32Ne);
                        f.instruction(&Instruction::If(wasm_encoder::BlockType::Result(
                            ValType::I32,
                        )));
                        {
                            // path_open failed → return Err("file open error")
                            let err_msg = b"file open error";
                            let err_len = err_msg.len() as i32;
                            f.instruction(&Instruction::GlobalGet(0));
                            f.instruction(&Instruction::I32Const(err_len));
                            f.instruction(&Instruction::I32Store(ma));
                            for (i, &byte) in err_msg.iter().enumerate() {
                                f.instruction(&Instruction::GlobalGet(0));
                                f.instruction(&Instruction::I32Const(4 + i as i32));
                                f.instruction(&Instruction::I32Add);
                                f.instruction(&Instruction::I32Const(byte as i32));
                                f.instruction(&Instruction::I32Store8(MemArg {
                                    offset: 0,
                                    align: 0,
                                    memory_index: 0,
                                }));
                            }
                            f.instruction(&Instruction::I32Const((FS_SCRATCH + 16) as i32));
                            f.instruction(&Instruction::GlobalGet(0));
                            f.instruction(&Instruction::I32Const(4));
                            f.instruction(&Instruction::I32Add);
                            f.instruction(&Instruction::I32Store(ma));
                            f.instruction(&Instruction::GlobalGet(0));
                            f.instruction(&Instruction::I32Const(4 + err_len));
                            f.instruction(&Instruction::I32Add);
                            f.instruction(&Instruction::GlobalSet(0));
                            // Build Err enum: [tag=1][payload=str_ptr]
                            f.instruction(&Instruction::GlobalGet(0));
                            f.instruction(&Instruction::GlobalGet(0));
                            f.instruction(&Instruction::I32Const(1));
                            f.instruction(&Instruction::I32Store(ma));
                            f.instruction(&Instruction::GlobalGet(0));
                            f.instruction(&Instruction::I32Const(4));
                            f.instruction(&Instruction::I32Add);
                            f.instruction(&Instruction::I32Const((FS_SCRATCH + 16) as i32));
                            f.instruction(&Instruction::I32Load(ma));
                            f.instruction(&Instruction::I32Store(ma));
                            f.instruction(&Instruction::GlobalGet(0));
                            f.instruction(&Instruction::I32Const(8));
                            f.instruction(&Instruction::I32Add);
                            f.instruction(&Instruction::GlobalSet(0));
                        }
                        f.instruction(&Instruction::Else);
                        {
                            // path_open succeeded — read file contents
                            f.instruction(&Instruction::I32Const((FS_SCRATCH + 20) as i32));
                            f.instruction(&Instruction::GlobalGet(0));
                            f.instruction(&Instruction::I32Store(ma)); // save buf_start
                            // Skip 4 bytes for length prefix
                            f.instruction(&Instruction::GlobalGet(0));
                            f.instruction(&Instruction::I32Const(4));
                            f.instruction(&Instruction::I32Add);
                            f.instruction(&Instruction::GlobalSet(0));

                            // Read loop
                            f.instruction(&Instruction::Block(wasm_encoder::BlockType::Empty));
                            f.instruction(&Instruction::Loop(wasm_encoder::BlockType::Empty));
                            {
                                // Set up iovec: [ptr=global(0), len=FS_BUF_SIZE]
                                f.instruction(&Instruction::I32Const(IOV_BASE as i32));
                                f.instruction(&Instruction::GlobalGet(0));
                                f.instruction(&Instruction::I32Store(ma));
                                f.instruction(&Instruction::I32Const((IOV_BASE + 4) as i32));
                                f.instruction(&Instruction::I32Const(FS_BUF_SIZE as i32));
                                f.instruction(&Instruction::I32Store(ma));

                                // fd_read(fd, &iov, 1, &nread)
                                f.instruction(&Instruction::I32Const(FS_SCRATCH as i32));
                                f.instruction(&Instruction::I32Load(ma));
                                f.instruction(&Instruction::I32Const(IOV_BASE as i32));
                                f.instruction(&Instruction::I32Const(1));
                                f.instruction(&Instruction::I32Const(FS_NREAD as i32));
                                f.instruction(&Instruction::Call(FN_FD_READ));
                                f.instruction(&Instruction::Drop);

                                // if nread == 0, break
                                f.instruction(&Instruction::I32Const(FS_NREAD as i32));
                                f.instruction(&Instruction::I32Load(ma));
                                f.instruction(&Instruction::I32Eqz);
                                f.instruction(&Instruction::BrIf(1)); // break outer block

                                // Bump heap by nread
                                f.instruction(&Instruction::GlobalGet(0));
                                f.instruction(&Instruction::I32Const(FS_NREAD as i32));
                                f.instruction(&Instruction::I32Load(ma));
                                f.instruction(&Instruction::I32Add);
                                f.instruction(&Instruction::GlobalSet(0));

                                f.instruction(&Instruction::Br(0)); // continue loop
                            }
                            f.instruction(&Instruction::End); // end loop
                            f.instruction(&Instruction::End); // end block

                            // Store total_len at buf_start
                            f.instruction(&Instruction::I32Const((FS_SCRATCH + 20) as i32));
                            f.instruction(&Instruction::I32Load(ma)); // buf_start
                            f.instruction(&Instruction::GlobalGet(0));
                            f.instruction(&Instruction::I32Const((FS_SCRATCH + 20) as i32));
                            f.instruction(&Instruction::I32Load(ma));
                            f.instruction(&Instruction::I32Const(4));
                            f.instruction(&Instruction::I32Add);
                            f.instruction(&Instruction::I32Sub); // total_len = end - (start+4)
                            f.instruction(&Instruction::I32Store(ma));

                            // fd_close(fd)
                            f.instruction(&Instruction::I32Const(FS_SCRATCH as i32));
                            f.instruction(&Instruction::I32Load(ma));
                            f.instruction(&Instruction::Call(FN_FD_CLOSE));
                            f.instruction(&Instruction::Drop);

                            // str_ptr = buf_start + 4
                            f.instruction(&Instruction::I32Const((FS_SCRATCH + 24) as i32));
                            f.instruction(&Instruction::I32Const((FS_SCRATCH + 20) as i32));
                            f.instruction(&Instruction::I32Load(ma));
                            f.instruction(&Instruction::I32Const(4));
                            f.instruction(&Instruction::I32Add);
                            f.instruction(&Instruction::I32Store(ma));

                            // Build Ok enum: [tag=0][payload=str_ptr]
                            f.instruction(&Instruction::GlobalGet(0));
                            f.instruction(&Instruction::GlobalGet(0));
                            f.instruction(&Instruction::I32Const(0));
                            f.instruction(&Instruction::I32Store(ma));
                            f.instruction(&Instruction::GlobalGet(0));
                            f.instruction(&Instruction::I32Const(4));
                            f.instruction(&Instruction::I32Add);
                            f.instruction(&Instruction::I32Const((FS_SCRATCH + 24) as i32));
                            f.instruction(&Instruction::I32Load(ma));
                            f.instruction(&Instruction::I32Store(ma));
                            f.instruction(&Instruction::GlobalGet(0));
                            f.instruction(&Instruction::I32Const(8));
                            f.instruction(&Instruction::I32Add);
                            f.instruction(&Instruction::GlobalSet(0));
                        }
                        f.instruction(&Instruction::End); // end if/else
                    }
                    "fs_write_file" => {
                        // fs_write_file(path: String, content: String) -> Result<(), String>
                        let ma = MemArg {
                            offset: 0,
                            align: 2,
                            memory_index: 0,
                        };

                        // Save path_ptr to FS_SCRATCH+8
                        f.instruction(&Instruction::I32Const((FS_SCRATCH + 8) as i32));
                        self.emit_operand(f, &args[0]);
                        f.instruction(&Instruction::I32Store(ma));

                        // Save content_ptr to FS_SCRATCH+28
                        f.instruction(&Instruction::I32Const((FS_SCRATCH + 28) as i32));
                        self.emit_operand(f, &args[1]);
                        f.instruction(&Instruction::I32Store(ma));

                        // Get path_len
                        f.instruction(&Instruction::I32Const((FS_SCRATCH + 12) as i32));
                        f.instruction(&Instruction::I32Const((FS_SCRATCH + 8) as i32));
                        f.instruction(&Instruction::I32Load(ma));
                        f.instruction(&Instruction::I32Const(4));
                        f.instruction(&Instruction::I32Sub);
                        f.instruction(&Instruction::I32Load(ma));
                        f.instruction(&Instruction::I32Store(ma));

                        // Call path_open(dirfd=3, dirflags=0, path, path_len,
                        //   oflags=O_CREAT|O_TRUNC=9, rights=FD_WRITE(64), inheriting=0, fdflags=0, &opened_fd)
                        f.instruction(&Instruction::I32Const(3));
                        f.instruction(&Instruction::I32Const(0));
                        f.instruction(&Instruction::I32Const((FS_SCRATCH + 8) as i32));
                        f.instruction(&Instruction::I32Load(ma));
                        f.instruction(&Instruction::I32Const((FS_SCRATCH + 12) as i32));
                        f.instruction(&Instruction::I32Load(ma));
                        f.instruction(&Instruction::I32Const(9)); // O_CREAT(1) | O_TRUNC(8)
                        f.instruction(&Instruction::I64Const(64)); // FD_WRITE
                        f.instruction(&Instruction::I64Const(0));
                        f.instruction(&Instruction::I32Const(0));
                        f.instruction(&Instruction::I32Const(FS_SCRATCH as i32));
                        f.instruction(&Instruction::Call(FN_PATH_OPEN));

                        // Check errno
                        f.instruction(&Instruction::I32Const(0));
                        f.instruction(&Instruction::I32Ne);
                        f.instruction(&Instruction::If(wasm_encoder::BlockType::Result(
                            ValType::I32,
                        )));
                        {
                            // Error → Err("file write error")
                            let err_msg = b"file write error";
                            let err_len = err_msg.len() as i32;
                            f.instruction(&Instruction::GlobalGet(0));
                            f.instruction(&Instruction::I32Const(err_len));
                            f.instruction(&Instruction::I32Store(ma));
                            for (i, &byte) in err_msg.iter().enumerate() {
                                f.instruction(&Instruction::GlobalGet(0));
                                f.instruction(&Instruction::I32Const(4 + i as i32));
                                f.instruction(&Instruction::I32Add);
                                f.instruction(&Instruction::I32Const(byte as i32));
                                f.instruction(&Instruction::I32Store8(MemArg {
                                    offset: 0,
                                    align: 0,
                                    memory_index: 0,
                                }));
                            }
                            f.instruction(&Instruction::I32Const((FS_SCRATCH + 16) as i32));
                            f.instruction(&Instruction::GlobalGet(0));
                            f.instruction(&Instruction::I32Const(4));
                            f.instruction(&Instruction::I32Add);
                            f.instruction(&Instruction::I32Store(ma));
                            f.instruction(&Instruction::GlobalGet(0));
                            f.instruction(&Instruction::I32Const(4 + err_len));
                            f.instruction(&Instruction::I32Add);
                            f.instruction(&Instruction::GlobalSet(0));
                            // Err enum
                            f.instruction(&Instruction::GlobalGet(0));
                            f.instruction(&Instruction::GlobalGet(0));
                            f.instruction(&Instruction::I32Const(1));
                            f.instruction(&Instruction::I32Store(ma));
                            f.instruction(&Instruction::GlobalGet(0));
                            f.instruction(&Instruction::I32Const(4));
                            f.instruction(&Instruction::I32Add);
                            f.instruction(&Instruction::I32Const((FS_SCRATCH + 16) as i32));
                            f.instruction(&Instruction::I32Load(ma));
                            f.instruction(&Instruction::I32Store(ma));
                            f.instruction(&Instruction::GlobalGet(0));
                            f.instruction(&Instruction::I32Const(8));
                            f.instruction(&Instruction::I32Add);
                            f.instruction(&Instruction::GlobalSet(0));
                        }
                        f.instruction(&Instruction::Else);
                        {
                            // Write content using fd_write
                            f.instruction(&Instruction::I32Const(IOV_BASE as i32));
                            f.instruction(&Instruction::I32Const((FS_SCRATCH + 28) as i32));
                            f.instruction(&Instruction::I32Load(ma));
                            f.instruction(&Instruction::I32Store(ma));
                            f.instruction(&Instruction::I32Const((IOV_BASE + 4) as i32));
                            // content_len = mem[content_ptr - 4]
                            f.instruction(&Instruction::I32Const((FS_SCRATCH + 28) as i32));
                            f.instruction(&Instruction::I32Load(ma));
                            f.instruction(&Instruction::I32Const(4));
                            f.instruction(&Instruction::I32Sub);
                            f.instruction(&Instruction::I32Load(ma));
                            f.instruction(&Instruction::I32Store(ma));
                            // fd_write(fd, &iov, 1, &nwritten)
                            f.instruction(&Instruction::I32Const(FS_SCRATCH as i32));
                            f.instruction(&Instruction::I32Load(ma));
                            f.instruction(&Instruction::I32Const(IOV_BASE as i32));
                            f.instruction(&Instruction::I32Const(1));
                            f.instruction(&Instruction::I32Const(NWRITTEN as i32));
                            f.instruction(&Instruction::Call(FN_FD_WRITE));
                            f.instruction(&Instruction::Drop);
                            // fd_close
                            f.instruction(&Instruction::I32Const(FS_SCRATCH as i32));
                            f.instruction(&Instruction::I32Load(ma));
                            f.instruction(&Instruction::Call(FN_FD_CLOSE));
                            f.instruction(&Instruction::Drop);
                            // Build Ok(()) enum: [tag=0][payload=0]
                            f.instruction(&Instruction::GlobalGet(0));
                            f.instruction(&Instruction::GlobalGet(0));
                            f.instruction(&Instruction::I32Const(0));
                            f.instruction(&Instruction::I32Store(ma));
                            f.instruction(&Instruction::GlobalGet(0));
                            f.instruction(&Instruction::I32Const(4));
                            f.instruction(&Instruction::I32Add);
                            f.instruction(&Instruction::I32Const(0));
                            f.instruction(&Instruction::I32Store(ma));
                            f.instruction(&Instruction::GlobalGet(0));
                            f.instruction(&Instruction::I32Const(8));
                            f.instruction(&Instruction::I32Add);
                            f.instruction(&Instruction::GlobalSet(0));
                        }
                        f.instruction(&Instruction::End);
                    }
                    "map_i32_i32" => {
                        // map_i32_i32(vec, fn) -> call __map_i32 helper
                        for a in args {
                            self.emit_operand(f, a);
                        }
                        f.instruction(&Instruction::Call(FN_MAP_I32));
                    }
                    "filter_i32" => {
                        for a in args {
                            self.emit_operand(f, a);
                        }
                        f.instruction(&Instruction::Call(FN_FILTER_I32));
                    }
                    "fold_i32_i32" => {
                        for a in args {
                            self.emit_operand(f, a);
                        }
                        f.instruction(&Instruction::Call(FN_FOLD_I32));
                    }
                    "map_option_i32_i32" => {
                        for a in args {
                            self.emit_operand(f, a);
                        }
                        f.instruction(&Instruction::Call(FN_MAP_OPT_I32));
                    }
                    "any_i32" => {
                        for a in args {
                            self.emit_operand(f, a);
                        }
                        f.instruction(&Instruction::Call(FN_ANY_I32));
                    }
                    "find_i32" => {
                        for a in args {
                            self.emit_operand(f, a);
                        }
                        f.instruction(&Instruction::Call(FN_FIND_I32));
                    }
                    "Box_new" => {
                        // Box_new(value): allocate sizeof(enum) on heap, copy value, return pointer
                        // For enum payloads, the value is already a pointer to tag+payloads
                        // Box just stores that pointer
                        let ma = MemArg {
                            offset: 0,
                            align: 2,
                            memory_index: 0,
                        };
                        // Store the value (an enum pointer) at heap_ptr
                        f.instruction(&Instruction::GlobalGet(0)); // heap_ptr
                        if let Some(a) = args.first() {
                            self.emit_operand(f, a);
                        }
                        f.instruction(&Instruction::I32Store(ma));
                        // Return current heap_ptr, then bump by 4
                        f.instruction(&Instruction::GlobalGet(0));
                        f.instruction(&Instruction::GlobalGet(0));
                        f.instruction(&Instruction::I32Const(4));
                        f.instruction(&Instruction::I32Add);
                        f.instruction(&Instruction::GlobalSet(0));
                    }
                    "unbox" => {
                        // unbox(box_ptr): load the stored value (enum pointer) from box
                        let ma = MemArg {
                            offset: 0,
                            align: 2,
                            memory_index: 0,
                        };
                        if let Some(a) = args.first() {
                            self.emit_operand(f, a);
                        }
                        f.instruction(&Instruction::I32Load(ma));
                    }
                    "unwrap" => {
                        // unwrap(opt): Option/Result → payload at offset 4
                        // Assumes tag 0 = Some/Ok (has payload)
                        let ma = MemArg {
                            offset: 0,
                            align: 2,
                            memory_index: 0,
                        };
                        if let Some(a) = args.first() {
                            self.emit_operand(f, a);
                        }
                        f.instruction(&Instruction::I32Const(4));
                        f.instruction(&Instruction::I32Add);
                        f.instruction(&Instruction::I32Load(ma));
                    }
                    "unwrap_or" => {
                        // unwrap_or(opt, default): if Some/Ok return payload, else default
                        let ma = MemArg {
                            offset: 0,
                            align: 2,
                            memory_index: 0,
                        };
                        // Store opt ptr to scratch
                        f.instruction(&Instruction::I32Const(SCRATCH as i32 + 8));
                        if let Some(a) = args.first() {
                            self.emit_operand(f, a);
                        }
                        f.instruction(&Instruction::I32Store(ma));
                        // Check tag
                        f.instruction(&Instruction::I32Const(SCRATCH as i32 + 8));
                        f.instruction(&Instruction::I32Load(ma));
                        f.instruction(&Instruction::I32Load(ma)); // load tag
                        f.instruction(&Instruction::I32Eqz); // tag == 0 means Some/Ok
                        f.instruction(&Instruction::If(wasm_encoder::BlockType::Result(
                            ValType::I32,
                        )));
                        // Some: load payload
                        f.instruction(&Instruction::I32Const(SCRATCH as i32 + 8));
                        f.instruction(&Instruction::I32Load(ma));
                        f.instruction(&Instruction::I32Const(4));
                        f.instruction(&Instruction::I32Add);
                        f.instruction(&Instruction::I32Load(ma));
                        f.instruction(&Instruction::Else);
                        // None: return default
                        if let Some(a) = args.get(1) {
                            self.emit_operand(f, a);
                        }
                        f.instruction(&Instruction::End);
                    }
                    "is_some" => {
                        // is_some(opt): tag == 0 (Some)
                        let ma = MemArg {
                            offset: 0,
                            align: 2,
                            memory_index: 0,
                        };
                        if let Some(a) = args.first() {
                            self.emit_operand(f, a);
                        }
                        f.instruction(&Instruction::I32Load(ma)); // load tag
                        f.instruction(&Instruction::I32Eqz); // tag == 0 → true
                    }
                    "is_none" => {
                        // is_none(opt): tag != 0 (None)
                        let ma = MemArg {
                            offset: 0,
                            align: 2,
                            memory_index: 0,
                        };
                        if let Some(a) = args.first() {
                            self.emit_operand(f, a);
                        }
                        f.instruction(&Instruction::I32Load(ma)); // load tag
                        // tag != 0 → tag itself is truthy (None=1 → true)
                    }
                    "sqrt" => {
                        // sqrt(x: f64) -> f64: Wasm native f64.sqrt
                        if let Some(a) = args.first() {
                            self.emit_operand(f, a);
                        }
                        f.instruction(&Instruction::F64Sqrt);
                    }
                    "abs" => {
                        // abs(x: i32) -> i32: if x < 0 then -x else x
                        let ma = MemArg {
                            offset: 0,
                            align: 2,
                            memory_index: 0,
                        };
                        // Store x to SCRATCH+8
                        f.instruction(&Instruction::I32Const((SCRATCH + 8) as i32));
                        if let Some(a) = args.first() {
                            self.emit_operand(f, a);
                        }
                        f.instruction(&Instruction::I32Store(ma));
                        // if x < 0 then 0 - x else x
                        f.instruction(&Instruction::I32Const((SCRATCH + 8) as i32));
                        f.instruction(&Instruction::I32Load(ma));
                        f.instruction(&Instruction::I32Const(0));
                        f.instruction(&Instruction::I32LtS);
                        f.instruction(&Instruction::If(wasm_encoder::BlockType::Result(
                            ValType::I32,
                        )));
                        f.instruction(&Instruction::I32Const(0));
                        f.instruction(&Instruction::I32Const((SCRATCH + 8) as i32));
                        f.instruction(&Instruction::I32Load(ma));
                        f.instruction(&Instruction::I32Sub);
                        f.instruction(&Instruction::Else);
                        f.instruction(&Instruction::I32Const((SCRATCH + 8) as i32));
                        f.instruction(&Instruction::I32Load(ma));
                        f.instruction(&Instruction::End);
                    }
                    "min" => {
                        // min(a, b) -> i32: if a <= b then a else b
                        let ma = MemArg {
                            offset: 0,
                            align: 2,
                            memory_index: 0,
                        };
                        f.instruction(&Instruction::I32Const((SCRATCH + 8) as i32));
                        if let Some(a) = args.first() {
                            self.emit_operand(f, a);
                        }
                        f.instruction(&Instruction::I32Store(ma));
                        f.instruction(&Instruction::I32Const((SCRATCH + 12) as i32));
                        if let Some(a) = args.get(1) {
                            self.emit_operand(f, a);
                        }
                        f.instruction(&Instruction::I32Store(ma));
                        f.instruction(&Instruction::I32Const((SCRATCH + 8) as i32));
                        f.instruction(&Instruction::I32Load(ma));
                        f.instruction(&Instruction::I32Const((SCRATCH + 12) as i32));
                        f.instruction(&Instruction::I32Load(ma));
                        f.instruction(&Instruction::I32LeS);
                        f.instruction(&Instruction::If(wasm_encoder::BlockType::Result(
                            ValType::I32,
                        )));
                        f.instruction(&Instruction::I32Const((SCRATCH + 8) as i32));
                        f.instruction(&Instruction::I32Load(ma));
                        f.instruction(&Instruction::Else);
                        f.instruction(&Instruction::I32Const((SCRATCH + 12) as i32));
                        f.instruction(&Instruction::I32Load(ma));
                        f.instruction(&Instruction::End);
                    }
                    "max" => {
                        // max(a, b) -> i32: if a >= b then a else b
                        let ma = MemArg {
                            offset: 0,
                            align: 2,
                            memory_index: 0,
                        };
                        f.instruction(&Instruction::I32Const((SCRATCH + 8) as i32));
                        if let Some(a) = args.first() {
                            self.emit_operand(f, a);
                        }
                        f.instruction(&Instruction::I32Store(ma));
                        f.instruction(&Instruction::I32Const((SCRATCH + 12) as i32));
                        if let Some(a) = args.get(1) {
                            self.emit_operand(f, a);
                        }
                        f.instruction(&Instruction::I32Store(ma));
                        f.instruction(&Instruction::I32Const((SCRATCH + 8) as i32));
                        f.instruction(&Instruction::I32Load(ma));
                        f.instruction(&Instruction::I32Const((SCRATCH + 12) as i32));
                        f.instruction(&Instruction::I32Load(ma));
                        f.instruction(&Instruction::I32GeS);
                        f.instruction(&Instruction::If(wasm_encoder::BlockType::Result(
                            ValType::I32,
                        )));
                        f.instruction(&Instruction::I32Const((SCRATCH + 8) as i32));
                        f.instruction(&Instruction::I32Load(ma));
                        f.instruction(&Instruction::Else);
                        f.instruction(&Instruction::I32Const((SCRATCH + 12) as i32));
                        f.instruction(&Instruction::I32Load(ma));
                        f.instruction(&Instruction::End);
                    }
                    "panic" => {
                        // panic(msg: String) -> !: print to stderr, then unreachable
                        let ma = MemArg {
                            offset: 0,
                            align: 2,
                            memory_index: 0,
                        };
                        // Print "panic: " prefix to stderr
                        let prefix_ptr = self.alloc_length_prefixed_string("panic: ");
                        // Write prefix to stderr (fd=2)
                        f.instruction(&Instruction::I32Const(IOV_BASE as i32));
                        f.instruction(&Instruction::I32Const(prefix_ptr as i32));
                        f.instruction(&Instruction::I32Store(ma));
                        f.instruction(&Instruction::I32Const(IOV_BASE as i32 + 4));
                        f.instruction(&Instruction::I32Const(7)); // "panic: " len
                        f.instruction(&Instruction::I32Store(ma));
                        f.instruction(&Instruction::I32Const(2)); // fd=stderr
                        f.instruction(&Instruction::I32Const(IOV_BASE as i32));
                        f.instruction(&Instruction::I32Const(1));
                        f.instruction(&Instruction::I32Const(NWRITTEN as i32));
                        f.instruction(&Instruction::Call(FN_FD_WRITE));
                        f.instruction(&Instruction::Drop);
                        // Write user message to stderr
                        // Store msg ptr to SCRATCH+8
                        f.instruction(&Instruction::I32Const((SCRATCH + 8) as i32));
                        if let Some(a) = args.first() {
                            self.emit_operand(f, a);
                        }
                        f.instruction(&Instruction::I32Store(ma));
                        // iov_base = msg_ptr, iov_len = msg_len
                        f.instruction(&Instruction::I32Const(IOV_BASE as i32));
                        f.instruction(&Instruction::I32Const((SCRATCH + 8) as i32));
                        f.instruction(&Instruction::I32Load(ma)); // msg_ptr
                        f.instruction(&Instruction::I32Store(ma));
                        f.instruction(&Instruction::I32Const(IOV_BASE as i32 + 4));
                        f.instruction(&Instruction::I32Const((SCRATCH + 8) as i32));
                        f.instruction(&Instruction::I32Load(ma)); // msg_ptr
                        f.instruction(&Instruction::I32Const(4));
                        f.instruction(&Instruction::I32Sub);
                        f.instruction(&Instruction::I32Load(ma)); // msg_len
                        f.instruction(&Instruction::I32Store(ma));
                        f.instruction(&Instruction::I32Const(2)); // fd=stderr
                        f.instruction(&Instruction::I32Const(IOV_BASE as i32));
                        f.instruction(&Instruction::I32Const(1));
                        f.instruction(&Instruction::I32Const(NWRITTEN as i32));
                        f.instruction(&Instruction::Call(FN_FD_WRITE));
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
                        f.instruction(&Instruction::Call(FN_FD_WRITE));
                        f.instruction(&Instruction::Drop);
                        f.instruction(&Instruction::Unreachable);
                    }
                    "assert" => {
                        // assert(cond: bool): if !cond, panic
                        self.emit_operand(f, &args[0]);
                        f.instruction(&Instruction::I32Eqz);
                        f.instruction(&Instruction::If(wasm_encoder::BlockType::Empty));
                        self.emit_static_panic(f, "assertion failed");
                        f.instruction(&Instruction::End);
                    }
                    "assert_eq" => {
                        // assert_eq(a: i32, b: i32): if a != b, panic
                        self.emit_operand(f, &args[0]);
                        self.emit_operand(f, &args[1]);
                        f.instruction(&Instruction::I32Ne);
                        f.instruction(&Instruction::If(wasm_encoder::BlockType::Empty));
                        self.emit_static_panic(f, "assertion failed: assert_eq");
                        f.instruction(&Instruction::End);
                    }
                    "assert_ne" => {
                        // assert_ne(a: i32, b: i32): if a == b, panic
                        self.emit_operand(f, &args[0]);
                        self.emit_operand(f, &args[1]);
                        f.instruction(&Instruction::I32Eq);
                        f.instruction(&Instruction::If(wasm_encoder::BlockType::Empty));
                        self.emit_static_panic(f, "assertion failed: assert_ne");
                        f.instruction(&Instruction::End);
                    }
                    "assert_eq_str" => {
                        // assert_eq_str(a: String, b: String): if !str_eq(a, b), panic
                        self.emit_operand(f, &args[0]);
                        self.emit_operand(f, &args[1]);
                        f.instruction(&Instruction::Call(FN_STR_EQ));
                        f.instruction(&Instruction::I32Eqz);
                        f.instruction(&Instruction::If(wasm_encoder::BlockType::Empty));
                        self.emit_static_panic(f, "assertion failed: assert_eq_str");
                        f.instruction(&Instruction::End);
                    }
                    "assert_eq_i64" => {
                        // assert_eq_i64(a: i64, b: i64): if a != b, panic
                        self.emit_operand(f, &args[0]);
                        self.emit_operand(f, &args[1]);
                        f.instruction(&Instruction::I64Ne);
                        f.instruction(&Instruction::If(wasm_encoder::BlockType::Empty));
                        self.emit_static_panic(f, "assertion failed: assert_eq_i64");
                        f.instruction(&Instruction::End);
                    }
                    "clone" => {
                        // clone(s: String) -> String: deep copy a length-prefixed string
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
                        // Save src ptr to SCRATCH+8
                        f.instruction(&Instruction::I32Const((SCRATCH + 8) as i32));
                        if let Some(a) = args.first() {
                            self.emit_operand(f, a);
                        }
                        f.instruction(&Instruction::I32Store(ma));
                        // Load src len
                        f.instruction(&Instruction::I32Const((SCRATCH + 12) as i32));
                        f.instruction(&Instruction::I32Const((SCRATCH + 8) as i32));
                        f.instruction(&Instruction::I32Load(ma));
                        f.instruction(&Instruction::I32Const(4));
                        f.instruction(&Instruction::I32Sub);
                        f.instruction(&Instruction::I32Load(ma)); // len
                        f.instruction(&Instruction::I32Store(ma));
                        // Write len at heap_ptr
                        f.instruction(&Instruction::GlobalGet(0));
                        f.instruction(&Instruction::I32Const((SCRATCH + 12) as i32));
                        f.instruction(&Instruction::I32Load(ma));
                        f.instruction(&Instruction::I32Store(ma));
                        // Copy bytes: heap_ptr+4+i = src+i for i in 0..len
                        f.instruction(&Instruction::I32Const(NWRITTEN as i32));
                        f.instruction(&Instruction::I32Const(0));
                        f.instruction(&Instruction::I32Store(ma)); // i = 0
                        f.instruction(&Instruction::Block(wasm_encoder::BlockType::Empty));
                        f.instruction(&Instruction::Loop(wasm_encoder::BlockType::Empty));
                        f.instruction(&Instruction::I32Const(NWRITTEN as i32));
                        f.instruction(&Instruction::I32Load(ma)); // i
                        f.instruction(&Instruction::I32Const((SCRATCH + 12) as i32));
                        f.instruction(&Instruction::I32Load(ma)); // len
                        f.instruction(&Instruction::I32GeU);
                        f.instruction(&Instruction::BrIf(1));
                        // dst[i] = src[i]
                        f.instruction(&Instruction::GlobalGet(0));
                        f.instruction(&Instruction::I32Const(4));
                        f.instruction(&Instruction::I32Add);
                        f.instruction(&Instruction::I32Const(NWRITTEN as i32));
                        f.instruction(&Instruction::I32Load(ma));
                        f.instruction(&Instruction::I32Add); // dst = heap+4+i
                        f.instruction(&Instruction::I32Const((SCRATCH + 8) as i32));
                        f.instruction(&Instruction::I32Load(ma));
                        f.instruction(&Instruction::I32Const(NWRITTEN as i32));
                        f.instruction(&Instruction::I32Load(ma));
                        f.instruction(&Instruction::I32Add); // src = src_ptr+i
                        f.instruction(&Instruction::I32Load8U(ma0));
                        f.instruction(&Instruction::I32Store8(ma0));
                        // i++
                        f.instruction(&Instruction::I32Const(NWRITTEN as i32));
                        f.instruction(&Instruction::I32Const(NWRITTEN as i32));
                        f.instruction(&Instruction::I32Load(ma));
                        f.instruction(&Instruction::I32Const(1));
                        f.instruction(&Instruction::I32Add);
                        f.instruction(&Instruction::I32Store(ma));
                        f.instruction(&Instruction::Br(0));
                        f.instruction(&Instruction::End);
                        f.instruction(&Instruction::End);
                        // Result = heap_ptr + 4
                        f.instruction(&Instruction::GlobalGet(0));
                        f.instruction(&Instruction::I32Const(4));
                        f.instruction(&Instruction::I32Add);
                        // Bump heap
                        f.instruction(&Instruction::GlobalGet(0));
                        f.instruction(&Instruction::I32Const(4));
                        f.instruction(&Instruction::I32Add);
                        f.instruction(&Instruction::I32Const((SCRATCH + 12) as i32));
                        f.instruction(&Instruction::I32Load(ma));
                        f.instruction(&Instruction::I32Add);
                        f.instruction(&Instruction::GlobalSet(0));
                    }
                    "push_char" => {
                        // push_char(s: String, c: char) -> (): append byte to string
                        // String layout: [len:4][data:N] — s points to data (len is at s-4)
                        // We need to copy the string to heap with the char appended
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
                        // Store s ptr and char value
                        f.instruction(&Instruction::I32Const((SCRATCH + 8) as i32));
                        if let Some(a) = args.first() {
                            self.emit_operand(f, a);
                        }
                        f.instruction(&Instruction::I32Store(ma));
                        f.instruction(&Instruction::I32Const((SCRATCH + 12) as i32));
                        if let Some(a) = args.get(1) {
                            self.emit_operand(f, a);
                        }
                        f.instruction(&Instruction::I32Store(ma));
                        // Load old len
                        f.instruction(&Instruction::I32Const((SCRATCH + 16) as i32));
                        f.instruction(&Instruction::I32Const((SCRATCH + 8) as i32));
                        f.instruction(&Instruction::I32Load(ma));
                        f.instruction(&Instruction::I32Const(4));
                        f.instruction(&Instruction::I32Sub);
                        f.instruction(&Instruction::I32Load(ma)); // old_len
                        f.instruction(&Instruction::I32Store(ma));
                        // Write new_len = old_len + 1 at heap_ptr
                        f.instruction(&Instruction::GlobalGet(0));
                        f.instruction(&Instruction::I32Const((SCRATCH + 16) as i32));
                        f.instruction(&Instruction::I32Load(ma));
                        f.instruction(&Instruction::I32Const(1));
                        f.instruction(&Instruction::I32Add);
                        f.instruction(&Instruction::I32Store(ma));
                        // Copy old data bytes
                        f.instruction(&Instruction::I32Const(NWRITTEN as i32));
                        f.instruction(&Instruction::I32Const(0));
                        f.instruction(&Instruction::I32Store(ma)); // i = 0
                        f.instruction(&Instruction::Block(wasm_encoder::BlockType::Empty));
                        f.instruction(&Instruction::Loop(wasm_encoder::BlockType::Empty));
                        f.instruction(&Instruction::I32Const(NWRITTEN as i32));
                        f.instruction(&Instruction::I32Load(ma));
                        f.instruction(&Instruction::I32Const((SCRATCH + 16) as i32));
                        f.instruction(&Instruction::I32Load(ma));
                        f.instruction(&Instruction::I32GeU);
                        f.instruction(&Instruction::BrIf(1));
                        // heap[4+i] = src[i]
                        f.instruction(&Instruction::GlobalGet(0));
                        f.instruction(&Instruction::I32Const(4));
                        f.instruction(&Instruction::I32Add);
                        f.instruction(&Instruction::I32Const(NWRITTEN as i32));
                        f.instruction(&Instruction::I32Load(ma));
                        f.instruction(&Instruction::I32Add);
                        f.instruction(&Instruction::I32Const((SCRATCH + 8) as i32));
                        f.instruction(&Instruction::I32Load(ma));
                        f.instruction(&Instruction::I32Const(NWRITTEN as i32));
                        f.instruction(&Instruction::I32Load(ma));
                        f.instruction(&Instruction::I32Add);
                        f.instruction(&Instruction::I32Load8U(ma0));
                        f.instruction(&Instruction::I32Store8(ma0));
                        f.instruction(&Instruction::I32Const(NWRITTEN as i32));
                        f.instruction(&Instruction::I32Const(NWRITTEN as i32));
                        f.instruction(&Instruction::I32Load(ma));
                        f.instruction(&Instruction::I32Const(1));
                        f.instruction(&Instruction::I32Add);
                        f.instruction(&Instruction::I32Store(ma));
                        f.instruction(&Instruction::Br(0));
                        f.instruction(&Instruction::End);
                        f.instruction(&Instruction::End);
                        // Write char byte at heap[4+old_len]
                        f.instruction(&Instruction::GlobalGet(0));
                        f.instruction(&Instruction::I32Const(4));
                        f.instruction(&Instruction::I32Add);
                        f.instruction(&Instruction::I32Const((SCRATCH + 16) as i32));
                        f.instruction(&Instruction::I32Load(ma)); // old_len
                        f.instruction(&Instruction::I32Add);
                        f.instruction(&Instruction::I32Const((SCRATCH + 12) as i32));
                        f.instruction(&Instruction::I32Load(ma)); // char value
                        f.instruction(&Instruction::I32Store8(ma0));
                        // Result ptr = heap_ptr + 4
                        // (push_char is void, but we need to update the local var's pointer)
                        // For now push_char returns the new string ptr on stack
                        f.instruction(&Instruction::GlobalGet(0));
                        f.instruction(&Instruction::I32Const(4));
                        f.instruction(&Instruction::I32Add);
                        // Bump heap past [len:4][data:old_len+1]
                        f.instruction(&Instruction::GlobalGet(0));
                        f.instruction(&Instruction::I32Const(4));
                        f.instruction(&Instruction::I32Add);
                        f.instruction(&Instruction::I32Const((SCRATCH + 16) as i32));
                        f.instruction(&Instruction::I32Load(ma));
                        f.instruction(&Instruction::I32Add);
                        f.instruction(&Instruction::I32Const(1));
                        f.instruction(&Instruction::I32Add);
                        f.instruction(&Instruction::GlobalSet(0));
                    }
                    "to_lower" => {
                        // to_lower(s: String) -> String: ASCII lowercase
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
                        // Save src ptr
                        f.instruction(&Instruction::I32Const((SCRATCH + 8) as i32));
                        if let Some(a) = args.first() {
                            self.emit_operand(f, a);
                        }
                        f.instruction(&Instruction::I32Store(ma));
                        // Load len
                        f.instruction(&Instruction::I32Const((SCRATCH + 12) as i32));
                        f.instruction(&Instruction::I32Const((SCRATCH + 8) as i32));
                        f.instruction(&Instruction::I32Load(ma));
                        f.instruction(&Instruction::I32Const(4));
                        f.instruction(&Instruction::I32Sub);
                        f.instruction(&Instruction::I32Load(ma));
                        f.instruction(&Instruction::I32Store(ma));
                        // Write len at heap
                        f.instruction(&Instruction::GlobalGet(0));
                        f.instruction(&Instruction::I32Const((SCRATCH + 12) as i32));
                        f.instruction(&Instruction::I32Load(ma));
                        f.instruction(&Instruction::I32Store(ma));
                        // Copy with lowercase: for each byte, if 'A'<=b<='Z' then b+32
                        f.instruction(&Instruction::I32Const(NWRITTEN as i32));
                        f.instruction(&Instruction::I32Const(0));
                        f.instruction(&Instruction::I32Store(ma));
                        f.instruction(&Instruction::Block(wasm_encoder::BlockType::Empty));
                        f.instruction(&Instruction::Loop(wasm_encoder::BlockType::Empty));
                        f.instruction(&Instruction::I32Const(NWRITTEN as i32));
                        f.instruction(&Instruction::I32Load(ma));
                        f.instruction(&Instruction::I32Const((SCRATCH + 12) as i32));
                        f.instruction(&Instruction::I32Load(ma));
                        f.instruction(&Instruction::I32GeU);
                        f.instruction(&Instruction::BrIf(1));
                        // Load byte
                        f.instruction(&Instruction::I32Const((SCRATCH + 16) as i32));
                        f.instruction(&Instruction::I32Const((SCRATCH + 8) as i32));
                        f.instruction(&Instruction::I32Load(ma));
                        f.instruction(&Instruction::I32Const(NWRITTEN as i32));
                        f.instruction(&Instruction::I32Load(ma));
                        f.instruction(&Instruction::I32Add);
                        f.instruction(&Instruction::I32Load8U(ma0));
                        f.instruction(&Instruction::I32Store(ma));
                        // If 'A' (65) <= byte <= 'Z' (90), add 32
                        f.instruction(&Instruction::I32Const((SCRATCH + 16) as i32));
                        f.instruction(&Instruction::I32Load(ma));
                        f.instruction(&Instruction::I32Const(65));
                        f.instruction(&Instruction::I32GeU);
                        f.instruction(&Instruction::I32Const((SCRATCH + 16) as i32));
                        f.instruction(&Instruction::I32Load(ma));
                        f.instruction(&Instruction::I32Const(90));
                        f.instruction(&Instruction::I32LeU);
                        f.instruction(&Instruction::I32And);
                        f.instruction(&Instruction::If(wasm_encoder::BlockType::Empty));
                        f.instruction(&Instruction::I32Const((SCRATCH + 16) as i32));
                        f.instruction(&Instruction::I32Const((SCRATCH + 16) as i32));
                        f.instruction(&Instruction::I32Load(ma));
                        f.instruction(&Instruction::I32Const(32));
                        f.instruction(&Instruction::I32Add);
                        f.instruction(&Instruction::I32Store(ma));
                        f.instruction(&Instruction::End);
                        // Store byte
                        f.instruction(&Instruction::GlobalGet(0));
                        f.instruction(&Instruction::I32Const(4));
                        f.instruction(&Instruction::I32Add);
                        f.instruction(&Instruction::I32Const(NWRITTEN as i32));
                        f.instruction(&Instruction::I32Load(ma));
                        f.instruction(&Instruction::I32Add);
                        f.instruction(&Instruction::I32Const((SCRATCH + 16) as i32));
                        f.instruction(&Instruction::I32Load(ma));
                        f.instruction(&Instruction::I32Store8(ma0));
                        // i++
                        f.instruction(&Instruction::I32Const(NWRITTEN as i32));
                        f.instruction(&Instruction::I32Const(NWRITTEN as i32));
                        f.instruction(&Instruction::I32Load(ma));
                        f.instruction(&Instruction::I32Const(1));
                        f.instruction(&Instruction::I32Add);
                        f.instruction(&Instruction::I32Store(ma));
                        f.instruction(&Instruction::Br(0));
                        f.instruction(&Instruction::End);
                        f.instruction(&Instruction::End);
                        // Result = heap_ptr + 4
                        f.instruction(&Instruction::GlobalGet(0));
                        f.instruction(&Instruction::I32Const(4));
                        f.instruction(&Instruction::I32Add);
                        // Bump heap
                        f.instruction(&Instruction::GlobalGet(0));
                        f.instruction(&Instruction::I32Const(4));
                        f.instruction(&Instruction::I32Add);
                        f.instruction(&Instruction::I32Const((SCRATCH + 12) as i32));
                        f.instruction(&Instruction::I32Load(ma));
                        f.instruction(&Instruction::I32Add);
                        f.instruction(&Instruction::GlobalSet(0));
                    }
                    "to_upper" => {
                        // to_upper(s: String) -> String: ASCII uppercase
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
                        f.instruction(&Instruction::I32Const((SCRATCH + 8) as i32));
                        if let Some(a) = args.first() {
                            self.emit_operand(f, a);
                        }
                        f.instruction(&Instruction::I32Store(ma));
                        f.instruction(&Instruction::I32Const((SCRATCH + 12) as i32));
                        f.instruction(&Instruction::I32Const((SCRATCH + 8) as i32));
                        f.instruction(&Instruction::I32Load(ma));
                        f.instruction(&Instruction::I32Const(4));
                        f.instruction(&Instruction::I32Sub);
                        f.instruction(&Instruction::I32Load(ma));
                        f.instruction(&Instruction::I32Store(ma));
                        f.instruction(&Instruction::GlobalGet(0));
                        f.instruction(&Instruction::I32Const((SCRATCH + 12) as i32));
                        f.instruction(&Instruction::I32Load(ma));
                        f.instruction(&Instruction::I32Store(ma));
                        f.instruction(&Instruction::I32Const(NWRITTEN as i32));
                        f.instruction(&Instruction::I32Const(0));
                        f.instruction(&Instruction::I32Store(ma));
                        f.instruction(&Instruction::Block(wasm_encoder::BlockType::Empty));
                        f.instruction(&Instruction::Loop(wasm_encoder::BlockType::Empty));
                        f.instruction(&Instruction::I32Const(NWRITTEN as i32));
                        f.instruction(&Instruction::I32Load(ma));
                        f.instruction(&Instruction::I32Const((SCRATCH + 12) as i32));
                        f.instruction(&Instruction::I32Load(ma));
                        f.instruction(&Instruction::I32GeU);
                        f.instruction(&Instruction::BrIf(1));
                        f.instruction(&Instruction::I32Const((SCRATCH + 16) as i32));
                        f.instruction(&Instruction::I32Const((SCRATCH + 8) as i32));
                        f.instruction(&Instruction::I32Load(ma));
                        f.instruction(&Instruction::I32Const(NWRITTEN as i32));
                        f.instruction(&Instruction::I32Load(ma));
                        f.instruction(&Instruction::I32Add);
                        f.instruction(&Instruction::I32Load8U(ma0));
                        f.instruction(&Instruction::I32Store(ma));
                        // If 'a' (97) <= byte <= 'z' (122), subtract 32
                        f.instruction(&Instruction::I32Const((SCRATCH + 16) as i32));
                        f.instruction(&Instruction::I32Load(ma));
                        f.instruction(&Instruction::I32Const(97));
                        f.instruction(&Instruction::I32GeU);
                        f.instruction(&Instruction::I32Const((SCRATCH + 16) as i32));
                        f.instruction(&Instruction::I32Load(ma));
                        f.instruction(&Instruction::I32Const(122));
                        f.instruction(&Instruction::I32LeU);
                        f.instruction(&Instruction::I32And);
                        f.instruction(&Instruction::If(wasm_encoder::BlockType::Empty));
                        f.instruction(&Instruction::I32Const((SCRATCH + 16) as i32));
                        f.instruction(&Instruction::I32Const((SCRATCH + 16) as i32));
                        f.instruction(&Instruction::I32Load(ma));
                        f.instruction(&Instruction::I32Const(32));
                        f.instruction(&Instruction::I32Sub);
                        f.instruction(&Instruction::I32Store(ma));
                        f.instruction(&Instruction::End);
                        f.instruction(&Instruction::GlobalGet(0));
                        f.instruction(&Instruction::I32Const(4));
                        f.instruction(&Instruction::I32Add);
                        f.instruction(&Instruction::I32Const(NWRITTEN as i32));
                        f.instruction(&Instruction::I32Load(ma));
                        f.instruction(&Instruction::I32Add);
                        f.instruction(&Instruction::I32Const((SCRATCH + 16) as i32));
                        f.instruction(&Instruction::I32Load(ma));
                        f.instruction(&Instruction::I32Store8(ma0));
                        f.instruction(&Instruction::I32Const(NWRITTEN as i32));
                        f.instruction(&Instruction::I32Const(NWRITTEN as i32));
                        f.instruction(&Instruction::I32Load(ma));
                        f.instruction(&Instruction::I32Const(1));
                        f.instruction(&Instruction::I32Add);
                        f.instruction(&Instruction::I32Store(ma));
                        f.instruction(&Instruction::Br(0));
                        f.instruction(&Instruction::End);
                        f.instruction(&Instruction::End);
                        f.instruction(&Instruction::GlobalGet(0));
                        f.instruction(&Instruction::I32Const(4));
                        f.instruction(&Instruction::I32Add);
                        f.instruction(&Instruction::GlobalGet(0));
                        f.instruction(&Instruction::I32Const(4));
                        f.instruction(&Instruction::I32Add);
                        f.instruction(&Instruction::I32Const((SCRATCH + 12) as i32));
                        f.instruction(&Instruction::I32Load(ma));
                        f.instruction(&Instruction::I32Add);
                        f.instruction(&Instruction::GlobalSet(0));
                    }
                    "sort_String" => {
                        // sort_String(v: Vec<String>): bubble sort comparing strings lexicographically
                        // Uses __str_eq helper and byte comparison for ordering
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
                        let s_i = SCRATCH;
                        let s_j = SCRATCH + 4;
                        let s_n = SCRATCH + 8;
                        let s_data = SCRATCH + 12;
                        let s_tmp = SCRATCH + 16;
                        // n = len(v)
                        f.instruction(&Instruction::I32Const(s_n as i32));
                        if let Some(v) = args.first() {
                            self.emit_operand(f, v);
                        }
                        f.instruction(&Instruction::I32Load(ma));
                        f.instruction(&Instruction::I32Store(ma));
                        // data_ptr
                        f.instruction(&Instruction::I32Const(s_data as i32));
                        if let Some(v) = args.first() {
                            self.emit_operand(f, v);
                        }
                        f.instruction(&Instruction::I32Const(8));
                        f.instruction(&Instruction::I32Add);
                        f.instruction(&Instruction::I32Load(ma));
                        f.instruction(&Instruction::I32Store(ma));
                        // i = 0
                        f.instruction(&Instruction::I32Const(s_i as i32));
                        f.instruction(&Instruction::I32Const(0));
                        f.instruction(&Instruction::I32Store(ma));
                        // Outer loop
                        f.instruction(&Instruction::Block(wasm_encoder::BlockType::Empty));
                        f.instruction(&Instruction::Loop(wasm_encoder::BlockType::Empty));
                        f.instruction(&Instruction::I32Const(s_i as i32));
                        f.instruction(&Instruction::I32Load(ma));
                        f.instruction(&Instruction::I32Const(s_n as i32));
                        f.instruction(&Instruction::I32Load(ma));
                        f.instruction(&Instruction::I32Const(1));
                        f.instruction(&Instruction::I32Sub);
                        f.instruction(&Instruction::I32GeU);
                        f.instruction(&Instruction::BrIf(1));
                        // j = 0
                        f.instruction(&Instruction::I32Const(s_j as i32));
                        f.instruction(&Instruction::I32Const(0));
                        f.instruction(&Instruction::I32Store(ma));
                        // Inner loop
                        f.instruction(&Instruction::Block(wasm_encoder::BlockType::Empty));
                        f.instruction(&Instruction::Loop(wasm_encoder::BlockType::Empty));
                        f.instruction(&Instruction::I32Const(s_j as i32));
                        f.instruction(&Instruction::I32Load(ma));
                        f.instruction(&Instruction::I32Const(s_n as i32));
                        f.instruction(&Instruction::I32Load(ma));
                        f.instruction(&Instruction::I32Const(s_i as i32));
                        f.instruction(&Instruction::I32Load(ma));
                        f.instruction(&Instruction::I32Sub);
                        f.instruction(&Instruction::I32Const(1));
                        f.instruction(&Instruction::I32Sub);
                        f.instruction(&Instruction::I32GeU);
                        f.instruction(&Instruction::BrIf(1));
                        // Compare strings: data[j] > data[j+1] lexicographically
                        // Load ptr_a = data[j], ptr_b = data[j+1]
                        // Use scratch area at SCRATCH+20..SCRATCH+40
                        let s_ptr_a = SCRATCH + 20;
                        let s_ptr_b = SCRATCH + 24;
                        let s_len_a = SCRATCH + 28;
                        let s_len_b = SCRATCH + 32;
                        let s_k = SCRATCH + 36;
                        let s_min_len = SCRATCH + 40;
                        let s_swap = SCRATCH + 44;
                        // ptr_a = *(data_ptr + j*4)
                        f.instruction(&Instruction::I32Const(s_ptr_a as i32));
                        f.instruction(&Instruction::I32Const(s_data as i32));
                        f.instruction(&Instruction::I32Load(ma));
                        f.instruction(&Instruction::I32Const(s_j as i32));
                        f.instruction(&Instruction::I32Load(ma));
                        f.instruction(&Instruction::I32Const(4));
                        f.instruction(&Instruction::I32Mul);
                        f.instruction(&Instruction::I32Add);
                        f.instruction(&Instruction::I32Load(ma));
                        f.instruction(&Instruction::I32Store(ma));
                        // ptr_b = *(data_ptr + (j+1)*4)
                        f.instruction(&Instruction::I32Const(s_ptr_b as i32));
                        f.instruction(&Instruction::I32Const(s_data as i32));
                        f.instruction(&Instruction::I32Load(ma));
                        f.instruction(&Instruction::I32Const(s_j as i32));
                        f.instruction(&Instruction::I32Load(ma));
                        f.instruction(&Instruction::I32Const(1));
                        f.instruction(&Instruction::I32Add);
                        f.instruction(&Instruction::I32Const(4));
                        f.instruction(&Instruction::I32Mul);
                        f.instruction(&Instruction::I32Add);
                        f.instruction(&Instruction::I32Load(ma));
                        f.instruction(&Instruction::I32Store(ma));
                        // len_a, len_b (from length-prefix at ptr-4)
                        f.instruction(&Instruction::I32Const(s_len_a as i32));
                        f.instruction(&Instruction::I32Const(s_ptr_a as i32));
                        f.instruction(&Instruction::I32Load(ma));
                        f.instruction(&Instruction::I32Const(4));
                        f.instruction(&Instruction::I32Sub);
                        f.instruction(&Instruction::I32Load(ma));
                        f.instruction(&Instruction::I32Store(ma));
                        f.instruction(&Instruction::I32Const(s_len_b as i32));
                        f.instruction(&Instruction::I32Const(s_ptr_b as i32));
                        f.instruction(&Instruction::I32Load(ma));
                        f.instruction(&Instruction::I32Const(4));
                        f.instruction(&Instruction::I32Sub);
                        f.instruction(&Instruction::I32Load(ma));
                        f.instruction(&Instruction::I32Store(ma));
                        // min_len = if len_a < len_b then len_a else len_b
                        f.instruction(&Instruction::I32Const(s_min_len as i32));
                        f.instruction(&Instruction::I32Const(s_len_a as i32));
                        f.instruction(&Instruction::I32Load(ma));
                        f.instruction(&Instruction::I32Const(s_len_b as i32));
                        f.instruction(&Instruction::I32Load(ma));
                        f.instruction(&Instruction::I32LtU);
                        f.instruction(&Instruction::If(wasm_encoder::BlockType::Result(
                            ValType::I32,
                        )));
                        f.instruction(&Instruction::I32Const(s_len_a as i32));
                        f.instruction(&Instruction::I32Load(ma));
                        f.instruction(&Instruction::Else);
                        f.instruction(&Instruction::I32Const(s_len_b as i32));
                        f.instruction(&Instruction::I32Load(ma));
                        f.instruction(&Instruction::End);
                        f.instruction(&Instruction::I32Store(ma));
                        // swap = 0; k = 0
                        f.instruction(&Instruction::I32Const(s_swap as i32));
                        f.instruction(&Instruction::I32Const(0));
                        f.instruction(&Instruction::I32Store(ma));
                        f.instruction(&Instruction::I32Const(s_k as i32));
                        f.instruction(&Instruction::I32Const(0));
                        f.instruction(&Instruction::I32Store(ma));
                        // Compare loop
                        f.instruction(&Instruction::Block(wasm_encoder::BlockType::Empty));
                        f.instruction(&Instruction::Loop(wasm_encoder::BlockType::Empty));
                        f.instruction(&Instruction::I32Const(s_k as i32));
                        f.instruction(&Instruction::I32Load(ma));
                        f.instruction(&Instruction::I32Const(s_min_len as i32));
                        f.instruction(&Instruction::I32Load(ma));
                        f.instruction(&Instruction::I32GeU);
                        f.instruction(&Instruction::BrIf(1));
                        // byte_a = ptr_a[k], byte_b = ptr_b[k]
                        f.instruction(&Instruction::I32Const(s_ptr_a as i32));
                        f.instruction(&Instruction::I32Load(ma));
                        f.instruction(&Instruction::I32Const(s_k as i32));
                        f.instruction(&Instruction::I32Load(ma));
                        f.instruction(&Instruction::I32Add);
                        f.instruction(&Instruction::I32Load8U(ma0));
                        f.instruction(&Instruction::I32Const(s_ptr_b as i32));
                        f.instruction(&Instruction::I32Load(ma));
                        f.instruction(&Instruction::I32Const(s_k as i32));
                        f.instruction(&Instruction::I32Load(ma));
                        f.instruction(&Instruction::I32Add);
                        f.instruction(&Instruction::I32Load8U(ma0));
                        f.instruction(&Instruction::I32GtU);
                        f.instruction(&Instruction::If(wasm_encoder::BlockType::Empty));
                        f.instruction(&Instruction::I32Const(s_swap as i32));
                        f.instruction(&Instruction::I32Const(1));
                        f.instruction(&Instruction::I32Store(ma));
                        f.instruction(&Instruction::Br(2)); // break compare loop
                        f.instruction(&Instruction::End);
                        // if byte_a < byte_b → no swap, break
                        f.instruction(&Instruction::I32Const(s_ptr_a as i32));
                        f.instruction(&Instruction::I32Load(ma));
                        f.instruction(&Instruction::I32Const(s_k as i32));
                        f.instruction(&Instruction::I32Load(ma));
                        f.instruction(&Instruction::I32Add);
                        f.instruction(&Instruction::I32Load8U(ma0));
                        f.instruction(&Instruction::I32Const(s_ptr_b as i32));
                        f.instruction(&Instruction::I32Load(ma));
                        f.instruction(&Instruction::I32Const(s_k as i32));
                        f.instruction(&Instruction::I32Load(ma));
                        f.instruction(&Instruction::I32Add);
                        f.instruction(&Instruction::I32Load8U(ma0));
                        f.instruction(&Instruction::I32LtU);
                        f.instruction(&Instruction::BrIf(1)); // break compare loop
                        // k++
                        f.instruction(&Instruction::I32Const(s_k as i32));
                        f.instruction(&Instruction::I32Const(s_k as i32));
                        f.instruction(&Instruction::I32Load(ma));
                        f.instruction(&Instruction::I32Const(1));
                        f.instruction(&Instruction::I32Add);
                        f.instruction(&Instruction::I32Store(ma));
                        f.instruction(&Instruction::Br(0));
                        f.instruction(&Instruction::End); // end compare loop
                        f.instruction(&Instruction::End); // end compare block
                        // If bytes were equal, compare lengths: swap if len_a > len_b
                        f.instruction(&Instruction::I32Const(s_swap as i32));
                        f.instruction(&Instruction::I32Load(ma));
                        f.instruction(&Instruction::I32Eqz);
                        f.instruction(&Instruction::If(wasm_encoder::BlockType::Empty));
                        f.instruction(&Instruction::I32Const(s_len_a as i32));
                        f.instruction(&Instruction::I32Load(ma));
                        f.instruction(&Instruction::I32Const(s_len_b as i32));
                        f.instruction(&Instruction::I32Load(ma));
                        f.instruction(&Instruction::I32GtU);
                        f.instruction(&Instruction::If(wasm_encoder::BlockType::Empty));
                        f.instruction(&Instruction::I32Const(s_swap as i32));
                        f.instruction(&Instruction::I32Const(1));
                        f.instruction(&Instruction::I32Store(ma));
                        f.instruction(&Instruction::End);
                        f.instruction(&Instruction::End);
                        // If swap needed, swap data[j] and data[j+1]
                        f.instruction(&Instruction::I32Const(s_swap as i32));
                        f.instruction(&Instruction::I32Load(ma));
                        f.instruction(&Instruction::If(wasm_encoder::BlockType::Empty));
                        // tmp = data[j]
                        f.instruction(&Instruction::I32Const(s_tmp as i32));
                        f.instruction(&Instruction::I32Const(s_data as i32));
                        f.instruction(&Instruction::I32Load(ma));
                        f.instruction(&Instruction::I32Const(s_j as i32));
                        f.instruction(&Instruction::I32Load(ma));
                        f.instruction(&Instruction::I32Const(4));
                        f.instruction(&Instruction::I32Mul);
                        f.instruction(&Instruction::I32Add);
                        f.instruction(&Instruction::I32Load(ma));
                        f.instruction(&Instruction::I32Store(ma));
                        // data[j] = data[j+1]
                        f.instruction(&Instruction::I32Const(s_data as i32));
                        f.instruction(&Instruction::I32Load(ma));
                        f.instruction(&Instruction::I32Const(s_j as i32));
                        f.instruction(&Instruction::I32Load(ma));
                        f.instruction(&Instruction::I32Const(4));
                        f.instruction(&Instruction::I32Mul);
                        f.instruction(&Instruction::I32Add);
                        f.instruction(&Instruction::I32Const(s_data as i32));
                        f.instruction(&Instruction::I32Load(ma));
                        f.instruction(&Instruction::I32Const(s_j as i32));
                        f.instruction(&Instruction::I32Load(ma));
                        f.instruction(&Instruction::I32Const(1));
                        f.instruction(&Instruction::I32Add);
                        f.instruction(&Instruction::I32Const(4));
                        f.instruction(&Instruction::I32Mul);
                        f.instruction(&Instruction::I32Add);
                        f.instruction(&Instruction::I32Load(ma));
                        f.instruction(&Instruction::I32Store(ma));
                        // data[j+1] = tmp
                        f.instruction(&Instruction::I32Const(s_data as i32));
                        f.instruction(&Instruction::I32Load(ma));
                        f.instruction(&Instruction::I32Const(s_j as i32));
                        f.instruction(&Instruction::I32Load(ma));
                        f.instruction(&Instruction::I32Const(1));
                        f.instruction(&Instruction::I32Add);
                        f.instruction(&Instruction::I32Const(4));
                        f.instruction(&Instruction::I32Mul);
                        f.instruction(&Instruction::I32Add);
                        f.instruction(&Instruction::I32Const(s_tmp as i32));
                        f.instruction(&Instruction::I32Load(ma));
                        f.instruction(&Instruction::I32Store(ma));
                        f.instruction(&Instruction::End); // end if swap
                        // j++
                        f.instruction(&Instruction::I32Const(s_j as i32));
                        f.instruction(&Instruction::I32Const(s_j as i32));
                        f.instruction(&Instruction::I32Load(ma));
                        f.instruction(&Instruction::I32Const(1));
                        f.instruction(&Instruction::I32Add);
                        f.instruction(&Instruction::I32Store(ma));
                        f.instruction(&Instruction::Br(0));
                        f.instruction(&Instruction::End); // end inner loop
                        f.instruction(&Instruction::End); // end inner block
                        // i++
                        f.instruction(&Instruction::I32Const(s_i as i32));
                        f.instruction(&Instruction::I32Const(s_i as i32));
                        f.instruction(&Instruction::I32Load(ma));
                        f.instruction(&Instruction::I32Const(1));
                        f.instruction(&Instruction::I32Add);
                        f.instruction(&Instruction::I32Store(ma));
                        f.instruction(&Instruction::Br(0));
                        f.instruction(&Instruction::End); // end outer loop
                        f.instruction(&Instruction::End); // end outer block
                    }
                    "f32_to_string" => {
                        // f32_to_string(x: f32) -> String: promote to f64 then use f64_to_string
                        if let Some(a) = args.first() {
                            self.emit_operand(f, a);
                        }
                        f.instruction(&Instruction::F64PromoteF32);
                        f.instruction(&Instruction::Call(FN_F64_TO_STR));
                    }
                    other => {
                        for a in args {
                            self.emit_operand(f, a);
                        }
                        if let Some(idx) = self.resolve_fn(other) {
                            f.instruction(&Instruction::Call(idx));
                        } else {
                            f.instruction(&Instruction::I32Const(0));
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
                ..
            } => {
                self.emit_operand(f, cond);
                f.instruction(&Instruction::If(wasm_encoder::BlockType::Result(
                    ValType::I32,
                )));
                // Track depth for break/continue
                if let Some(d) = self.loop_depths.last_mut() {
                    *d += 1;
                }
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
                    if matches!(r.as_ref(), Operand::Unit) {
                        // Dead branch in exhaustive match — unreachable satisfies any type
                        f.instruction(&Instruction::Unreachable);
                    } else {
                        self.emit_operand(f, r);
                    }
                } else {
                    f.instruction(&Instruction::I32Const(0));
                }
                f.instruction(&Instruction::End);
                if let Some(d) = self.loop_depths.last_mut() {
                    *d -= 1;
                }
            }
            Operand::StructInit { name, fields } => {
                let ma_i32 = MemArg {
                    offset: 0,
                    align: 2,
                    memory_index: 0,
                };
                let ma_f64 = MemArg {
                    offset: 0,
                    align: 3,
                    memory_index: 0,
                };
                let total_size = self.struct_total_size(name).max(fields.len() as u32 * 4);
                let layout_fields = self.struct_layouts.get(name).cloned().unwrap_or_default();
                let save_addr = STRUCT_BASE + self.struct_init_depth * 4;
                self.struct_init_depth += 1;
                // Save base = current heap_ptr to scratch
                f.instruction(&Instruction::I32Const(save_addr as i32));
                f.instruction(&Instruction::GlobalGet(0));
                f.instruction(&Instruction::I32Store(ma_i32));
                // Bump heap for this struct
                f.instruction(&Instruction::GlobalGet(0));
                f.instruction(&Instruction::I32Const(total_size as i32));
                f.instruction(&Instruction::I32Add);
                f.instruction(&Instruction::GlobalSet(0));
                // Store each field at saved_base + field_offset
                let mut offset = 0u32;
                for (i, (_fname, fval)) in fields.iter().enumerate() {
                    let (fsize, is_f64) = layout_fields
                        .get(i)
                        .map(|(_, ftype)| Self::field_type_info(ftype))
                        .unwrap_or((4, false));
                    // Load saved base
                    f.instruction(&Instruction::I32Const(save_addr as i32));
                    f.instruction(&Instruction::I32Load(ma_i32));
                    if offset > 0 {
                        f.instruction(&Instruction::I32Const(offset as i32));
                        f.instruction(&Instruction::I32Add);
                    }
                    self.emit_operand(f, fval);
                    if is_f64 {
                        f.instruction(&Instruction::F64Store(ma_f64));
                    } else {
                        f.instruction(&Instruction::I32Store(ma_i32));
                    }
                    offset += fsize;
                }
                self.struct_init_depth -= 1;
                // Push saved base as result
                f.instruction(&Instruction::I32Const(save_addr as i32));
                f.instruction(&Instruction::I32Load(ma_i32));
            }
            Operand::FieldAccess {
                object,
                struct_name,
                field,
            } => {
                let ma_i32 = MemArg {
                    offset: 0,
                    align: 2,
                    memory_index: 0,
                };
                let ma_f64 = MemArg {
                    offset: 0,
                    align: 3,
                    memory_index: 0,
                };
                let (field_offset, is_f64) = self.struct_field_info(struct_name, field);
                self.emit_operand(f, object);
                if field_offset > 0 {
                    f.instruction(&Instruction::I32Const(field_offset as i32));
                    f.instruction(&Instruction::I32Add);
                }
                if is_f64 {
                    f.instruction(&Instruction::F64Load(ma_f64));
                } else {
                    f.instruction(&Instruction::I32Load(ma_i32));
                }
            }
            Operand::EnumInit {
                enum_name,
                variant,
                tag,
                payload,
            } => {
                let ma_i32 = MemArg {
                    offset: 0,
                    align: 2,
                    memory_index: 0,
                };
                let ma_f64 = MemArg {
                    offset: 0,
                    align: 3,
                    memory_index: 0,
                };
                let total_size = self.enum_variant_total_size(enum_name, variant);
                let depth = self.enum_init_depth;
                self.enum_init_depth += 1;
                // Pre-allocate: bump heap pointer and save base to scratch
                // Save current heap_ptr as base for this enum
                f.instruction(&Instruction::I32Const((ENUM_BASE + depth * 4) as i32));
                f.instruction(&Instruction::GlobalGet(0));
                f.instruction(&Instruction::I32Store(ma_i32));
                // Bump heap pointer past this enum's allocation
                f.instruction(&Instruction::GlobalGet(0));
                f.instruction(&Instruction::I32Const(total_size as i32));
                f.instruction(&Instruction::I32Add);
                f.instruction(&Instruction::GlobalSet(0));
                // Store tag at base + 0
                f.instruction(&Instruction::I32Const((ENUM_BASE + depth * 4) as i32));
                f.instruction(&Instruction::I32Load(ma_i32));
                f.instruction(&Instruction::I32Const(*tag));
                f.instruction(&Instruction::I32Store(ma_i32));
                // Store each payload value with proper type
                for (i, pval) in payload.iter().enumerate() {
                    let (offset, is_f64) = self.enum_payload_info(enum_name, variant, i);
                    // Load saved base pointer
                    f.instruction(&Instruction::I32Const((ENUM_BASE + depth * 4) as i32));
                    f.instruction(&Instruction::I32Load(ma_i32));
                    f.instruction(&Instruction::I32Const(offset as i32));
                    f.instruction(&Instruction::I32Add);
                    self.emit_operand(f, pval);
                    if is_f64 {
                        f.instruction(&Instruction::F64Store(ma_f64));
                    } else {
                        f.instruction(&Instruction::I32Store(ma_i32));
                    }
                }
                // Push base pointer as result
                f.instruction(&Instruction::I32Const((ENUM_BASE + depth * 4) as i32));
                f.instruction(&Instruction::I32Load(ma_i32));
                self.enum_init_depth -= 1;
            }
            Operand::EnumTag(inner) => {
                let ma = MemArg {
                    offset: 0,
                    align: 2,
                    memory_index: 0,
                };
                // Load tag (i32) from enum pointer at offset 0
                self.emit_operand(f, inner);
                f.instruction(&Instruction::I32Load(ma));
            }
            Operand::EnumPayload {
                object,
                index,
                enum_name,
                variant_name,
            } => {
                let (offset, is_f64) =
                    self.enum_payload_info(enum_name, variant_name, *index as usize);
                self.emit_operand(f, object);
                f.instruction(&Instruction::I32Const(offset as i32));
                f.instruction(&Instruction::I32Add);
                if is_f64 {
                    let ma = MemArg {
                        offset: 0,
                        align: 3,
                        memory_index: 0,
                    };
                    f.instruction(&Instruction::F64Load(ma));
                } else {
                    let ma = MemArg {
                        offset: 0,
                        align: 2,
                        memory_index: 0,
                    };
                    f.instruction(&Instruction::I32Load(ma));
                }
            }
            Operand::LoopExpr {
                init: _,
                body,
                result,
            } => {
                for stmt in body {
                    self.emit_stmt(f, stmt);
                }
                self.emit_operand(f, result);
            }
            Operand::TryExpr { expr } => {
                // expr? on Result<T, E>:
                // 1. Evaluate expr → Result ptr
                // 2. Store to SCRATCH+24
                // 3. Load tag: if Err (tag=1), return the same Result ptr
                // 4. If Ok (tag=0), load payload at offset 4
                f.instruction(&Instruction::I32Const(SCRATCH as i32 + 24));
                self.emit_operand(f, expr);
                f.instruction(&Instruction::I32Store(MemArg {
                    offset: 0,
                    align: 2,
                    memory_index: 0,
                }));

                // Check tag
                f.instruction(&Instruction::I32Const(SCRATCH as i32 + 24));
                f.instruction(&Instruction::I32Load(MemArg {
                    offset: 0,
                    align: 2,
                    memory_index: 0,
                }));
                f.instruction(&Instruction::I32Load(MemArg {
                    offset: 0,
                    align: 2,
                    memory_index: 0,
                }));
                // Stack: [tag]
                f.instruction(&Instruction::If(wasm_encoder::BlockType::Empty));
                {
                    // tag != 0 (Err) — early return with the Result ptr
                    f.instruction(&Instruction::I32Const(SCRATCH as i32 + 24));
                    f.instruction(&Instruction::I32Load(MemArg {
                        offset: 0,
                        align: 2,
                        memory_index: 0,
                    }));
                    f.instruction(&Instruction::Return);
                }
                f.instruction(&Instruction::End);
                // tag == 0 (Ok) — extract payload at offset 4
                f.instruction(&Instruction::I32Const(SCRATCH as i32 + 24));
                f.instruction(&Instruction::I32Load(MemArg {
                    offset: 0,
                    align: 2,
                    memory_index: 0,
                }));
                f.instruction(&Instruction::I32Load(MemArg {
                    offset: 4,
                    align: 2,
                    memory_index: 0,
                }));
            }
            Operand::FnRef(name) => {
                // Push the function's table index (== function index)
                if let Some(idx) = self.resolve_fn(name) {
                    f.instruction(&Instruction::I32Const(idx as i32));
                } else {
                    f.instruction(&Instruction::I32Const(0));
                }
            }
            Operand::CallIndirect { callee, args } => {
                // Push arguments first
                for arg in args {
                    self.emit_operand(f, arg);
                }
                // Push the function table index (callee)
                self.emit_operand(f, callee);
                // Determine the type signature: (i32 × n_args) -> i32
                let params: Vec<ValType> = args.iter().map(|_| ValType::I32).collect();
                let results = vec![ValType::I32];
                let type_idx = self.lookup_or_register_indirect_type(params, results);
                f.instruction(&Instruction::CallIndirect {
                    type_index: type_idx,
                    table_index: 0,
                });
            }
            Operand::ArrayInit { elements } => {
                // Allocate [element0][element1]...[elementN-1] in linear memory.
                // Each element is 4 bytes (i32). Returns pointer to element 0.
                let n = elements.len() as i32;
                let ma = wasm_encoder::MemArg {
                    offset: 0,
                    align: 2,
                    memory_index: 0,
                };
                // Save current bump pointer (= array base) to SCRATCH
                f.instruction(&Instruction::I32Const(SCRATCH as i32));
                f.instruction(&Instruction::GlobalGet(0));
                f.instruction(&Instruction::I32Store(ma));
                // Advance bump pointer by n*4
                f.instruction(&Instruction::GlobalGet(0));
                f.instruction(&Instruction::I32Const(n * 4));
                f.instruction(&Instruction::I32Add);
                f.instruction(&Instruction::GlobalSet(0));
                // Store each element: mem[base + i*4] = elem
                for (i, elem) in elements.iter().enumerate() {
                    f.instruction(&Instruction::I32Const(SCRATCH as i32));
                    f.instruction(&Instruction::I32Load(ma));
                    f.instruction(&Instruction::I32Const((i as i32) * 4));
                    f.instruction(&Instruction::I32Add);
                    self.emit_operand(f, elem);
                    f.instruction(&Instruction::I32Store(ma));
                }
                // Leave base pointer on stack as the array value
                f.instruction(&Instruction::I32Const(SCRATCH as i32));
                f.instruction(&Instruction::I32Load(ma));
            }
            Operand::IndexAccess { object, index } => {
                let ma = wasm_encoder::MemArg {
                    offset: 0,
                    align: 2,
                    memory_index: 0,
                };
                // array_ptr + index * 4
                self.emit_operand(f, object);
                self.emit_operand(f, index);
                f.instruction(&Instruction::I32Const(4));
                f.instruction(&Instruction::I32Mul);
                f.instruction(&Instruction::I32Add);
                f.instruction(&Instruction::I32Load(ma));
            }
            Operand::Unit => { /* nothing to push */ }
            other => {
                eprintln!(
                    "ICE: unhandled operand in emit_operand: {:?}",
                    std::mem::discriminant(other)
                );
                f.instruction(&Instruction::I32Const(0));
            }
        }
    }

    fn emit_binop(&mut self, f: &mut Function, op: &BinOp) {
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
        };
    }

    fn emit_binop_i64(&mut self, f: &mut Function, op: &BinOp) {
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
            BinOp::Eq => {
                f.instruction(&Instruction::I64Eq);
            }
            BinOp::Ne => {
                f.instruction(&Instruction::I64Ne);
            }
            BinOp::Lt => {
                f.instruction(&Instruction::I64LtS);
            }
            BinOp::Le => {
                f.instruction(&Instruction::I64LeS);
            }
            BinOp::Gt => {
                f.instruction(&Instruction::I64GtS);
            }
            BinOp::Ge => {
                f.instruction(&Instruction::I64GeS);
            }
            BinOp::And => {
                f.instruction(&Instruction::I64And);
            }
            BinOp::Or => {
                f.instruction(&Instruction::I64Or);
            }
            BinOp::BitAnd => {
                f.instruction(&Instruction::I64And);
            }
            BinOp::BitOr => {
                f.instruction(&Instruction::I64Or);
            }
            BinOp::BitXor => {
                f.instruction(&Instruction::I64Xor);
            }
            BinOp::Shl => {
                f.instruction(&Instruction::I64Shl);
            }
            BinOp::Shr => {
                f.instruction(&Instruction::I64ShrS);
            }
        };
    }

    fn emit_binop_f64(&mut self, f: &mut Function, op: &BinOp) {
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
            BinOp::Eq => {
                f.instruction(&Instruction::F64Eq);
            }
            BinOp::Ne => {
                f.instruction(&Instruction::F64Ne);
            }
            BinOp::Lt => {
                f.instruction(&Instruction::F64Lt);
            }
            BinOp::Le => {
                f.instruction(&Instruction::F64Le);
            }
            BinOp::Gt => {
                f.instruction(&Instruction::F64Gt);
            }
            BinOp::Ge => {
                f.instruction(&Instruction::F64Ge);
            }
            // Mod and bitwise ops don't apply to f64, fall back to i32
            _ => {
                self.emit_binop(f, op);
            }
        };
    }

    fn is_f64_operand(&self, op: &Operand) -> bool {
        match op {
            Operand::ConstF64(_) => true,
            Operand::Place(Place::Local(id)) => self.f64_locals.contains(&id.0),
            Operand::BinOp(_, l, r) => self.is_f64_operand(l) || self.is_f64_operand(r),
            Operand::UnaryOp(_, inner) => self.is_f64_operand(inner),
            Operand::Call(name, _) => matches!(
                normalize_intrinsic_name(name.as_str()),
                "sqrt" | "parse_f64"
            ),
            _ => false,
        }
    }

    fn is_i64_operand(&self, op: &Operand) -> bool {
        match op {
            Operand::ConstI64(_) => true,
            Operand::Place(Place::Local(id)) => self.i64_locals.contains(&id.0),
            Operand::BinOp(_, l, r) => self.is_i64_operand(l) || self.is_i64_operand(r),
            Operand::UnaryOp(_, inner) => self.is_i64_operand(inner),
            Operand::Call(name, _) => {
                matches!(
                    normalize_intrinsic_name(name.as_str()),
                    "parse_i64" | "clock_now"
                )
            }
            _ => false,
        }
    }

    fn is_bool_operand(&self, op: &Operand) -> bool {
        match op {
            Operand::ConstBool(_) => true,
            Operand::Place(Place::Local(id)) => self.bool_locals.contains(&id.0),
            Operand::BinOp(op, _, _) => {
                matches!(
                    op,
                    BinOp::Eq
                        | BinOp::Ne
                        | BinOp::Lt
                        | BinOp::Le
                        | BinOp::Gt
                        | BinOp::Ge
                        | BinOp::And
                        | BinOp::Or
                )
            }
            Operand::UnaryOp(op, _) => matches!(op, UnaryOp::Not),
            _ => false,
        }
    }

    /// Emit an operand as f64, promoting i32/i64 constants if needed
    fn emit_f64_operand(&mut self, f: &mut Function, op: &Operand) {
        match op {
            Operand::ConstI32(v) => {
                f.instruction(&Instruction::F64Const(*v as f64));
            }
            Operand::ConstI64(v) => {
                f.instruction(&Instruction::F64Const(*v as f64));
            }
            _ => self.emit_operand(f, op),
        }
    }

    /// Emit an operand as i64, promoting i32 constants if needed
    fn emit_i64_operand(&mut self, f: &mut Function, op: &Operand) {
        match op {
            Operand::ConstI32(v) => {
                f.instruction(&Instruction::I64Const(*v as i64));
            }
            _ => self.emit_operand(f, op),
        }
    }

    fn emit_unaryop(&mut self, f: &mut Function, op: &UnaryOp, inner: &Operand) {
        match op {
            UnaryOp::Neg => {
                let is_f64 = matches!(inner, Operand::ConstF64(_))
                    || matches!(inner, Operand::Place(Place::Local(id)) if self.f64_locals.contains(&id.0));
                if is_f64 {
                    self.emit_operand(f, inner);
                    f.instruction(&Instruction::F64Neg);
                } else {
                    f.instruction(&Instruction::I32Const(0));
                    self.emit_operand(f, inner);
                    f.instruction(&Instruction::I32Sub);
                }
            }
            UnaryOp::Not => {
                self.emit_operand(f, inner);
                f.instruction(&Instruction::I32Eqz);
            }
            UnaryOp::BitNot => {
                self.emit_operand(f, inner);
                f.instruction(&Instruction::I32Const(-1));
                f.instruction(&Instruction::I32Xor);
            }
        }
    }
}
