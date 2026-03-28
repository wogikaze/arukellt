use crate::mir::{MirFunction, MirStmt, Operand, Rvalue};
use super::OptimizationSummary;

pub(crate) fn string_concat_opt(function: &mut MirFunction) -> OptimizationSummary {
    let mut summary = OptimizationSummary::default();
    for block in &mut function.blocks {
        for stmt in &mut block.stmts {
            if let MirStmt::Assign(_, Rvalue::Use(Operand::Call(name, args))) = stmt {
                if name == "concat" && args.len() == 2 {
                    summary.string_concat_normalized += 1;
                }
            }
        }
    }
    summary
}
