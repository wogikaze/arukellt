//! GC-hint annotation MIR pass.
//!
//! Detects short-lived struct allocations — objects that are allocated and last
//! used within the same basic block (or loop iteration body) without escaping —
//! and inserts a [`MirStmt::GcHint`] annotation immediately after each such
//! allocation.  Downstream backends (e.g. the selfhost emitter
//! `src/compiler/emitter.ark`) can use these hints to
//! reduce GC pressure when the runtime supports the optimisation; if the
//! runtime ignores the hint the observable execution result is identical.
//!
//! **Minimum opt-level**: O3
//! **Depends on**: nothing (pairs well with escape_analysis for better recall)
//! **Safe to run multiple times**: yes (idempotent — already-annotated locals
//!   are not annotated twice because `struct_alloc_local` only fires on
//!   assignment statements, not on `GcHint` statements)
//!
//! ## Pattern detected
//!
//! ```text
//! // Inside a WhileStmt body:
//! _3 = StructInit { ... }   // allocation
//! // ... uses of _3 that do not escape ...
//! ```
//!
//! becomes:
//!
//! ```text
//! _3 = StructInit { ... }
//! gc_hint _3 ShortLived     // inserted hint
//! // ... uses of _3 ...
//! ```

use super::PassStats;
use crate::mir::MirModule;
use crate::opt::{OptimizationPass, run_single_pass};
use crate::opt_level::OptLevel;

/// Minimum optimization level required to run this pass.
pub const MIN_LEVEL: OptLevel = OptLevel::O3;

/// Run the GC-hint annotation pass over every function in `module`.
///
/// Returns immediately (no-op) when `level < MIN_LEVEL`.
pub fn run(module: &mut MirModule, level: OptLevel) -> PassStats {
    if !level.at_least(MIN_LEVEL) {
        return PassStats::default();
    }
    let summary = run_single_pass(module, OptimizationPass::GcHint).unwrap_or_default();
    PassStats {
        name: "gc_hint",
        changed: summary.gc_hinted,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::mir::{
        AggregateKind, BasicBlock, BlockId, FnId, GcHintKind, InstanceKey, LocalId, MirFunction,
        MirLocal, MirModule, MirSourceMap, MirStats, MirStmt, Operand, Place, Rvalue, SourceInfo,
        Terminator, TypeTable,
    };
    use ark_typecheck::types::Type;
    use std::collections::HashMap;

    fn module_with_while_body(body: Vec<MirStmt>) -> MirModule {
        let func = MirFunction {
            id: FnId(0),
            name: "test_gc_hint".to_string(),
            instance: InstanceKey {
                item: "test_gc_hint".to_string(),
                substitution: vec![],
                target_shape: String::new(),
            },
            params: vec![],
            return_ty: Type::Unit,
            locals: vec![MirLocal {
                id: LocalId(0),
                name: None,
                ty: Type::I32,
            }],
            blocks: vec![BasicBlock {
                id: BlockId(0),
                stmts: vec![MirStmt::WhileStmt {
                    cond: Operand::ConstBool(true),
                    body,
                }],
                terminator: Terminator::Return(None),
                source: SourceInfo::default(),
            }],
            entry: BlockId(0),
            struct_typed_locals: HashMap::new(),
            enum_typed_locals: HashMap::new(),
            type_params: vec![],
            source: SourceInfo::default(),
            is_exported: false,
        };

        MirModule {
            functions: vec![func],
            entry_fn: Some(FnId(0)),
            type_table: TypeTable::default(),
            struct_defs: HashMap::new(),
            enum_defs: HashMap::new(),
            imports: vec![],
            source_map: MirSourceMap::default(),
            stats: MirStats::default(),
        }
    }

    /// A struct allocation that does not escape should receive a `GcHint`
    /// annotation immediately after the allocation statement.
    #[test]
    fn short_lived_struct_in_loop_gets_gc_hint() {
        // _0 = MyStruct { field0 }   (allocation, does not escape)
        let alloc = MirStmt::Assign(
            Place::Local(LocalId(0)),
            Rvalue::Aggregate(
                AggregateKind::Struct("MyStruct".to_string()),
                vec![Operand::ConstI32(42)],
            ),
        );
        let mut module = module_with_while_body(vec![alloc]);

        let stats = run(&mut module, OptLevel::O3);

        // Pass must report at least one change.
        assert!(stats.did_change(), "expected gc_hint pass to fire");
        assert_eq!(stats.name, "gc_hint");

        // The loop body should now contain the original alloc + the GcHint.
        let body = match &module.functions[0].blocks[0].stmts[0] {
            MirStmt::WhileStmt { body, .. } => body,
            other => panic!("expected WhileStmt, got {:?}", other),
        };
        assert_eq!(body.len(), 2, "expected alloc + gc_hint in body");
        assert!(
            matches!(
                &body[1],
                MirStmt::GcHint {
                    local: LocalId(0),
                    hint: GcHintKind::ShortLived
                }
            ),
            "expected GcHint(ShortLived) for local _0, got {:?}",
            &body[1],
        );
    }

    /// The pass must be a no-op below O3.
    #[test]
    fn pass_skipped_below_o3() {
        let alloc = MirStmt::Assign(
            Place::Local(LocalId(0)),
            Rvalue::Aggregate(AggregateKind::Struct("MyStruct".to_string()), vec![]),
        );
        let mut module = module_with_while_body(vec![alloc]);

        for level in [OptLevel::None, OptLevel::O1, OptLevel::O2] {
            let cloned = module.functions[0].blocks[0].stmts.clone();
            let stats = run(&mut module, level);
            assert!(!stats.did_change(), "pass should not fire at {:?}", level);
            // Restore for next iteration
            module.functions[0].blocks[0].stmts = cloned;
        }
    }

    /// A struct allocation that escapes via a Call argument must NOT be hinted.
    #[test]
    fn escaping_struct_not_hinted() {
        // _0 = MyStruct {}
        // call fn#1(_0)      <- _0 escapes
        let alloc = MirStmt::Assign(
            Place::Local(LocalId(0)),
            Rvalue::Aggregate(AggregateKind::Struct("MyStruct".to_string()), vec![]),
        );
        let call = MirStmt::Call {
            dest: None,
            func: FnId(1),
            args: vec![Operand::Place(Place::Local(LocalId(0)))],
        };
        let mut module = module_with_while_body(vec![alloc, call]);

        let stats = run(&mut module, OptLevel::O3);

        assert!(
            !stats.did_change(),
            "escaping allocation must not be hinted"
        );
    }
}
