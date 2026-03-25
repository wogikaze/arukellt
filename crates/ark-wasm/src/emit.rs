//! Wasm module emission using wasm-encoder.
//!
//! Generates a WASI Preview 1 compatible Wasm module from MIR.
//! Uses linear memory. Direct recursive emission (no planning phase).

use ark_diagnostics::DiagnosticSink;
use ark_mir::mir::*;
use std::collections::HashSet;
use wasm_encoder::{
    CodeSection, DataSection, ExportKind, ExportSection, Function,
    FunctionSection, ImportSection, Instruction, MemArg, MemorySection, MemoryType,
    Module, TypeSection, ValType,
};

const IOV_BASE: u32 = 0;
const NWRITTEN: u32 = 8;
const SCRATCH: u32 = 16;  // temp for i32_to_string length
const I32BUF: u32 = 48;   // buffer for i32_to_string output (20 bytes max)
const BOOL_TRUE: u32 = 80; // "true" (4 bytes)
const BOOL_FALSE: u32 = 84; // "false" (5 bytes)
const NEWLINE: u32 = 89;   // "\n" (1 byte)
const DATA_START: u32 = 256;

// Function indices:
// 0 = fd_write (import)
// 1 = __i32_to_string (helper: i32 -> offset,len pair stored at SCRATCH/SCRATCH+4)
// 2 = __print_i32_ln (helper: prints i32 as decimal + newline)
// 3 = __print_bool_ln (helper: prints bool as "true"/"false" + newline)
// 4 = __print_str_ln (helper: prints length-prefixed string + newline)
// 5+ = user functions
const FN_FD_WRITE: u32 = 0;
const FN_I32_TO_STR: u32 = 1;
const FN_PRINT_I32_LN: u32 = 2;
const FN_PRINT_BOOL_LN: u32 = 3;
const FN_PRINT_STR_LN: u32 = 4;
const FN_USER_BASE: u32 = 5;

pub fn emit(mir: &MirModule, _sink: &mut DiagnosticSink) -> Vec<u8> {
    let mut ctx = EmitCtx {
        string_literals: Vec::new(),
        data_offset: DATA_START,
        fn_names: mir.functions.iter().map(|f| f.name.clone()).collect(),
        loop_depths: Vec::new(),
        string_locals: HashSet::new(),
        fn_return_types: mir.functions.iter()
            .map(|f| (f.name.clone(), f.return_ty.clone()))
            .collect(),
    };
    ctx.emit_module(mir)
}

struct EmitCtx {
    string_literals: Vec<(u32, Vec<u8>)>,
    data_offset: u32,
    fn_names: Vec<String>,
    /// Stack of extra block depths for break/continue inside loops.
    loop_depths: Vec<u32>,
    /// Locals known to hold string values (for println dispatch).
    string_locals: HashSet<u32>,
    /// Function return types (for println dispatch on user function calls).
    fn_return_types: std::collections::HashMap<String, ark_typecheck::types::Type>,
}

