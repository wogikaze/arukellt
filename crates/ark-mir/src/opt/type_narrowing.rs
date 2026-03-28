//! Number type narrowing pass.
//!
//! Detects `i64` locals whose values provably fit in i32 range and narrows
//! their declared type to `i32`.  Only constant-assigned locals are handled;
//! function parameters, returned locals, and call arguments are never narrowed.

use crate::mir::{MirFunction, MirStmt, Operand, Place, Rvalue, Terminator};
use ark_typecheck::types::Type;
use std::collections::HashSet;

use super::pipeline::OptimizationSummary;

fn fits_i32(v: i64) -> bool {
    v >= (i32::MIN as i64) && v <= (i32::MAX as i64)
}

/// Top-level entry point called by the pipeline dispatcher.
pub(crate) fn type_narrowing(function: &mut MirFunction) -> OptimizationSummary {
    let mut summary = OptimizationSummary::default();

    let param_ids: HashSet<u32> = function.params.iter().map(|p| p.id.0).collect();

    // Identify initial candidates: i64 locals that are not parameters.
    let mut candidates: HashSet<u32> = function
        .locals
        .iter()
        .filter(|l| l.ty == Type::I64 && !param_ids.contains(&l.id.0))
        .map(|l| l.id.0)
        .collect();

    if candidates.is_empty() {
        return summary;
    }

    // Phase 1: keep only locals whose *every* assignment is an i32-range constant.
    for block in &function.blocks {
        remove_non_constant_assigned(&block.stmts, &mut candidates);
    }

    if candidates.is_empty() {
        return summary;
    }

    // Phase 2: exclude locals with unsafe uses (fixpoint until stable).
    loop {
        let before = candidates.len();
        for block in &function.blocks {
            exclude_unsafe_in_stmts(&block.stmts, &mut candidates, function);
            exclude_unsafe_in_terminator(&block.terminator, &mut candidates);
        }
        if candidates.len() == before || candidates.is_empty() {
            break;
        }
    }

    if candidates.is_empty() {
        return summary;
    }

    // Phase 3: apply narrowing — change type and rewrite constants.
    let count = candidates.len();
    for local in &mut function.locals {
        if candidates.contains(&local.id.0) {
            local.ty = Type::I32;
        }
    }
    for block in &mut function.blocks {
        rewrite_stmts(&mut block.stmts, &candidates);
    }

    summary.types_narrowed = count;
    summary
}

// ── Phase 1 helpers ─────────────────────────────────────────────────────────

/// Remove any candidate whose assignment source is not an i32-range constant.
fn remove_non_constant_assigned(stmts: &[MirStmt], candidates: &mut HashSet<u32>) {
    for stmt in stmts {
        match stmt {
            MirStmt::Assign(place, rvalue) => {
                if let Some(lid) = local_of(place) {
                    if candidates.contains(&lid) && !is_i32_range_constant(rvalue) {
                        candidates.remove(&lid);
                    }
                }
            }
            // A local assigned from a call cannot be narrowed.
            MirStmt::Call {
                dest: Some(place), ..
            }
            | MirStmt::CallBuiltin {
                dest: Some(place), ..
            } => {
                if let Some(lid) = local_of(place) {
                    candidates.remove(&lid);
                }
            }
            MirStmt::IfStmt {
                then_body,
                else_body,
                ..
            } => {
                remove_non_constant_assigned(then_body, candidates);
                remove_non_constant_assigned(else_body, candidates);
            }
            MirStmt::WhileStmt { body, .. } => {
                remove_non_constant_assigned(body, candidates);
            }
            _ => {}
        }
    }
}

fn is_i32_range_constant(rvalue: &Rvalue) -> bool {
    match rvalue {
        Rvalue::Use(Operand::ConstI64(v)) => fits_i32(*v),
        Rvalue::Use(Operand::ConstI32(_)) => true,
        _ => false,
    }
}

// ── Phase 2 helpers ─────────────────────────────────────────────────────────

