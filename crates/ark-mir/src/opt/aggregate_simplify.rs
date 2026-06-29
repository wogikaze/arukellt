use crate::mir::{MirFunction, MirStmt, Rvalue};
use super::OptimizationSummary;

pub(crate) fn aggregate_simplify(function: &mut MirFunction) -> OptimizationSummary {
    let mut summary = OptimizationSummary::default();
    for block in &mut function.blocks {
        for stmt in &mut block.stmts {
            if let MirStmt::Assign(place, Rvalue::Aggregate(_, operands)) = stmt {
                if operands.len() == 1 {
                    let place = place.clone();
                    let operand = operands[0].clone();
                    *stmt = MirStmt::Assign(place, Rvalue::Use(operand));
                    summary.aggregate_simplified += 1;
                }
            }
        }
    }
    summary
}
