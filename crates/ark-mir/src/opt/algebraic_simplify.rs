use crate::mir::{BinOp, MirFunction, MirStmt, Operand, Rvalue, UnaryOp};
use super::OptimizationSummary;

/// Algebraic simplification: identity/absorbing element elimination.
///
/// Rules applied:
///   x + 0 → x,  0 + x → x
///   x - 0 → x
///   x * 1 → x,  1 * x → x
///   x * 0 → 0,  0 * x → 0
///   x / 1 → x
///   x & 0 → 0,  0 & x → 0
///   x | 0 → x,  0 | x → x
///   x ^ 0 → x,  0 ^ x → x
///   x && true → x, true && x → x
///   x && false → false
///   x || false → x, false || x → x
///   x || true → true
///   !!x → x (double negation)
///   --x → x (double negation for integers)
pub(crate) fn algebraic_simplify(function: &mut MirFunction) -> OptimizationSummary {
    let mut summary = OptimizationSummary::default();
    for block in &mut function.blocks {
        for stmt in &mut block.stmts {
            if let MirStmt::Assign(place, rvalue) = stmt {
                if let Some(simplified) = try_algebraic_simplify_rvalue(rvalue) {
                    *stmt = MirStmt::Assign(place.clone(), Rvalue::Use(simplified));
                    summary.algebraic_simplified += 1;
                }
            }
        }
    }
    summary
}

fn try_algebraic_simplify_rvalue(rvalue: &Rvalue) -> Option<Operand> {
    match rvalue {
        Rvalue::BinaryOp(op, lhs, rhs) => try_algebraic_simplify_binop(*op, lhs, rhs),
        Rvalue::UnaryOp(UnaryOp::Not, Operand::UnaryOp(UnaryOp::Not, inner)) => {
            Some((**inner).clone())
        }
        Rvalue::UnaryOp(UnaryOp::Neg, Operand::UnaryOp(UnaryOp::Neg, inner)) => {
            Some((**inner).clone())
        }
        _ => None,
    }
}

fn try_algebraic_simplify_binop(op: BinOp, lhs: &Operand, rhs: &Operand) -> Option<Operand> {
    let is_zero_i32 = |o: &Operand| matches!(o, Operand::ConstI32(0));
    let is_zero_i64 = |o: &Operand| matches!(o, Operand::ConstI64(0));
    let is_one_i32 = |o: &Operand| matches!(o, Operand::ConstI32(1));
    let is_one_i64 = |o: &Operand| matches!(o, Operand::ConstI64(1));

    match op {
        // x + 0 → x, 0 + x → x
        BinOp::Add => {
            if is_zero_i32(rhs) || is_zero_i64(rhs) {
                Some(lhs.clone())
            } else if is_zero_i32(lhs) || is_zero_i64(lhs) {
                Some(rhs.clone())
            } else {
                None
            }
        }
        // x - 0 → x
        BinOp::Sub => {
            if is_zero_i32(rhs) || is_zero_i64(rhs) {
                Some(lhs.clone())
            } else {
                None
            }
        }
        // x * 1 → x, 1 * x → x, x * 0 → 0, 0 * x → 0
        BinOp::Mul => {
            if is_one_i32(rhs) || is_one_i64(rhs) {
                Some(lhs.clone())
            } else if is_one_i32(lhs) || is_one_i64(lhs) {
                Some(rhs.clone())
            } else if is_zero_i32(rhs) || is_zero_i32(lhs) {
                Some(Operand::ConstI32(0))
            } else if is_zero_i64(rhs) || is_zero_i64(lhs) {
                Some(Operand::ConstI64(0))
            } else {
                None
            }
        }
        // x / 1 → x
        BinOp::Div => {
            if is_one_i32(rhs) || is_one_i64(rhs) {
                Some(lhs.clone())
            } else {
                None
            }
        }
        // x & 0 → 0, 0 & x → 0, x | 0 → x, 0 | x → x
        BinOp::BitAnd => {
            if is_zero_i32(rhs) || is_zero_i32(lhs) {
                Some(Operand::ConstI32(0))
            } else if is_zero_i64(rhs) || is_zero_i64(lhs) {
                Some(Operand::ConstI64(0))
            } else {
                None
            }
        }
        BinOp::BitOr | BinOp::BitXor => {
            if is_zero_i32(rhs) || is_zero_i64(rhs) {
                Some(lhs.clone())
            } else if is_zero_i32(lhs) || is_zero_i64(lhs) {
                Some(rhs.clone())
            } else {
                None
            }
        }
        // x && true → x, true && x → x, x && false → false
        BinOp::And => {
            if matches!(rhs, Operand::ConstBool(true)) {
                Some(lhs.clone())
            } else if matches!(lhs, Operand::ConstBool(true)) {
                Some(rhs.clone())
            } else if matches!(rhs, Operand::ConstBool(false))
                || matches!(lhs, Operand::ConstBool(false))
            {
                Some(Operand::ConstBool(false))
            } else {
                None
            }
        }
        // x || false → x, false || x → x, x || true → true
        BinOp::Or => {
            if matches!(rhs, Operand::ConstBool(false)) {
                Some(lhs.clone())
            } else if matches!(lhs, Operand::ConstBool(false)) {
                Some(rhs.clone())
            } else if matches!(rhs, Operand::ConstBool(true))
                || matches!(lhs, Operand::ConstBool(true))
            {
                Some(Operand::ConstBool(true))
            } else {
                None
            }
        }
        // x << 0 → x, x >> 0 → x
        BinOp::Shl | BinOp::Shr => {
            if is_zero_i32(rhs) || is_zero_i64(rhs) {
                Some(lhs.clone())
            } else {
                None
            }
        }
        _ => None,
    }
}
