use crate::mir::{LocalId, MirFunction, MirLocal, MirStmt, Operand, Place, Rvalue, Terminator};
use ark_typecheck::types::Type;
use std::collections::{HashMap, HashSet};

use super::OptimizationSummary;

/// Information about a struct allocation candidate for scalar replacement.
struct StructCandidate {
    /// The field names in declaration order.
    fields: Vec<String>,
    /// Whether this allocation escapes the current function.
    escapes: bool,
}

/// MIR pass: escape analysis + scalar replacement of aggregates (SROA).
///
/// Identifies struct allocations that don't escape the current function and
/// replaces them with individual scalar locals — one per field.
pub fn escape_analysis_pass(func: &mut MirFunction) -> OptimizationSummary {
    let mut summary = OptimizationSummary::default();

    // Phase 1: Find struct allocation candidates.
    let mut candidates: HashMap<u32, StructCandidate> = HashMap::new();

    for block in &func.blocks {
        for stmt in &block.stmts {
            if let MirStmt::Assign(Place::Local(dest), Rvalue::Use(Operand::StructInit { name: _, fields })) = stmt {
                candidates.insert(
                    dest.0,
                    StructCandidate {
                        fields: fields.iter().map(|(n, _)| n.clone()).collect(),
                        escapes: false,
                    },
                );
            }
        }
    }

    if candidates.is_empty() {
        return summary;
    }

    // Phase 2: Escape analysis — mark candidates that escape.
    let candidate_ids: HashSet<u32> = candidates.keys().copied().collect();

    for block in &func.blocks {
        for stmt in &block.stmts {
            mark_escaping_in_stmt(stmt, &candidate_ids, &mut candidates);
        }
        mark_escaping_in_terminator(&block.terminator, &candidate_ids, &mut candidates);
    }

    // Collect non-escaping candidates.
    let non_escaping: HashMap<u32, &StructCandidate> = candidates
        .iter()
        .filter(|(_, c)| !c.escapes)
        .map(|(id, c)| (*id, c))
        .collect();

    if non_escaping.is_empty() {
        return summary;
    }

    // Phase 3: Create scalar locals for each non-escaping struct's fields.
    let mut next_id = func
        .locals
        .iter()
        .chain(func.params.iter())
        .map(|l| l.id.0)
        .max()
        .unwrap_or(0)
        + 1;

    // Maps (original_local_id, field_name) → new scalar LocalId
    let mut scalar_map: HashMap<(u32, String), LocalId> = HashMap::new();

    for (&local_id, candidate) in &non_escaping {
        let orig_name = func
            .locals
            .iter()
            .find(|l| l.id.0 == local_id)
            .and_then(|l| l.name.clone());
        let base = orig_name.unwrap_or_else(|| format!("_s{}", local_id));

        for field_name in &candidate.fields {
            let new_local = MirLocal {
                id: LocalId(next_id),
                name: Some(format!("{}_{}", base, field_name)),
                ty: Type::Any,
            };
            scalar_map.insert((local_id, field_name.clone()), LocalId(next_id));
            func.locals.push(new_local);
            next_id += 1;
        }
    }

    // Phase 4: Rewrite statements.
    let replaced = rewrite_function(func, &non_escaping, &scalar_map);
    summary.scalar_replaced = replaced;

    summary
}

