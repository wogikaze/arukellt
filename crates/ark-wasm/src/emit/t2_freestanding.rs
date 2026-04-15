//! T2 `wasm32-freestanding` backend scaffold.
//!
//! This intentionally emits a minimal, structurally valid core Wasm module:
//! one page of linear memory, an empty `_start` function, and no WASI imports.
//! It exists to prove the target plumbing and validation path before real T2
//! lowering is implemented.

use ark_diagnostics::DiagnosticSink;
use ark_mir::mir::MirModule;
use wasm_encoder::{
    CodeSection, ExportKind, ExportSection, Function, FunctionSection, Instruction, MemorySection,
    MemoryType, Module, TypeSection, ValType,
};

pub fn emit(_mir: &MirModule, _sink: &mut DiagnosticSink) -> Vec<u8> {
    let mut types = TypeSection::new();
    types
        .ty()
        .function(Vec::<ValType>::new(), Vec::<ValType>::new());

    let mut functions = FunctionSection::new();
    functions.function(0);

    let mut memories = MemorySection::new();
    memories.memory(MemoryType {
        minimum: 1,
        maximum: Some(1),
        memory64: false,
        shared: false,
        page_size_log2: None,
    });

    let mut exports = ExportSection::new();
    exports.export("memory", ExportKind::Memory, 0);
    exports.export("_start", ExportKind::Func, 0);

    let mut body = Function::new(Vec::<(u32, ValType)>::new());
    body.instruction(&Instruction::End);

    let mut code = CodeSection::new();
    code.function(&body);

    let mut module = Module::new();
    module
        .section(&types)
        .section(&functions)
        .section(&memories)
        .section(&exports)
        .section(&code);
    module.finish()
}
