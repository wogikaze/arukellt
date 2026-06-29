//! Loop-unrolling MIR pass.
//!
//! For small loops with a statically-known constant trip count (≤ 16),
//! duplicates the loop body inline to eliminate branch overhead.
//!
//! After unrolling, runs `const_fold` and `dead_block_elim` automatically so
//! that newly-constant induction variable values can be folded away
//! immediately.
//!
//! **Minimum opt-level**: O2
//! **Depends on**: nothing (pairs well with `licm`, `const_prop`)
//! **Safe to run multiple times**: yes (already-unrolled loops have no
//! back-edge and will not be identified as candidates again)
//!
//! ## Acceptance constraints (from issue #083)
//!
//! 1. Only unroll loops whose trip count can be statically determined and
//!    is ≤ 16.
//! 2. After unrolling, run `const_fold` → `dead_block_elim` automatically.
//! 3. If the post-unroll instruction count would exceed 8× the pre-unroll
//!    count, skip (code-size guard).
//! 4. Only active at `--opt-level 2` or higher.

use super::PassStats;
use crate::mir::MirModule;
use crate::opt::{OptimizationPass, run_single_pass};
use crate::opt_level::OptLevel;

/// Minimum optimization level required to run this pass.
pub const MIN_LEVEL: OptLevel = OptLevel::O2;

/// Maximum allowed expansion factor (post / pre instruction count).
/// If a single unroll cycle would take the module beyond `MAX_SIZE_RATIO`×
/// its current size, the unroll is reverted.
const MAX_SIZE_RATIO: usize = 8;

/// Count every statement across all basic blocks of all functions.
fn count_stmts(module: &MirModule) -> usize {
    module
        .functions
        .iter()
        .flat_map(|f| f.blocks.iter())
        .map(|b| b.stmts.len())
        .sum()
}

