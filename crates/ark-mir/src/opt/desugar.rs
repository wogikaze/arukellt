//! Desugaring pass: convert backend-illegal operands (IfExpr, LoopExpr, TryExpr)
//! into statement form (IfStmt, WhileStmt, Assign) so `is_backend_legal_module` passes.
//!
//! This pass introduces fresh locals as temporaries for expression results.

use std::collections::HashMap;

use crate::mir::{
    BinOp, LocalId, MirFunction, MirLocal, MirStmt, Operand, Place, Rvalue, Terminator, UnaryOp,
};
use ark_typecheck::types::Type;

/// Desugar all backend-illegal operands in a function.
/// Returns the number of operands desugared.
pub fn desugar_exprs(func: &mut MirFunction, fn_return_types: &HashMap<String, Type>) -> usize {
    let mut counter = 0;
    let mut next_local = func.locals.iter().map(|l| l.id.0).max().unwrap_or(0) + 1;
    let params = func.params.clone();

    for block in &mut func.blocks {
        let mut new_stmts = Vec::with_capacity(block.stmts.len());
        for stmt in std::mem::take(&mut block.stmts) {
            let (desugared, c) = desugar_stmt(
                stmt,
                &mut next_local,
                &params,
                &mut func.locals,
                fn_return_types,
            );
            counter += c;
            new_stmts.extend(desugared);
        }
        block.stmts = new_stmts;

        // ── Desugar Terminator::Return(Some(IfExpr)) ──
        // Convert:
        //   terminator: Return(Some(IfExpr { cond, then_result, else_result, ... }))
        // to:
        //   stmts: [..., IfStmt { then_body: [Return(then_result)], else_body: [Return(else_result)] }]
        //   terminator: Unreachable
        //
        // This preserves Return(Some(Call(...))) in each branch so the T3 emitter's
        // tail-call detection (return_call emission) can fire for recursive tail calls.
        if let Terminator::Return(Some(Operand::IfExpr { .. })) = &block.terminator
            && let Terminator::Return(Some(Operand::IfExpr {
                cond,
                then_body,
                then_result,
                else_body,
                else_result,
            })) = std::mem::replace(&mut block.terminator, Terminator::Unreachable)
        {
            let (new_cond, mut pre, c1) = desugar_operand(
                *cond,
                &mut next_local,
                &params,
                &mut func.locals,
                fn_return_types,
            );

            let (mut then_stmts, c2) = desugar_stmt_list(
                then_body,
                &mut next_local,
                &params,
                &mut func.locals,
                fn_return_types,
            );
            match then_result {
                Some(r) => {
                    let (new_r, r_pre, _) = desugar_operand(
                        *r,
                        &mut next_local,
                        &params,
                        &mut func.locals,
                        fn_return_types,
                    );
                    then_stmts.extend(r_pre);
                    then_stmts.push(MirStmt::Return(Some(new_r)));
                }
                None => {
                    then_stmts.push(MirStmt::Return(None));
                }
            }

            let (mut else_stmts, c3) = desugar_stmt_list(
                else_body,
                &mut next_local,
                &params,
                &mut func.locals,
                fn_return_types,
            );
            match else_result {
                Some(r) => {
                    let (new_r, r_pre, _) = desugar_operand(
                        *r,
                        &mut next_local,
                        &params,
                        &mut func.locals,
                        fn_return_types,
                    );
                    else_stmts.extend(r_pre);
                    else_stmts.push(MirStmt::Return(Some(new_r)));
                }
                None => {
                    else_stmts.push(MirStmt::Return(None));
                }
            }

            pre.push(MirStmt::IfStmt {
                cond: new_cond,
                then_body: then_stmts,
                else_body: else_stmts,
            });
            block.stmts.extend(pre);
            // terminator was already replaced with Unreachable above
            counter += 1 + c1 + c2 + c3;
        }
    }
    counter
}

/// Desugar only `Operand::IfExpr` nodes in a function (leave LoopExpr/TryExpr intact).
/// Used at lowering time so that the MIR is IfExpr-free before the optimisation pipeline.
pub fn desugar_if_exprs(func: &mut MirFunction, fn_return_types: &HashMap<String, Type>) -> usize {
    let mut counter = 0;
    let mut next_local = func.locals.iter().map(|l| l.id.0).max().unwrap_or(0) + 1;
    let params = func.params.clone();

    for block in &mut func.blocks {
        let mut new_stmts = Vec::with_capacity(block.stmts.len());
        for stmt in std::mem::take(&mut block.stmts) {
            let (desugared, c) = desugar_if_stmt(
                stmt,
                &mut next_local,
                &params,
                &mut func.locals,
                fn_return_types,
            );
            counter += c;
            new_stmts.extend(desugared);
        }
        block.stmts = new_stmts;

        // Desugar Terminator::Return(Some(IfExpr)) — same as in desugar_exprs
        if let Terminator::Return(Some(Operand::IfExpr { .. })) = &block.terminator
            && let Terminator::Return(Some(Operand::IfExpr {
                cond,
                then_body,
                then_result,
                else_body,
                else_result,
            })) = std::mem::replace(&mut block.terminator, Terminator::Unreachable)
        {
            let (new_cond, mut pre, c1) = desugar_if_operand(
                *cond,
                &mut next_local,
                &params,
                &mut func.locals,
                fn_return_types,
            );

            let (mut then_stmts, c2) = desugar_if_stmt_list(
                then_body,
                &mut next_local,
                &params,
                &mut func.locals,
                fn_return_types,
            );
            match then_result {
                Some(r) => {
                    let (new_r, r_pre, _) = desugar_if_operand(
                        *r,
                        &mut next_local,
                        &params,
                        &mut func.locals,
                        fn_return_types,
                    );
                    then_stmts.extend(r_pre);
                    then_stmts.push(MirStmt::Return(Some(new_r)));
                }
                None => {
                    then_stmts.push(MirStmt::Return(None));
                }
            }

            let (mut else_stmts, c3) = desugar_if_stmt_list(
                else_body,
                &mut next_local,
                &params,
                &mut func.locals,
                fn_return_types,
            );
            match else_result {
                Some(r) => {
                    let (new_r, r_pre, _) = desugar_if_operand(
                        *r,
                        &mut next_local,
                        &params,
                        &mut func.locals,
                        fn_return_types,
                    );
                    else_stmts.extend(r_pre);
                    else_stmts.push(MirStmt::Return(Some(new_r)));
                }
                None => {
                    else_stmts.push(MirStmt::Return(None));
                }
            }

            pre.push(MirStmt::IfStmt {
                cond: new_cond,
                then_body: then_stmts,
                else_body: else_stmts,
            });
            block.stmts.extend(pre);
            counter += 1 + c1 + c2 + c3;
        }
    }
    counter
}

