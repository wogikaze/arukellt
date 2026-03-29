use super::OptimizationSummary;
use crate::mir::{
    AggregateKind, GcHintKind, LocalId, MirFunction, MirStmt, Operand, Place, Rvalue,
};
use std::collections::HashSet;

/// Detect short-lived struct allocations inside loops and annotate them with
/// `GcHint::ShortLived` so downstream GC passes can optimise their lifetime.
/// Returns `true` if any hints were added.
pub fn gc_hint_pass(func: &mut MirFunction) -> bool {
    let summary = gc_hint_pass_inner(func);
    summary.gc_hinted > 0
}

pub(crate) fn gc_hint_pass_inner(func: &mut MirFunction) -> OptimizationSummary {
    let mut summary = OptimizationSummary::default();
    for block in &mut func.blocks {
        process_stmts(&mut block.stmts, &mut summary);
    }
    summary
}

/// Walk statements recursively, looking for `WhileStmt` bodies to annotate.
fn process_stmts(stmts: &mut [MirStmt], summary: &mut OptimizationSummary) {
    for stmt in stmts.iter_mut() {
        match stmt {
            MirStmt::WhileStmt { body, .. } => {
                // Recurse into nested control flow first
                process_stmts(body, summary);
                annotate_short_lived(body, summary);
            }
            MirStmt::IfStmt {
                then_body,
                else_body,
                ..
            } => {
                process_stmts(then_body, summary);
                process_stmts(else_body, summary);
            }
            _ => {}
        }
    }
}

/// For each struct allocation in `body`, check whether the local is short-lived
/// (used only within this body and never escaping) and insert a GcHint after it.
fn annotate_short_lived(body: &mut Vec<MirStmt>, summary: &mut OptimizationSummary) {
    let mut escaping = HashSet::new();
    for stmt in body.iter() {
        collect_escaping(stmt, &mut escaping);
    }

    // Collect (index, local) pairs for struct allocations whose local doesn't escape.
    let mut hints: Vec<(usize, LocalId)> = Vec::new();
    for (i, stmt) in body.iter().enumerate() {
        if let Some(local) = struct_alloc_local(stmt) {
            if !escaping.contains(&local.0) {
                hints.push((i, local));
            }
        }
    }

    // Insert hints in reverse order to preserve indices.
    for (idx, local) in hints.into_iter().rev() {
        body.insert(
            idx + 1,
            MirStmt::GcHint {
                local,
                hint: GcHintKind::ShortLived,
            },
        );
        summary.gc_hinted += 1;
    }
}

/// If `stmt` is a struct allocation assigned to a plain local, return that local.
fn struct_alloc_local(stmt: &MirStmt) -> Option<LocalId> {
    match stmt {
        MirStmt::Assign(Place::Local(id), Rvalue::Use(Operand::StructInit { .. })) => Some(*id),
        MirStmt::Assign(Place::Local(id), Rvalue::Aggregate(AggregateKind::Struct(_), _)) => {
            Some(*id)
        }
        _ => None,
    }
}

/// Collect locals that escape via calls, returns, or stores into struct fields.
fn collect_escaping(stmt: &MirStmt, escaping: &mut HashSet<u32>) {
    match stmt {
        MirStmt::Call { args, .. } | MirStmt::CallBuiltin { args, .. } => {
            for arg in args {
                collect_escaping_operand(arg, escaping);
            }
        }
        MirStmt::Return(Some(op)) => {
            collect_escaping_operand(op, escaping);
        }
        MirStmt::Assign(Place::Field(..), Rvalue::Use(op)) => {
            collect_escaping_operand(op, escaping);
        }
        MirStmt::IfStmt {
            then_body,
            else_body,
            ..
        } => {
            for s in then_body {
                collect_escaping(s, escaping);
            }
            for s in else_body {
                collect_escaping(s, escaping);
            }
        }
        MirStmt::WhileStmt { body, .. } => {
            for s in body {
                collect_escaping(s, escaping);
            }
        }
        _ => {}
    }
}

fn collect_escaping_operand(op: &Operand, escaping: &mut HashSet<u32>) {
    if let Operand::Place(place) = op {
        collect_escaping_place(place, escaping)
    }
}

fn collect_escaping_place(place: &Place, escaping: &mut HashSet<u32>) {
    match place {
        Place::Local(id) => {
            escaping.insert(id.0);
        }
        Place::Field(inner, _) | Place::Index(inner, _) => collect_escaping_place(inner, escaping),
    }
}
