//! Loop-Invariant Code Motion (LICM)
//!
//! Hoists pure, loop-invariant assignments out of `WhileStmt` bodies and places
//! them immediately before the loop (in a conceptual "pre-header").
//!
//! A statement `x = rvalue` is loop-invariant when:
//!   1. The rvalue is side-effect-free (BinaryOp, UnaryOp, or Use).
//!   2. None of the operands in the rvalue refer to a local that is *assigned*
//!      anywhere inside the loop body (direct or nested).
//!
//! This pass works on the tree-structured (non-CFG) MIR where while loops are
//! represented as `MirStmt::WhileStmt { cond, body }`.

use super::OptimizationSummary;
use crate::mir::{MirFunction, MirStmt, Operand, Place, Rvalue};
use std::collections::HashSet;

/// Run LICM over all blocks in `function`.
pub(crate) fn licm(function: &mut MirFunction) -> OptimizationSummary {
    let mut summary = OptimizationSummary::default();
    for block in &mut function.blocks {
        let mut new_stmts: Vec<MirStmt> = Vec::with_capacity(block.stmts.len());
        for stmt in block.stmts.drain(..) {
            match stmt {
                MirStmt::WhileStmt { cond, body } => {
                    let (hoisted, remaining) = extract_invariants(body);
                    summary.licm_hoisted += hoisted.len();
                    new_stmts.extend(hoisted);
                    new_stmts.push(MirStmt::WhileStmt {
                        cond,
                        body: remaining,
                    });
                }
                other => new_stmts.push(other),
            }
        }
        block.stmts = new_stmts;
    }
    summary
}

/// Returns `true` if `rvalue` is side-effect-free (pure: no calls, no I/O).
fn is_pure_rvalue(rvalue: &Rvalue) -> bool {
    matches!(
        rvalue,
        Rvalue::BinaryOp(_, _, _) | Rvalue::UnaryOp(_, _) | Rvalue::Use(_)
    )
}

/// Collect the set of `LocalId` values that are the *destination* of an
/// assignment anywhere in `stmts` (including nested loops/if bodies).
/// Also includes locals written by `Call`/`CallBuiltin` destinations.
fn collect_assigned_locals(stmts: &[MirStmt]) -> HashSet<u32> {
    let mut assigned = HashSet::new();
    collect_assigned_locals_impl(stmts, &mut assigned);
    assigned
}

fn collect_assigned_locals_impl(stmts: &[MirStmt], assigned: &mut HashSet<u32>) {
    for stmt in stmts {
        match stmt {
            MirStmt::Assign(Place::Local(id), _) => {
                assigned.insert(id.0);
            }
            MirStmt::Call {
                dest: Some(Place::Local(id)),
                ..
            }
            | MirStmt::CallBuiltin {
                dest: Some(Place::Local(id)),
                ..
            } => {
                assigned.insert(id.0);
            }
            MirStmt::WhileStmt { body, .. } => {
                collect_assigned_locals_impl(body, assigned);
            }
            MirStmt::IfStmt {
                then_body,
                else_body,
                ..
            } => {
                collect_assigned_locals_impl(then_body, assigned);
                collect_assigned_locals_impl(else_body, assigned);
            }
            _ => {}
        }
    }
}

/// Returns `true` if `operand` references any local in `locals`.
fn operand_uses_locals(op: &Operand, locals: &HashSet<u32>) -> bool {
    match op {
        Operand::Place(Place::Local(id)) => locals.contains(&id.0),
        Operand::Place(_) => false,
        // Constants / unit never reference locals
        Operand::ConstI32(_)
        | Operand::ConstI64(_)
        | Operand::ConstF32(_)
        | Operand::ConstF64(_)
        | Operand::ConstBool(_)
        | Operand::ConstChar(_)
        | Operand::ConstString(_)
        | Operand::ConstU8(_)
        | Operand::ConstU16(_)
        | Operand::ConstU32(_)
        | Operand::ConstU64(_)
        | Operand::ConstI8(_)
        | Operand::ConstI16(_)
        | Operand::Unit
        | Operand::FnRef(_) => false,
        // For complex operands, be conservative.
        _ => true,
    }
}

/// Returns `true` if `rvalue` transitively references any local in `locals`.
fn rvalue_uses_locals(rvalue: &Rvalue, locals: &HashSet<u32>) -> bool {
    match rvalue {
        Rvalue::BinaryOp(_, lhs, rhs) => {
            operand_uses_locals(lhs, locals) || operand_uses_locals(rhs, locals)
        }
        Rvalue::UnaryOp(_, op) | Rvalue::Use(op) => operand_uses_locals(op, locals),
        // Aggregate and Ref: conservative — assume they use loop variables.
        _ => true,
    }
}

/// Partition `body` into (hoisted, remaining).
///
/// A statement is hoisted when it is a simple `Place::Local = pure_rvalue`
/// whose rvalue does not depend on any locally-assigned variable.
fn extract_invariants(body: Vec<MirStmt>) -> (Vec<MirStmt>, Vec<MirStmt>) {
    let assigned_in_loop = collect_assigned_locals(&body);
    let mut hoisted = Vec::new();
    let mut remaining = Vec::new();

    for stmt in body {
        match &stmt {
            MirStmt::Assign(Place::Local(_), rvalue)
                if is_pure_rvalue(rvalue) && !rvalue_uses_locals(rvalue, &assigned_in_loop) =>
            {
                hoisted.push(stmt);
            }
            _ => remaining.push(stmt),
        }
    }
    (hoisted, remaining)
}
