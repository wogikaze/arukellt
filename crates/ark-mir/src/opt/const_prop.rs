use crate::mir::{MirFunction, MirStmt, Operand, Place, Rvalue};
use super::helpers::{rewrite_stmt_with_replacements, rewrite_terminator_with_replacements};
use super::OptimizationSummary;

pub(crate) fn const_prop(function: &mut MirFunction) -> OptimizationSummary {
    let mut summary = OptimizationSummary::default();
    for block in &mut function.blocks {
        let mut constants = std::collections::HashMap::new();
        for stmt in &mut block.stmts {
            if let MirStmt::Assign(Place::Local(dest), Rvalue::Use(value)) = stmt {
                if matches!(
                    value,
                    Operand::ConstI32(_) | Operand::ConstI64(_) | Operand::ConstBool(_)
                ) {
                    constants.insert(dest.0, value.clone());
                }
            }
            if rewrite_stmt_with_replacements(stmt, &constants) {
                summary.const_propagated += 1;
            }
        }
        if rewrite_terminator_with_replacements(&mut block.terminator, &constants) {
            summary.const_propagated += 1;
        }
    }
    summary
}
