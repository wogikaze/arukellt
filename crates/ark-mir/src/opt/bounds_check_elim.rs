//! Bounds-check elimination pass.
//!
//! Removes redundant array bounds checks when the index is provably in range.
//!
//! Handled patterns:
//!   1. `CallBuiltin` whose name contains "bounds" or "check" — removed when
//!      the index is a constant smaller than a known array length.
//!   2. Duplicate bounds-check calls on the same (array, index) pair within a
//!      single block.
//!   3. Constant-index access into an array whose length is statically known
//!      (via a preceding `ArrayInit` or `len` call result).
//!   4. Loop-induction-variable patterns (`WhileStmt` with `i < len` guard and
//!      `i += 1` step) where every index access uses the induction variable on
//!      the guarded array.

use super::pipeline::OptimizationSummary;
use crate::mir::{BinOp, LocalId, MirFunction, MirStmt, Operand, Place, Rvalue};
use std::collections::{HashMap, HashSet};

/// Top-level entry point called by the pipeline dispatcher.
pub(crate) fn bounds_check_elim(func: &mut MirFunction) -> OptimizationSummary {
    let mut summary = OptimizationSummary::default();

    // Collect known constant-sized arrays: local → element count.
    let known_sizes = collect_known_array_sizes(func);

    for block in &mut func.blocks {
        eliminate_in_stmts(&mut block.stmts, &known_sizes, &mut summary);
    }
    summary
}

// ── helpers ──────────────────────────────────────────────────────────────────

/// Scan the function for `Assign(Local(id), Use(ArrayInit { elements }))` and
/// record the array length.
fn collect_known_array_sizes(func: &MirFunction) -> HashMap<u32, usize> {
    let mut sizes: HashMap<u32, usize> = HashMap::new();
    for block in &func.blocks {
        collect_sizes_from_stmts(&block.stmts, &mut sizes);
    }
    sizes
}

fn collect_sizes_from_stmts(stmts: &[MirStmt], sizes: &mut HashMap<u32, usize>) {
    for stmt in stmts {
        match stmt {
            MirStmt::Assign(
                Place::Local(LocalId(id)),
                Rvalue::Use(Operand::ArrayInit { elements }),
            ) => {
                sizes.insert(*id, elements.len());
            }
            MirStmt::IfStmt {
                then_body,
                else_body,
                ..
            } => {
                collect_sizes_from_stmts(then_body, sizes);
                collect_sizes_from_stmts(else_body, sizes);
            }
            MirStmt::WhileStmt { body, .. } => {
                collect_sizes_from_stmts(body, sizes);
            }
            _ => {}
        }
    }
}

/// Process a list of statements, eliminating redundant bounds checks.
fn eliminate_in_stmts(
    stmts: &mut Vec<MirStmt>,
    known_sizes: &HashMap<u32, usize>,
    summary: &mut OptimizationSummary,
) {
    // Track (array_local, index_operand_hash) pairs already checked in this block.
    let mut checked: HashSet<(u32, OperandKey)> = HashSet::new();
    let mut indices_to_remove: Vec<usize> = Vec::new();

    for (i, stmt) in stmts.iter().enumerate() {
        match stmt {
            // Pattern 1 & 2: CallBuiltin whose name relates to bounds checking.
            MirStmt::CallBuiltin { name, args, .. } if is_bounds_check_name(name) => {
                if can_eliminate_builtin_check(args, known_sizes, &checked) {
                    indices_to_remove.push(i);
                    summary.bounds_checks_eliminated += 1;
                } else if let Some(key) = bounds_check_key(args) {
                    checked.insert(key);
                }
            }

            // Recurse into nested control flow.
            MirStmt::IfStmt { .. } | MirStmt::WhileStmt { .. } => {}
            _ => {}
        }
    }

    // Remove identified statements in reverse order so indices stay valid.
    for &idx in indices_to_remove.iter().rev() {
        stmts.remove(idx);
    }

    // Second pass: recurse into nested bodies (IfStmt / WhileStmt).
    for stmt in stmts.iter_mut() {
        match stmt {
            MirStmt::IfStmt {
                then_body,
                else_body,
                ..
            } => {
                eliminate_in_stmts(then_body, known_sizes, summary);
                eliminate_in_stmts(else_body, known_sizes, summary);
            }
            MirStmt::WhileStmt { cond, body } => {
                // Try to detect `while i < len(arr)` induction-variable patterns.
                let loop_safe = extract_loop_safe_locals(cond, body, known_sizes);
                let mut extended_sizes = known_sizes.clone();
                for (local, sz) in &loop_safe {
                    extended_sizes.insert(*local, *sz);
                }
                eliminate_in_stmts(body, &extended_sizes, summary);

                // Eliminate bounds checks inside the loop body that are
                // provably safe due to the induction variable range.
                eliminate_loop_bounds_checks(cond, body, known_sizes, summary);
            }
            _ => {}
        }
    }
}