/// Exclude candidates that appear in unsafe read positions.
fn exclude_unsafe_in_stmts(
    stmts: &[MirStmt],
    candidates: &mut HashSet<u32>,
    func: &MirFunction,
) {
    for stmt in stmts {
        match stmt {
            MirStmt::Assign(dest, rvalue) => {
                // Place-level reads (e.g. Index) are unsafe.
                exclude_place_reads(dest, candidates);
                exclude_unsafe_in_rvalue(rvalue, dest, candidates, func);
            }
            MirStmt::Call { args, .. } | MirStmt::CallBuiltin { args, .. } => {
                for arg in args {
                    exclude_in_operand(arg, candidates);
                }
            }
            MirStmt::Return(Some(op)) => {
                exclude_in_operand(op, candidates);
            }
            MirStmt::IfStmt {
                cond,
                then_body,
                else_body,
            } => {
                exclude_in_operand(cond, candidates);
                exclude_unsafe_in_stmts(then_body, candidates, func);
                exclude_unsafe_in_stmts(else_body, candidates, func);
            }
            MirStmt::WhileStmt { cond, body } => {
                exclude_in_operand(cond, candidates);
                exclude_unsafe_in_stmts(body, candidates, func);
            }
            _ => {}
        }
    }
}

fn exclude_unsafe_in_terminator(term: &Terminator, candidates: &mut HashSet<u32>) {
    match term {
        Terminator::Return(Some(op)) => exclude_in_operand(op, candidates),
        Terminator::If { cond, .. } => exclude_in_operand(cond, candidates),
        Terminator::Switch { scrutinee, .. } => exclude_in_operand(scrutinee, candidates),
        _ => {}
    }
}

fn exclude_unsafe_in_rvalue(
    rvalue: &Rvalue,
    dest: &Place,
    candidates: &mut HashSet<u32>,
    func: &MirFunction,
) {
    match rvalue {
        Rvalue::Use(op) => {
            // A simple copy `dest = Use(Place(Local(id)))` is safe only if the
            // destination is also a candidate or already i32-typed.
            if let Some(src_lid) = operand_local(op) {
                if candidates.contains(&src_lid) {
                    if let Some(dst_lid) = local_of(dest) {
                        let dst_ty = find_local_type(func, dst_lid);
                        if dst_ty != Some(&Type::I32) && !candidates.contains(&dst_lid) {
                            candidates.remove(&src_lid);
                        }
                    } else {
                        // Destination is a field/index projection — not safe.
                        candidates.remove(&src_lid);
                    }
                }
            }
            // For non-local operands (nested exprs), conservatively exclude.
            if operand_local(op).is_none() {
                exclude_in_operand(op, candidates);
            }
        }
        Rvalue::BinaryOp(_, lhs, rhs) => {
            exclude_in_operand(lhs, candidates);
            exclude_in_operand(rhs, candidates);
        }
        Rvalue::UnaryOp(_, op) => {
            exclude_in_operand(op, candidates);
        }
        Rvalue::Aggregate(_, ops) => {
            for op in ops {
                exclude_in_operand(op, candidates);
            }
        }
        Rvalue::Ref(place) => {
            if let Some(lid) = local_of(place) {
                candidates.remove(&lid);
            }
        }
    }
}