/// Check whether an operand uses a candidate local in a way that is NOT a
/// simple `FieldAccess` on that local. If so, the local escapes.
fn operand_escapes(
    operand: &Operand,
    candidate_ids: &HashSet<u32>,
    candidates: &mut HashMap<u32, StructCandidate>,
) {
    match operand {
        // A bare reference to a candidate local (not wrapped in FieldAccess)
        // means escape — it's used as a whole value.
        Operand::Place(Place::Local(id)) if candidate_ids.contains(&id.0) => {
            // Bare local use (not FieldAccess) → escapes, BUT we handle FieldAccess
            // as a separate arm below, so reaching here means it's a non-field use.
            // However, we need to be careful: this arm is also matched for reads that
            // become FieldAccess in FieldAccess's object sub-operand.
            // The FieldAccess arm handles its own object recursion, so a bare
            // Place::Local inside FieldAccess is fine — but here it means a raw use.
            if let Some(c) = candidates.get_mut(&id.0) {
                c.escapes = true;
            }
        }
        Operand::FieldAccess { object, .. } => {
            // A FieldAccess on a candidate is fine (non-escaping).
            // But we need to check the object: if the object is Place::Local(candidate),
            // that's the expected pattern. If it's something else, recurse.
            if let Operand::Place(Place::Local(id)) = object.as_ref() {
                if candidate_ids.contains(&id.0) {
                    // This is a valid field access — does not escape.
                    return;
                }
            }
            operand_escapes(object, candidate_ids, candidates);
        }
        Operand::BinOp(_, lhs, rhs) => {
            operand_escapes(lhs, candidate_ids, candidates);
            operand_escapes(rhs, candidate_ids, candidates);
        }
        Operand::UnaryOp(_, inner) | Operand::EnumTag(inner) => {
            operand_escapes(inner, candidate_ids, candidates);
        }
        Operand::Call(_, args) | Operand::ArrayInit { elements: args } => {
            for arg in args {
                operand_escapes(arg, candidate_ids, candidates);
            }
        }
        Operand::CallIndirect { callee, args } => {
            operand_escapes(callee, candidate_ids, candidates);
            for arg in args {
                operand_escapes(arg, candidate_ids, candidates);
            }
        }
        Operand::StructInit { fields, .. } => {
            for (_, val) in fields {
                // If a candidate local is stored into another struct's field, it escapes.
                mark_operand_escapes_if_candidate(val, candidate_ids, candidates);
            }
        }
        Operand::EnumInit { payload, .. } => {
            for p in payload {
                mark_operand_escapes_if_candidate(p, candidate_ids, candidates);
            }
        }
        Operand::EnumPayload { object, .. } => {
            operand_escapes(object, candidate_ids, candidates);
        }
        Operand::IndexAccess { object, index } => {
            operand_escapes(object, candidate_ids, candidates);
            operand_escapes(index, candidate_ids, candidates);
        }
        Operand::IfExpr {
            cond,
            then_body,
            then_result,
            else_body,
            else_result,
        } => {
            operand_escapes(cond, candidate_ids, candidates);
            for s in then_body {
                mark_escaping_in_stmt(s, candidate_ids, candidates);
            }
            if let Some(r) = then_result {
                operand_escapes(r, candidate_ids, candidates);
            }
            for s in else_body {
                mark_escaping_in_stmt(s, candidate_ids, candidates);
            }
            if let Some(r) = else_result {
                operand_escapes(r, candidate_ids, candidates);
            }
        }
        Operand::LoopExpr { init, body, result } => {
            operand_escapes(init, candidate_ids, candidates);
            for s in body {
                mark_escaping_in_stmt(s, candidate_ids, candidates);
            }
            operand_escapes(result, candidate_ids, candidates);
        }
        Operand::TryExpr { expr, .. } => {
            operand_escapes(expr, candidate_ids, candidates);
        }
        Operand::Place(Place::Field(inner_place, _)) => {
            place_escapes(inner_place, candidate_ids, candidates);
        }
        Operand::Place(Place::Index(inner_place, idx)) => {
            place_escapes(inner_place, candidate_ids, candidates);
            operand_escapes(idx, candidate_ids, candidates);
        }
        // Constants, FnRef, Unit — no locals referenced.
        _ => {}
    }
}

/// If an operand is a bare reference to a candidate, mark it as escaping.
fn mark_operand_escapes_if_candidate(
    operand: &Operand,
    candidate_ids: &HashSet<u32>,
    candidates: &mut HashMap<u32, StructCandidate>,
) {
    match operand {
        Operand::Place(Place::Local(id)) if candidate_ids.contains(&id.0) => {
            if let Some(c) = candidates.get_mut(&id.0) {
                c.escapes = true;
            }
        }
        _ => operand_escapes(operand, candidate_ids, candidates),
    }
}

fn place_escapes(
    place: &Place,
    candidate_ids: &HashSet<u32>,
    candidates: &mut HashMap<u32, StructCandidate>,
) {
    match place {
        Place::Local(id) if candidate_ids.contains(&id.0) => {
            if let Some(c) = candidates.get_mut(&id.0) {
                c.escapes = true;
            }
        }
        Place::Field(inner, _) => place_escapes(inner, candidate_ids, candidates),
        Place::Index(inner, idx) => {
            place_escapes(inner, candidate_ids, candidates);
            operand_escapes(idx, candidate_ids, candidates);
        }
        _ => {}
    }
}