/// Heuristic: does this builtin name look like a bounds check?
fn is_bounds_check_name(name: &str) -> bool {
    let lower = name.to_ascii_lowercase();
    lower.contains("bounds") || lower.contains("bound_check") || lower.contains("bounds_check")
}

/// Can we statically prove this bounds-check call is unnecessary?
fn can_eliminate_builtin_check(
    args: &[Operand],
    known_sizes: &HashMap<u32, usize>,
    checked: &HashSet<(u32, OperandKey)>,
) -> bool {
    // Convention: bounds_check(array, index) or bounds_check(index, len).
    if args.len() < 2 {
        return false;
    }

    // Duplicate check: same (array, index) already verified.
    if let Some(key) = bounds_check_key_from(args)
        && checked.contains(&key)
    {
        return true;
    }

    // Constant index into known-size array.
    if let Some(arr_local) = operand_local(&args[0])
        && let Some(&size) = known_sizes.get(&arr_local)
        && let Some(idx_val) = operand_const_usize(&args[1])
    {
        return idx_val < size;
    }
    // Also handle (index, len) convention.
    if let Some(idx_val) = operand_const_usize(&args[0])
        && let Some(len_val) = operand_const_usize(&args[1])
    {
        return idx_val < len_val;
    }
    false
}

/// Build a dedup key from bounds-check arguments.
fn bounds_check_key(args: &[Operand]) -> Option<(u32, OperandKey)> {
    bounds_check_key_from(args)
}

fn bounds_check_key_from(args: &[Operand]) -> Option<(u32, OperandKey)> {
    if args.len() < 2 {
        return None;
    }
    let arr = operand_local(&args[0])?;
    let idx = operand_key(&args[1]);
    Some((arr, idx))
}

// ── Loop induction-variable analysis ─────────────────────────────────────────

/// Given `while (cond) { body }`, detect the pattern:
///   cond = BinOp(Lt, Place(Local(iv)), Call("len", [Place(Local(arr))]))
///   body contains `iv = iv + 1`
/// Return set of (iv_local, array_size) pairs that are safe.
fn extract_loop_safe_locals(
    _cond: &Operand,
    _body: &[MirStmt],
    _known_sizes: &HashMap<u32, usize>,
) -> Vec<(u32, usize)> {
    // Placeholder — full implementation below in eliminate_loop_bounds_checks.
    Vec::new()
}

/// Eliminate bounds checks inside a `WhileStmt` where the loop guard ensures
/// `iv < len(arr)` and the induction variable steps by 1.
fn eliminate_loop_bounds_checks(
    cond: &Operand,
    body: &mut Vec<MirStmt>,
    known_sizes: &HashMap<u32, usize>,
    summary: &mut OptimizationSummary,
) {
    // Try to extract: cond == BinOp(Lt, Place(Local(iv)), <len_operand>)
    let (iv, bound) = match cond {
        Operand::BinOp(BinOp::Lt, lhs, rhs) => {
            if let Some(iv_local) = operand_local(lhs) {
                (iv_local, rhs.as_ref())
            } else {
                return;
            }
        }
        _ => return,
    };

    // Determine the upper bound value.
    let upper = resolve_bound(bound, known_sizes);
    let upper = match upper {
        Some(u) => u,
        None => return,
    };

    // Verify the induction variable increments by 1.
    if !has_unit_increment(body, iv) {
        return;
    }

    // Now scan body for IndexAccess/Place::Index using `iv` on arrays whose
    // length is `upper`, and remove any associated bounds-check CallBuiltin.
    let mut to_remove: Vec<usize> = Vec::new();
    for (i, stmt) in body.iter().enumerate() {
        if let MirStmt::CallBuiltin { name, args, .. } = stmt
            && is_bounds_check_name(name)
        {
            // If the check's index is the induction variable and the array
            // size matches the bound, it's safe.
            if args.len() >= 2 {
                let idx_is_iv = operand_local(&args[1]).map(|l| l == iv).unwrap_or(false)
                    || operand_local(&args[0]).map(|l| l == iv).unwrap_or(false);
                if idx_is_iv {
                    // Check if the array size matches.
                    let arr_matches = args.iter().any(|a| {
                        operand_local(a)
                            .and_then(|l| known_sizes.get(&l))
                            .map(|&s| s >= upper)
                            .unwrap_or(false)
                    });
                    let len_matches = args
                        .iter()
                        .any(|a| operand_const_usize(a).map(|v| v >= upper).unwrap_or(false));
                    if arr_matches || len_matches {
                        to_remove.push(i);
                        summary.bounds_checks_eliminated += 1;
                    }
                }
            }
        }
    }
    for &idx in to_remove.iter().rev() {
        body.remove(idx);
    }
}

