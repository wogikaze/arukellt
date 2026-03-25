//! Lower typed AST to MIR.
//!
//! This pass converts the high-level AST into a CFG-based MIR,
//! performing monomorphization along the way.

use ark_diagnostics::DiagnosticSink;
use ark_parser::ast;
use ark_typecheck::TypeChecker;

use crate::mir::*;

/// Lower a type-checked module to MIR.
pub fn lower_to_mir(
    module: &ast::Module,
    _checker: &TypeChecker,
    _sink: &mut DiagnosticSink,
) -> MirModule {
    let mut mir = MirModule::new();
    let mut next_fn_id = 0u32;

    for item in &module.items {
        if let ast::Item::FnDef(f) = item {
            let fn_id = FnId(next_fn_id);
            next_fn_id += 1;

            let entry = BlockId(0);
            let mir_fn = MirFunction {
                id: fn_id,
                name: f.name.clone(),
                params: Vec::new(),
                return_ty: ark_typecheck::types::Type::Unit,
                locals: Vec::new(),
                blocks: vec![BasicBlock {
                    id: entry,
                    stmts: Vec::new(),
                    terminator: Terminator::Return(None),
                }],
                entry,
            };

            if f.name == "main" {
                mir.entry_fn = Some(fn_id);
            }

            mir.functions.push(mir_fn);
        }
    }

    mir
}
