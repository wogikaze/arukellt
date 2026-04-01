//! Desugaring pass: convert backend-illegal operands (IfExpr, LoopExpr, TryExpr)
//! into statement form (IfStmt, WhileStmt, Assign) so `is_backend_legal_module` passes.
//!
//! This pass introduces fresh locals as temporaries for expression results.

use crate::mir::{
    LocalId, MirFunction, MirLocal, MirStmt, Operand, Place, Rvalue,
};
use ark_typecheck::types::Type;

/// Desugar all backend-illegal operands in a function.
/// Returns the number of operands desugared.
pub fn desugar_exprs(func: &mut MirFunction) -> usize {
    let mut counter = 0;
    let mut next_local = func
        .locals
        .iter()
        .map(|l| l.id.0)
        .max()
        .unwrap_or(0)
        + 1;

    for block in &mut func.blocks {
        let mut new_stmts = Vec::with_capacity(block.stmts.len());
        for stmt in std::mem::take(&mut block.stmts) {
            let (desugared, c) = desugar_stmt(stmt, &mut next_local, &mut func.locals);
            counter += c;
            new_stmts.extend(desugared);
        }
        block.stmts = new_stmts;
    }
    counter
}

fn fresh_local(
    next_id: &mut u32,
    locals: &mut Vec<MirLocal>,
    name: &str,
) -> LocalId {
    let id = LocalId(*next_id);
    *next_id += 1;
    locals.push(MirLocal {
        id,
        name: Some(name.to_string()),
        ty: Type::Any,
    });
    id
}

/// Desugar a single statement, possibly expanding into multiple statements.
fn desugar_stmt(
    stmt: MirStmt,
    next_id: &mut u32,
    locals: &mut Vec<MirLocal>,
) -> (Vec<MirStmt>, usize) {
    match stmt {
        MirStmt::Assign(place, rvalue) => {
            let (new_rvalue, pre_stmts, count) = desugar_rvalue(rvalue, next_id, locals);
            let mut result = pre_stmts;
            result.push(MirStmt::Assign(place, new_rvalue));
            (result, count)
        }
        MirStmt::Call { dest, func, args } => {
            let (new_args, pre_stmts, count) = desugar_operand_list(args, next_id, locals);
            let mut result = pre_stmts;
            result.push(MirStmt::Call {
                dest,
                func,
                args: new_args,
            });
            (result, count)
        }
        MirStmt::CallBuiltin { dest, name, args } => {
            let (new_args, pre_stmts, count) = desugar_operand_list(args, next_id, locals);
            let mut result = pre_stmts;
            result.push(MirStmt::CallBuiltin {
                dest,
                name,
                args: new_args,
            });
            (result, count)
        }
        MirStmt::IfStmt {
            cond,
            then_body,
            else_body,
        } => {
            let (new_cond, mut pre, c1) = desugar_operand(cond, next_id, locals);
            let (new_then, c2) = desugar_stmt_list(then_body, next_id, locals);
            let (new_else, c3) = desugar_stmt_list(else_body, next_id, locals);
            pre.push(MirStmt::IfStmt {
                cond: new_cond,
                then_body: new_then,
                else_body: new_else,
            });
            (pre, c1 + c2 + c3)
        }
        MirStmt::WhileStmt { cond, body } => {
            let (new_cond, pre, c1) = desugar_operand(cond, next_id, locals);
            let (new_body, c2) = desugar_stmt_list(body, next_id, locals);
            // If the condition itself needed desugaring, we can't hoist it out of the loop.
            // In that case the pre-statements go inside the loop too.
            if pre.is_empty() {
                (
                    vec![MirStmt::WhileStmt {
                        cond: new_cond,
                        body: new_body,
                    }],
                    c1 + c2,
                )
            } else {
                // Move condition computation into the loop body as an if-break
                let mut loop_body = pre;
                loop_body.push(MirStmt::IfStmt {
                    cond: Operand::UnaryOp(
                        crate::mir::UnaryOp::Not,
                        Box::new(new_cond),
                    ),
                    then_body: vec![MirStmt::Break],
                    else_body: vec![],
                });
                loop_body.extend(new_body);
                (
                    vec![MirStmt::WhileStmt {
                        cond: Operand::ConstBool(true),
                        body: loop_body,
                    }],
                    c1 + c2,
                )
            }
        }
        MirStmt::Return(Some(op)) => {
            let (new_op, mut pre, c) = desugar_operand(op, next_id, locals);
            pre.push(MirStmt::Return(Some(new_op)));
            (pre, c)
        }
        // Statements without operands pass through unchanged
        other => (vec![other], 0),
    }
}