fn fresh_local(next_id: &mut u32, locals: &mut Vec<MirLocal>, name: &str, ty: Type) -> LocalId {
    let id = LocalId(*next_id);
    *next_id += 1;
    locals.push(MirLocal {
        id,
        name: Some(name.to_string()),
        ty,
    });
    id
}

fn lookup_local_type(params: &[MirLocal], locals: &[MirLocal], id: LocalId) -> Option<Type> {
    params
        .iter()
        .chain(locals.iter())
        .find(|local| local.id == id)
        .map(|local| local.ty.clone())
}

fn canonical_call_name(name: &str) -> &str {
    let short = name.rsplit("::").next().unwrap_or(name);
    short.strip_prefix("__intrinsic_").unwrap_or(short)
}

fn merge_types(lhs: Type, rhs: Type) -> Type {
    if matches!(lhs, Type::Never | Type::Unit) {
        return rhs;
    }
    if matches!(rhs, Type::Never | Type::Unit) {
        return lhs;
    }
    if lhs == Type::Any {
        return rhs;
    }
    if rhs == Type::Any {
        return lhs;
    }
    if lhs == rhs {
        return lhs;
    }
    match (&lhs, &rhs) {
        (Type::I64, Type::U64) | (Type::U64, Type::I64) => Type::I64,
        _ => Type::Any,
    }
}

fn infer_call_type(
    name: &str,
    args: &[Operand],
    params: &[MirLocal],
    locals: &[MirLocal],
    fn_return_types: &HashMap<String, Type>,
) -> Type {
    let canonical = canonical_call_name(name);
    match name {
        "std::host::http::get" | "http::get" => {
            return Type::Result(Box::new(Type::String), Box::new(Type::String));
        }
        "std::host::http::request" | "http::request" => {
            return Type::Result(Box::new(Type::String), Box::new(Type::String));
        }
        _ => {}
    }
    if let Some(ty) = fn_return_types
        .get(name)
        .or_else(|| fn_return_types.get(canonical))
    {
        return ty.clone();
    }
    match canonical {
        "String_from" | "String_new" | "concat" | "to_string" | "i32_to_string"
        | "f64_to_string" | "f32_to_string" | "i64_to_string" | "bool_to_string"
        | "char_to_string" | "join" | "slice" | "substring" | "trim" | "replace" | "read_line"
        | "to_lower" | "to_upper" => Type::String,
        "clone" => args
            .first()
            .map(|arg| infer_operand_type(arg, params, locals, fn_return_types))
            .unwrap_or(Type::Any),
        "Vec_new_i32" => Type::Vec(Box::new(Type::I32)),
        "Vec_new_i64" => Type::Vec(Box::new(Type::I64)),
        "Vec_new_f64" => Type::Vec(Box::new(Type::F64)),
        "Vec_new_String" => Type::Vec(Box::new(Type::String)),
        "parse_i32" => Type::Result(Box::new(Type::I32), Box::new(Type::String)),
        "parse_i64" => Type::Result(Box::new(Type::I64), Box::new(Type::String)),
        "parse_f64" => Type::Result(Box::new(Type::F64), Box::new(Type::String)),
        "read_to_string" | "fs_read_file" | "http_get" | "http_request" => {
            Type::Result(Box::new(Type::String), Box::new(Type::String))
        }
        "env_var" | "var" => Type::Option(Box::new(Type::String)),
        "get" | "get_unchecked" => args
            .first()
            .map(|arg| infer_operand_type(arg, params, locals, fn_return_types))
            .and_then(|ty| match ty {
                Type::Vec(inner) | Type::Option(inner) => Some(*inner),
                _ => None,
            })
            .unwrap_or(Type::Any),
        "len"
        | "arg_count"
        | "arg_at"
        | "index_of"
        | "char_at"
        | "digit_value"
        | "get_byte"
        | "HashMap_i32_i32_len"
        | "HashMap_i32_i32_contains_key"
        | "contains_i32"
        | "contains_String"
        | "random_i32" => Type::I32,
        "clock_now" | "clock_now_ms" | "monotonic_now" | "now_ms" => Type::I64,
        "sqrt" | "random_next_f64" | "next_f64" => Type::F64,
        "panic" | "assert" | "assert_eq" | "assert_ne" | "assert_eq_str" | "assert_eq_i64"
        | "exit" | "proc_exit" | "process_exit" | "process_abort" => Type::Never,
        "println"
        | "print"
        | "eprintln"
        | "print_i32_ln"
        | "print_bool_ln"
        | "print_str_ln"
        | "set"
        | "sort_i32"
        | "sort_String"
        | "sort_i64"
        | "sort_f64"
        | "reverse_i32"
        | "reverse_String"
        | "remove_i32"
        | "HashMap_i32_i32_insert"
        | "push_char" => Type::Unit,
        _ => Type::Any,
    }
}