/// Recursively remove any candidate local referenced in an operand tree.
fn exclude_in_operand(op: &Operand, candidates: &mut HashSet<u32>) {
    match op {
        Operand::Place(place) => {
            if let Some(lid) = local_of(place) {
                candidates.remove(&lid);
            }
            exclude_place_reads(place, candidates);
        }
        Operand::BinOp(_, lhs, rhs) => {
            exclude_in_operand(lhs, candidates);
            exclude_in_operand(rhs, candidates);
        }
        Operand::UnaryOp(_, inner) => {
            exclude_in_operand(inner, candidates);
        }
        Operand::Call(_, args) | Operand::ArrayInit { elements: args } => {
            for a in args {
                exclude_in_operand(a, candidates);
            }
        }
        Operand::CallIndirect { callee, args } => {
            exclude_in_operand(callee, candidates);
            for a in args {
                exclude_in_operand(a, candidates);
            }
        }
        Operand::IfExpr {
            cond,
            then_result,
            else_result,
            then_body,
            else_body,
        } => {
            exclude_in_operand(cond, candidates);
            if let Some(r) = then_result {
                exclude_in_operand(r, candidates);
            }
            if let Some(r) = else_result {
                exclude_in_operand(r, candidates);
            }
            exclude_in_nested_stmts(then_body, candidates);
            exclude_in_nested_stmts(else_body, candidates);
        }
        Operand::FieldAccess { object, .. }
        | Operand::EnumTag(object)
        | Operand::TryExpr { expr: object, .. } => {
            exclude_in_operand(object, candidates);
        }
        Operand::EnumPayload { object, .. } => {
            exclude_in_operand(object, candidates);
        }
        Operand::EnumInit { payload, .. } => {
            for p in payload {
                exclude_in_operand(p, candidates);
            }
        }
        Operand::StructInit { fields, .. } => {
            for (_, v) in fields {
                exclude_in_operand(v, candidates);
            }
        }
        Operand::IndexAccess { object, index } => {
            exclude_in_operand(object, candidates);
            exclude_in_operand(index, candidates);
        }
        Operand::LoopExpr {
            init,
            body,
            result,
        } => {
            exclude_in_operand(init, candidates);
            exclude_in_nested_stmts(body, candidates);
            exclude_in_operand(result, candidates);
        }
        Operand::FnRef(_)
        | Operand::ConstI32(_)
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
        | Operand::Unit => {}
    }
}

fn exclude_in_nested_stmts(stmts: &[MirStmt], candidates: &mut HashSet<u32>) {
    for stmt in stmts {
        match stmt {
            MirStmt::Assign(place, rvalue) => {
                exclude_place_reads(place, candidates);
                exclude_in_rvalue_simple(rvalue, candidates);
            }
            MirStmt::Call { args, .. } | MirStmt::CallBuiltin { args, .. } => {
                for a in args {
                    exclude_in_operand(a, candidates);
                }
            }
            MirStmt::Return(Some(op)) => exclude_in_operand(op, candidates),
            MirStmt::IfStmt {
                cond,
                then_body,
                else_body,
            } => {
                exclude_in_operand(cond, candidates);
                exclude_in_nested_stmts(then_body, candidates);
                exclude_in_nested_stmts(else_body, candidates);
            }
            MirStmt::WhileStmt { cond, body } => {
                exclude_in_operand(cond, candidates);
                exclude_in_nested_stmts(body, candidates);
            }
            _ => {}
        }
    }
}

fn exclude_in_rvalue_simple(rvalue: &Rvalue, candidates: &mut HashSet<u32>) {
    match rvalue {
        Rvalue::Use(op) => exclude_in_operand(op, candidates),
        Rvalue::BinaryOp(_, lhs, rhs) => {
            exclude_in_operand(lhs, candidates);
            exclude_in_operand(rhs, candidates);
        }
        Rvalue::UnaryOp(_, op) => exclude_in_operand(op, candidates),
        Rvalue::Aggregate(_, ops) => {
            for op in ops {
                exclude_in_operand(op, candidates);
            }
        }
        Rvalue::Ref(place) => {
            if let Some(lid) = local_of(place) {
                candidates.remove(&lid);
            }
        }
    }
}

/// Exclude candidates read through place projections (Field, Index).
fn exclude_place_reads(place: &Place, candidates: &mut HashSet<u32>) {
    match place {
        Place::Local(_) => {}
        Place::Field(base, _) => {
            if let Some(lid) = local_of(base) {
                candidates.remove(&lid);
            }
            exclude_place_reads(base, candidates);
        }
        Place::Index(base, idx) => {
            if let Some(lid) = local_of(base) {
                candidates.remove(&lid);
            }
            exclude_place_reads(base, candidates);
            exclude_in_operand(idx, candidates);
        }
    }
}

