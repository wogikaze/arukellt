//! Struct field layout optimization (opt_level >= 2).
//!
//! Reorders struct fields by descending access frequency so that
//! hot fields get lower Wasm GC field indices, improving JIT locality.

use ark_mir::mir::*;
use std::collections::HashMap;

/// Scan reachable MIR functions and compute a field reorder map.
///
/// Returns `struct_name → permutation` where `permutation[old_idx] = new_idx`.
/// Only structs with 3+ fields are reordered (1-2 fields aren't worth it).
pub(super) fn compute_field_reorder(
    mir: &MirModule,
    reachable: &[usize],
    struct_defs: &HashMap<String, Vec<(String, String)>>,
) -> HashMap<String, Vec<usize>> {
    // (struct_name, field_name) → access count
    let mut freq: HashMap<(String, String), usize> = HashMap::new();

    for &idx in reachable {
        let func = &mir.functions[idx];
        for block in &func.blocks {
            for stmt in &block.stmts {
                count_stmt(&mut freq, stmt, &func.struct_typed_locals);
            }
            count_terminator(&mut freq, &block.terminator);
        }
    }

    let mut remap = HashMap::new();
    for (sname, fields) in struct_defs {
        if fields.len() < 3 {
            continue;
        }

        // Build (old_index, frequency) pairs
        let mut indexed: Vec<(usize, usize)> = fields
            .iter()
            .enumerate()
            .map(|(i, (fname, _))| {
                let count = freq
                    .get(&(sname.clone(), fname.clone()))
                    .copied()
                    .unwrap_or(0);
                (i, count)
            })
            .collect();

        // Sort by descending frequency, stable (preserves def order for ties)
        indexed.sort_by(|a, b| b.1.cmp(&a.1));

        // Check if the sort actually changed anything
        let is_identity = indexed
            .iter()
            .enumerate()
            .all(|(new, (old, _))| new == *old);
        if is_identity {
            continue;
        }

        // Build permutation: perm[old_idx] = new_idx
        let mut perm = vec![0usize; fields.len()];
        for (new_idx, (old_idx, _)) in indexed.iter().enumerate() {
            perm[*old_idx] = new_idx;
        }
        remap.insert(sname.clone(), perm);
    }

    remap
}

/// Apply a field reorder to a struct layout, returning the reordered layout.
pub(super) fn reorder_layout(
    layout: &[(String, String)],
    remap: &[usize],
) -> Vec<(String, String)> {
    let mut reordered = vec![("".to_string(), "".to_string()); layout.len()];
    for (old_idx, entry) in layout.iter().enumerate() {
        reordered[remap[old_idx]] = entry.clone();
    }
    reordered
}

// ── Counting helpers ─────────────────────────────────────────────

fn count_stmt(
    freq: &mut HashMap<(String, String), usize>,
    stmt: &MirStmt,
    struct_locals: &std::collections::HashMap<u32, String>,
) {
    match stmt {
        MirStmt::Assign(place, rvalue) => {
            count_place(freq, place, struct_locals);
            match rvalue {
                Rvalue::Use(op) => count_operand(freq, op),
                Rvalue::BinaryOp(_, a, b) => {
                    count_operand(freq, a);
                    count_operand(freq, b);
                }
                Rvalue::UnaryOp(_, a) => count_operand(freq, a),
                Rvalue::Aggregate(_, ops) => {
                    for o in ops {
                        count_operand(freq, o);
                    }
                }
                Rvalue::Ref(p) => count_place(freq, p, struct_locals),
            }
        }
        MirStmt::Call { args, .. } => {
            for a in args {
                count_operand(freq, a);
            }
        }
        MirStmt::CallBuiltin { args, .. } => {
            for a in args {
                count_operand(freq, a);
            }
        }
        MirStmt::IfStmt {
            cond,
            then_body,
            else_body,
        } => {
            count_operand(freq, cond);
            for s in then_body {
                count_stmt(freq, s, struct_locals);
            }
            for s in else_body {
                count_stmt(freq, s, struct_locals);
            }
        }
        MirStmt::WhileStmt { cond, body } => {
            count_operand(freq, cond);
            for s in body {
                count_stmt(freq, s, struct_locals);
            }
        }
        MirStmt::Return(Some(op)) => count_operand(freq, op),
        MirStmt::GcHint { .. } | MirStmt::Break | MirStmt::Continue | MirStmt::Return(None) => {}
    }
}

