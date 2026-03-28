//! Wasm section building: type, import, function, table, memory, globals,
//! exports, elements, code, and data sections.

use super::*;
use ark_mir::mir::MirModule;
use wasm_encoder::{
    CodeSection, DataSection, ExportKind, ExportSection, FunctionSection, ImportSection,
    MemorySection, MemoryType, Module, TypeSection,
};

impl EmitCtx {
    pub(super) fn emit_module(&mut self, mir: &MirModule) -> Vec<u8> {
        let mut module = Module::new();

        // Type section — register types dynamically
        let mut types = TypeSection::new();
        // Pre-register known helper types
        let ty_fd_write = self.register_type(&mut types, vec![ValType::I32; 4], vec![ValType::I32]);
        let ty_void_void = self.register_type(&mut types, vec![], vec![]);
        let ty_void_i32 = self.register_type(&mut types, vec![], vec![ValType::I32]);
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
        // HOF i64/f64 indirect call types
        let _ty_i64_i64 = self.register_type(&mut types, vec![ValType::I64], vec![ValType::I64]);
        let _ty_i64x2_i64 =
            self.register_type(&mut types, vec![ValType::I64; 2], vec![ValType::I64]);
        let _ty_f64_f64 = self.register_type(&mut types, vec![ValType::F64], vec![ValType::F64]);
        // fold_i64 helper type: (vec_ptr:i32, init:i64, fn_idx:i32) -> i64
        let ty_fold_i64 = self.register_type(
            &mut types,
            vec![ValType::I32, ValType::I64, ValType::I32],
            vec![ValType::I64],
        );
        // Register HashMap helper types
        let ty_i32x3_void = self.register_type(&mut types, vec![ValType::I32; 3], vec![]);
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
        functions.function(ty_void_i32); // __hashmap_i32_new: ()->i32
        functions.function(ty_i32x3_void); // __hashmap_i32_insert: (m,k,v)->()
        functions.function(ty_i32_i32_i32); // __hashmap_i32_get: (m,k)->i32(option_ptr)
        functions.function(ty_i32_i32_i32); // __hashmap_i32_contains: (m,k)->i32(bool)
        functions.function(ty_i32_i32); // __hashmap_i32_len: (m)->i32
        // HOF helpers for i64/f64 types
        functions.function(ty_i32_i32_i32); // __map_i64: (vec,fn)->vec
        functions.function(ty_i32_i32_i32); // __filter_i64: (vec,fn)->vec
        functions.function(ty_fold_i64); // __fold_i64: (vec,init:i64,fn)->i64
        functions.function(ty_i32_i32_i32); // __map_f64: (vec,fn)->vec
        functions.function(ty_i32_i32_i32); // __filter_f64: (vec,fn)->vec
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
            FN_USER_BASE as u64 + mir.functions.len() as u64 + if needs_start_wrapper { 1 } else { 0 };
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
        code.function(&self.build_hashmap_i32_new());
        code.function(&self.build_hashmap_i32_insert());
        code.function(&self.build_hashmap_i32_get());
        code.function(&self.build_hashmap_i32_contains());
        code.function(&self.build_hashmap_i32_len());
        code.function(&self.build_map_i64());
        code.function(&self.build_filter_i64());
        code.function(&self.build_fold_i64());
        code.function(&self.build_map_f64());
        code.function(&self.build_filter_f64());
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
}
