use crate::mir::{MirFunction, MirStmt, Place, Rvalue};
use super::OptimizationSummary;

const INLINE_SMALL_LEAF_BUDGET: usize = 8;

pub(crate) fn inline_small_leaf(function: &mut MirFunction) -> OptimizationSummary {
    let mut summary = OptimizationSummary::default();
    if function
        .blocks
        .iter()
        .map(|block| block.stmts.len())
        .sum::<usize>()
        > INLINE_SMALL_LEAF_BUDGET
    {
        return summary;
    }
    for block in &mut function.blocks {
        for stmt in &mut block.stmts {
            if let MirStmt::CallBuiltin { name, args, .. } = stmt {
                if name == "identity" && args.len() == 1 {
                    *stmt = MirStmt::Assign(
                        Place::Local(crate::mir::LocalId(0)),
                        Rvalue::Use(args[0].clone()),
                    );
                    summary.inline_small_leaf += 1;
                }
            }
        }
    }
    summary
}