fn desugar_stmt_list(
    stmts: Vec<MirStmt>,
    next_id: &mut u32,
    locals: &mut Vec<MirLocal>,
) -> (Vec<MirStmt>, usize) {
    let mut result = Vec::new();
    let mut total = 0;
    for s in stmts {
        let (desugared, c) = desugar_stmt(s, next_id, locals);
        total += c;
        result.extend(desugared);
    }
    (result, total)
}

fn desugar_rvalue(
    rvalue: Rvalue,
    next_id: &mut u32,
    locals: &mut Vec<MirLocal>,
) -> (Rvalue, Vec<MirStmt>, usize) {
    match rvalue {
        Rvalue::Use(op) => {
            let (new_op, pre, c) = desugar_operand(op, next_id, locals);
            (Rvalue::Use(new_op), pre, c)
        }
        Rvalue::BinaryOp(op, lhs, rhs) => {
            let (new_lhs, mut pre1, c1) = desugar_operand(lhs, next_id, locals);
            let (new_rhs, pre2, c2) = desugar_operand(rhs, next_id, locals);
            pre1.extend(pre2);
            (Rvalue::BinaryOp(op, new_lhs, new_rhs), pre1, c1 + c2)
        }
        Rvalue::UnaryOp(op, inner) => {
            let (new_inner, pre, c) = desugar_operand(inner, next_id, locals);
            (Rvalue::UnaryOp(op, new_inner), pre, c)
        }
        Rvalue::Aggregate(name, ops) => {
            let (new_ops, pre, c) = desugar_operand_list(ops, next_id, locals);
            (Rvalue::Aggregate(name, new_ops), pre, c)
        }
        Rvalue::Ref(p) => (Rvalue::Ref(p), vec![], 0),
    }
}

fn desugar_operand_list(
    ops: Vec<Operand>,
    next_id: &mut u32,
    locals: &mut Vec<MirLocal>,
) -> (Vec<Operand>, Vec<MirStmt>, usize) {
    let mut result = Vec::new();
    let mut pre = Vec::new();
    let mut total = 0;
    for op in ops {
        let (new_op, op_pre, c) = desugar_operand(op, next_id, locals);
        total += c;
        pre.extend(op_pre);
        result.push(new_op);
    }
    (result, pre, total)
}

