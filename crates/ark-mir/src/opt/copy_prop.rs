use crate::mir::{MirFunction, MirStmt, Operand, Place, Rvalue};
use super::helpers::{collect_assigned_locals, rewrite_stmt_with_replacements, rewrite_terminator_with_replacements};
use super::OptimizationSummary;

pub(crate) fn copy_prop(function: &mut MirFunction) -> OptimizationSummary {
    let mut summary = OptimizationSummary::default();
    for block in &mut function.blocks {
        let mut replacements = std::collections::HashMap::new();
        for stmt in &mut block.stmts {
            if let MirStmt::Assign(
                Place::Local(dest),
                Rvalue::Use(Operand::Place(Place::Local(src))),
            ) = stmt
            {
                replacements.insert(dest.0, Operand::Place(Place::Local(*src)));
                summary.copy_propagated += 1;
                continue;
            }
            rewrite_stmt_with_replacements(stmt, &replacements);
            // After a while loop the modified variables can hold any value —
            // remove them from the known-copy map so subsequent statements
            // don't receive stale pre-loop aliases.
            if let MirStmt::WhileStmt { body, .. } = stmt {
                for id in collect_assigned_locals(body) {
                    replacements.remove(&id);
                }
            }
        }
        rewrite_terminator_with_replacements(&mut block.terminator, &replacements);
    }
    summary
}
