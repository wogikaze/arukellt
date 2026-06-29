use crate::mir::{BinOp, LocalId, MirFunction, MirStmt, Operand, Place, Rvalue, UnaryOp};
use super::OptimizationSummary;

/// Common Subexpression Elimination: within each basic block, replace duplicate
/// pure BinaryOp/UnaryOp computations with a reference to the first result.
pub(crate) fn cse(function: &mut MirFunction) -> OptimizationSummary {
    use std::collections::HashMap;

    let mut summary = OptimizationSummary::default();

    for block in &mut function.blocks {
        // Map from (op_key) -> local that holds the result
        let mut seen: HashMap<CseKey, Place> = HashMap::new();

        for stmt in &mut block.stmts {
            match stmt {
                MirStmt::Assign(place, Rvalue::BinaryOp(op, lhs, rhs)) => {
                    if is_pure_binop(*op) {
                        let key = CseKey::Binary(*op, operand_key(lhs), operand_key(rhs));
                        if let Some(prev_place) = seen.get(&key) {
                            *stmt = MirStmt::Assign(
                                place.clone(),
                                Rvalue::Use(Operand::Place(prev_place.clone())),
                            );
                            summary.cse_eliminated += 1;
                        } else {
                            seen.insert(key, place.clone());
                        }
                    }
                }
                MirStmt::Assign(place, Rvalue::UnaryOp(op, operand)) => {
                    let key = CseKey::Unary(*op, operand_key(operand));
                    if let Some(prev_place) = seen.get(&key) {
                        *stmt = MirStmt::Assign(
                            place.clone(),
                            Rvalue::Use(Operand::Place(prev_place.clone())),
                        );
                        summary.cse_eliminated += 1;
                    } else {
                        seen.insert(key, place.clone());
                    }
                }
                // Any assignment to a local invalidates entries that read from it
                MirStmt::Assign(place, _) => {
                    seen.retain(|k, _| !k.references_place(place));
                }
                // Calls and other side-effecting stmts clear the table
                MirStmt::Call { .. } | MirStmt::CallBuiltin { .. } => {
                    seen.clear();
                }
                _ => {}
            }
        }
    }

    summary
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
enum CseKey {
    Binary(BinOp, OperandKey, OperandKey),
    Unary(UnaryOp, OperandKey),
}

impl CseKey {
    fn references_place(&self, place: &Place) -> bool {
        match self {
            CseKey::Binary(_, l, r) => l.is_place(place) || r.is_place(place),
            CseKey::Unary(_, o) => o.is_place(place),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
enum OperandKey {
    ConstI32(i32),
    ConstI64(i64),
    ConstBool(bool),
    ConstF64(u64), // bit pattern for Hash/Eq
    Local(LocalId),
    Other,
}

impl OperandKey {
    fn is_place(&self, place: &Place) -> bool {
        matches!((self, place), (OperandKey::Local(id), Place::Local(pid)) if id == pid)
    }
}

fn operand_key(op: &Operand) -> OperandKey {
    match op {
        Operand::ConstI32(v) => OperandKey::ConstI32(*v),
        Operand::ConstI64(v) => OperandKey::ConstI64(*v),
        Operand::ConstBool(v) => OperandKey::ConstBool(*v),
        Operand::ConstF64(v) => OperandKey::ConstF64(v.to_bits()),
        Operand::Place(Place::Local(id)) => OperandKey::Local(*id),
        _ => OperandKey::Other,
    }
}

fn is_pure_binop(op: BinOp) -> bool {
    matches!(
        op,
        BinOp::Add
            | BinOp::Sub
            | BinOp::Mul
            | BinOp::Div
            | BinOp::Mod
            | BinOp::Eq
            | BinOp::Ne
            | BinOp::Lt
            | BinOp::Le
            | BinOp::Gt
            | BinOp::Ge
            | BinOp::And
            | BinOp::Or
            | BinOp::BitAnd
            | BinOp::BitOr
            | BinOp::BitXor
            | BinOp::Shl
            | BinOp::Shr
    )
}