fn infer_operand_type(
    op: &Operand,
    params: &[MirLocal],
    locals: &[MirLocal],
    fn_return_types: &HashMap<String, Type>,
) -> Type {
    match op {
        Operand::Place(Place::Local(id)) => {
            lookup_local_type(params, locals, *id).unwrap_or(Type::Any)
        }
        Operand::Place(Place::Index(base, _)) => match base.as_ref() {
            Place::Local(id) => match lookup_local_type(params, locals, *id).unwrap_or(Type::Any) {
                Type::Vec(inner) | Type::Array(inner, _) | Type::Slice(inner) => *inner,
                _ => Type::Any,
            },
            _ => Type::Any,
        },
        Operand::Place(_) => Type::Any,
        Operand::ConstI32(_)
        | Operand::ConstU8(_)
        | Operand::ConstU16(_)
        | Operand::ConstU32(_)
        | Operand::ConstI8(_)
        | Operand::ConstI16(_) => Type::I32,
        Operand::ConstI64(_) | Operand::ConstU64(_) => Type::I64,
        Operand::ConstF32(_) => Type::F32,
        Operand::ConstF64(_) => Type::F64,
        Operand::ConstBool(_) => Type::Bool,
        Operand::ConstChar(_) => Type::Char,
        Operand::ConstString(_) => Type::String,
        Operand::Unit => Type::Unit,
        Operand::BinOp(op, lhs, rhs) => match op {
            BinOp::Eq
            | BinOp::Ne
            | BinOp::Lt
            | BinOp::Le
            | BinOp::Gt
            | BinOp::Ge
            | BinOp::And
            | BinOp::Or => Type::Bool,
            _ => {
                let lhs_ty = infer_operand_type(lhs, params, locals, fn_return_types);
                let rhs_ty = infer_operand_type(rhs, params, locals, fn_return_types);
                merge_types(lhs_ty, rhs_ty)
            }
        },
        Operand::UnaryOp(op, inner) => match op {
            UnaryOp::Not => Type::Bool,
            _ => infer_operand_type(inner, params, locals, fn_return_types),
        },
        Operand::Call(name, args) => infer_call_type(name, args, params, locals, fn_return_types),
        Operand::IfExpr {
            then_result,
            else_result,
            ..
        } => merge_types(
            then_result
                .as_deref()
                .map(|op| infer_operand_type(op, params, locals, fn_return_types))
                .unwrap_or(Type::Unit),
            else_result
                .as_deref()
                .map(|op| infer_operand_type(op, params, locals, fn_return_types))
                .unwrap_or(Type::Unit),
        ),
        Operand::StructInit { .. } => Type::Any,
        Operand::FieldAccess { .. } => Type::Any,
        Operand::EnumInit {
            enum_name,
            variant,
            payload,
            ..
        } => match (enum_name.as_str(), variant.as_str(), payload.as_slice()) {
            ("Option", "Some", [inner]) => Type::Option(Box::new(infer_operand_type(
                inner,
                params,
                locals,
                fn_return_types,
            ))),
            ("Option", "None", _) => Type::Option(Box::new(Type::Any)),
            ("Result", "Ok", [inner]) => Type::Result(
                Box::new(infer_operand_type(inner, params, locals, fn_return_types)),
                Box::new(Type::Any),
            ),
            ("Result", "Err", [inner]) => Type::Result(
                Box::new(Type::Any),
                Box::new(infer_operand_type(inner, params, locals, fn_return_types)),
            ),
            _ => Type::Any,
        },
        Operand::EnumTag(_) => Type::I32,
        Operand::EnumPayload {
            object,
            variant_name,
            ..
        } => match infer_operand_type(object, params, locals, fn_return_types) {
            Type::Option(inner) => *inner,
            Type::Result(ok, _) if variant_name == "Ok" => *ok,
            Type::Result(_, err) if variant_name == "Err" => *err,
            _ => Type::Any,
        },
        Operand::LoopExpr { result, .. } => {
            infer_operand_type(result, params, locals, fn_return_types)
        }
        Operand::TryExpr { expr, .. } => {
            match infer_operand_type(expr, params, locals, fn_return_types) {
                Type::Option(inner) => *inner,
                Type::Result(ok, _) => *ok,
                _ => Type::Any,
            }
        }
        Operand::FnRef(_) | Operand::CallIndirect { .. } => Type::Any,
        Operand::ArrayInit { elements } => elements
            .first()
            .map(|el| {
                Type::Array(
                    Box::new(infer_operand_type(el, params, locals, fn_return_types)),
                    elements.len() as u64,
                )
            })
            .unwrap_or_else(|| Type::Array(Box::new(Type::Any), 0)),
        Operand::IndexAccess { object, .. } => {
            match infer_operand_type(object, params, locals, fn_return_types) {
                Type::Vec(inner) | Type::Array(inner, _) | Type::Slice(inner) => *inner,
                _ => Type::Any,
            }
        }
    }
}