fn count_terminator(freq: &mut HashMap<(String, String), usize>, term: &Terminator) {
    match term {
        Terminator::If { cond, .. } => count_operand(freq, cond),
        Terminator::Switch { scrutinee, .. } => count_operand(freq, scrutinee),
        Terminator::Return(Some(op)) => count_operand(freq, op),
        Terminator::TailCall { args, .. } => {
            for arg in args {
                count_operand(freq, arg);
            }
        }
        Terminator::TailCallIndirect { callee, args } => {
            count_operand(freq, callee);
            for arg in args {
                count_operand(freq, arg);
            }
        }
        Terminator::Goto(_) | Terminator::Return(None) | Terminator::Unreachable => {}
    }
}

fn count_operand(freq: &mut HashMap<(String, String), usize>, op: &Operand) {
    match op {
        Operand::FieldAccess {
            object,
            struct_name,
            field,
        } => {
            *freq
                .entry((struct_name.clone(), field.clone()))
                .or_insert(0) += 1;
            count_operand(freq, object);
        }
        Operand::StructInit { fields, .. } => {
            for (_, val) in fields {
                count_operand(freq, val);
            }
        }
        Operand::BinOp(_, a, b) => {
            count_operand(freq, a);
            count_operand(freq, b);
        }
        Operand::UnaryOp(_, a) => count_operand(freq, a),
        Operand::Call(_, args) => {
            for a in args {
                count_operand(freq, a);
            }
        }
        Operand::IfExpr {
            cond,
            then_body,
            then_result,
            else_body,
            else_result,
        } => {
            count_operand(freq, cond);
            for s in then_body {
                count_stmt_no_locals(freq, s);
            }
            if let Some(r) = then_result {
                count_operand(freq, r);
            }
            for s in else_body {
                count_stmt_no_locals(freq, s);
            }
            if let Some(r) = else_result {
                count_operand(freq, r);
            }
        }
        Operand::LoopExpr {
            init, body, result, ..
        } => {
            count_operand(freq, init);
            for s in body {
                count_stmt_no_locals(freq, s);
            }
            count_operand(freq, result);
        }
        Operand::TryExpr { expr, .. } => count_operand(freq, expr),
        Operand::CallIndirect { callee, args } => {
            count_operand(freq, callee);
            for a in args {
                count_operand(freq, a);
            }
        }
        Operand::ArrayInit { elements } => {
            for e in elements {
                count_operand(freq, e);
            }
        }
        Operand::IndexAccess { object, index } => {
            count_operand(freq, object);
            count_operand(freq, index);
        }
        Operand::EnumInit { payload, .. } => {
            for p in payload {
                count_operand(freq, p);
            }
        }
        Operand::EnumTag(inner) | Operand::EnumPayload { object: inner, .. } => {
            count_operand(freq, inner);
        }
        Operand::Place(place) => {
            count_place_no_locals(freq, place);
        }
        // Leaf operands — no field accesses
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
        | Operand::FnRef(_) => {}
    }
}

fn count_place(
    freq: &mut HashMap<(String, String), usize>,
    place: &Place,
    struct_locals: &std::collections::HashMap<u32, String>,
) {
    match place {
        Place::Field(inner, field_name) => {
            // Resolve struct name from the inner place's local
            if let Place::Local(id) = inner.as_ref()
                && let Some(sname) = struct_locals.get(&id.0)
            {
                *freq.entry((sname.clone(), field_name.clone())).or_insert(0) += 1;
            }
            count_place(freq, inner, struct_locals);
        }
        Place::Index(inner, idx_op) => {
            count_place(freq, inner, struct_locals);
            count_operand(freq, idx_op);
        }
        Place::Local(_) => {}
    }
}

/// Count places without struct-local context (used inside operand expressions).
fn count_place_no_locals(freq: &mut HashMap<(String, String), usize>, place: &Place) {
    match place {
        Place::Field(inner, _) => count_place_no_locals(freq, inner),
        Place::Index(inner, idx_op) => {
            count_place_no_locals(freq, inner);
            count_operand(freq, idx_op);
        }
        Place::Local(_) => {}
    }
}

/// Count stmt without struct_typed_locals context.
fn count_stmt_no_locals(freq: &mut HashMap<(String, String), usize>, stmt: &MirStmt) {
    let empty = std::collections::HashMap::new();
    count_stmt(freq, stmt, &empty);
}

#[cfg(test)]
mod tests {
    use super::{compute_field_reorder, reorder_layout};
    use ark_mir::mir::*;
    use ark_typecheck::types::Type;
    use std::collections::HashMap;