/// Run the loop-unrolling pass over every function in `module`.
///
/// Returns immediately (no-op) when `level < MIN_LEVEL`.
///
/// When the pass fires:
/// 1. Loops with a constant trip count that fits within the underlying
///    implementation's limit (≤ 16 iterations per the issue requirement,
///    conservatively ≤ 4 in the current `opt::loop_unroll` kernel) are
///    unrolled into straight-line blocks.
/// 2. If the resulting instruction count exceeds `MAX_SIZE_RATIO × before`,
///    the module is restored from a pre-unroll snapshot and 0 is reported.
/// 3. On a successful unroll, `const_fold` and `dead_block_elim` are run
///    automatically as follow-on cleanup passes.
pub fn run(module: &mut MirModule, level: OptLevel) -> PassStats {
    if !level.at_least(MIN_LEVEL) {
        return PassStats::default();
    }

    let stmts_before = count_stmts(module);

    // Take a snapshot so we can revert if the code-size guard fires.
    let snapshot = module.clone();

    let unroll_summary = run_single_pass(module, OptimizationPass::LoopUnroll).unwrap_or_default();

    if unroll_summary.loops_unrolled == 0 {
        return PassStats {
            name: "loop_unroll",
            changed: 0,
        };
    }

    // Code-size guard: revert if the module grew beyond 8× its prior size.
    let stmts_after = count_stmts(module);
    if stmts_before > 0 && stmts_after > MAX_SIZE_RATIO * stmts_before {
        *module = snapshot;
        return PassStats {
            name: "loop_unroll",
            changed: 0,
        };
    }

    // Follow-on passes: propagate constants that become obvious after
    // induction-variable unrolling, then remove the now-dead original body
    // blocks.
    let _ = run_single_pass(module, OptimizationPass::ConstFold);
    let _ = run_single_pass(module, OptimizationPass::DeadBlockElim);

    PassStats {
        name: "loop_unroll",
        changed: unroll_summary.loops_unrolled,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::mir::{
        BasicBlock, BinOp, BlockId, FnId, InstanceKey, LocalId, MirFunction, MirLocal, MirModule,
        MirSourceMap, MirStats, Operand, Place, Rvalue, SourceInfo, Terminator, TypeTable,
    };
    use ark_typecheck::types::Type;
    use std::collections::HashMap;

    // ── helpers ────────────────────────────────────────────────────────────

    fn make_module(blocks: Vec<BasicBlock>) -> MirModule {
        let entry = blocks.first().map(|b| b.id).unwrap_or(BlockId(0));
        let func = MirFunction {
            id: FnId(0),
            name: "test_lu".to_string(),
            instance: InstanceKey {
                item: "test_lu".to_string(),
                substitution: vec![],
                target_shape: String::new(),
            },
            params: vec![],
            return_ty: Type::Unit,
            locals: vec![MirLocal {
                id: LocalId(0),
                name: Some("i".to_string()),
                ty: Type::I32,
            }],
            blocks,
            entry,
            struct_typed_locals: HashMap::new(),
            enum_typed_locals: HashMap::new(),
            type_params: vec![],
            source: SourceInfo::default(),
            is_exported: false,
        };
        MirModule {
            functions: vec![func],
            entry_fn: Some(FnId(0)),
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

    /// Build a simple counting loop:
    ///   header (bb0):  i = 0;  if i < `bound` → bb1, bb2
    ///   body   (bb1):  i = i + 1;  goto bb0
    ///   exit   (bb2):  return
    fn counting_loop(bound: i32) -> MirModule {
        let header = BasicBlock {
            id: BlockId(0),
            stmts: vec![crate::mir::MirStmt::Assign(
                Place::Local(LocalId(0)),
                Rvalue::Use(Operand::ConstI32(0)),
            )],
            terminator: Terminator::If {
                cond: Operand::BinOp(
                    BinOp::Lt,
                    Box::new(Operand::Place(Place::Local(LocalId(0)))),
                    Box::new(Operand::ConstI32(bound)),
                ),
                then_block: BlockId(1),
                else_block: BlockId(2),
                hint: None,
            },
            source: SourceInfo::default(),
        };
        let body = BasicBlock {
            id: BlockId(1),
            stmts: vec![crate::mir::MirStmt::Assign(
                Place::Local(LocalId(0)),
                Rvalue::BinaryOp(
                    BinOp::Add,
                    Operand::Place(Place::Local(LocalId(0))),
                    Operand::ConstI32(1),
                ),
            )],
            terminator: Terminator::Goto(BlockId(0)),
            source: SourceInfo::default(),
        };
        let exit = BasicBlock {
            id: BlockId(2),
            stmts: vec![],
            terminator: Terminator::Return(None),
            source: SourceInfo::default(),
        };
        make_module(vec![header, body, exit])
    }

    // ── tests ──────────────────────────────────────────────────────────────

    /// Pass is a no-op below O2.
    #[test]
    fn pass_skipped_below_min_level() {
        let mut module = counting_loop(3);
        let stats = run(&mut module, OptLevel::O1);
        assert_eq!(stats.changed, 0, "pass must be skipped below O2");
        // Original structure intact: header If terminator still present.
        assert!(
            matches!(
                module.functions[0].blocks[0].terminator,
                Terminator::If { .. }
            ),
            "header terminator must not be rewritten at O1"
        );
    }

    /// A small constant loop (trip count = 3) is unrolled at O2
    /// and the pass reports the correct change count.
    #[test]
    fn pass_fires_on_small_constant_loop_at_o2() {
        let mut module = counting_loop(3);
        let stats = run(&mut module, OptLevel::O2);
        assert_eq!(stats.changed, 1, "one loop should be unrolled");
        assert_eq!(stats.name, "loop_unroll");
        // The header now unconditionally jumps to the first unrolled block.
        assert!(
            matches!(
                module.functions[0].blocks[0].terminator,
                Terminator::Goto(_)
            ),
            "header should now be a Goto after unrolling"
        );
        // After the follow-on dead_block_elim the unreachable original body
        // is cleaned up, so no block in the function should have an If
        // terminator that points back to block 0 (the back-edge is gone).
        let has_back_edge = module.functions[0]
            .blocks
            .iter()
            .any(|b| matches!(b.terminator, Terminator::Goto(id) if id == BlockId(0)));
        assert!(!has_back_edge, "back-edge must be gone after unrolling");
        // There should be at least one Goto chain among the new unrolled
        // blocks (trip_count = 3 produces 3 new blocks).
        let goto_count = module.functions[0]
            .blocks
            .iter()
            .filter(|b| matches!(b.terminator, Terminator::Goto(_)))
            .count();
        assert!(goto_count >= 2, "unrolled blocks should form a Goto chain");
    }

    /// A loop with no statically-known trip count is not unrolled.
    #[test]
    fn no_unroll_without_constant_bound() {
        // Build a loop where the bound is a local variable, not a constant.
        let header = BasicBlock {
            id: BlockId(0),
            stmts: vec![],
            terminator: Terminator::If {
                // i < _1  (non-constant bound)
                cond: Operand::BinOp(
                    BinOp::Lt,
                    Box::new(Operand::Place(Place::Local(LocalId(0)))),
                    Box::new(Operand::Place(Place::Local(LocalId(1)))),
                ),
                then_block: BlockId(1),
                else_block: BlockId(2),
                hint: None,
            },
            source: SourceInfo::default(),
        };
        let body = BasicBlock {
            id: BlockId(1),
            stmts: vec![crate::mir::MirStmt::Assign(
                Place::Local(LocalId(0)),
                Rvalue::BinaryOp(
                    BinOp::Add,
                    Operand::Place(Place::Local(LocalId(0))),
                    Operand::ConstI32(1),
                ),
            )],
            terminator: Terminator::Goto(BlockId(0)),
            source: SourceInfo::default(),
        };
        let exit = BasicBlock {
            id: BlockId(2),
            stmts: vec![],
            terminator: Terminator::Return(None),
            source: SourceInfo::default(),
        };
        let mut module = make_module(vec![header, body, exit]);
        // Add an extra local for the non-constant bound.
        module.functions[0].locals.push(MirLocal {
            id: LocalId(1),
            name: Some("n".to_string()),
            ty: Type::I32,
        });
        let stats = run(&mut module, OptLevel::O2);
        assert_eq!(stats.changed, 0, "non-constant bound must not be unrolled");
    }

    /// Pass is a no-op when there are no loops.
    #[test]
    fn no_loops_is_noop() {
        let block = BasicBlock {
            id: BlockId(0),
            stmts: vec![],
            terminator: Terminator::Return(None),
            source: SourceInfo::default(),
        };
        let mut module = make_module(vec![block]);
        let stats = run(&mut module, OptLevel::O2);
        assert_eq!(stats.changed, 0);
    }
}
