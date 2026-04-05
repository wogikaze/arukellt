use crate::mir::{MirStmt, Operand, Place, Rvalue, Terminator};

/// Collect the set of local IDs that are the *destination* of any assignment
/// (direct `Assign`, or a `Call`/`CallBuiltin` with a dest) anywhere in
/// `stmts`, including nested loop and if bodies.
pub(crate) fn collect_assigned_locals(stmts: &[MirStmt]) -> std::collections::HashSet<u32> {
    let mut assigned = std::collections::HashSet::new();
    collect_assigned_locals_impl(stmts, &mut assigned);
    assigned
}

fn collect_assigned_locals_impl(stmts: &[MirStmt], assigned: &mut std::collections::HashSet<u32>) {
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

pub(crate) fn rewrite_stmt_with_replacements(
    stmt: &mut MirStmt,
    replacements: &std::collections::HashMap<u32, Operand>,
) -> bool {
    let mut changed = false;
    match stmt {
        MirStmt::Assign(_, rvalue) => changed |= rewrite_rvalue(rvalue, replacements),
        MirStmt::Call { args, .. } | MirStmt::CallBuiltin { args, .. } => {
            for arg in args {
                changed |= rewrite_operand(arg, replacements);
            }
        }
        MirStmt::IfStmt {
            cond,
            then_body,
            else_body,
        } => {
            changed |= rewrite_operand(cond, replacements);
            for stmt in then_body {
                changed |= rewrite_stmt_with_replacements(stmt, replacements);
            }
            for stmt in else_body {
                changed |= rewrite_stmt_with_replacements(stmt, replacements);
            }
        }
        MirStmt::WhileStmt { cond, body } => {
            // Do not propagate constants/copies for variables that are written
            // inside the loop body.  A pre-loop value such as `i = 0` must not
            // be substituted into the loop condition or body, because the loop
            // modifies `i` on every iteration.
            let loop_modified = collect_assigned_locals(body);
            let safe: std::collections::HashMap<u32, Operand> = replacements
                .iter()
                .filter(|(k, _)| !loop_modified.contains(*k))
                .map(|(k, v)| (*k, v.clone()))
                .collect();
            changed |= rewrite_operand(cond, &safe);
            for stmt in body {
                changed |= rewrite_stmt_with_replacements(stmt, &safe);
            }
        }
        MirStmt::Break | MirStmt::Continue => {}
        MirStmt::Return(value) => {
            if let Some(value) = value {
                changed |= rewrite_operand(value, replacements);
            }
        }
        MirStmt::GcHint { .. } => {}
    }
    changed
}

pub(crate) fn rewrite_terminator_with_replacements(
    terminator: &mut Terminator,
    replacements: &std::collections::HashMap<u32, Operand>,
) -> bool {
    match terminator {
        Terminator::If { cond, .. } => rewrite_operand(cond, replacements),
        Terminator::Switch { scrutinee, .. } => rewrite_operand(scrutinee, replacements),
        Terminator::Return(value) => value
            .as_mut()
            .is_some_and(|value| rewrite_operand(value, replacements)),
        Terminator::Goto(_) | Terminator::Unreachable => false,
    }
}

fn rewrite_rvalue(
    rvalue: &mut Rvalue,
    replacements: &std::collections::HashMap<u32, Operand>,
) -> bool {
    match rvalue {
        Rvalue::Use(operand) => rewrite_operand(operand, replacements),
        Rvalue::BinaryOp(_, lhs, rhs) => {
            rewrite_operand(lhs, replacements) | rewrite_operand(rhs, replacements)
        }
        Rvalue::UnaryOp(_, operand) => rewrite_operand(operand, replacements),
        Rvalue::Aggregate(_, operands) => operands.iter_mut().fold(false, |changed, operand| {
            rewrite_operand(operand, replacements) || changed
        }),
        Rvalue::Ref(place) => rewrite_place(place, replacements),
    }
}

fn rewrite_place(
    place: &mut Place,
    replacements: &std::collections::HashMap<u32, Operand>,
) -> bool {
    match place {
        Place::Local(_) => false,
        Place::Field(place, _) => rewrite_place(place, replacements),
        Place::Index(place, index) => {
            rewrite_place(place, replacements) | rewrite_operand(index, replacements)
        }
    }
}

fn rewrite_operand(
    operand: &mut Operand,
    replacements: &std::collections::HashMap<u32, Operand>,
) -> bool {
    match operand {
        Operand::Place(Place::Local(local)) => {
            if let Some(replacement) = replacements.get(&local.0) {
                *operand = replacement.clone();
                return true;
            }
            false
        }
        Operand::BinOp(_, lhs, rhs) => {
            rewrite_operand(lhs, replacements) | rewrite_operand(rhs, replacements)
        }
        Operand::UnaryOp(_, operand)
        | Operand::EnumTag(operand)
        | Operand::FieldAccess {
            object: operand, ..
        } => rewrite_operand(operand, replacements),
        Operand::Call(_, args) | Operand::ArrayInit { elements: args } => {
            args.iter_mut().fold(false, |changed, operand| {
                rewrite_operand(operand, replacements) || changed
            })
        }
        Operand::IfExpr {
            cond,
            then_body,
            then_result,
            else_body,
            else_result,
        } => {
            let mut changed = rewrite_operand(cond, replacements);
            for stmt in then_body {
                changed |= rewrite_stmt_with_replacements(stmt, replacements);
            }
            if let Some(result) = then_result {
                changed |= rewrite_operand(result, replacements);
            }
            for stmt in else_body {
                changed |= rewrite_stmt_with_replacements(stmt, replacements);
            }
            if let Some(result) = else_result {
                changed |= rewrite_operand(result, replacements);
            }
            changed
        }
        Operand::StructInit { fields, .. } => {
            fields.iter_mut().fold(false, |changed, (_, operand)| {
                rewrite_operand(operand, replacements) || changed
            })
        }
        Operand::EnumInit { payload, .. } => payload.iter_mut().fold(false, |changed, operand| {
            rewrite_operand(operand, replacements) || changed
        }),
        Operand::EnumPayload { object, .. } => rewrite_operand(object, replacements),
        Operand::LoopExpr { init, body, result } => {
            let mut changed = rewrite_operand(init, replacements);
            for stmt in body {
                changed |= rewrite_stmt_with_replacements(stmt, replacements);
            }
            changed |= rewrite_operand(result, replacements);
            changed
        }
        Operand::TryExpr { expr, .. } => rewrite_operand(expr, replacements),
        Operand::CallIndirect { callee, args } => {
            let mut changed = rewrite_operand(callee, replacements);
            for arg in args {
                changed |= rewrite_operand(arg, replacements);
            }
            changed
        }
        Operand::IndexAccess { object, index } => {
            rewrite_operand(object, replacements) | rewrite_operand(index, replacements)
        }
        Operand::Place(Place::Field(place, _)) => rewrite_place(place, replacements),
        Operand::Place(Place::Index(place, index)) => {
            rewrite_place(place, replacements) | rewrite_operand(index, replacements)
        }
        Operand::ConstI32(_)
        | Operand::ConstI64(_)
        | Operand::ConstF32(_)
        | Operand::ConstF64(_)
        | Operand::ConstU8(_)
        | Operand::ConstU16(_)
        | Operand::ConstU32(_)
        | Operand::ConstU64(_)
        | Operand::ConstI8(_)
        | Operand::ConstI16(_)
        | Operand::ConstBool(_)
        | Operand::ConstChar(_)
        | Operand::ConstString(_)
        | Operand::Unit
        | Operand::FnRef(_) => false,
    }
}

pub(crate) fn collect_stmt_locals(stmt: &MirStmt, used: &mut std::collections::HashSet<u32>) {
    match stmt {
        MirStmt::Assign(place, rvalue) => {
            collect_place_locals(place, used);
            collect_rvalue_locals(rvalue, used);
        }
        MirStmt::Call { args, .. } | MirStmt::CallBuiltin { args, .. } => {
            for arg in args {
                collect_operand_locals(arg, used);
            }
        }
        MirStmt::IfStmt {
            cond,
            then_body,
            else_body,
        } => {
            collect_operand_locals(cond, used);
            for stmt in then_body {
                collect_stmt_locals(stmt, used);
            }
            for stmt in else_body {
                collect_stmt_locals(stmt, used);
            }
        }
        MirStmt::WhileStmt { cond, body } => {
            collect_operand_locals(cond, used);
            for stmt in body {
                collect_stmt_locals(stmt, used);
            }
        }
        MirStmt::Break | MirStmt::Continue => {}
        MirStmt::Return(value) => {
            if let Some(value) = value {
                collect_operand_locals(value, used);
            }
        }
        MirStmt::GcHint { local, .. } => {
            used.insert(local.0);
        }
    }
}

pub(crate) fn collect_terminator_locals(terminator: &Terminator, used: &mut std::collections::HashSet<u32>) {
    match terminator {
        Terminator::If { cond, .. } => collect_operand_locals(cond, used),
        Terminator::Switch { scrutinee, .. } => collect_operand_locals(scrutinee, used),
        Terminator::Return(value) => {
            if let Some(value) = value {
                collect_operand_locals(value, used);
            }
        }
        Terminator::Goto(_) | Terminator::Unreachable => {}
    }
}

fn collect_rvalue_locals(rvalue: &Rvalue, used: &mut std::collections::HashSet<u32>) {
    match rvalue {
        Rvalue::Use(operand) => collect_operand_locals(operand, used),
        Rvalue::BinaryOp(_, lhs, rhs) => {
            collect_operand_locals(lhs, used);
            collect_operand_locals(rhs, used);
        }
        Rvalue::UnaryOp(_, operand) => collect_operand_locals(operand, used),
        Rvalue::Aggregate(_, operands) => {
            for operand in operands {
                collect_operand_locals(operand, used);
            }
        }
        Rvalue::Ref(place) => collect_place_locals(place, used),
    }
}

fn collect_place_locals(place: &Place, used: &mut std::collections::HashSet<u32>) {
    match place {
        Place::Local(local) => {
            used.insert(local.0);
        }
        Place::Field(place, _) => collect_place_locals(place, used),
        Place::Index(place, index) => {
            collect_place_locals(place, used);
            collect_operand_locals(index, used);
        }
    }
}

fn collect_operand_locals(operand: &Operand, used: &mut std::collections::HashSet<u32>) {
    match operand {
        Operand::Place(place) => collect_place_locals(place, used),
        Operand::BinOp(_, lhs, rhs) => {
            collect_operand_locals(lhs, used);
            collect_operand_locals(rhs, used);
        }
        Operand::UnaryOp(_, operand)
        | Operand::EnumTag(operand)
        | Operand::FieldAccess {
            object: operand, ..
        } => collect_operand_locals(operand, used),
        Operand::Call(_, args) | Operand::ArrayInit { elements: args } => {
            for arg in args {
                collect_operand_locals(arg, used);
            }
        }
        Operand::IfExpr {
            cond,
            then_body,
            then_result,
            else_body,
            else_result,
        } => {
            collect_operand_locals(cond, used);
            for stmt in then_body {
                collect_stmt_locals(stmt, used);
            }
            if let Some(result) = then_result {
                collect_operand_locals(result, used);
            }
            for stmt in else_body {
                collect_stmt_locals(stmt, used);
            }
            if let Some(result) = else_result {
                collect_operand_locals(result, used);
            }
        }
        Operand::StructInit { fields, .. } => {
            for (_, operand) in fields {
                collect_operand_locals(operand, used);
            }
        }
        Operand::EnumInit { payload, .. } => {
            for operand in payload {
                collect_operand_locals(operand, used);
            }
        }
        Operand::EnumPayload { object, .. } => collect_operand_locals(object, used),
        Operand::LoopExpr { init, body, result } => {
            collect_operand_locals(init, used);
            for stmt in body {
                collect_stmt_locals(stmt, used);
            }
            collect_operand_locals(result, used);
        }
        Operand::TryExpr { expr, .. } => collect_operand_locals(expr, used),
        Operand::CallIndirect { callee, args } => {
            collect_operand_locals(callee, used);
            for arg in args {
                collect_operand_locals(arg, used);
            }
        }
        Operand::IndexAccess { object, index } => {
            collect_operand_locals(object, used);
            collect_operand_locals(index, used);
        }
        Operand::ConstI32(_)
        | Operand::ConstI64(_)
        | Operand::ConstF32(_)
        | Operand::ConstF64(_)
        | Operand::ConstU8(_)
        | Operand::ConstU16(_)
        | Operand::ConstU32(_)
        | Operand::ConstU64(_)
        | Operand::ConstI8(_)
        | Operand::ConstI16(_)
        | Operand::ConstBool(_)
        | Operand::ConstChar(_)
        | Operand::ConstString(_)
        | Operand::Unit
        | Operand::FnRef(_) => {}
    }
}
