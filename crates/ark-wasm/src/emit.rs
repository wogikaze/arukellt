//! Wasm module emission using wasm-encoder.

use ark_diagnostics::DiagnosticSink;
use ark_mir::*;
use wasm_encoder::{
    CodeSection, ExportKind, ExportSection, Function, FunctionSection,
    Instruction, Module, TypeSection,
};

/// Generates a Wasm binary from MIR.
pub struct WasmEmitter {
    module: Module,
}

impl WasmEmitter {
    pub fn new() -> Self {
        Self { module: Module::new() }
    }

    /// Emit a Wasm binary from a MIR module.
    pub fn emit(&mut self, mir: &MirModule, _sink: &mut DiagnosticSink) -> Vec<u8> {
        let mut types = TypeSection::new();
        let mut functions = FunctionSection::new();
        let mut exports = ExportSection::new();
        let mut code = CodeSection::new();

        // For each MIR function, generate Wasm
        for (idx, func) in mir.functions.iter().enumerate() {
            // Function type: () -> ()  (simplified for now)
            types.ty().function(vec![], vec![]);
            functions.function(idx as u32);

            let mut f = Function::new(vec![]);

            // Translate MIR blocks to Wasm instructions
            for block in &func.blocks {
                for stmt in &block.stmts {
                    self.emit_stmt(&mut f, stmt);
                }
                self.emit_terminator(&mut f, &block.terminator);
            }

            f.instruction(&Instruction::End);
            code.function(&f);

            // Export main
            if func.name == "main" {
                exports.export("_start", ExportKind::Func, idx as u32);
            }
        }

        self.module.section(&types);
        self.module.section(&functions);
        self.module.section(&exports);
        self.module.section(&code);

        self.module.clone().finish()
    }

    fn emit_stmt(&self, _func: &mut Function, _stmt: &MirStmt) {
        // TODO: translate MIR statements to Wasm instructions
    }

    fn emit_terminator(&self, _func: &mut Function, _term: &Terminator) {
        // TODO: translate terminators
    }
}

impl Default for WasmEmitter {
    fn default() -> Self {
        Self::new()
    }
}