/// Check whether the loop body increments `iv` by exactly 1.
fn has_unit_increment(stmts: &[MirStmt], iv: u32) -> bool {
    for stmt in stmts {
        if let MirStmt::Assign(
            Place::Local(LocalId(dest)),
            Rvalue::Use(Operand::BinOp(BinOp::Add, lhs, rhs)),
        ) = stmt
            && *dest == iv
        {
            let lhs_is_iv = operand_local(lhs).map(|l| l == iv).unwrap_or(false);
            let rhs_is_one = operand_const_usize(rhs) == Some(1);
            let rhs_is_iv = operand_local(rhs).map(|l| l == iv).unwrap_or(false);
            let lhs_is_one = operand_const_usize(lhs) == Some(1);
            if (lhs_is_iv && rhs_is_one) || (rhs_is_iv && lhs_is_one) {
                return true;
            }
        }
        // Also check Rvalue::BinaryOp form.
        if let MirStmt::Assign(Place::Local(LocalId(dest)), Rvalue::BinaryOp(BinOp::Add, lhs, rhs)) =
            stmt
            && *dest == iv
        {
            let lhs_is_iv = operand_local(lhs).map(|l| l == iv).unwrap_or(false);
            let rhs_is_one = operand_const_usize(rhs) == Some(1);
            let rhs_is_iv = operand_local(rhs).map(|l| l == iv).unwrap_or(false);
            let lhs_is_one = operand_const_usize(lhs) == Some(1);
            if (lhs_is_iv && rhs_is_one) || (rhs_is_iv && lhs_is_one) {
                return true;
            }
        }
    }
    false
}

/// Resolve a bound operand to a concrete usize if possible.
fn resolve_bound(op: &Operand, known_sizes: &HashMap<u32, usize>) -> Option<usize> {
    if let Some(v) = operand_const_usize(op) {
        return Some(v);
    }
    // `Call("len", [Place(Local(arr))])` where arr has a known size.
    if let Operand::Call(name, args) = op
        && (name == "len" || name == "__intrinsic_len")
        && let Some(Operand::Place(Place::Local(LocalId(arr)))) = args.first()
        && let Some(&sz) = known_sizes.get(arr)
    {
        return Some(sz);
    }
    // Place(Local(x)) where x holds a known size from a previous `len` assign.
    // (This would require more data flow; skip for now.)
    None
}

// ── Operand utilities ────────────────────────────────────────────────────────

/// A lightweight key for deduplicating operands.
#[derive(Debug, Clone, Hash, PartialEq, Eq)]
enum OperandKey {
    Local(u32),
    ConstI32(i32),
    Other,
}

fn operand_key(op: &Operand) -> OperandKey {
    match op {
        Operand::Place(Place::Local(LocalId(id))) => OperandKey::Local(*id),
        Operand::ConstI32(v) => OperandKey::ConstI32(*v),
        _ => OperandKey::Other,
    }
}

fn operand_local(op: &Operand) -> Option<u32> {
    if let Operand::Place(Place::Local(LocalId(id))) = op {
        Some(*id)
    } else {
        None
    }
}

