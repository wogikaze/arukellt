//! Wasm module emission using wasm-encoder.
//!
//! Generates a WASI Preview 1 compatible Wasm module from MIR.
//! Uses linear memory. Direct recursive emission (no planning phase).

use ark_diagnostics::DiagnosticSink;
use ark_mir::mir::*;
use wasm_encoder::{
    CodeSection, DataSection, ExportKind, ExportSection, Function,
    FunctionSection, ImportSection, Instruction, MemArg, MemorySection, MemoryType,
    Module, TypeSection, ValType,
};

const IOV_BASE: u32 = 0;
const NWRITTEN: u32 = 8;
const SCRATCH: u32 = 16;  // temp for i32_to_string length
const I32BUF: u32 = 48;   // buffer for i32_to_string output (20 bytes max)
const DATA_START: u32 = 256;

// Function indices:
// 0 = fd_write (import)
// 1 = __i32_to_string (helper: i32 -> stores offset,len at SCRATCH/SCRATCH+4)
// 2 = __print_i32_ln (helper: prints i32 as decimal + newline)
// 3 = __print_bool_ln (helper: prints bool as "true\n" or "false\n")
// 4+ = user functions
const FN_FD_WRITE: u32 = 0;
const FN_I32_TO_STR: u32 = 1;
const FN_PRINT_I32_LN: u32 = 2;
const FN_PRINT_BOOL_LN: u32 = 3;
const FN_USER_BASE: u32 = 4;

pub fn emit(mir: &MirModule, _sink: &mut DiagnosticSink) -> Vec<u8> {
    let mut ctx = EmitCtx {
        string_literals: Vec::new(),
        data_offset: DATA_START,
        fn_names: mir.functions.iter().map(|f| f.name.clone()).collect(),
        loop_depth: None,
    };
    ctx.emit_module(mir)
}

struct EmitCtx {
    string_literals: Vec<(u32, Vec<u8>)>,
    data_offset: u32,
    fn_names: Vec<String>,
    /// Nesting depth inside a loop body (for break/continue branch depths).
    /// None means we're not inside a loop.
    loop_depth: Option<u32>,
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

    fn resolve_fn(&mut self, name: &str) -> Option<u32> {
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
        functions.function(3); // __i32_to_string: (i32)->()  [stores result in memory]
        functions.function(3); // __print_i32_ln: (i32)->()
        functions.function(3); // __print_bool_ln: (i32)->()
        for func in &mir.functions {
            functions.function(self.func_type_idx(func));
        }
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
        for (idx, func) in mir.functions.iter().enumerate() {
            if func.name == "main" {
                exports.export("_start", ExportKind::Func, FN_USER_BASE + idx as u32);
            }
        }
        module.section(&exports);

        // Code section
        let mut code = CodeSection::new();
        code.function(&self.build_i32_to_string());
        code.function(&self.build_print_i32_ln());
        code.function(&self.build_print_bool_ln());
        for func in &mir.functions {
            let f = self.build_user_fn(func);
            code.function(&f);
        }
        module.section(&code);

        // Data section
        let mut data = DataSection::new();
        // Pre-allocate a newline string
        let (nl_off, _) = self.alloc_string("\n");
        // Store newline offset for print_i32_ln (it'll be at a known position)
        // Actually we need to know it at code-gen time. Let me just hardcode it.
        let _ = nl_off; // We'll reference DATA_START-area for newline

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
        // iov[0].base = mem[SCRATCH], iov[0].len = mem[SCRATCH+4]
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

        // Print newline — use a static "\n" in memory
        // We know "\n" is at self.data_offset area, but we need to emit it.
        // Simpler: just write byte 10 to a known location and print it.
        // Write '\n' at SCRATCH+8
        let nl_addr = SCRATCH + 8;
        let ma0 = MemArg { offset: 0, align: 0, memory_index: 0 };
        f.instruction(&Instruction::I32Const(nl_addr as i32));
        f.instruction(&Instruction::I32Const(10)); // '\n'
        f.instruction(&Instruction::I32Store8(ma0));

        f.instruction(&Instruction::I32Const(IOV_BASE as i32));
        f.instruction(&Instruction::I32Const(nl_addr as i32));
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

        f.instruction(&Instruction::End);
        f
    }

