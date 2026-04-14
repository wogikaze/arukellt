//! Number type narrowing MIR pass.
//!
//! Detects `i64` locals whose values provably fit in i32 range and narrows
//! their declared type to `i32`, reducing memory footprint and matching Wasm
//! i32 instructions for improved WasmGC performance.
//!
//! **Minimum opt-level**: O2
//! **Depends on**: const_fold, const_prop (maximise constant visibility)
//! **Safe to run multiple times**: yes (idempotent)

use crate::mir::MirModule;
use crate::opt::{OptimizationPass, run_single_pass};
use crate::opt_level::OptLevel;
use super::PassStats;

/// Minimum optimization level required to run this pass.
pub const MIN_LEVEL: OptLevel = OptLevel::O2;

/// Run the type-narrowing pass over every function in `module`.
///
/// Returns immediately (no-op) when `level < MIN_LEVEL`.
pub fn run(module: &mut MirModule, level: OptLevel) -> PassStats {
    if !level.at_least(MIN_LEVEL) {
        return PassStats::default();
    }
    let summary = run_single_pass(module, OptimizationPass::TypeNarrowing)
        .unwrap_or_default();
    PassStats {
        name: "type_narrowing",
        changed: summary.types_narrowed,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::mir::{
        BasicBlock, BlockId, FnId, InstanceKey, LocalId, MirFunction, MirLocal, MirModule,
        MirSourceMap, MirStats, MirStmt, Operand, Place, Rvalue, SourceInfo, Terminator, TypeTable,
    };
    use ark_typecheck::types::Type;
    use std::collections::HashMap;

    fn module_with_fn(func: MirFunction) -> MirModule {
        MirModule {
            functions: vec![func],
            entry_fn: None,
            structs: vec![],
            enums: vec![],
            strings: vec![],
            source_map: MirSourceMap::default(),
            stats: MirStats::default(),
            type_table: TypeTable::default(),
        }
    }

    fn simple_func(stmts: Vec<MirStmt>, locals: Vec<MirLocal>) -> MirFunction {
        MirFunction {
            id: FnId(0),
            name: "test_narrowing".to_string(),
            instance: InstanceKey {
                item: "test_narrowing".to_string(),
                substitution: vec![],
                target_shape: String::new(),
            },
            params: vec![],
            return_ty: Type::Unit,
            locals,
            blocks: vec![BasicBlock {
                id: BlockId(0),
                stmts,
                terminator: Terminator::Return(None),
                source: SourceInfo::default(),
            }],
            entry: BlockId(0),
            struct_typed_locals: HashMap::new(),
            enum_typed_locals: HashMap::new(),
            type_params: vec![],
            source: SourceInfo::default(),
            is_exported: false,
        }
    }

    #[test]
    fn test_no_op_below_o2() {
        let locals = vec![MirLocal { id: LocalId(0), name: None, ty: Type::I64 }];
        let stmts = vec![MirStmt::Assign(
            Place::Local(LocalId(0)),
            Rvalue::Use(Operand::ConstI64(42)),
        )];
        let func = simple_func(stmts, locals);
        let mut module = module_with_fn(func);

        let stats = run(&mut module, OptLevel::O1);
        assert_eq!(stats.changed, 0, "should be no-op at O1");
        // Local should remain i64 since pass was skipped.
        assert_eq!(module.functions[0].locals[0].ty, Type::I64);
    }

    #[test]
    fn test_narrows_i32_range_constant() {
        // An i64 local assigned a value that fits in i32 should be narrowed.
        let locals = vec![MirLocal { id: LocalId(0), name: None, ty: Type::I64 }];
        let stmts = vec![MirStmt::Assign(
            Place::Local(LocalId(0)),
            Rvalue::Use(Operand::ConstI64(100)),
        )];
        let func = simple_func(stmts, locals);
        let mut module = module_with_fn(func);

        let stats = run(&mut module, OptLevel::O2);
        assert_eq!(stats.name, "type_narrowing");
        assert_eq!(stats.changed, 1, "should narrow one local");
        assert_eq!(module.functions[0].locals[0].ty, Type::I32);
    }

    #[test]
    fn test_does_not_narrow_out_of_range() {
        // An i64 local assigned a value outside i32 range must NOT be narrowed.
        let locals = vec![MirLocal { id: LocalId(0), name: None, ty: Type::I64 }];
        let large_val: i64 = (i32::MAX as i64) + 1;
        let stmts = vec![MirStmt::Assign(
            Place::Local(LocalId(0)),
            Rvalue::Use(Operand::ConstI64(large_val)),
        )];
        let func = simple_func(stmts, locals);
        let mut module = module_with_fn(func);

        let stats = run(&mut module, OptLevel::O2);
        assert_eq!(stats.changed, 0, "out-of-range constant must not be narrowed");
        assert_eq!(module.functions[0].locals[0].ty, Type::I64);
    }
}
