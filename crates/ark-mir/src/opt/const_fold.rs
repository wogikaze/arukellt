use crate::mir::{BinOp, MirFunction, MirStmt, Operand, Place, Rvalue};
use super::OptimizationSummary;

pub(crate) fn const_fold(function: &mut MirFunction) -> OptimizationSummary {
    let mut summary = OptimizationSummary::default();
    for block in &mut function.blocks {
        for stmt in &mut block.stmts {
            if let MirStmt::Assign(_, Rvalue::BinaryOp(op, lhs, rhs)) = stmt {
                if let Some(folded) = fold_binary(*op, lhs, rhs) {
                    let place = extract_assign_target(stmt);
                    *stmt = MirStmt::Assign(place, Rvalue::Use(folded));
                    summary.const_folded += 1;
                }
            }
        }
    }
    summary
}

fn fold_binary(op: BinOp, lhs: &Operand, rhs: &Operand) -> Option<Operand> {
    match (op, lhs, rhs) {
        (BinOp::Add, Operand::ConstI32(a), Operand::ConstI32(b)) => Some(Operand::ConstI32(a + b)),
        (BinOp::Sub, Operand::ConstI32(a), Operand::ConstI32(b)) => Some(Operand::ConstI32(a - b)),
        (BinOp::Mul, Operand::ConstI32(a), Operand::ConstI32(b)) => Some(Operand::ConstI32(a * b)),
        (BinOp::Eq, Operand::ConstI32(a), Operand::ConstI32(b)) => Some(Operand::ConstBool(a == b)),
        (BinOp::Eq, Operand::ConstBool(a), Operand::ConstBool(b)) => {
            Some(Operand::ConstBool(a == b))
        }
        _ => None,
    }
}

fn extract_assign_target(stmt: &MirStmt) -> Place {
    match stmt {
        MirStmt::Assign(place, _) => place.clone(),
        _ => Place::Local(crate::mir::LocalId(0)),
    }
}