impl EmitCtx {
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
        self.string_literals.push((offset, len.to_le_bytes().to_vec()));
        // Write data
        self.string_literals.push((offset + 4, bytes.to_vec()));
        self.data_offset += 4 + len;
        offset + 4 // pointer to data start
    }

    fn resolve_fn(&self, name: &str) -> Option<u32> {
        self.fn_names.iter().position(|n| n == name).map(|i| FN_USER_BASE + i as u32)
    }

    fn emit_module(&mut self, mir: &MirModule) -> Vec<u8> {
        let mut module = Module::new();

        // Type section
        let mut types = TypeSection::new();
        // 0: fd_write (i32,i32,i32,i32)->i32
        types.ty().function(vec![ValType::I32; 4], vec![ValType::I32]);
        // 1: ()->()
        types.ty().function(vec![], vec![]);
        // 2: ()->i32
        types.ty().function(vec![], vec![ValType::I32]);
        // 3: (i32)->()
        types.ty().function(vec![ValType::I32], vec![]);
        // 4: (i32,i32)->i32
        types.ty().function(vec![ValType::I32, ValType::I32], vec![ValType::I32]);
        // 5: (i32)->i32
        types.ty().function(vec![ValType::I32], vec![ValType::I32]);
        module.section(&types);

        // Import section
        let mut imports = ImportSection::new();
        imports.import("wasi_snapshot_preview1", "fd_write",
            wasm_encoder::EntityType::Function(0));
        module.section(&imports);

        // Function section (declare types for helpers + user funcs)
        let mut functions = FunctionSection::new();
        functions.function(3); // __i32_to_string: (i32)->()
        functions.function(3); // __print_i32_ln: (i32)->()
        functions.function(3); // __print_bool_ln: (i32)->()
        functions.function(3); // __print_str_ln: (i32)->()
        let mut needs_start_wrapper = false;
        for func in &mir.functions {
            functions.function(self.func_type_idx(func));
            if func.name == "main" && !matches!(func.return_ty, ark_typecheck::types::Type::Unit) {
                needs_start_wrapper = true;
            }
        }
        let start_wrapper_idx = if needs_start_wrapper {
            let idx = FN_USER_BASE + mir.functions.len() as u32;
            functions.function(1); // ()->()
            Some(idx)
        } else {
            None
        };
        module.section(&functions);

        // Memory section
        let mut memory = MemorySection::new();
        memory.memory(MemoryType {
            minimum: 1, maximum: None, memory64: false, shared: false, page_size_log2: None,
        });
        module.section(&memory);

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

        // Code section
        let mut code = CodeSection::new();
        code.function(&self.build_i32_to_string());
        code.function(&self.build_print_i32_ln());
        code.function(&self.build_print_bool_ln());
        code.function(&self.build_print_str_ln());
        for func in &mir.functions {
            let f = self.build_user_fn(func);
            code.function(&f);
        }
        if needs_start_wrapper {
            // _start wrapper: call main, drop result
            let main_idx = mir.functions.iter().position(|f| f.name == "main")
                .map(|i| FN_USER_BASE + i as u32).unwrap();
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
        data.active(0, &wasm_encoder::ConstExpr::i32_const(BOOL_TRUE as i32),
            b"true".iter().copied());
        // "false" at BOOL_FALSE (84)
        data.active(0, &wasm_encoder::ConstExpr::i32_const(BOOL_FALSE as i32),
            b"false".iter().copied());
        // "\n" at NEWLINE (89)
        data.active(0, &wasm_encoder::ConstExpr::i32_const(NEWLINE as i32),
            b"\n".iter().copied());

        for (offset, bytes) in &self.string_literals {
            data.active(0, &wasm_encoder::ConstExpr::i32_const(*offset as i32),
                bytes.iter().copied());
        }
        module.section(&data);

        module.finish()
    }

    fn func_type_idx(&self, func: &MirFunction) -> u32 {
        let np = func.params.len();
        let has_ret = !matches!(func.return_ty, ark_typecheck::types::Type::Unit);
        match (np, has_ret) {
            (0, false) => 1,
            (0, true) => 2,
            (1, false) => 3,
            (1, true) => 5,
            (2, true) => 4,
            _ => 1, // fallback
        }
    }

    /// __i32_to_string(value: i32) -> void
    /// Writes decimal string to I32BUF, stores offset at SCRATCH, length at SCRATCH+4
    fn build_i32_to_string(&self) -> Function {
        let ma = MemArg { offset: 0, align: 0, memory_index: 0 };
        let ma2 = MemArg { offset: 0, align: 2, memory_index: 0 };
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
        let ma2 = MemArg { offset: 0, align: 2, memory_index: 0 };
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
        let ma2 = MemArg { offset: 0, align: 2, memory_index: 0 };
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
        let ma2 = MemArg { offset: 0, align: 2, memory_index: 0 };
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
        let ma2 = MemArg { offset: 0, align: 2, memory_index: 0 };
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

    fn build_user_fn(&mut self, func: &MirFunction) -> Function {
        let num_params = func.params.len() as u32;
        let num_locals = func.locals.len() as u32;
        let extra = if num_locals > num_params { num_locals - num_params } else { 0 };

        let mut locals = Vec::new();
        if extra > 0 {
            locals.push((extra, ValType::I32));
        }

        let mut f = Function::new(locals);

        // Identify string locals from MIR type info and operand scanning
        self.string_locals.clear();
        for local in &func.locals {
            if matches!(local.ty, ark_typecheck::types::Type::String) {
                self.string_locals.insert(local.id.0);
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
                MirStmt::IfStmt { then_body, else_body, .. } => {
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
                if matches!(name.as_str(), "String_from") {
                    return true;
                }
                // Check if function returns String
                if self.fn_return_types.get(name.as_str())
                    .map_or(false, |t| matches!(t, ark_typecheck::types::Type::String)) {
                    return true;
                }
                // Heuristic: if any arg is a string, generic function might return string
                args.iter().any(|a| self.is_string_operand(a))
            }
            Operand::IfExpr { then_result, else_result, .. } => {
                let t = then_result.as_ref().map_or(false, |r| self.is_string_operand(r));
                let e = else_result.as_ref().map_or(false, |r| self.is_string_operand(r));
                t || e
            }
            Operand::Place(Place::Local(id)) => {
                self.string_locals.contains(&id.0)
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
                match name.as_str() {
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
                    other => {
                        // User function call
                        for arg in args {
                            self.emit_operand(f, arg);
                        }
                        if let Some(idx) = self.resolve_fn(other) {
                            f.instruction(&Instruction::Call(idx));
                            // If function returns a value, drop it (called as statement)
                            // TODO: check return type. For now, assume void statements.
                        }
                    }
                }
            }
            MirStmt::IfStmt { cond, then_body, else_body } => {
                self.emit_operand(f, cond);
                f.instruction(&Instruction::If(wasm_encoder::BlockType::Empty));
                // Track block depth for break/continue
                if let Some(d) = self.loop_depths.last_mut() { *d += 1; }
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
                if let Some(d) = self.loop_depths.last_mut() { *d -= 1; }
            }
            MirStmt::WhileStmt { cond, body } => {
                f.instruction(&Instruction::Block(wasm_encoder::BlockType::Empty));  // break target
                f.instruction(&Instruction::Loop(wasm_encoder::BlockType::Empty));   // continue target
                self.emit_operand(f, cond);
                f.instruction(&Instruction::I32Eqz);
                f.instruction(&Instruction::BrIf(1));  // exit if cond is false
                self.loop_depths.push(0);
                for s in body {
                    self.emit_stmt(f, s);
                }
                self.loop_depths.pop();
                f.instruction(&Instruction::Br(0));    // loop back
                f.instruction(&Instruction::End);      // end loop
                f.instruction(&Instruction::End);      // end block
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
            _ => {}
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
                    match name.as_str() {
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
                        "char_to_string" => {
                            // Write char byte to scratch, print it + newline
                            if let Some(inner) = inner_args.first() {
                                let ma0 = MemArg { offset: 0, align: 0, memory_index: 0 };
                                let ma2 = MemArg { offset: 0, align: 2, memory_index: 0 };
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
                            // User function call — emit call, then print result
                            for a in inner_args {
                                self.emit_operand(f, a);
                            }
                            if let Some(idx) = self.resolve_fn(other) {
                                f.instruction(&Instruction::Call(idx));
                                let is_str = self.fn_return_types.get(other)
                                    .map_or(false, |t| matches!(t, ark_typecheck::types::Type::String));
                                // Heuristic for generic functions: if args produce strings,
                                // the return is likely also a string
                                let args_suggest_str = inner_args.iter()
                                    .any(|a| self.is_string_operand(a));
                                if is_str || args_suggest_str {
                                    f.instruction(&Instruction::Call(FN_PRINT_STR_LN));
                                } else {
                                    f.instruction(&Instruction::Call(FN_PRINT_I32_LN));
                                }
                            } else {
                                // Unknown function, try as i32
                                f.instruction(&Instruction::Call(FN_PRINT_I32_LN));
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
                    // Generic: try printing as i32
                    self.emit_operand(f, arg);
                    f.instruction(&Instruction::Call(FN_PRINT_I32_LN));
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
                    let ma2 = MemArg { offset: 0, align: 2, memory_index: 0 };
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
        let ma2 = MemArg { offset: 0, align: 2, memory_index: 0 };
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
            Operand::ConstI32(v) => { f.instruction(&Instruction::I32Const(*v)); }
            Operand::ConstI64(v) => { f.instruction(&Instruction::I64Const(*v)); }
            Operand::ConstBool(v) => { f.instruction(&Instruction::I32Const(if *v { 1 } else { 0 })); }
            Operand::ConstChar(c) => { f.instruction(&Instruction::I32Const(*c as i32)); }
            Operand::ConstString(s) => {
                // Allocate as length-prefixed string, return pointer
                let ptr = self.alloc_length_prefixed_string(s);
                f.instruction(&Instruction::I32Const(ptr as i32));
            }
            Operand::Place(Place::Local(id)) => { f.instruction(&Instruction::LocalGet(id.0)); }
            Operand::BinOp(op, left, right) => {
                self.emit_operand(f, left);
                self.emit_operand(f, right);
                self.emit_binop(f, op);
            }
            Operand::UnaryOp(op, inner) => {
                self.emit_unaryop(f, op, inner);
            }
            Operand::Call(name, args) => {
                match name.as_str() {
                    "i32_to_string" => {
                        // Returns string — for now just call helper (puts in memory, not useful as value)
                        if let Some(a) = args.first() {
                            self.emit_operand(f, a);
                        }
                        f.instruction(&Instruction::Call(FN_I32_TO_STR));
                        f.instruction(&Instruction::I32Const(0)); // placeholder value
                    }
                    "bool_to_string" => {
                        // Similar to i32_to_string — not directly usable as a value
                        if let Some(a) = args.first() {
                            self.emit_operand(f, a);
                        }
                        f.instruction(&Instruction::I32Const(0)); // placeholder
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
            Operand::IfExpr { cond, then_body, then_result, else_body, else_result, .. } => {
                self.emit_operand(f, cond);
                f.instruction(&Instruction::If(wasm_encoder::BlockType::Result(ValType::I32)));
                // Track depth for break/continue
                if let Some(d) = self.loop_depths.last_mut() { *d += 1; }
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
                if let Some(d) = self.loop_depths.last_mut() { *d -= 1; }
            }
            Operand::Unit => { /* nothing to push */ }
            _ => { f.instruction(&Instruction::I32Const(0)); }
        }
    }

    fn emit_binop(&mut self, f: &mut Function, op: &BinOp) {
        match op {
            BinOp::Add => { f.instruction(&Instruction::I32Add); }
            BinOp::Sub => { f.instruction(&Instruction::I32Sub); }
            BinOp::Mul => { f.instruction(&Instruction::I32Mul); }
            BinOp::Div => { f.instruction(&Instruction::I32DivS); }
            BinOp::Mod => { f.instruction(&Instruction::I32RemS); }
            BinOp::Eq => { f.instruction(&Instruction::I32Eq); }
            BinOp::Ne => { f.instruction(&Instruction::I32Ne); }
            BinOp::Lt => { f.instruction(&Instruction::I32LtS); }
            BinOp::Le => { f.instruction(&Instruction::I32LeS); }
            BinOp::Gt => { f.instruction(&Instruction::I32GtS); }
            BinOp::Ge => { f.instruction(&Instruction::I32GeS); }
            BinOp::And => { f.instruction(&Instruction::I32And); }
            BinOp::Or => { f.instruction(&Instruction::I32Or); }
            BinOp::BitAnd => { f.instruction(&Instruction::I32And); }
            BinOp::BitOr => { f.instruction(&Instruction::I32Or); }
            BinOp::BitXor => { f.instruction(&Instruction::I32Xor); }
            BinOp::Shl => { f.instruction(&Instruction::I32Shl); }
            BinOp::Shr => { f.instruction(&Instruction::I32ShrS); }
        };
    }

    fn emit_unaryop(&mut self, f: &mut Function, op: &UnaryOp, inner: &Operand) {
        match op {
            UnaryOp::Neg => {
                f.instruction(&Instruction::I32Const(0));
                self.emit_operand(f, inner);
                f.instruction(&Instruction::I32Sub);
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