fn mark_escaping_in_stmt(
    stmt: &MirStmt,
    candidate_ids: &HashSet<u32>,
    candidates: &mut HashMap<u32, StructCandidate>,
) {
    match stmt {
        MirStmt::Assign(place, rvalue) => {
            // Check lhs: writing into a field of a candidate is OK, but
            // assigning the candidate itself to a non-local place escapes it.
            match place {
                Place::Field(inner, _) => {
                    // Writing to candidate.field is fine for the candidate in `inner`.
                    // But if `inner` is not a candidate local, recurse.
                    if let Place::Local(id) = inner.as_ref() {
                        if candidate_ids.contains(&id.0) {
                            // Writing to a field of a non-escaping candidate — OK.
                        } else {
                            place_escapes(inner, candidate_ids, candidates);
                        }
                    } else {
                        place_escapes(inner, candidate_ids, candidates);
                    }
                }
                Place::Index(inner, idx) => {
                    place_escapes(inner, candidate_ids, candidates);
                    operand_escapes(idx, candidate_ids, candidates);
                }
                Place::Local(_) => {
                    // Assigning to a plain local is fine (this is the StructInit def site).
                }
            }
            // Check rhs.
            mark_escaping_in_rvalue(rvalue, candidate_ids, candidates);
        }
        MirStmt::Call { args, .. } | MirStmt::CallBuiltin { args, .. } => {
            // Any candidate passed as argument escapes.
            for arg in args {
                mark_operand_escapes_if_candidate(arg, candidate_ids, candidates);
            }
        }
        MirStmt::Return(Some(val)) => {
            mark_operand_escapes_if_candidate(val, candidate_ids, candidates);
        }
        MirStmt::IfStmt {
            cond,
            then_body,
            else_body,
        } => {
            operand_escapes(cond, candidate_ids, candidates);
            for s in then_body {
                mark_escaping_in_stmt(s, candidate_ids, candidates);
            }
            for s in else_body {
                mark_escaping_in_stmt(s, candidate_ids, candidates);
            }
        }
        MirStmt::WhileStmt { cond, body } => {
            operand_escapes(cond, candidate_ids, candidates);
            for s in body {
                mark_escaping_in_stmt(s, candidate_ids, candidates);
            }
        }
        MirStmt::Return(None) | MirStmt::Break | MirStmt::Continue | MirStmt::GcHint { .. } => {}
    }
}

fn mark_escaping_in_rvalue(
    rvalue: &Rvalue,
    candidate_ids: &HashSet<u32>,
    candidates: &mut HashMap<u32, StructCandidate>,
) {
    match rvalue {
        Rvalue::Use(operand) => operand_escapes(operand, candidate_ids, candidates),
        Rvalue::BinaryOp(_, lhs, rhs) => {
            operand_escapes(lhs, candidate_ids, candidates);
            operand_escapes(rhs, candidate_ids, candidates);
        }
        Rvalue::UnaryOp(_, operand) => operand_escapes(operand, candidate_ids, candidates),
        Rvalue::Aggregate(_, operands) => {
            for op in operands {
                mark_operand_escapes_if_candidate(op, candidate_ids, candidates);
            }
        }
        Rvalue::Ref(place) => {
            // Taking a reference to a candidate → escapes.
            if let Place::Local(id) = place {
                if candidate_ids.contains(&id.0) {
                    if let Some(c) = candidates.get_mut(&id.0) {
                        c.escapes = true;
                    }
                }
            }
            place_escapes(place, candidate_ids, candidates);
        }
    }
}

fn mark_escaping_in_terminator(
    terminator: &Terminator,
    candidate_ids: &HashSet<u32>,
    candidates: &mut HashMap<u32, StructCandidate>,
) {
    match terminator {
        Terminator::Return(Some(val)) => {
            mark_operand_escapes_if_candidate(val, candidate_ids, candidates);
        }
        Terminator::If { cond, .. } => {
            operand_escapes(cond, candidate_ids, candidates);
        }
        Terminator::Switch { scrutinee, .. } => {
            operand_escapes(scrutinee, candidate_ids, candidates);
        }
        Terminator::Return(None) | Terminator::Goto(_) | Terminator::Unreachable => {}
    }
}

// ---------------------------------------------------------------------------
// Phase 4: Rewrite
// ---------------------------------------------------------------------------