// ── Phase 3 helpers ─────────────────────────────────────────────────────────

/// Rewrite `ConstI64(v)` → `ConstI32(v as i32)` in assignments to narrowed locals.
fn rewrite_stmts(stmts: &mut [MirStmt], narrowed: &HashSet<u32>) {
    for stmt in stmts.iter_mut() {
        match stmt {
            MirStmt::Assign(place, rvalue) => {
                if let Some(lid) = local_of(place) {
                    if narrowed.contains(&lid) {
                        rewrite_rvalue(rvalue);
                    }
                }
            }
            MirStmt::IfStmt {
                then_body,
                else_body,
                ..
            } => {
                rewrite_stmts(then_body, narrowed);
                rewrite_stmts(else_body, narrowed);
            }
            MirStmt::WhileStmt { body, .. } => {
                rewrite_stmts(body, narrowed);
            }
            _ => {}
        }
    }
}

fn rewrite_rvalue(rvalue: &mut Rvalue) {
    if let Rvalue::Use(Operand::ConstI64(v)) = rvalue {
        *rvalue = Rvalue::Use(Operand::ConstI32(*v as i32));
    }
}

// ── Utilities ───────────────────────────────────────────────────────────────

fn local_of(place: &Place) -> Option<u32> {
    match place {
        Place::Local(id) => Some(id.0),
        _ => None,
    }
}

fn operand_local(op: &Operand) -> Option<u32> {
    match op {
        Operand::Place(Place::Local(id)) => Some(id.0),
        _ => None,
    }
}