/// The core: convert an IfExpr/LoopExpr/TryExpr operand into pre-statements + a simple operand.
fn desugar_operand(
    op: Operand,
    next_id: &mut u32,
    locals: &mut Vec<MirLocal>,
) -> (Operand, Vec<MirStmt>, usize) {
    match op {
        Operand::IfExpr {
            cond,
            then_body,
            then_result,
            else_body,
            else_result,
        } => {
            // Recursively desugar subexpressions
            let (new_cond, mut pre, c1) = desugar_operand(*cond, next_id, locals);

            let tmp = fresh_local(next_id, locals, "_if_result");

            let (mut then_stmts, c2) = desugar_stmt_list(then_body, next_id, locals);
            let then_assign = match then_result {
                Some(r) => {
                    let (new_r, r_pre, _) = desugar_operand(*r, next_id, locals);
                    then_stmts.extend(r_pre);
                    MirStmt::Assign(Place::Local(tmp), Rvalue::Use(new_r))
                }
                None => MirStmt::Assign(Place::Local(tmp), Rvalue::Use(Operand::Unit)),
            };
            then_stmts.push(then_assign);

            let (mut else_stmts, c3) = desugar_stmt_list(else_body, next_id, locals);
            let else_assign = match else_result {
                Some(r) => {
                    let (new_r, r_pre, _) = desugar_operand(*r, next_id, locals);
                    else_stmts.extend(r_pre);
                    MirStmt::Assign(Place::Local(tmp), Rvalue::Use(new_r))
                }
                None => MirStmt::Assign(Place::Local(tmp), Rvalue::Use(Operand::Unit)),
            };
            else_stmts.push(else_assign);

            pre.push(MirStmt::IfStmt {
                cond: new_cond,
                then_body: then_stmts,
                else_body: else_stmts,
            });
            (Operand::Place(Place::Local(tmp)), pre, 1 + c1 + c2 + c3)
        }
        Operand::LoopExpr { init, body, result } => {
            let (new_init, mut pre, c1) = desugar_operand(*init, next_id, locals);
            let tmp = fresh_local(next_id, locals, "_loop_result");
            pre.push(MirStmt::Assign(
                Place::Local(tmp),
                Rvalue::Use(new_init),
            ));

            let (new_body, c2) = desugar_stmt_list(body, next_id, locals);
            // After loop body, assign result to tmp
            let (new_result, result_pre, c3) = desugar_operand(*result, next_id, locals);
            let mut loop_body = new_body;
            loop_body.extend(result_pre);
            loop_body.push(MirStmt::Assign(
                Place::Local(tmp),
                Rvalue::Use(new_result),
            ));

            pre.push(MirStmt::WhileStmt {
                cond: Operand::ConstBool(true),
                body: loop_body,
            });
            (Operand::Place(Place::Local(tmp)), pre, 1 + c1 + c2 + c3)
        }
        Operand::TryExpr { expr, from_fn } => {
            let (new_expr, mut pre, c1) = desugar_operand(*expr, next_id, locals);
            let expr_tmp = fresh_local(next_id, locals, "_try_expr");
            pre.push(MirStmt::Assign(
                Place::Local(expr_tmp),
                Rvalue::Use(new_expr),
            ));

            let ok_tmp = fresh_local(next_id, locals, "_try_ok");

            // tag = EnumTag(expr)
            let tag = Operand::EnumTag(Box::new(Operand::Place(Place::Local(expr_tmp))));
            // If tag != 0 (Err), early-return
            let err_payload = Operand::EnumPayload {
                object: Box::new(Operand::Place(Place::Local(expr_tmp))),
                index: 0,
                enum_name: "Result".to_string(),
                variant_name: "Err".to_string(),
            };
            let err_return_val = if let Some(ref conv_fn) = from_fn {
                Operand::Call(conv_fn.clone(), vec![err_payload])
            } else {
                // Re-wrap in Err for the calling function's return type
                Operand::EnumInit {
                    enum_name: "Result".to_string(),
                    variant: "Err".to_string(),
                    tag: 1,
                    payload: vec![err_payload],
                }
            };

            let ok_payload = Operand::EnumPayload {
                object: Box::new(Operand::Place(Place::Local(expr_tmp))),
                index: 0,
                enum_name: "Result".to_string(),
                variant_name: "Ok".to_string(),
            };

            // if tag != 0 { return Err(...) } else { ok_tmp = Ok_payload }
            pre.push(MirStmt::IfStmt {
                cond: Operand::BinOp(
                    crate::mir::BinOp::Ne,
                    Box::new(tag),
                    Box::new(Operand::ConstI32(0)),
                ),
                then_body: vec![MirStmt::Return(Some(err_return_val))],
                else_body: vec![MirStmt::Assign(
                    Place::Local(ok_tmp),
                    Rvalue::Use(ok_payload),
                )],
            });

            (Operand::Place(Place::Local(ok_tmp)), pre, 1 + c1)
        }
        // Recursively desugar nested operands
        Operand::BinOp(op, lhs, rhs) => {
            let (new_lhs, mut pre, c1) = desugar_operand(*lhs, next_id, locals);
            let (new_rhs, pre2, c2) = desugar_operand(*rhs, next_id, locals);
            pre.extend(pre2);
            (
                Operand::BinOp(op, Box::new(new_lhs), Box::new(new_rhs)),
                pre,
                c1 + c2,
            )
        }
        Operand::UnaryOp(op, inner) => {
            let (new_inner, pre, c) = desugar_operand(*inner, next_id, locals);
            (Operand::UnaryOp(op, Box::new(new_inner)), pre, c)
        }
        Operand::Call(name, args) => {
            let (new_args, pre, c) = desugar_operand_list(args, next_id, locals);
            (Operand::Call(name, new_args), pre, c)
        }
        Operand::FieldAccess {
            object,
            struct_name,
            field,
        } => {
            let (new_obj, pre, c) = desugar_operand(*object, next_id, locals);
            (
                Operand::FieldAccess {
                    object: Box::new(new_obj),
                    struct_name,
                    field,
                },
                pre,
                c,
            )
        }
        Operand::EnumTag(inner) => {
            let (new_inner, pre, c) = desugar_operand(*inner, next_id, locals);
            (Operand::EnumTag(Box::new(new_inner)), pre, c)
        }
        Operand::EnumPayload {
            object,
            index,
            enum_name,
            variant_name,
        } => {
            let (new_obj, pre, c) = desugar_operand(*object, next_id, locals);
            (
                Operand::EnumPayload {
                    object: Box::new(new_obj),
                    index,
                    enum_name,
                    variant_name,
                },
                pre,
                c,
            )
        }
        Operand::CallIndirect { callee, args } => {
            let (new_callee, mut pre, c1) = desugar_operand(*callee, next_id, locals);
            let (new_args, pre2, c2) = desugar_operand_list(args, next_id, locals);
            pre.extend(pre2);
            (
                Operand::CallIndirect {
                    callee: Box::new(new_callee),
                    args: new_args,
                },
                pre,
                c1 + c2,
            )
        }
        // All other operands are already backend-legal
        other => (other, vec![], 0),
    }
}