fn rewrite_function(
    func: &mut MirFunction,
    non_escaping: &HashMap<u32, &StructCandidate>,
    scalar_map: &HashMap<(u32, String), LocalId>,
) -> usize {
    let mut count = 0;
    for block in &mut func.blocks {
        let mut new_stmts: Vec<MirStmt> = Vec::with_capacity(block.stmts.len());
        for stmt in block.stmts.drain(..) {
            count += rewrite_stmt(stmt, non_escaping, scalar_map, &mut new_stmts);
        }
        block.stmts = new_stmts;

        rewrite_terminator_operands(&mut block.terminator, non_escaping, scalar_map);
    }
    count
}

/// Rewrite a single statement. Returns the number of scalar replacements performed.
fn rewrite_stmt(
    stmt: MirStmt,
    non_escaping: &HashMap<u32, &StructCandidate>,
    scalar_map: &HashMap<(u32, String), LocalId>,
    out: &mut Vec<MirStmt>,
) -> usize {
    match stmt {
        // Replace StructInit assignment with individual scalar assignments.
        MirStmt::Assign(
            Place::Local(dest),
            Rvalue::Use(Operand::StructInit { name: _, fields }),
        ) if non_escaping.contains_key(&dest.0) => {
            for (field_name, value) in fields {
                if let Some(&scalar_id) = scalar_map.get(&(dest.0, field_name)) {
                    let rewritten_value = rewrite_operand_deep(value, non_escaping, scalar_map);
                    out.push(MirStmt::Assign(
                        Place::Local(scalar_id),
                        Rvalue::Use(rewritten_value),
                    ));
                }
            }
            1
        }
        // Replace writes to candidate.field with writes to the scalar local.
        MirStmt::Assign(Place::Field(inner_place, field_name), rvalue)
            if matches!(inner_place.as_ref(), Place::Local(id) if non_escaping.contains_key(&id.0)) =>
        {
            let local_id = match inner_place.as_ref() {
                Place::Local(id) => id.0,
                _ => unreachable!(),
            };
            if let Some(&scalar_id) = scalar_map.get(&(local_id, field_name.clone())) {
                let rewritten_rvalue = rewrite_rvalue_deep(rvalue, non_escaping, scalar_map);
                out.push(MirStmt::Assign(Place::Local(scalar_id), rewritten_rvalue));
                return 1;
            }
            out.push(MirStmt::Assign(
                Place::Field(inner_place, field_name),
                rvalue,
            ));
            0
        }
        // For all other assignments, rewrite operands within.
        MirStmt::Assign(place, rvalue) => {
            let rvalue = rewrite_rvalue_deep(rvalue, non_escaping, scalar_map);
            out.push(MirStmt::Assign(place, rvalue));
            0
        }
        MirStmt::Call { dest, func: fn_id, args } => {
            let args = args
                .into_iter()
                .map(|a| rewrite_operand_deep(a, non_escaping, scalar_map))
                .collect();
            out.push(MirStmt::Call { dest, func: fn_id, args });
            0
        }
        MirStmt::CallBuiltin { dest, name, args } => {
            let args = args
                .into_iter()
                .map(|a| rewrite_operand_deep(a, non_escaping, scalar_map))
                .collect();
            out.push(MirStmt::CallBuiltin { dest, name, args });
            0
        }
        MirStmt::IfStmt {
            cond,
            then_body,
            else_body,
        } => {
            let cond = rewrite_operand_deep(cond, non_escaping, scalar_map);
            let then_body = rewrite_stmts_vec(then_body, non_escaping, scalar_map);
            let else_body = rewrite_stmts_vec(else_body, non_escaping, scalar_map);
            out.push(MirStmt::IfStmt {
                cond,
                then_body,
                else_body,
            });
            0
        }
        MirStmt::WhileStmt { cond, body } => {
            let cond = rewrite_operand_deep(cond, non_escaping, scalar_map);
            let body = rewrite_stmts_vec(body, non_escaping, scalar_map);
            out.push(MirStmt::WhileStmt { cond, body });
            0
        }
        MirStmt::Return(Some(val)) => {
            let val = rewrite_operand_deep(val, non_escaping, scalar_map);
            out.push(MirStmt::Return(Some(val)));
            0
        }
        other => {
            out.push(other);
            0
        }
    }
}

