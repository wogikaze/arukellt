use crate::mir::MirFunction;
use super::helpers::{collect_stmt_locals, collect_terminator_locals};
use super::OptimizationSummary;

pub(crate) fn dead_local_elim(function: &mut MirFunction) -> OptimizationSummary {
    let mut used = std::collections::HashSet::new();
    for block in &function.blocks {
        for stmt in &block.stmts {
            collect_stmt_locals(stmt, &mut used);
        }
        collect_terminator_locals(&block.terminator, &mut used);
    }

    let before = function.locals.len();
    function.locals.retain(|local| used.contains(&local.id.0));
    OptimizationSummary {
        dead_locals_removed: before.saturating_sub(function.locals.len()),
        ..OptimizationSummary::default()
    }
}