fn find_local_type(func: &MirFunction, lid: u32) -> Option<&Type> {
    func.locals
        .iter()
        .chain(func.params.iter())
        .find(|l| l.id.0 == lid)
        .map(|l| &l.ty)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::mir::{
        BasicBlock, BlockId, FnId, InstanceKey, LocalId, MirLocal, MirModule, SourceInfo,
    };

    fn make_function(locals: Vec<MirLocal>, stmts: Vec<MirStmt>) -> MirFunction {
        MirFunction {
            id: FnId(0),
            name: "test".into(),
            instance: InstanceKey::simple("test"),
            params: vec![],
            return_ty: Type::Unit,
            locals,
            blocks: vec![BasicBlock {
                id: BlockId(0),
                stmts,
                terminator: Terminator::Return(None),
                source: SourceInfo::default(),
            }],
            entry: BlockId(0),
            struct_typed_locals: Default::default(),
            enum_typed_locals: Default::default(),
            type_params: vec![],
            source: SourceInfo::default(),
            is_exported: false,
        }
    }

    #[test]
    fn narrows_i64_constant_in_range() {
        let locals = vec![MirLocal {
            id: LocalId(0),
            name: Some("x".into()),
            ty: Type::I64,
        }];
        let stmts = vec![MirStmt::Assign(
            Place::Local(LocalId(0)),
            Rvalue::Use(Operand::ConstI64(42)),
        )];
        let mut func = make_function(locals, stmts);
        let summary = type_narrowing(&mut func);

        assert_eq!(summary.types_narrowed, 1);
        assert_eq!(func.locals[0].ty, Type::I32);
        // The constant should be rewritten.
        if let MirStmt::Assign(_, Rvalue::Use(Operand::ConstI32(v))) = &func.blocks[0].stmts[0] {
            assert_eq!(*v, 42);
        } else {
            panic!("expected ConstI32 after narrowing");
        }
    }

    #[test]
    fn does_not_narrow_out_of_range() {
        let locals = vec![MirLocal {
            id: LocalId(0),
            name: None,
            ty: Type::I64,
        }];
        let stmts = vec![MirStmt::Assign(
            Place::Local(LocalId(0)),
            Rvalue::Use(Operand::ConstI64(i64::MAX)),
        )];
        let mut func = make_function(locals, stmts);
        let summary = type_narrowing(&mut func);

        assert_eq!(summary.types_narrowed, 0);
        assert_eq!(func.locals[0].ty, Type::I64);
    }

    #[test]
    fn does_not_narrow_params() {
        let mut func = make_function(vec![], vec![]);
        func.params.push(MirLocal {
            id: LocalId(0),
            name: Some("p".into()),
            ty: Type::I64,
        });
        func.blocks[0].stmts.push(MirStmt::Assign(
            Place::Local(LocalId(0)),
            Rvalue::Use(Operand::ConstI64(1)),
        ));
        let summary = type_narrowing(&mut func);
        assert_eq!(summary.types_narrowed, 0);
    }

    #[test]
    fn does_not_narrow_returned_local() {
        let locals = vec![MirLocal {
            id: LocalId(0),
            name: None,
            ty: Type::I64,
        }];
        let stmts = vec![
            MirStmt::Assign(
                Place::Local(LocalId(0)),
                Rvalue::Use(Operand::ConstI64(5)),
            ),
            MirStmt::Return(Some(Operand::Place(Place::Local(LocalId(0))))),
        ];
        let mut func = make_function(locals, stmts);
        let summary = type_narrowing(&mut func);
        assert_eq!(summary.types_narrowed, 0);
    }

    #[test]
    fn does_not_narrow_call_argument() {
        let locals = vec![MirLocal {
            id: LocalId(0),
            name: None,
            ty: Type::I64,
        }];
        let stmts = vec![
            MirStmt::Assign(
                Place::Local(LocalId(0)),
                Rvalue::Use(Operand::ConstI64(10)),
            ),
            MirStmt::Call {
                dest: None,
                func: FnId(1),
                args: vec![Operand::Place(Place::Local(LocalId(0)))],
            },
        ];
        let mut func = make_function(locals, stmts);
        let summary = type_narrowing(&mut func);
        assert_eq!(summary.types_narrowed, 0);
    }

    #[test]
    fn does_not_narrow_non_constant_assignment() {
        let locals = vec![
            MirLocal {
                id: LocalId(0),
                name: None,
                ty: Type::I64,
            },
            MirLocal {
                id: LocalId(1),
                name: None,
                ty: Type::I64,
            },
        ];
        let stmts = vec![
            MirStmt::Assign(
                Place::Local(LocalId(0)),
                Rvalue::Use(Operand::ConstI64(1)),
            ),
            // Assigned from another local — not a constant.
            MirStmt::Assign(
                Place::Local(LocalId(0)),
                Rvalue::Use(Operand::Place(Place::Local(LocalId(1)))),
            ),
        ];
        let mut func = make_function(locals, stmts);
        let summary = type_narrowing(&mut func);
        assert_eq!(summary.types_narrowed, 0);
    }

    #[test]
    fn narrows_negative_in_range() {
        let locals = vec![MirLocal {
            id: LocalId(0),
            name: None,
            ty: Type::I64,
        }];
        let stmts = vec![MirStmt::Assign(
            Place::Local(LocalId(0)),
            Rvalue::Use(Operand::ConstI64(i32::MIN as i64)),
        )];
        let mut func = make_function(locals, stmts);
        let summary = type_narrowing(&mut func);

        assert_eq!(summary.types_narrowed, 1);
        assert_eq!(func.locals[0].ty, Type::I32);
    }

    #[test]
    fn pipeline_integration() {
        let locals = vec![MirLocal {
            id: LocalId(0),
            name: Some("x".into()),
            ty: Type::I64,
        }];
        let stmts = vec![MirStmt::Assign(
            Place::Local(LocalId(0)),
            Rvalue::Use(Operand::ConstI64(99)),
        )];
        let func = make_function(locals, stmts);
        let mut module = MirModule::default();
        module.functions.push(func);

        let summary =
            crate::opt::pipeline::run_single_pass(&mut module, crate::opt::OptimizationPass::TypeNarrowing)
                .unwrap();
        assert_eq!(summary.types_narrowed, 1);
    }
}