fn rewrite_stmts_vec(
    stmts: Vec<MirStmt>,
    non_escaping: &HashMap<u32, &StructCandidate>,
    scalar_map: &HashMap<(u32, String), LocalId>,
) -> Vec<MirStmt> {
    let mut out = Vec::with_capacity(stmts.len());
    for s in stmts {
        rewrite_stmt(s, non_escaping, scalar_map, &mut out);
    }
    out
}

fn rewrite_rvalue_deep(
    rvalue: Rvalue,
    non_escaping: &HashMap<u32, &StructCandidate>,
    scalar_map: &HashMap<(u32, String), LocalId>,
) -> Rvalue {
    match rvalue {
        Rvalue::Use(op) => Rvalue::Use(rewrite_operand_deep(op, non_escaping, scalar_map)),
        Rvalue::BinaryOp(binop, lhs, rhs) => Rvalue::BinaryOp(
            binop,
            rewrite_operand_deep(lhs, non_escaping, scalar_map),
            rewrite_operand_deep(rhs, non_escaping, scalar_map),
        ),
        Rvalue::UnaryOp(unop, op) => {
            Rvalue::UnaryOp(unop, rewrite_operand_deep(op, non_escaping, scalar_map))
        }
        Rvalue::Aggregate(kind, ops) => Rvalue::Aggregate(
            kind,
            ops.into_iter()
                .map(|o| rewrite_operand_deep(o, non_escaping, scalar_map))
                .collect(),
        ),
        Rvalue::Ref(p) => Rvalue::Ref(p),
    }
}

/// Rewrite a single operand, replacing `FieldAccess` on non-escaping candidates
/// with a read from the corresponding scalar local.
fn rewrite_operand_deep(
    operand: Operand,
    non_escaping: &HashMap<u32, &StructCandidate>,
    scalar_map: &HashMap<(u32, String), LocalId>,
) -> Operand {
    match operand {
        Operand::FieldAccess {
            object,
            struct_name,
            field,
        } => {
            if let Operand::Place(Place::Local(id)) = object.as_ref() {
                if non_escaping.contains_key(&id.0) {
                    if let Some(&scalar_id) = scalar_map.get(&(id.0, field.clone())) {
                        return Operand::Place(Place::Local(scalar_id));
                    }
                }
            }
            // Not a candidate field access — recurse into object.
            Operand::FieldAccess {
                object: Box::new(rewrite_operand_deep(*object, non_escaping, scalar_map)),
                struct_name,
                field,
            }
        }
        Operand::BinOp(op, lhs, rhs) => Operand::BinOp(
            op,
            Box::new(rewrite_operand_deep(*lhs, non_escaping, scalar_map)),
            Box::new(rewrite_operand_deep(*rhs, non_escaping, scalar_map)),
        ),
        Operand::UnaryOp(op, inner) => Operand::UnaryOp(
            op,
            Box::new(rewrite_operand_deep(*inner, non_escaping, scalar_map)),
        ),
        Operand::Call(name, args) => Operand::Call(
            name,
            args.into_iter()
                .map(|a| rewrite_operand_deep(a, non_escaping, scalar_map))
                .collect(),
        ),
        Operand::CallIndirect { callee, args } => Operand::CallIndirect {
            callee: Box::new(rewrite_operand_deep(*callee, non_escaping, scalar_map)),
            args: args
                .into_iter()
                .map(|a| rewrite_operand_deep(a, non_escaping, scalar_map))
                .collect(),
        },
        Operand::StructInit { name, fields } => Operand::StructInit {
            name,
            fields: fields
                .into_iter()
                .map(|(n, v)| (n, rewrite_operand_deep(v, non_escaping, scalar_map)))
                .collect(),
        },
        Operand::EnumInit {
            enum_name,
            variant,
            tag,
            payload,
        } => Operand::EnumInit {
            enum_name,
            variant,
            tag,
            payload: payload
                .into_iter()
                .map(|p| rewrite_operand_deep(p, non_escaping, scalar_map))
                .collect(),
        },
        Operand::EnumTag(inner) => {
            Operand::EnumTag(Box::new(rewrite_operand_deep(*inner, non_escaping, scalar_map)))
        }
        Operand::EnumPayload {
            object,
            index,
            enum_name,
            variant_name,
        } => Operand::EnumPayload {
            object: Box::new(rewrite_operand_deep(*object, non_escaping, scalar_map)),
            index,
            enum_name,
            variant_name,
        },
        Operand::ArrayInit { elements } => Operand::ArrayInit {
            elements: elements
                .into_iter()
                .map(|e| rewrite_operand_deep(e, non_escaping, scalar_map))
                .collect(),
        },
        Operand::IndexAccess { object, index } => Operand::IndexAccess {
            object: Box::new(rewrite_operand_deep(*object, non_escaping, scalar_map)),
            index: Box::new(rewrite_operand_deep(*index, non_escaping, scalar_map)),
        },
        Operand::IfExpr {
            cond,
            then_body,
            then_result,
            else_body,
            else_result,
        } => Operand::IfExpr {
            cond: Box::new(rewrite_operand_deep(*cond, non_escaping, scalar_map)),
            then_body: rewrite_stmts_vec(then_body, non_escaping, scalar_map),
            then_result: then_result
                .map(|r| Box::new(rewrite_operand_deep(*r, non_escaping, scalar_map))),
            else_body: rewrite_stmts_vec(else_body, non_escaping, scalar_map),
            else_result: else_result
                .map(|r| Box::new(rewrite_operand_deep(*r, non_escaping, scalar_map))),
        },
        Operand::LoopExpr { init, body, result } => Operand::LoopExpr {
            init: Box::new(rewrite_operand_deep(*init, non_escaping, scalar_map)),
            body: rewrite_stmts_vec(body, non_escaping, scalar_map),
            result: Box::new(rewrite_operand_deep(*result, non_escaping, scalar_map)),
        },
        Operand::TryExpr { expr, from_fn } => Operand::TryExpr {
            expr: Box::new(rewrite_operand_deep(*expr, non_escaping, scalar_map)),
            from_fn,
        },
        // Place, constants, FnRef, Unit — no rewriting needed.
        other => other,
    }
}

