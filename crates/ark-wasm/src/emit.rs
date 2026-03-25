//! Wasm module emission using wasm-encoder.
//!
//! Generates a WASI Preview 1 compatible Wasm module from MIR.
//! Uses linear memory with `fd_write` for I/O.

use ark_diagnostics::DiagnosticSink;
use ark_mir::*;
use wasm_encoder::{
    CodeSection, DataSection, ExportKind, ExportSection, Function,
    FunctionSection, ImportSection, Instruction, MemArg, MemorySection, MemoryType,
    Module, TypeSection, ValType,
};

/// Generates a Wasm binary from MIR.
pub struct WasmEmitter {
    /// String literals collected during emission, stored in data section.
    string_literals: Vec<StringLiteral>,
    /// Current offset in linear memory for data allocation.
    data_offset: u32,
}

struct StringLiteral {
    offset: u32,
    bytes: Vec<u8>,
}

/// Memory layout:
/// 0..IOV_AREA: reserved for iov structs and nwritten
/// IOV_AREA..: string data
const IOV_AREA: u32 = 256;

impl WasmEmitter {
    pub fn new() -> Self {
        Self {
            string_literals: Vec::new(),
            data_offset: IOV_AREA,
        }
    }

    /// Allocate a string literal in the data section, returning (offset, len).
    fn alloc_string(&mut self, s: &str) -> (u32, u32) {
        let bytes = s.as_bytes().to_vec();
        let len = bytes.len() as u32;
        let offset = self.data_offset;
        self.string_literals.push(StringLiteral { offset, bytes });
        self.data_offset += len;
        (offset, len)
    }

    /// Emit a Wasm binary from a MIR module.
    pub fn emit(&mut self, mir: &MirModule, _sink: &mut DiagnosticSink) -> Vec<u8> {
        let mut module = Module::new();

        // === Type section ===
        // type 0: fd_write signature (i32, i32, i32, i32) -> i32
        // type 1: _start signature () -> ()
        let mut types = TypeSection::new();
        types.ty().function(
            vec![ValType::I32, ValType::I32, ValType::I32, ValType::I32],
            vec![ValType::I32],
        );
        types.ty().function(vec![], vec![]);
        module.section(&types);

        // === Import section ===
        // import 0: wasi_snapshot_preview1.fd_write
        let mut imports = ImportSection::new();
        imports.import(
            "wasi_snapshot_preview1",
            "fd_write",
            wasm_encoder::EntityType::Function(0), // type index 0
        );
        module.section(&imports);

        // === Function section ===
        // function 0 is the import (fd_write)
        // function 1+ are our functions
        let mut functions = FunctionSection::new();
        for _ in &mir.functions {
            functions.function(1); // all user functions are type 1: () -> ()
        }
        module.section(&functions);

        // === Memory section ===
        let mut memory = MemorySection::new();
        memory.memory(MemoryType {
            minimum: 1,
            maximum: None,
            memory64: false,
            shared: false,
            page_size_log2: None,
        });
        module.section(&memory);

        // === Export section ===
        let mut exports = ExportSection::new();
        exports.export("memory", ExportKind::Memory, 0);
        for (idx, func) in mir.functions.iter().enumerate() {
            if func.name == "main" {
                // WASI entry point is "_start"; func index = idx + 1 (offset by import)
                exports.export("_start", ExportKind::Func, (idx as u32) + 1);
            }
        }
        module.section(&exports);

        // === Code section ===
        // Pre-scan all functions to collect string literals and build instruction plans
        let plans: Vec<Vec<WasmInstr>> = mir.functions.iter()
            .map(|func| self.plan_function(func))
            .collect();

        let mut code = CodeSection::new();
        for plan in &plans {
            let mut f = Function::new(vec![]);
            for instr in plan {
                self.emit_instr(&mut f, instr);
            }
            f.instruction(&Instruction::End);
            code.function(&f);
        }
        module.section(&code);

        // === Data section ===
        let mut data = DataSection::new();
        for lit in &self.string_literals {
            let offset_expr = wasm_encoder::ConstExpr::i32_const(lit.offset as i32);
            data.active(0, &offset_expr, lit.bytes.iter().copied());
        }
        module.section(&data);

        module.finish()
    }

    /// Plan the instructions for a function (collecting string literals along the way).
    fn plan_function(&mut self, func: &MirFunction) -> Vec<WasmInstr> {
        let mut instrs = Vec::new();
        for block in &func.blocks {
            for stmt in &block.stmts {
                self.plan_stmt(stmt, &mut instrs);
            }
            // Terminator: for Return(None), we just fall through to End
        }
        instrs
    }

    fn plan_stmt(&mut self, stmt: &MirStmt, instrs: &mut Vec<WasmInstr>) {
        match stmt {
            MirStmt::CallBuiltin { name, args, .. } => {
                match name.as_str() {
                    "println" => {
                        if let Some(Operand::ConstString(s)) = args.first() {
                            let msg = format!("{}\n", s);
                            let (str_offset, str_len) = self.alloc_string(&msg);
                            instrs.push(WasmInstr::WriteFd {
                                fd: 1, // stdout
                                str_offset,
                                str_len,
                            });
                        }
                    }
                    "print" => {
                        if let Some(Operand::ConstString(s)) = args.first() {
                            let (str_offset, str_len) = self.alloc_string(s);
                            instrs.push(WasmInstr::WriteFd {
                                fd: 1,
                                str_offset,
                                str_len,
                            });
                        }
                    }
                    "eprintln" => {
                        if let Some(Operand::ConstString(s)) = args.first() {
                            let msg = format!("{}\n", s);
                            let (str_offset, str_len) = self.alloc_string(&msg);
                            instrs.push(WasmInstr::WriteFd {
                                fd: 2, // stderr
                                str_offset,
                                str_len,
                            });
                        }
                    }
                    _ => {} // TODO: other builtins
                }
            }
            _ => {} // TODO: other MIR stmts
        }
    }

    fn emit_instr(&self, f: &mut Function, instr: &WasmInstr) {
        match instr {
            WasmInstr::WriteFd { fd, str_offset, str_len } => {
                // Build iov struct at offset 0:
                //   iov_base (i32) at byte 0 = str_offset
                //   iov_len  (i32) at byte 4 = str_len
                let memarg = MemArg { offset: 0, align: 2, memory_index: 0 };

                // Store iov_base at address 0
                f.instruction(&Instruction::I32Const(0));
                f.instruction(&Instruction::I32Const(*str_offset as i32));
                f.instruction(&Instruction::I32Store(memarg));

                // Store iov_len at address 4
                f.instruction(&Instruction::I32Const(4));
                f.instruction(&Instruction::I32Const(*str_len as i32));
                f.instruction(&Instruction::I32Store(memarg));

                // Call fd_write(fd, iovs=0, iovs_len=1, nwritten=8)
                f.instruction(&Instruction::I32Const(*fd as i32)); // fd
                f.instruction(&Instruction::I32Const(0));           // iovs pointer
                f.instruction(&Instruction::I32Const(1));           // iovs count
                f.instruction(&Instruction::I32Const(8));           // nwritten pointer
                f.instruction(&Instruction::Call(0));               // fd_write is import index 0
                f.instruction(&Instruction::Drop);                  // discard return value
            }
        }
    }
}

/// Internal instruction representation for planning.
enum WasmInstr {
    WriteFd {
        fd: u32,
        str_offset: u32,
        str_len: u32,
    },
}

impl Default for WasmEmitter {
    fn default() -> Self {
        Self::new()
    }
}