    /// __print_bool_ln(value: i32) -> void
    /// Prints "true\n" or "false\n" to stdout
    fn build_print_bool_ln(&mut self) -> Function {
        let ma2 = MemArg { offset: 0, align: 2, memory_index: 0 };
        let mut f = Function::new(vec![]);

        // Pre-allocate "true\n" and "false\n" in data section
        let (true_off, true_len) = self.alloc_string("true\n");
        let (false_off, false_len) = self.alloc_string("false\n");

        f.instruction(&Instruction::LocalGet(0));
        f.instruction(&Instruction::If(wasm_encoder::BlockType::Empty));
        {
            // print "true\n"
            f.instruction(&Instruction::I32Const(IOV_BASE as i32));
            f.instruction(&Instruction::I32Const(true_off as i32));
            f.instruction(&Instruction::I32Store(ma2));
            f.instruction(&Instruction::I32Const((IOV_BASE + 4) as i32));
            f.instruction(&Instruction::I32Const(true_len as i32));
            f.instruction(&Instruction::I32Store(ma2));
        }
        f.instruction(&Instruction::Else);
        {
            // print "false\n"
            f.instruction(&Instruction::I32Const(IOV_BASE as i32));
            f.instruction(&Instruction::I32Const(false_off as i32));
            f.instruction(&Instruction::I32Store(ma2));
            f.instruction(&Instruction::I32Const((IOV_BASE + 4) as i32));
            f.instruction(&Instruction::I32Const(false_len as i32));
            f.instruction(&Instruction::I32Store(ma2));
        }
        f.instruction(&Instruction::End);

        // fd_write
        f.instruction(&Instruction::I32Const(1));
        f.instruction(&Instruction::I32Const(IOV_BASE as i32));
        f.instruction(&Instruction::I32Const(1));
        f.instruction(&Instruction::I32Const(NWRITTEN as i32));
        f.instruction(&Instruction::Call(FN_FD_WRITE));
        f.instruction(&Instruction::Drop);

        f.instruction(&Instruction::End);
        f
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
                        // TODO: stderr
                        self.emit_println(f, args);
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
                // If inside a loop, increment depth for nested break/continue
                if let Some(d) = &mut self.loop_depth { *d += 1; }
                for s in then_body {
                    self.emit_stmt(f, s);
                }
                if !else_body.is_empty() {
                    f.instruction(&Instruction::Else);
                    for s in else_body {
                        self.emit_stmt(f, s);
                    }
                }
                if let Some(d) = &mut self.loop_depth { *d -= 1; }
                f.instruction(&Instruction::End);
            }
            MirStmt::WhileStmt { cond, body } => {
                let prev_loop_depth = self.loop_depth;
                self.loop_depth = Some(0);

                f.instruction(&Instruction::Block(wasm_encoder::BlockType::Empty));
                f.instruction(&Instruction::Loop(wasm_encoder::BlockType::Empty));
                // Evaluate condition; branch out if false
                self.emit_operand(f, cond);
                f.instruction(&Instruction::I32Eqz);
                f.instruction(&Instruction::BrIf(1));
                for s in body {
                    self.emit_stmt(f, s);
                }
                f.instruction(&Instruction::Br(0));
                f.instruction(&Instruction::End); // loop
                f.instruction(&Instruction::End); // block

                self.loop_depth = prev_loop_depth;
            }
            MirStmt::Break => {
                // br to the enclosing block (break target)
                // Inside loop: block > loop > [depth] levels of if/else > here
                // break target = loop_depth + 1 (skip to block end)
                let depth = self.loop_depth.unwrap_or(0);
                f.instruction(&Instruction::Br(depth + 1));
            }
            MirStmt::Continue => {
                // br to the enclosing loop (continue target)
                // continue target = loop_depth (skip to loop start)
                let depth = self.loop_depth.unwrap_or(0);
                f.instruction(&Instruction::Br(depth));
            }
            MirStmt::Return(op) => {
                if let Some(operand) = op {
                    self.emit_operand(f, operand);
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
                Operand::Call(name, inner_args) if name == "i32_to_string" => {
                    if let Some(inner) = inner_args.first() {
                        self.emit_operand(f, inner);
                        f.instruction(&Instruction::Call(FN_PRINT_I32_LN));
                    }
                }
                Operand::Call(name, inner_args) if name == "bool_to_string" => {
                    if let Some(inner) = inner_args.first() {
                        self.emit_operand(f, inner);
                        f.instruction(&Instruction::Call(FN_PRINT_BOOL_LN));
                    }
                }
                _ => {
                    // Try printing as i32
                    self.emit_operand(f, arg);
                    f.instruction(&Instruction::Call(FN_PRINT_I32_LN));
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
            Operand::ConstString(_) => { f.instruction(&Instruction::I32Const(0)); } // strings as values not yet supported
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
                for a in args {
                    self.emit_operand(f, a);
                }
                match name.as_str() {
                    "i32_to_string" | "bool_to_string" | "String_from" => {
                        // These return strings — not useful as i32 value operands,
                        // but the println handler recognizes them by pattern.
                        if name == "i32_to_string" {
                            f.instruction(&Instruction::Call(FN_I32_TO_STR));
                        }
                        f.instruction(&Instruction::I32Const(0)); // placeholder
                    }
                    other => {
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
                if let Some(d) = &mut self.loop_depth { *d += 1; }
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
                if let Some(d) = &mut self.loop_depth { *d -= 1; }
                f.instruction(&Instruction::End);
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