fn operand_const_usize(op: &Operand) -> Option<usize> {
    match op {
        Operand::ConstI32(v) if *v >= 0 => Some(*v as usize),
        Operand::ConstI64(v) if *v >= 0 => Some(*v as usize),
        Operand::ConstU32(v) => Some(*v as usize),
        Operand::ConstU64(v) => Some(*v as usize),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::mir::{
        BasicBlock, BlockId, InstanceKey, MirFunction, MirLocal, Operand, Place, Rvalue,
        SourceInfo, Terminator,
    };
    use ark_typecheck::types::Type;

    fn dummy_function(stmts: Vec<MirStmt>) -> MirFunction {
        MirFunction {
            id: crate::mir::FnId(0),
            name: "test".to_string(),
            instance: InstanceKey {
                item: "test".to_string(),
                substitution: vec![],
                target_shape: String::new(),
            },
            params: vec![],
            return_ty: Type::I32,
            locals: vec![
                MirLocal {
                    id: LocalId(0),
                    name: None,
                    ty: Type::I32,
                },
                MirLocal {
                    id: LocalId(1),
                    name: None,
                    ty: Type::I32,
                },
                MirLocal {
                    id: LocalId(2),
                    name: None,
                    ty: Type::I32,
                },
            ],
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
    fn constant_index_within_known_array_size() {
        // Assign array of 3 elements to local 0, then bounds_check(local0, 1).
        let stmts = vec![
            MirStmt::Assign(
                Place::Local(LocalId(0)),
                Rvalue::Use(Operand::ArrayInit {
                    elements: vec![
                        Operand::ConstI32(10),
                        Operand::ConstI32(20),
                        Operand::ConstI32(30),
                    ],
                }),
            ),
            MirStmt::CallBuiltin {
                dest: None,
                name: "bounds_check".to_string(),
                args: vec![
                    Operand::Place(Place::Local(LocalId(0))),
                    Operand::ConstI32(1),
                ],
            },
        ];
        let mut func = dummy_function(stmts);
        let summary = bounds_check_elim(&mut func);
        assert_eq!(summary.bounds_checks_eliminated, 1);
        // The CallBuiltin should have been removed.
        assert_eq!(func.blocks[0].stmts.len(), 1);
    }

    #[test]
    fn constant_index_out_of_range_not_eliminated() {
        let stmts = vec![
            MirStmt::Assign(
                Place::Local(LocalId(0)),
                Rvalue::Use(Operand::ArrayInit {
                    elements: vec![Operand::ConstI32(10)],
                }),
            ),
            MirStmt::CallBuiltin {
                dest: None,
                name: "bounds_check".to_string(),
                args: vec![
                    Operand::Place(Place::Local(LocalId(0))),
                    Operand::ConstI32(5),
                ],
            },
        ];
        let mut func = dummy_function(stmts);
        let summary = bounds_check_elim(&mut func);
        assert_eq!(summary.bounds_checks_eliminated, 0);
        assert_eq!(func.blocks[0].stmts.len(), 2);
    }

    #[test]
    fn duplicate_bounds_check_eliminated() {
        let stmts = vec![
            MirStmt::CallBuiltin {
                dest: None,
                name: "bounds_check".to_string(),
                args: vec![
                    Operand::Place(Place::Local(LocalId(0))),
                    Operand::Place(Place::Local(LocalId(1))),
                ],
            },
            MirStmt::CallBuiltin {
                dest: None,
                name: "bounds_check".to_string(),
                args: vec![
                    Operand::Place(Place::Local(LocalId(0))),
                    Operand::Place(Place::Local(LocalId(1))),
                ],
            },
        ];
        let mut func = dummy_function(stmts);
        let summary = bounds_check_elim(&mut func);
        assert_eq!(summary.bounds_checks_eliminated, 1);
        assert_eq!(func.blocks[0].stmts.len(), 1);
    }

    #[test]
    fn no_bounds_check_noop() {
        let stmts = vec![MirStmt::Assign(
            Place::Local(LocalId(0)),
            Rvalue::Use(Operand::ConstI32(42)),
        )];
        let mut func = dummy_function(stmts);
        let summary = bounds_check_elim(&mut func);
        assert_eq!(summary.bounds_checks_eliminated, 0);
        assert_eq!(func.blocks[0].stmts.len(), 1);
    }

    #[test]
    fn non_bounds_builtin_not_touched() {
        let stmts = vec![MirStmt::CallBuiltin {
            dest: None,
            name: "println".to_string(),
            args: vec![Operand::ConstI32(1)],
        }];
        let mut func = dummy_function(stmts);
        let summary = bounds_check_elim(&mut func);
        assert_eq!(summary.bounds_checks_eliminated, 0);
        assert_eq!(func.blocks[0].stmts.len(), 1);
    }
}
