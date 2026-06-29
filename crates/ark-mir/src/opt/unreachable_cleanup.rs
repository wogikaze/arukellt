use crate::mir::{MirFunction, MirStmt};
use super::OptimizationSummary;

pub(crate) fn unreachable_cleanup(function: &mut MirFunction) -> OptimizationSummary {
    let mut summary = OptimizationSummary::default();
    for block in &mut function.blocks {
        if let Some(index) = block
            .stmts
            .iter()
            .position(|stmt| matches!(stmt, MirStmt::Return(_)))
        {
            if index + 1 < block.stmts.len() {
                block.stmts.truncate(index + 1);
                summary.unreachable_cleaned += 1;
            }
        }
    }
    summary
}
