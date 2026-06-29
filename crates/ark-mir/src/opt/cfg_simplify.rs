use crate::mir::{MirFunction, Terminator};
use super::OptimizationSummary;

pub(crate) fn cfg_simplify(function: &mut MirFunction) -> OptimizationSummary {
    let mut summary = OptimizationSummary::default();
    for block in &mut function.blocks {
        if matches!(block.terminator, Terminator::Goto(_)) && block.stmts.is_empty() {
            summary.cfg_simplified += 1;
        }
    }
    summary
}
