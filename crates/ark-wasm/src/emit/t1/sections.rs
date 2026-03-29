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

        // Determine which canonical functions are needed
        let needed = collect_needed_fns(mir);

        // WASI imports in canonical order
        const WASI_ORDER: &[(u32, &str)] = &[
            (FN_FD_WRITE, "fd_write"),
            (FN_PATH_OPEN, "path_open"),
            (FN_FD_READ, "fd_read"),
            (FN_FD_CLOSE, "fd_close"),
            (FN_CLOCK_TIME_GET, "clock_time_get"),
            (FN_RANDOM_GET, "random_get"),
        ];
        // Stdlib helpers in canonical order (indices 6..29)
        const STDLIB_ORDER: &[u32] = &[
            FN_I32_TO_STR,
            FN_PRINT_I32_LN,
            FN_PRINT_BOOL_LN,
            FN_PRINT_STR_LN,
            FN_STR_EQ,
            FN_CONCAT,
            FN_F64_TO_STR,
            FN_I64_TO_STR,
            FN_MAP_I32,
            FN_FILTER_I32,
            FN_FOLD_I32,
            FN_MAP_OPT_I32,
            FN_ANY_I32,
            FN_FIND_I32,
            FN_HASHMAP_I32_NEW,
            FN_HASHMAP_I32_INSERT,
            FN_HASHMAP_I32_GET,
            FN_HASHMAP_I32_CONTAINS,
            FN_HASHMAP_I32_LEN,
            FN_MAP_I64,
            FN_FILTER_I64,
            FN_FOLD_I64,
            FN_MAP_F64,
            FN_FILTER_F64,
        ];

        // Build fn_map: canonical index -> actual Wasm function index
        let fn_map_size = FN_USER_BASE as usize + mir.functions.len() + 2;
        let mut fn_map = vec![u32::MAX; fn_map_size];
        let mut next_idx = 0u32;

        for &(canonical, _) in WASI_ORDER {
            if needed.contains(&canonical) {
                fn_map[canonical as usize] = next_idx;
                next_idx += 1;
            }
        }

        for &canonical in STDLIB_ORDER {
            if needed.contains(&canonical) {
                fn_map[canonical as usize] = next_idx;
                next_idx += 1;
            }
        }

        let user_base_actual = next_idx;
        for i in 0..mir.functions.len() {
            fn_map[FN_USER_BASE as usize + i] = user_base_actual + i as u32;
        }
        next_idx += mir.functions.len() as u32;

        // Store fn_map
        self.fn_map = fn_map;

        // Type section
        let mut types = TypeSection::new();
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
        let ty_f64_i32 = self.register_type(&mut types, vec![ValType::F64], vec![ValType::I32]);
        let ty_i64_i32 = self.register_type(&mut types, vec![ValType::I64], vec![ValType::I32]);
        let ty_i32x3_i32 =
            self.register_type(&mut types, vec![ValType::I32; 3], vec![ValType::I32]);
        let _ty_i64_i64 = self.register_type(&mut types, vec![ValType::I64], vec![ValType::I64]);
        let _ty_i64x2_i64 =
            self.register_type(&mut types, vec![ValType::I64; 2], vec![ValType::I64]);
        let _ty_f64_f64 = self.register_type(&mut types, vec![ValType::F64], vec![ValType::F64]);
        let ty_fold_i64 = self.register_type(
            &mut types,
            vec![ValType::I32, ValType::I64, ValType::I32],
            vec![ValType::I64],
        );
        let ty_i32x3_void = self.register_type(&mut types, vec![ValType::I32; 3], vec![]);
        let ty_path_open = self.register_type(
            &mut types,
            vec![
                ValType::I32, ValType::I32, ValType::I32, ValType::I32, ValType::I32,
                ValType::I64, ValType::I64, ValType::I32, ValType::I32,
            ],
            vec![ValType::I32],
        );
        let ty_clock_time_get = self.register_type(
            &mut types,
            vec![ValType::I32, ValType::I64, ValType::I32],
            vec![ValType::I32],
        );
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

        // Import section - only needed WASI imports
        let mut imports = ImportSection::new();
        for &(canonical, wasi_name) in WASI_ORDER {
            if needed.contains(&canonical) {
                let ty = match canonical {
                    x if x == FN_FD_WRITE => ty_fd_write,
                    x if x == FN_PATH_OPEN => ty_path_open,
                    x if x == FN_FD_READ => ty_fd_write,
                    x if x == FN_FD_CLOSE => ty_i32_i32,
                    x if x == FN_CLOCK_TIME_GET => ty_clock_time_get,
                    x if x == FN_RANDOM_GET => ty_i32_i32_i32,
                    _ => ty_fd_write,
                };
                imports.import(
                    "wasi_snapshot_preview1",
                    wasi_name,
                    wasm_encoder::EntityType::Function(ty),
                );
            }
        }
        module.section(&imports);

        // Function section - only needed stdlib helpers + user functions
        let mut functions = FunctionSection::new();
        for &canonical in STDLIB_ORDER {
            if needed.contains(&canonical) {
                let ty = match canonical {
                    x if x == FN_I32_TO_STR => ty_i32_void,
                    x if x == FN_PRINT_I32_LN => ty_i32_void,
                    x if x == FN_PRINT_BOOL_LN => ty_i32_void,
                    x if x == FN_PRINT_STR_LN => ty_i32_void,
                    x if x == FN_STR_EQ => ty_i32_i32_i32,
                    x if x == FN_CONCAT => ty_i32_i32_i32,
                    x if x == FN_F64_TO_STR => ty_f64_i32,
                    x if x == FN_I64_TO_STR => ty_i64_i32,
                    x if x == FN_MAP_I32 => ty_i32_i32_i32,
                    x if x == FN_FILTER_I32 => ty_i32_i32_i32,
                    x if x == FN_FOLD_I32 => ty_i32x3_i32,
                    x if x == FN_MAP_OPT_I32 => ty_i32_i32_i32,
                    x if x == FN_ANY_I32 => ty_i32_i32_i32,
                    x if x == FN_FIND_I32 => ty_i32_i32_i32,
                    x if x == FN_HASHMAP_I32_NEW => ty_void_i32,
                    x if x == FN_HASHMAP_I32_INSERT => ty_i32x3_void,
                    x if x == FN_HASHMAP_I32_GET => ty_i32_i32_i32,
                    x if x == FN_HASHMAP_I32_CONTAINS => ty_i32_i32_i32,
                    x if x == FN_HASHMAP_I32_LEN => ty_i32_i32,
                    x if x == FN_MAP_I64 => ty_i32_i32_i32,
                    x if x == FN_FILTER_I64 => ty_i32_i32_i32,
                    x if x == FN_FOLD_I64 => ty_fold_i64,
                    x if x == FN_MAP_F64 => ty_i32_i32_i32,
                    x if x == FN_FILTER_F64 => ty_i32_i32_i32,
                    _ => ty_i32_void,
                };
                functions.function(ty);
            }
        }
        let mut needs_start_wrapper = false;
        for (i, func) in mir.functions.iter().enumerate() {
            functions.function(user_func_type_indices[i]);
            if func.name == "main" && !matches!(func.return_ty, ark_typecheck::types::Type::Unit) {
                needs_start_wrapper = true;
            }
        }
        let start_wrapper_actual_idx = if needs_start_wrapper {
            let idx = next_idx;
            next_idx += 1;
            functions.function(ty_void_void);
            Some(idx)
        } else {
            None
        };
        module.section(&functions);

        // Table section
        let total_funcs = next_idx as u64;
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
            minimum: 64,
            maximum: None,
            memory64: false,
            shared: false,
            page_size_log2: None,
        });
        module.section(&memory);

        // Global section
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
        if let Some(wrapper_idx) = start_wrapper_actual_idx {
            exports.export("_start", ExportKind::Func, wrapper_idx);
        } else {
            for (idx, func) in mir.functions.iter().enumerate() {
                if func.name == "main" {
                    exports.export("_start", ExportKind::Func, user_base_actual + idx as u32);
                }
            }
        }
        module.section(&exports);

        // Element section
        let func_indices: Vec<u32> = (0..next_idx).collect();
        let mut elements = wasm_encoder::ElementSection::new();
        elements.active(
            Some(0),
            &wasm_encoder::ConstExpr::i32_const(0),
            wasm_encoder::Elements::Functions(std::borrow::Cow::Borrowed(&func_indices)),
        );
        module.section(&elements);

        // Code section
        let mut code = CodeSection::new();
        for &canonical in STDLIB_ORDER {
            if !needed.contains(&canonical) { continue; }
            match canonical {
                x if x == FN_I32_TO_STR => { let f = self.build_i32_to_string(); code.function(&f); }
                x if x == FN_PRINT_I32_LN => { let f = self.build_print_i32_ln(); code.function(&f); }
                x if x == FN_PRINT_BOOL_LN => { let f = self.build_print_bool_ln(); code.function(&f); }
                x if x == FN_PRINT_STR_LN => { let f = self.build_print_str_ln(); code.function(&f); }
                x if x == FN_STR_EQ => { let f = self.build_str_eq(); code.function(&f); }
                x if x == FN_CONCAT => { let f = self.build_concat(); code.function(&f); }
                x if x == FN_F64_TO_STR => { let f = self.build_f64_to_string(); code.function(&f); }
                x if x == FN_I64_TO_STR => { let f = self.build_i64_to_string(); code.function(&f); }
                x if x == FN_MAP_I32 => { let f = self.build_map_i32(); code.function(&f); }
                x if x == FN_FILTER_I32 => { let f = self.build_filter_i32(); code.function(&f); }
                x if x == FN_FOLD_I32 => { let f = self.build_fold_i32(); code.function(&f); }
                x if x == FN_MAP_OPT_I32 => { let f = self.build_map_option_i32(); code.function(&f); }
                x if x == FN_ANY_I32 => { let f = self.build_any_i32(); code.function(&f); }
                x if x == FN_FIND_I32 => { let f = self.build_find_i32(); code.function(&f); }
                x if x == FN_HASHMAP_I32_NEW => { let f = self.build_hashmap_i32_new(); code.function(&f); }
                x if x == FN_HASHMAP_I32_INSERT => { let f = self.build_hashmap_i32_insert(); code.function(&f); }
                x if x == FN_HASHMAP_I32_GET => { let f = self.build_hashmap_i32_get(); code.function(&f); }
                x if x == FN_HASHMAP_I32_CONTAINS => { let f = self.build_hashmap_i32_contains(); code.function(&f); }
                x if x == FN_HASHMAP_I32_LEN => { let f = self.build_hashmap_i32_len(); code.function(&f); }
                x if x == FN_MAP_I64 => { let f = self.build_map_i64(); code.function(&f); }
                x if x == FN_FILTER_I64 => { let f = self.build_filter_i64(); code.function(&f); }
                x if x == FN_FOLD_I64 => { let f = self.build_fold_i64(); code.function(&f); }
                x if x == FN_MAP_F64 => { let f = self.build_map_f64(); code.function(&f); }
                x if x == FN_FILTER_F64 => { let f = self.build_filter_f64(); code.function(&f); }
                _ => {}
            }
        }
        for func in &mir.functions {
            let f = self.build_user_fn(func);
            code.function(&f);
        }
        if needs_start_wrapper {
            let main_idx = mir
                .functions
                .iter()
                .position(|f| f.name == "main")
                .map(|i| user_base_actual + i as u32)
                .unwrap();
            let mut wrapper = Function::new(vec![]);
            wrapper.instruction(&Instruction::Call(main_idx));
            wrapper.instruction(&Instruction::Drop);
            wrapper.instruction(&Instruction::End);
            code.function(&wrapper);
        }
        module.section(&code);

        // Data section
        let mut data = DataSection::new();
        data.active(
            0,
            &wasm_encoder::ConstExpr::i32_const(BOOL_TRUE as i32),
            b"true".iter().copied(),
        );
        data.active(
            0,
            &wasm_encoder::ConstExpr::i32_const(BOOL_FALSE as i32),
            b"false".iter().copied(),
        );
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