/// Desugar a single statement, possibly expanding into multiple statements.
fn desugar_stmt(
    stmt: MirStmt,
    next_id: &mut u32,
    params: &[MirLocal],
    locals: &mut Vec<MirLocal>,
    fn_return_types: &HashMap<String, Type>,
) -> (Vec<MirStmt>, usize) {
    match stmt {
        MirStmt::Assign(place, rvalue) => {
            let (new_rvalue, pre_stmts, count) =
                desugar_rvalue(rvalue, next_id, params, locals, fn_return_types);
            let mut result = pre_stmts;
            result.push(MirStmt::Assign(place, new_rvalue));
            (result, count)
        }
        MirStmt::Call { dest, func, args } => {
            let (new_args, pre_stmts, count) =
                desugar_operand_list(args, next_id, params, locals, fn_return_types);
            let mut result = pre_stmts;
            result.push(MirStmt::Call {
                dest,
                func,
                args: new_args,
            });
            (result, count)
        }
        MirStmt::CallBuiltin { dest, name, args } => {
            let (new_args, pre_stmts, count) =
                desugar_operand_list(args, next_id, params, locals, fn_return_types);
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
            let (new_cond, mut pre, c1) =
                desugar_operand(cond, next_id, params, locals, fn_return_types);
            let (new_then, c2) =
                desugar_stmt_list(then_body, next_id, params, locals, fn_return_types);
            let (new_else, c3) =
                desugar_stmt_list(else_body, next_id, params, locals, fn_return_types);
            pre.push(MirStmt::IfStmt {
                cond: new_cond,
                then_body: new_then,
                else_body: new_else,
            });
            (pre, c1 + c2 + c3)
        }
        MirStmt::WhileStmt { cond, body } => {
            let (new_cond, pre, c1) =
                desugar_operand(cond, next_id, params, locals, fn_return_types);
            let (new_body, c2) = desugar_stmt_list(body, next_id, params, locals, fn_return_types);
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
                    cond: Operand::UnaryOp(crate::mir::UnaryOp::Not, Box::new(new_cond)),
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
            let (new_op, mut pre, c) =
                desugar_operand(op, next_id, params, locals, fn_return_types);
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
    params: &[MirLocal],
    locals: &mut Vec<MirLocal>,
    fn_return_types: &HashMap<String, Type>,
) -> (Vec<MirStmt>, usize) {
    let mut result = Vec::new();
    let mut total = 0;
    for s in stmts {
        let (desugared, c) = desugar_stmt(s, next_id, params, locals, fn_return_types);
        total += c;
        result.extend(desugared);
    }
    (result, total)
}

fn desugar_rvalue(
    rvalue: Rvalue,
    next_id: &mut u32,
    params: &[MirLocal],
    locals: &mut Vec<MirLocal>,
    fn_return_types: &HashMap<String, Type>,
) -> (Rvalue, Vec<MirStmt>, usize) {
    match rvalue {
        Rvalue::Use(op) => {
            let (new_op, pre, c) = desugar_operand(op, next_id, params, locals, fn_return_types);
            (Rvalue::Use(new_op), pre, c)
        }
        Rvalue::BinaryOp(op, lhs, rhs) => {
            let (new_lhs, mut pre1, c1) =
                desugar_operand(lhs, next_id, params, locals, fn_return_types);
            let (new_rhs, pre2, c2) =
                desugar_operand(rhs, next_id, params, locals, fn_return_types);
            pre1.extend(pre2);
            (Rvalue::BinaryOp(op, new_lhs, new_rhs), pre1, c1 + c2)
        }
        Rvalue::UnaryOp(op, inner) => {
            let (new_inner, pre, c) =
                desugar_operand(inner, next_id, params, locals, fn_return_types);
            (Rvalue::UnaryOp(op, new_inner), pre, c)
        }
        Rvalue::Aggregate(name, ops) => {
            let (new_ops, pre, c) =
                desugar_operand_list(ops, next_id, params, locals, fn_return_types);
            (Rvalue::Aggregate(name, new_ops), pre, c)
        }
        Rvalue::Ref(p) => (Rvalue::Ref(p), vec![], 0),
    }
}

fn desugar_operand_list(
    ops: Vec<Operand>,
    next_id: &mut u32,
    params: &[MirLocal],
    locals: &mut Vec<MirLocal>,
    fn_return_types: &HashMap<String, Type>,
) -> (Vec<Operand>, Vec<MirStmt>, usize) {
    let mut result = Vec::new();
    let mut pre = Vec::new();
    let mut total = 0;
    for op in ops {
        let (new_op, op_pre, c) = desugar_operand(op, next_id, params, locals, fn_return_types);
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
    params: &[MirLocal],
    locals: &mut Vec<MirLocal>,
    fn_return_types: &HashMap<String, Type>,
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
            let (new_cond, mut pre, c1) =
                desugar_operand(*cond, next_id, params, locals, fn_return_types);

            let tmp_ty = merge_types(
                then_result
                    .as_deref()
                    .map(|op| infer_operand_type(op, params, locals, fn_return_types))
                    .unwrap_or(Type::Unit),
                else_result
                    .as_deref()
                    .map(|op| infer_operand_type(op, params, locals, fn_return_types))
                    .unwrap_or(Type::Unit),
            );
            let tmp = fresh_local(next_id, locals, "_if_result", tmp_ty);

            let (mut then_stmts, c2) =
                desugar_stmt_list(then_body, next_id, params, locals, fn_return_types);
            let then_assign = match then_result {
                Some(r) => {
                    let (new_r, r_pre, _) =
                        desugar_operand(*r, next_id, params, locals, fn_return_types);
                    then_stmts.extend(r_pre);
                    MirStmt::Assign(Place::Local(tmp), Rvalue::Use(new_r))
                }
                None => MirStmt::Assign(Place::Local(tmp), Rvalue::Use(Operand::Unit)),
            };
            then_stmts.push(then_assign);

            let (mut else_stmts, c3) =
                desugar_stmt_list(else_body, next_id, params, locals, fn_return_types);
            let else_assign = match else_result {
                Some(r) => {
                    let (new_r, r_pre, _) =
                        desugar_operand(*r, next_id, params, locals, fn_return_types);
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
            let result_ty = infer_operand_type(&result, params, locals, fn_return_types);
            let (new_init, mut pre, c1) =
                desugar_operand(*init, next_id, params, locals, fn_return_types);
            let tmp = fresh_local(next_id, locals, "_loop_result", result_ty);
            pre.push(MirStmt::Assign(Place::Local(tmp), Rvalue::Use(new_init)));

            // body already contains a WhileStmt (emitted by the lowerer); do NOT
            // wrap it in another WhileStmt or the outer loop will never exit.
            let (new_body, c2) = desugar_stmt_list(body, next_id, params, locals, fn_return_types);
            pre.extend(new_body);

            // After the WhileStmt exits (via Break), capture the loop result.
            let (new_result, result_pre, c3) =
                desugar_operand(*result, next_id, params, locals, fn_return_types);
            pre.extend(result_pre);
            pre.push(MirStmt::Assign(Place::Local(tmp), Rvalue::Use(new_result)));

            (Operand::Place(Place::Local(tmp)), pre, 1 + c1 + c2 + c3)
        }
        Operand::TryExpr { expr, from_fn } => {
            let expr_ty = infer_operand_type(&expr, params, locals, fn_return_types);
            let (new_expr, mut pre, c1) =
                desugar_operand(*expr, next_id, params, locals, fn_return_types);
            let expr_tmp = fresh_local(next_id, locals, "_try_expr", expr_ty.clone());
            pre.push(MirStmt::Assign(
                Place::Local(expr_tmp),
                Rvalue::Use(new_expr),
            ));

            let ok_ty = match expr_ty {
                Type::Option(inner) => *inner,
                Type::Result(ok, _) => *ok,
                _ => Type::Any,
            };
            let ok_tmp = fresh_local(next_id, locals, "_try_ok", ok_ty);

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
                Operand::EnumInit {
                    enum_name: "Result".to_string(),
                    variant: "Err".to_string(),
                    tag: 1,
                    payload: vec![Operand::Call(conv_fn.clone(), vec![err_payload])],
                }
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
            let (new_lhs, mut pre, c1) =
                desugar_operand(*lhs, next_id, params, locals, fn_return_types);
            let (new_rhs, pre2, c2) =
                desugar_operand(*rhs, next_id, params, locals, fn_return_types);
            pre.extend(pre2);
            (
                Operand::BinOp(op, Box::new(new_lhs), Box::new(new_rhs)),
                pre,
                c1 + c2,
            )
        }
        Operand::UnaryOp(op, inner) => {
            let (new_inner, pre, c) =
                desugar_operand(*inner, next_id, params, locals, fn_return_types);
            (Operand::UnaryOp(op, Box::new(new_inner)), pre, c)
        }
        Operand::Call(name, args) => {
            let (new_args, pre, c) =
                desugar_operand_list(args, next_id, params, locals, fn_return_types);
            (Operand::Call(name, new_args), pre, c)
        }
        Operand::FieldAccess {
            object,
            struct_name,
            field,
        } => {
            let (new_obj, pre, c) =
                desugar_operand(*object, next_id, params, locals, fn_return_types);
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
            let (new_inner, pre, c) =
                desugar_operand(*inner, next_id, params, locals, fn_return_types);
            (Operand::EnumTag(Box::new(new_inner)), pre, c)
        }
        Operand::EnumPayload {
            object,
            index,
            enum_name,
            variant_name,
        } => {
            let (new_obj, pre, c) =
                desugar_operand(*object, next_id, params, locals, fn_return_types);
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
            let (new_callee, mut pre, c1) =
                desugar_operand(*callee, next_id, params, locals, fn_return_types);
            let (new_args, pre2, c2) =
                desugar_operand_list(args, next_id, params, locals, fn_return_types);
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

// ── LoopExpr-only desugar helpers (used at lowering time) ──────────────────

/// Desugar only `Operand::LoopExpr` nodes in a function (leave IfExpr/TryExpr intact).
/// Used at lowering time so that the MIR is LoopExpr-free before the backend.
pub fn desugar_loop_exprs(
    func: &mut MirFunction,
    fn_return_types: &HashMap<String, Type>,
) -> usize {
    let mut counter = 0;
    let mut next_local = func.locals.iter().map(|l| l.id.0).max().unwrap_or(0) + 1;
    let params = func.params.clone();

    for block in &mut func.blocks {
        let mut new_stmts = Vec::with_capacity(block.stmts.len());
        for stmt in std::mem::take(&mut block.stmts) {
            let (desugared, c) = desugar_loop_stmt(
                stmt,
                &mut next_local,
                &params,
                &mut func.locals,
                fn_return_types,
            );
            counter += c;
            new_stmts.extend(desugared);
        }
        block.stmts = new_stmts;

        // Desugar Terminator::Return(Some(LoopExpr))
        if let Terminator::Return(Some(Operand::LoopExpr { .. })) = &block.terminator
            && let Terminator::Return(Some(Operand::LoopExpr { init, body, result })) =
                std::mem::replace(&mut block.terminator, Terminator::Unreachable)
        {
            let result_ty = infer_operand_type(&result, &params, &func.locals, fn_return_types);
            let (new_init, mut pre, c1) = desugar_loop_operand(
                *init,
                &mut next_local,
                &params,
                &mut func.locals,
                fn_return_types,
            );
            let tmp = fresh_local(&mut next_local, &mut func.locals, "_loop_result", result_ty);
            pre.push(MirStmt::Assign(Place::Local(tmp), Rvalue::Use(new_init)));

            let (new_body, c2) = desugar_loop_stmt_list(
                body,
                &mut next_local,
                &params,
                &mut func.locals,
                fn_return_types,
            );
            pre.extend(new_body);

            let (new_result, result_pre, c3) = desugar_loop_operand(
                *result,
                &mut next_local,
                &params,
                &mut func.locals,
                fn_return_types,
            );
            pre.extend(result_pre);
            pre.push(MirStmt::Return(Some(new_result)));

            block.stmts.extend(pre);
            counter += 1 + c1 + c2 + c3;
        }
    }
    counter
}

fn desugar_loop_stmt(
    stmt: MirStmt,
    next_id: &mut u32,
    params: &[MirLocal],
    locals: &mut Vec<MirLocal>,
    fn_return_types: &HashMap<String, Type>,
) -> (Vec<MirStmt>, usize) {
    match stmt {
        MirStmt::Assign(place, rvalue) => {
            let (new_rvalue, pre_stmts, count) =
                desugar_loop_rvalue(rvalue, next_id, params, locals, fn_return_types);
            let mut result = pre_stmts;
            result.push(MirStmt::Assign(place, new_rvalue));
            (result, count)
        }
        MirStmt::Call { dest, func, args } => {
            let (new_args, pre_stmts, count) =
                desugar_loop_operand_list(args, next_id, params, locals, fn_return_types);
            let mut result = pre_stmts;
            result.push(MirStmt::Call {
                dest,
                func,
                args: new_args,
            });
            (result, count)
        }
        MirStmt::CallBuiltin { dest, name, args } => {
            let (new_args, pre_stmts, count) =
                desugar_loop_operand_list(args, next_id, params, locals, fn_return_types);
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
            let (new_cond, mut pre, c1) =
                desugar_loop_operand(cond, next_id, params, locals, fn_return_types);
            let (new_then, c2) =
                desugar_loop_stmt_list(then_body, next_id, params, locals, fn_return_types);
            let (new_else, c3) =
                desugar_loop_stmt_list(else_body, next_id, params, locals, fn_return_types);
            pre.push(MirStmt::IfStmt {
                cond: new_cond,
                then_body: new_then,
                else_body: new_else,
            });
            (pre, c1 + c2 + c3)
        }
        MirStmt::WhileStmt { cond, body } => {
            let (new_cond, pre, c1) =
                desugar_loop_operand(cond, next_id, params, locals, fn_return_types);
            let (new_body, c2) =
                desugar_loop_stmt_list(body, next_id, params, locals, fn_return_types);
            if pre.is_empty() {
                (
                    vec![MirStmt::WhileStmt {
                        cond: new_cond,
                        body: new_body,
                    }],
                    c1 + c2,
                )
            } else {
                let mut loop_body = pre;
                loop_body.push(MirStmt::IfStmt {
                    cond: Operand::UnaryOp(crate::mir::UnaryOp::Not, Box::new(new_cond)),
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
            let (new_op, mut pre, c) =
                desugar_loop_operand(op, next_id, params, locals, fn_return_types);
            pre.push(MirStmt::Return(Some(new_op)));
            (pre, c)
        }
        other => (vec![other], 0),
    }
}

fn desugar_loop_stmt_list(
    stmts: Vec<MirStmt>,
    next_id: &mut u32,
    params: &[MirLocal],
    locals: &mut Vec<MirLocal>,
    fn_return_types: &HashMap<String, Type>,
) -> (Vec<MirStmt>, usize) {
    let mut result = Vec::new();
    let mut total = 0;
    for s in stmts {
        let (desugared, c) = desugar_loop_stmt(s, next_id, params, locals, fn_return_types);
        total += c;
        result.extend(desugared);
    }
    (result, total)
}

fn desugar_loop_rvalue(
    rvalue: Rvalue,
    next_id: &mut u32,
    params: &[MirLocal],
    locals: &mut Vec<MirLocal>,
    fn_return_types: &HashMap<String, Type>,
) -> (Rvalue, Vec<MirStmt>, usize) {
    match rvalue {
        Rvalue::Use(op) => {
            let (new_op, pre, c) =
                desugar_loop_operand(op, next_id, params, locals, fn_return_types);
            (Rvalue::Use(new_op), pre, c)
        }
        Rvalue::BinaryOp(op, lhs, rhs) => {
            let (new_lhs, mut pre1, c1) =
                desugar_loop_operand(lhs, next_id, params, locals, fn_return_types);
            let (new_rhs, pre2, c2) =
                desugar_loop_operand(rhs, next_id, params, locals, fn_return_types);
            pre1.extend(pre2);
            (Rvalue::BinaryOp(op, new_lhs, new_rhs), pre1, c1 + c2)
        }
        Rvalue::UnaryOp(op, inner) => {
            let (new_inner, pre, c) =
                desugar_loop_operand(inner, next_id, params, locals, fn_return_types);
            (Rvalue::UnaryOp(op, new_inner), pre, c)
        }
        Rvalue::Aggregate(name, ops) => {
            let (new_ops, pre, c) =
                desugar_loop_operand_list(ops, next_id, params, locals, fn_return_types);
            (Rvalue::Aggregate(name, new_ops), pre, c)
        }
        Rvalue::Ref(p) => (Rvalue::Ref(p), vec![], 0),
    }
}

fn desugar_loop_operand_list(
    ops: Vec<Operand>,
    next_id: &mut u32,
    params: &[MirLocal],
    locals: &mut Vec<MirLocal>,
    fn_return_types: &HashMap<String, Type>,
) -> (Vec<Operand>, Vec<MirStmt>, usize) {
    let mut result = Vec::new();
    let mut pre = Vec::new();
    let mut total = 0;
    for op in ops {
        let (new_op, op_pre, c) =
            desugar_loop_operand(op, next_id, params, locals, fn_return_types);
        total += c;
        pre.extend(op_pre);
        result.push(new_op);
    }
    (result, pre, total)
}

/// Convert a LoopExpr operand into pre-statements + a simple operand.
/// IfExpr and TryExpr are left untouched.
fn desugar_loop_operand(
    op: Operand,
    next_id: &mut u32,
    params: &[MirLocal],
    locals: &mut Vec<MirLocal>,
    fn_return_types: &HashMap<String, Type>,
) -> (Operand, Vec<MirStmt>, usize) {
    match op {
        Operand::LoopExpr { init, body, result } => {
            let result_ty = infer_operand_type(&result, params, locals, fn_return_types);
            let (new_init, mut pre, c1) =
                desugar_loop_operand(*init, next_id, params, locals, fn_return_types);
            let tmp = fresh_local(next_id, locals, "_loop_result", result_ty);
            pre.push(MirStmt::Assign(Place::Local(tmp), Rvalue::Use(new_init)));

            // body already contains a WhileStmt (emitted by the lowerer); do NOT
            // wrap it in another WhileStmt or the outer loop will never exit.
            let (new_body, c2) =
                desugar_loop_stmt_list(body, next_id, params, locals, fn_return_types);
            pre.extend(new_body);

            // After the WhileStmt exits (via Break), capture the loop result.
            let (new_result, result_pre, c3) =
                desugar_loop_operand(*result, next_id, params, locals, fn_return_types);
            pre.extend(result_pre);
            pre.push(MirStmt::Assign(Place::Local(tmp), Rvalue::Use(new_result)));

            (Operand::Place(Place::Local(tmp)), pre, 1 + c1 + c2 + c3)
        }
        // Recursively desugar LoopExpr inside nested operands
        Operand::BinOp(op, lhs, rhs) => {
            let (new_lhs, mut pre, c1) =
                desugar_loop_operand(*lhs, next_id, params, locals, fn_return_types);
            let (new_rhs, pre2, c2) =
                desugar_loop_operand(*rhs, next_id, params, locals, fn_return_types);
            pre.extend(pre2);
            (
                Operand::BinOp(op, Box::new(new_lhs), Box::new(new_rhs)),
                pre,
                c1 + c2,
            )
        }
        Operand::UnaryOp(op, inner) => {
            let (new_inner, pre, c) =
                desugar_loop_operand(*inner, next_id, params, locals, fn_return_types);
            (Operand::UnaryOp(op, Box::new(new_inner)), pre, c)
        }
        Operand::Call(name, args) => {
            let (new_args, pre, c) =
                desugar_loop_operand_list(args, next_id, params, locals, fn_return_types);
            (Operand::Call(name, new_args), pre, c)
        }
        Operand::FieldAccess {
            object,
            struct_name,
            field,
        } => {
            let (new_obj, pre, c) =
                desugar_loop_operand(*object, next_id, params, locals, fn_return_types);
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
            let (new_inner, pre, c) =
                desugar_loop_operand(*inner, next_id, params, locals, fn_return_types);
            (Operand::EnumTag(Box::new(new_inner)), pre, c)
        }
        Operand::EnumPayload {
            object,
            index,
            enum_name,
            variant_name,
        } => {
            let (new_obj, pre, c) =
                desugar_loop_operand(*object, next_id, params, locals, fn_return_types);
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
            let (new_callee, mut pre, c1) =
                desugar_loop_operand(*callee, next_id, params, locals, fn_return_types);
            let (new_args, pre2, c2) =
                desugar_loop_operand_list(args, next_id, params, locals, fn_return_types);
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
        // Leave all other operands (including IfExpr, TryExpr) untouched
        other => (other, vec![], 0),
    }
}

// ── IfExpr-only desugar helpers (used at lowering time) ────────────────────

/// Desugar a single statement, expanding only IfExpr operands.
fn desugar_if_stmt(
    stmt: MirStmt,
    next_id: &mut u32,
    params: &[MirLocal],
    locals: &mut Vec<MirLocal>,
    fn_return_types: &HashMap<String, Type>,
) -> (Vec<MirStmt>, usize) {
    match stmt {
        MirStmt::Assign(place, rvalue) => {
            let (new_rvalue, pre_stmts, count) =
                desugar_if_rvalue(rvalue, next_id, params, locals, fn_return_types);
            let mut result = pre_stmts;
            result.push(MirStmt::Assign(place, new_rvalue));
            (result, count)
        }
        MirStmt::Call { dest, func, args } => {
            let (new_args, pre_stmts, count) =
                desugar_if_operand_list(args, next_id, params, locals, fn_return_types);
            let mut result = pre_stmts;
            result.push(MirStmt::Call {
                dest,
                func,
                args: new_args,
            });
            (result, count)
        }
        MirStmt::CallBuiltin { dest, name, args } => {
            let (new_args, pre_stmts, count) =
                desugar_if_operand_list(args, next_id, params, locals, fn_return_types);
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
            let (new_cond, mut pre, c1) =
                desugar_if_operand(cond, next_id, params, locals, fn_return_types);
            let (new_then, c2) =
                desugar_if_stmt_list(then_body, next_id, params, locals, fn_return_types);
            let (new_else, c3) =
                desugar_if_stmt_list(else_body, next_id, params, locals, fn_return_types);
            pre.push(MirStmt::IfStmt {
                cond: new_cond,
                then_body: new_then,
                else_body: new_else,
            });
            (pre, c1 + c2 + c3)
        }
        MirStmt::WhileStmt { cond, body } => {
            let (new_cond, pre, c1) =
                desugar_if_operand(cond, next_id, params, locals, fn_return_types);
            let (new_body, c2) =
                desugar_if_stmt_list(body, next_id, params, locals, fn_return_types);
            if pre.is_empty() {
                (
                    vec![MirStmt::WhileStmt {
                        cond: new_cond,
                        body: new_body,
                    }],
                    c1 + c2,
                )
            } else {
                let mut loop_body = pre;
                loop_body.push(MirStmt::IfStmt {
                    cond: Operand::UnaryOp(crate::mir::UnaryOp::Not, Box::new(new_cond)),
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
            let (new_op, mut pre, c) =
                desugar_if_operand(op, next_id, params, locals, fn_return_types);
            pre.push(MirStmt::Return(Some(new_op)));
            (pre, c)
        }
        other => (vec![other], 0),
    }
}

fn desugar_if_stmt_list(
    stmts: Vec<MirStmt>,
    next_id: &mut u32,
    params: &[MirLocal],
    locals: &mut Vec<MirLocal>,
    fn_return_types: &HashMap<String, Type>,
) -> (Vec<MirStmt>, usize) {
    let mut result = Vec::new();
    let mut total = 0;
    for s in stmts {
        let (desugared, c) = desugar_if_stmt(s, next_id, params, locals, fn_return_types);
        total += c;
        result.extend(desugared);
    }
    (result, total)
}

fn desugar_if_rvalue(
    rvalue: Rvalue,
    next_id: &mut u32,
    params: &[MirLocal],
    locals: &mut Vec<MirLocal>,
    fn_return_types: &HashMap<String, Type>,
) -> (Rvalue, Vec<MirStmt>, usize) {
    match rvalue {
        Rvalue::Use(op) => {
            let (new_op, pre, c) = desugar_if_operand(op, next_id, params, locals, fn_return_types);
            (Rvalue::Use(new_op), pre, c)
        }
        Rvalue::BinaryOp(op, lhs, rhs) => {
            let (new_lhs, mut pre1, c1) =
                desugar_if_operand(lhs, next_id, params, locals, fn_return_types);
            let (new_rhs, pre2, c2) =
                desugar_if_operand(rhs, next_id, params, locals, fn_return_types);
            pre1.extend(pre2);
            (Rvalue::BinaryOp(op, new_lhs, new_rhs), pre1, c1 + c2)
        }
        Rvalue::UnaryOp(op, inner) => {
            let (new_inner, pre, c) =
                desugar_if_operand(inner, next_id, params, locals, fn_return_types);
            (Rvalue::UnaryOp(op, new_inner), pre, c)
        }
        Rvalue::Aggregate(name, ops) => {
            let (new_ops, pre, c) =
                desugar_if_operand_list(ops, next_id, params, locals, fn_return_types);
            (Rvalue::Aggregate(name, new_ops), pre, c)
        }
        Rvalue::Ref(p) => (Rvalue::Ref(p), vec![], 0),
    }
}

fn desugar_if_operand_list(
    ops: Vec<Operand>,
    next_id: &mut u32,
    params: &[MirLocal],
    locals: &mut Vec<MirLocal>,
    fn_return_types: &HashMap<String, Type>,
) -> (Vec<Operand>, Vec<MirStmt>, usize) {
    let mut result = Vec::new();
    let mut pre = Vec::new();
    let mut total = 0;
    for op in ops {
        let (new_op, op_pre, c) = desugar_if_operand(op, next_id, params, locals, fn_return_types);
        total += c;
        pre.extend(op_pre);
        result.push(new_op);
    }
    (result, pre, total)
}

/// Convert an IfExpr operand into pre-statements + a simple operand.
/// LoopExpr and TryExpr are left untouched.
fn desugar_if_operand(
    op: Operand,
    next_id: &mut u32,
    params: &[MirLocal],
    locals: &mut Vec<MirLocal>,
    fn_return_types: &HashMap<String, Type>,
) -> (Operand, Vec<MirStmt>, usize) {
    match op {
        Operand::IfExpr {
            cond,
            then_body,
            then_result,
            else_body,
            else_result,
        } => {
            let (new_cond, mut pre, c1) =
                desugar_if_operand(*cond, next_id, params, locals, fn_return_types);

            let tmp_ty = merge_types(
                then_result
                    .as_deref()
                    .map(|op| infer_operand_type(op, params, locals, fn_return_types))
                    .unwrap_or(Type::Unit),
                else_result
                    .as_deref()
                    .map(|op| infer_operand_type(op, params, locals, fn_return_types))
                    .unwrap_or(Type::Unit),
            );
            let tmp = fresh_local(next_id, locals, "_if_result", tmp_ty);

            let (mut then_stmts, c2) =
                desugar_if_stmt_list(then_body, next_id, params, locals, fn_return_types);
            let then_assign = match then_result {
                Some(r) => {
                    let (new_r, r_pre, _) =
                        desugar_if_operand(*r, next_id, params, locals, fn_return_types);
                    then_stmts.extend(r_pre);
                    MirStmt::Assign(Place::Local(tmp), Rvalue::Use(new_r))
                }
                None => MirStmt::Assign(Place::Local(tmp), Rvalue::Use(Operand::Unit)),
            };
            then_stmts.push(then_assign);

            let (mut else_stmts, c3) =
                desugar_if_stmt_list(else_body, next_id, params, locals, fn_return_types);
            let else_assign = match else_result {
                Some(r) => {
                    let (new_r, r_pre, _) =
                        desugar_if_operand(*r, next_id, params, locals, fn_return_types);
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
        // Recursively desugar IfExpr inside nested operands
        Operand::BinOp(op, lhs, rhs) => {
            let (new_lhs, mut pre, c1) =
                desugar_if_operand(*lhs, next_id, params, locals, fn_return_types);
            let (new_rhs, pre2, c2) =
                desugar_if_operand(*rhs, next_id, params, locals, fn_return_types);
            pre.extend(pre2);
            (
                Operand::BinOp(op, Box::new(new_lhs), Box::new(new_rhs)),
                pre,
                c1 + c2,
            )
        }
        Operand::UnaryOp(op, inner) => {
            let (new_inner, pre, c) =
                desugar_if_operand(*inner, next_id, params, locals, fn_return_types);
            (Operand::UnaryOp(op, Box::new(new_inner)), pre, c)
        }
        Operand::Call(name, args) => {
            let (new_args, pre, c) =
                desugar_if_operand_list(args, next_id, params, locals, fn_return_types);
            (Operand::Call(name, new_args), pre, c)
        }
        Operand::FieldAccess {
            object,
            struct_name,
            field,
        } => {
            let (new_obj, pre, c) =
                desugar_if_operand(*object, next_id, params, locals, fn_return_types);
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
            let (new_inner, pre, c) =
                desugar_if_operand(*inner, next_id, params, locals, fn_return_types);
            (Operand::EnumTag(Box::new(new_inner)), pre, c)
        }
        Operand::EnumPayload {
            object,
            index,
            enum_name,
            variant_name,
        } => {
            let (new_obj, pre, c) =
                desugar_if_operand(*object, next_id, params, locals, fn_return_types);
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
            let (new_callee, mut pre, c1) =
                desugar_if_operand(*callee, next_id, params, locals, fn_return_types);
            let (new_args, pre2, c2) =
                desugar_if_operand_list(args, next_id, params, locals, fn_return_types);
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
        // Leave all other operands (including LoopExpr, TryExpr) untouched
        other => (other, vec![], 0),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::mir::{
        BasicBlock, BlockId, FnId, InstanceKey, MirFunction, default_block_source,
        default_function_source,
    };

    #[test]
    fn desugar_tryexpr_with_from_wraps_converted_err_in_result() {
        let mut func = MirFunction {
            id: FnId(0),
            name: "process".to_string(),
            instance: InstanceKey::simple("process"),
            params: Vec::new(),
            return_ty: Type::Result(Box::new(Type::I32), Box::new(Type::Any)),
            locals: vec![
                MirLocal {
                    id: LocalId(0),
                    name: Some("input".to_string()),
                    ty: Type::Result(Box::new(Type::String), Box::new(Type::String)),
                },
                MirLocal {
                    id: LocalId(1),
                    name: Some("value".to_string()),
                    ty: Type::String,
                },
            ],
            blocks: vec![BasicBlock {
                id: BlockId(0),
                stmts: vec![MirStmt::Assign(
                    Place::Local(LocalId(1)),
                    Rvalue::Use(Operand::TryExpr {
                        expr: Box::new(Operand::Place(Place::Local(LocalId(0)))),
                        from_fn: Some("AppError__from".to_string()),
                    }),
                )],
                terminator: Terminator::Return(None),
                source: default_block_source(),
            }],
            entry: BlockId(0),
            struct_typed_locals: Default::default(),
            enum_typed_locals: Default::default(),
            type_params: vec![],
            source: default_function_source(),
            is_exported: false,
        };

        let mut fn_return_types = HashMap::new();
        fn_return_types.insert("AppError__from".to_string(), Type::Any);

        assert_eq!(desugar_exprs(&mut func, &fn_return_types), 1);

        let then_body = func.blocks[0]
            .stmts
            .iter()
            .find_map(|stmt| match stmt {
                MirStmt::IfStmt { then_body, .. } => Some(then_body),
                _ => None,
            })
            .expect("desugared try expression should emit an IfStmt");

        match then_body.as_slice() {
            [
                MirStmt::Return(Some(Operand::EnumInit {
                    enum_name,
                    variant,
                    tag,
                    payload,
                })),
            ] => {
                assert_eq!(enum_name, "Result");
                assert_eq!(variant, "Err");
                assert_eq!(*tag, 1);
                match payload.as_slice() {
                    [Operand::Call(name, args)] => {
                        assert_eq!(name, "AppError__from");
                        assert!(matches!(
                            args.as_slice(),
                            [Operand::EnumPayload { variant_name, .. }] if variant_name == "Err"
                        ));
                    }
                    other => panic!("expected converted payload call, got {other:?}"),
                }
            }
            other => panic!("expected Err-wrapped early return, got {other:?}"),
        }
    }
}
