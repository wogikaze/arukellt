use crate::mir::{BinOp, MirFunction, MirStmt, Operand, Rvalue};
use super::OptimizationSummary;

/// Strength reduction: replace expensive operations with cheaper ones.
///
/// Rules:
///   x * 2^n → x << n  (for constant power-of-2 multiplier)
///   x / 2^n → x >> n  (for constant power-of-2 divisor, unsigned semantics)
pub(crate) fn strength_reduction(function: &mut MirFunction) -> OptimizationSummary {
    let mut summary = OptimizationSummary::default();
    for block in &mut function.blocks {
        for stmt in &mut block.stmts {
            if let MirStmt::Assign(place, Rvalue::BinaryOp(op, lhs, rhs)) = stmt {
                if let Some(replacement) = try_strength_reduce(*op, lhs, rhs) {
                    *stmt = MirStmt::Assign(place.clone(), replacement);
                    summary.strength_reduced += 1;
                }
            }
        }
    }
    summary
}

fn try_strength_reduce(op: BinOp, lhs: &Operand, rhs: &Operand) -> Option<Rvalue> {
    match op {
        BinOp::Mul => {
            // x * 2^n → x << n
            if let Some(shift) = power_of_two_i32(rhs) {
                Some(Rvalue::BinaryOp(
                    BinOp::Shl,
                    lhs.clone(),
                    Operand::ConstI32(shift),
                ))
            } else if let Some(shift) = power_of_two_i32(lhs) {
                Some(Rvalue::BinaryOp(
                    BinOp::Shl,
                    rhs.clone(),
                    Operand::ConstI32(shift),
                ))
            } else {
                None
            }
        }
        BinOp::Div => {
            // x / 2^n → x >> n (unsigned)
            if let Some(shift) = power_of_two_i32(rhs) {
                Some(Rvalue::BinaryOp(
                    BinOp::Shr,
                    lhs.clone(),
                    Operand::ConstI32(shift),
                ))
            } else {
                None
            }
        }
        _ => None,
    }
}

fn power_of_two_i32(op: &Operand) -> Option<i32> {
    match op {
        Operand::ConstI32(n) if *n > 1 && n.count_ones() == 1 => Some(n.trailing_zeros() as i32),
        _ => None,
    }
}
