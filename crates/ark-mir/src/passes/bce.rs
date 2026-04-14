//! Bounds-check elimination (BCE) MIR pass.
//!
//! Wraps the underlying `opt::bounds_check_elim` implementation and
//! exposes it through the standard pass interface used by the pipeline.
//!
//! **Minimum opt-level**: O2
//! **Depends on**: nothing (pairs well with LICM for max effect)
//! **Safe to run multiple times**: yes (idempotent)
//!
//! ## What is eliminated
//!
//! 1. `CallBuiltin` whose name contains `"bounds"` / `"bound_check"`:
//!    removed when the index is a compile-time constant smaller than the
//!    statically-known array length.
//! 2. Duplicate bounds-check calls on the same (array, index) pair inside
//!    the same basic block.
//! 3. Bounds checks inside `while i < len(arr)` loops where the induction
//!    variable is incremented by 1 — the guard already ensures safety.

use crate::mir::MirModule;
use crate::opt::{OptimizationPass, run_single_pass};
use crate::opt_level::OptLevel;
use super::PassStats;

/// Minimum optimization level required to run this pass.
pub const MIN_LEVEL: OptLevel = OptLevel::O2;

/// Run the bounds-check-elimination pass over every function in `module`.
///
/// Returns immediately (no-op) when `level < MIN_LEVEL`.
pub fn run(module: &mut MirModule, level: OptLevel) -> PassStats {
    if !level.at_least(MIN_LEVEL) {
        return PassStats::default();
    }
    let summary = run_single_pass(module, OptimizationPass::BoundsCheckElim)
        .unwrap_or_default();
    PassStats {
        name: "bounds_check_elim",
        changed: summary.bounds_checks_eliminated,
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

    fn module_with_stmts(stmts: Vec<MirStmt>) -> MirModule {
        let func = MirFunction {
            id: FnId(0),
            name: "test_bce".to_string(),
            instance: InstanceKey {
                item: "test_bce".to_string(),
                substitution: vec![],
                target_shape: String::new(),
            },
            params: vec![],
            return_ty: Type::Unit,
            locals: vec![
                MirLocal { id: LocalId(0), name: None, ty: Type::I32 },
                MirLocal { id: LocalId(1), name: None, ty: Type::I32 },
            ],
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
        };
        MirModule {
            functions: vec![func],
            entry_fn: None,
            type_table: TypeTable {
                struct_defs: HashMap::new(),
                enum_defs: HashMap::new(),
                fn_sigs: HashMap::new(),
            },
            struct_defs: HashMap::new(),
            enum_defs: HashMap::new(),
            imports: vec![],
            source_map: MirSourceMap {
                function_spans: HashMap::new(),
                block_spans: HashMap::new(),
                stmt_spans: HashMap::new(),
            },
            stats: MirStats::default(),
        }
    }

    /// The pass is a no-op at O1 (below MIN_LEVEL).
    #[test]
    fn pass_skipped_below_min_level() {
        let mut module = module_with_stmts(vec![MirStmt::CallBuiltin {
            dest: None,
            name: "bounds_check".to_string(),
            args: vec![
                Operand::Place(Place::Local(LocalId(0))),
                Operand::ConstI32(0),
            ],
        }]);
        let stats = run(&mut module, OptLevel::O1);
        assert_eq!(stats.changed, 0, "pass must be skipped below O2");
        assert_eq!(
            module.functions[0].blocks[0].stmts.len(),
            1,
            "no statements should be removed"
        );
    }

    /// Constant-index bounds check on a known-size array is eliminated at O2.
    #[test]
    fn constant_index_check_eliminated_at_o2() {
        let stmts = vec![
            // local_0 = [10, 20, 30]  (array of 3)
            MirStmt::Assign(
                Place::Local(LocalId(0)),
                Rvalue::Use(Operand::ArrayInit {
                    elements: vec![
                        Operand::ConstI32(10),
                        Operand::ConstI32(20),
                        Operand::ConstI32(30),
                    ],
                }),
            ),
            // bounds_check(local_0, 1)  -- index 1 < size 3, safe
            MirStmt::CallBuiltin {
                dest: None,
                name: "bounds_check".to_string(),
                args: vec![
                    Operand::Place(Place::Local(LocalId(0))),
                    Operand::ConstI32(1),
                ],
            },
        ];
        let mut module = module_with_stmts(stmts);
        let stats = run(&mut module, OptLevel::O2);
        assert_eq!(stats.changed, 1, "one bounds check should be eliminated");
        assert_eq!(
            module.functions[0].blocks[0].stmts.len(),
            1,
            "the bounds_check CallBuiltin should have been removed"
        );
    }

    /// Out-of-range constant index is NOT eliminated (safety preserved).
    #[test]
    fn out_of_range_check_not_eliminated() {
        let stmts = vec![
            MirStmt::Assign(
                Place::Local(LocalId(0)),
                Rvalue::Use(Operand::ArrayInit {
                    elements: vec![Operand::ConstI32(1)],
                }),
            ),
            // index 5 >= size 1 — must not eliminate
            MirStmt::CallBuiltin {
                dest: None,
                name: "bounds_check".to_string(),
                args: vec![
                    Operand::Place(Place::Local(LocalId(0))),
                    Operand::ConstI32(5),
                ],
            },
        ];
        let mut module = module_with_stmts(stmts);
        let stats = run(&mut module, OptLevel::O2);
        assert_eq!(stats.changed, 0);
        assert_eq!(module.functions[0].blocks[0].stmts.len(), 2);
    }

    /// `name` field counts eliminated checks correctly.
    #[test]
    fn pass_stats_name_is_bounds_check_elim() {
        let mut module = module_with_stmts(vec![]);
        let stats = run(&mut module, OptLevel::O2);
        assert_eq!(stats.name, "bounds_check_elim");
    }
}
