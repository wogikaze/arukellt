use crate::mir::{MirFunction, Operand, Terminator};
use super::OptimizationSummary;

pub(crate) fn branch_fold(function: &mut MirFunction) -> OptimizationSummary {
    let mut summary = OptimizationSummary::default();
    for block in &mut function.blocks {
        if let Terminator::If {
            cond: Operand::ConstBool(value),
            then_block,
            else_block,
        } = &block.terminator
        {
            block.terminator = Terminator::Goto(if *value { *then_block } else { *else_block });
            summary.branch_folded += 1;
        }
    }
    summary
}