fn rewrite_terminator_operands(
    terminator: &mut Terminator,
    non_escaping: &HashMap<u32, &StructCandidate>,
    scalar_map: &HashMap<(u32, String), LocalId>,
) {
    match terminator {
        Terminator::Return(Some(val)) => {
            let taken = std::mem::replace(val, Operand::Unit);
            *val = rewrite_operand_deep(taken, non_escaping, scalar_map);
        }
        Terminator::If { cond, .. } => {
            let taken = std::mem::replace(cond, Operand::Unit);
            *cond = rewrite_operand_deep(taken, non_escaping, scalar_map);
        }
        Terminator::Switch { scrutinee, .. } => {
            let taken = std::mem::replace(scrutinee, Operand::Unit);
            *scrutinee = rewrite_operand_deep(taken, non_escaping, scalar_map);
        }
        Terminator::Return(None) | Terminator::Goto(_) | Terminator::Unreachable => {}
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::mir::{
        BasicBlock, BlockId, FnId, LocalId, MirFunction, MirLocal, MirStmt, Operand, Place,
        Rvalue, SourceInfo, Terminator,
    };
    use ark_typecheck::types::Type;

    fn make_instance() -> crate::mir::InstanceKey {
        crate::mir::InstanceKey {
            item: String::new(),
            substitution: vec![],
            target_shape: String::new(),
        }
    }

    #[test]
    fn test_non_escaping_struct_is_scalar_replaced() {
        // let tmp = Point { x: 1, y: 2 }
        // let a = tmp.x
        // let b = tmp.y
        let mut func = MirFunction {
            id: FnId(0),
            name: "test".into(),
            instance: make_instance(),
            params: vec![],
            return_ty: Type::Unit,
            locals: vec![
                MirLocal { id: LocalId(0), name: Some("tmp".into()), ty: Type::I32 },
                MirLocal { id: LocalId(1), name: Some("a".into()), ty: Type::I32 },
                MirLocal { id: LocalId(2), name: Some("b".into()), ty: Type::I32 },
            ],
            blocks: vec![BasicBlock {
                id: BlockId(0),
                stmts: vec![
                    MirStmt::Assign(
                        Place::Local(LocalId(0)),
                        Rvalue::Use(Operand::StructInit {
                            name: "Point".into(),
                            fields: vec![
                                ("x".into(), Operand::ConstI32(1)),
                                ("y".into(), Operand::ConstI32(2)),
                            ],
                        }),
                    ),
                    MirStmt::Assign(
                        Place::Local(LocalId(1)),
                        Rvalue::Use(Operand::FieldAccess {
                            object: Box::new(Operand::Place(Place::Local(LocalId(0)))),
                            struct_name: "Point".into(),
                            field: "x".into(),
                        }),
                    ),
                    MirStmt::Assign(
                        Place::Local(LocalId(2)),
                        Rvalue::Use(Operand::FieldAccess {
                            object: Box::new(Operand::Place(Place::Local(LocalId(0)))),
                            struct_name: "Point".into(),
                            field: "y".into(),
                        }),
                    ),
                ],
                terminator: Terminator::Return(None),
                source: SourceInfo::unknown(),
            }],
            entry: BlockId(0),
            struct_typed_locals: Default::default(),
            enum_typed_locals: Default::default(),
            type_params: vec![],
            source: SourceInfo::unknown(),
            is_exported: false,
        };

        let summary = escape_analysis_pass(&mut func);
        assert_eq!(summary.scalar_replaced, 1);

        // The StructInit should be replaced with two scalar assignments.
        let stmts = &func.blocks[0].stmts;
        // Two assignments for StructInit fields + two reads rewritten.
        assert_eq!(stmts.len(), 4);

        // First two stmts: assign to scalar locals (tmp_x, tmp_y).
        // Next two stmts: reads from scalar locals instead of FieldAccess.
        for stmt in stmts {
            if let MirStmt::Assign(_, Rvalue::Use(op)) = stmt {
                // No FieldAccess on LocalId(0) should remain.
                assert!(!matches!(op, Operand::FieldAccess { .. }));
            }
        }
    }

    #[test]
    fn test_escaping_struct_is_not_replaced() {
        // let tmp = Point { x: 1, y: 2 }
        // return tmp  ← escapes!
        let mut func = MirFunction {
            id: FnId(0),
            name: "test".into(),
            instance: make_instance(),
            params: vec![],
            return_ty: Type::I32,
            locals: vec![MirLocal {
                id: LocalId(0),
                name: Some("tmp".into()),
                ty: Type::I32,
            }],
            blocks: vec![BasicBlock {
                id: BlockId(0),
                stmts: vec![MirStmt::Assign(
                    Place::Local(LocalId(0)),
                    Rvalue::Use(Operand::StructInit {
                        name: "Point".into(),
                        fields: vec![
                            ("x".into(), Operand::ConstI32(1)),
                            ("y".into(), Operand::ConstI32(2)),
                        ],
                    }),
                )],
                terminator: Terminator::Return(Some(Operand::Place(Place::Local(LocalId(0))))),
                source: SourceInfo::unknown(),
            }],
            entry: BlockId(0),
            struct_typed_locals: Default::default(),
            enum_typed_locals: Default::default(),
            type_params: vec![],
            source: SourceInfo::unknown(),
            is_exported: false,
        };

        let summary = escape_analysis_pass(&mut func);
        assert_eq!(summary.scalar_replaced, 0);

        // StructInit should remain untouched.
        assert!(matches!(
            &func.blocks[0].stmts[0],
            MirStmt::Assign(_, Rvalue::Use(Operand::StructInit { .. }))
        ));
    }

    #[test]
    fn test_struct_passed_to_call_escapes() {
        let mut func = MirFunction {
            id: FnId(0),
            name: "test".into(),
            instance: make_instance(),
            params: vec![],
            return_ty: Type::Unit,
            locals: vec![MirLocal {
                id: LocalId(0),
                name: Some("tmp".into()),
                ty: Type::I32,
            }],
            blocks: vec![BasicBlock {
                id: BlockId(0),
                stmts: vec![
                    MirStmt::Assign(
                        Place::Local(LocalId(0)),
                        Rvalue::Use(Operand::StructInit {
                            name: "Point".into(),
                            fields: vec![
                                ("x".into(), Operand::ConstI32(1)),
                                ("y".into(), Operand::ConstI32(2)),
                            ],
                        }),
                    ),
                    MirStmt::Call {
                        dest: None,
                        func: FnId(1),
                        args: vec![Operand::Place(Place::Local(LocalId(0)))],
                    },
                ],
                terminator: Terminator::Return(None),
                source: SourceInfo::unknown(),
            }],
            entry: BlockId(0),
            struct_typed_locals: Default::default(),
            enum_typed_locals: Default::default(),
            type_params: vec![],
            source: SourceInfo::unknown(),
            is_exported: false,
        };

        let summary = escape_analysis_pass(&mut func);
        assert_eq!(summary.scalar_replaced, 0);
    }
}