    fn point_module(stmts: Vec<MirStmt>) -> MirModule {
        let mut mir = MirModule::new();
        let layout = vec![
            ("cold_a".to_string(), "i32".to_string()),
            ("hot".to_string(), "i32".to_string()),
            ("cold_b".to_string(), "i32".to_string()),
        ];
        mir.type_table
            .struct_defs
            .insert("Point".to_string(), layout.clone());
        mir.struct_defs.insert("Point".to_string(), layout);
        mir.functions.push(MirFunction {
            id: FnId(0),
            name: "main".to_string(),
            instance: InstanceKey::simple("main"),
            params: vec![],
            return_ty: Type::Unit,
            locals: vec![
                MirLocal {
                    id: LocalId(0),
                    name: Some("point".to_string()),
                    ty: Type::I32,
                },
                MirLocal {
                    id: LocalId(1),
                    name: Some("tmp".to_string()),
                    ty: Type::I32,
                },
            ],
            blocks: vec![BasicBlock {
                id: BlockId(0),
                stmts,
                terminator: Terminator::Return(None),
                source: SourceInfo::unknown(),
            }],
            entry: BlockId(0),
            struct_typed_locals: HashMap::from([(0, "Point".to_string())]),
            enum_typed_locals: HashMap::new(),
            type_params: vec![],
            source: SourceInfo::unknown(),
            is_exported: false,
        });
        mir.entry_fn = Some(FnId(0));
        mir
    }

    fn hot_field_read() -> MirStmt {
        MirStmt::Assign(
            Place::Local(LocalId(1)),
            Rvalue::Use(Operand::FieldAccess {
                object: Box::new(Operand::Place(Place::Local(LocalId(0)))),
                struct_name: "Point".to_string(),
                field: "hot".to_string(),
            }),
        )
    }

    #[test]
    fn compute_field_reorder_moves_hot_field_to_front() {
        let mir = point_module(vec![
            MirStmt::Assign(
                Place::Local(LocalId(0)),
                Rvalue::Use(Operand::StructInit {
                    name: "Point".to_string(),
                    fields: vec![
                        ("cold_a".to_string(), Operand::ConstI32(1)),
                        ("hot".to_string(), Operand::ConstI32(2)),
                        ("cold_b".to_string(), Operand::ConstI32(3)),
                    ],
                }),
            ),
            hot_field_read(),
            hot_field_read(),
            hot_field_read(),
            MirStmt::Assign(
                Place::Local(LocalId(1)),
                Rvalue::Use(Operand::FieldAccess {
                    object: Box::new(Operand::Place(Place::Local(LocalId(0)))),
                    struct_name: "Point".to_string(),
                    field: "cold_a".to_string(),
                }),
            ),
        ]);

        let remap = compute_field_reorder(&mir, &[0], &mir.type_table.struct_defs);
        let point_remap = remap.get("Point").expect("Point remap");
        assert_eq!(point_remap, &vec![1, 0, 2]);

        let reordered = reorder_layout(&mir.type_table.struct_defs["Point"], point_remap);
        let field_names: Vec<&str> = reordered.iter().map(|(name, _)| name.as_str()).collect();
        assert_eq!(field_names, vec!["hot", "cold_a", "cold_b"]);
    }

    #[test]
    fn compute_field_reorder_is_stable_for_equal_frequencies() {
        let mir = point_module(vec![
            MirStmt::Assign(
                Place::Local(LocalId(0)),
                Rvalue::Use(Operand::StructInit {
                    name: "Point".to_string(),
                    fields: vec![
                        ("cold_a".to_string(), Operand::ConstI32(1)),
                        ("hot".to_string(), Operand::ConstI32(2)),
                        ("cold_b".to_string(), Operand::ConstI32(3)),
                    ],
                }),
            ),
            hot_field_read(),
            MirStmt::Assign(
                Place::Local(LocalId(1)),
                Rvalue::Use(Operand::FieldAccess {
                    object: Box::new(Operand::Place(Place::Local(LocalId(0)))),
                    struct_name: "Point".to_string(),
                    field: "cold_a".to_string(),
                }),
            ),
            MirStmt::Assign(
                Place::Local(LocalId(1)),
                Rvalue::Use(Operand::FieldAccess {
                    object: Box::new(Operand::Place(Place::Local(LocalId(0)))),
                    struct_name: "Point".to_string(),
                    field: "cold_b".to_string(),
                }),
            ),
        ]);

        let remap = compute_field_reorder(&mir, &[0], &mir.type_table.struct_defs);
        assert!(remap.is_empty(), "equal-frequency fields should keep definition order");
    }
}
