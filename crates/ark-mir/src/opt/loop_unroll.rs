use crate::mir::{
    BasicBlock, BlockId, MirFunction, MirStmt, Operand, Place, Rvalue, Terminator,
    next_available_block_id, BinOp,
};
use super::OptimizationSummary;

const MAX_UNROLL_ITERATIONS: usize = 4;
const MAX_BODY_STMTS: usize = 8;

/// MIR optimization pass that unrolls small loops with statically known
/// iteration counts (at most [`MAX_UNROLL_ITERATIONS`] iterations, body
/// at most [`MAX_BODY_STMTS`] statements, single-block body, no nesting).
pub(crate) fn loop_unroll(function: &mut MirFunction) -> OptimizationSummary {
    let mut summary = OptimizationSummary::default();

    // Collect candidate loops before mutating.
    let candidates = find_loop_candidates(function);
    if candidates.is_empty() {
        return summary;
    }

    for candidate in candidates {
        if unroll_loop(function, &candidate) {
            summary.loops_unrolled += 1;
        }
    }

    summary
}

// ── Loop detection ──────────────────────────────────────────────────────────

/// A simple single-block loop we may be able to unroll.
struct LoopCandidate {
    /// Index of the header block (contains the conditional branch).
    header_idx: usize,
    /// Index of the body block (executed each iteration, jumps back to header).
    body_idx: usize,
    /// The block we exit to when the loop ends.
    exit_block: BlockId,
    /// Statically determined trip count (1..=MAX_UNROLL_ITERATIONS).
    trip_count: usize,
}

/// Scan the function for simple for-style loops:
///
/// ```text
///   header:  if cond -> body, exit
///   body:    <stmts>; goto header
/// ```
///
/// We only consider loops whose body is a single block with ≤ 8 stmts,
/// and where the trip count can be statically determined.
fn find_loop_candidates(function: &MirFunction) -> Vec<LoopCandidate> {
    let mut candidates = Vec::new();

    for (h_idx, header) in function.blocks.iter().enumerate() {
        // Pattern: header has `If { cond, then_block, else_block }` where
        // one branch goes to a body that loops back to the header.
        let (cond, then_block, else_block) = match &header.terminator {
            Terminator::If {
                cond,
                then_block,
                else_block,
                ..
            } => (cond, *then_block, *else_block),
            _ => continue,
        };

        // Try both orientations: body = then_block / body = else_block.
        for (body_id, exit_id) in [(then_block, else_block), (else_block, then_block)] {
            let Some(b_idx) = function.blocks.iter().position(|b| b.id == body_id) else {
                continue;
            };
            let body = &function.blocks[b_idx];

            // Body must jump back to header (back-edge).
            if !matches!(body.terminator, Terminator::Goto(target) if target == header.id) {
                continue;
            }
            // Single-block body, small enough.
            if body.stmts.len() > MAX_BODY_STMTS {
                continue;
            }
            // No nested loops (no back-edges within the body stmts).
            if contains_loop_construct(&body.stmts) {
                continue;
            }

            // Try to determine trip count from the condition + header stmts.
            if let Some(trip) = deduce_trip_count(header, body, cond, body_id == then_block) {
                if trip >= 1 && trip <= MAX_UNROLL_ITERATIONS {
                    candidates.push(LoopCandidate {
                        header_idx: h_idx,
                        body_idx: b_idx,
                        exit_block: exit_id,
                        trip_count: trip,
                    });
                    break; // only one candidate per header
                }
            }
        }
    }

    candidates
}

/// Returns `true` if any statement contains `WhileStmt` or `Break`/`Continue`,
/// which indicates nested loop constructs we don't want to unroll.
fn contains_loop_construct(stmts: &[MirStmt]) -> bool {
    for stmt in stmts {
        match stmt {
            MirStmt::WhileStmt { .. } | MirStmt::Break | MirStmt::Continue => return true,
            MirStmt::IfStmt {
                then_body,
                else_body,
                ..
            } => {
                if contains_loop_construct(then_body) || contains_loop_construct(else_body) {
                    return true;
                }
            }
            _ => {}
        }
    }
    false
}

// ── Trip-count deduction ────────────────────────────────────────────────────

/// Try to deduce a constant trip count from a simple counting loop pattern:
///
/// ```text
///   header:
///     %cond = %i < CONST_BOUND
///     if %cond -> body, exit
///   body:
///     ...
///     %i = %i + CONST_STEP
///     goto header
/// ```
///
/// We look for the induction variable `i`, a constant upper bound, and a
/// constant step.  `body_is_then` tells us whether the body executes when
/// cond is true (normal `while i < N` pattern).
fn deduce_trip_count(
    header: &BasicBlock,
    body: &BasicBlock,
    cond: &Operand,
    body_is_then: bool,
) -> Option<usize> {
    // The condition must be a comparison: i < N  or  i <= N  (or reversed).
    let (ind_local, bound, op) = extract_compare(cond)?;

    // Find the induction variable step in the body: ind = ind + step
    let step = find_induction_step(body, ind_local)?;
    if step == 0 {
        return None;
    }

    // Find the initial value of ind in the header stmts.
    let init = find_const_assign_in_block(header, ind_local)
        .or_else(|| {
            // The init might not be in the header itself — accept 0 as fallback
            // only when the header has no assignment to ind_local.
            None
        })?;

    compute_trip_count(init, bound, step, op, body_is_then)
}

/// Extract `(local_id, const_bound, comparison_op)` from a comparison operand.
fn extract_compare(cond: &Operand) -> Option<(u32, i64, BinOp)> {
    match cond {
        Operand::BinOp(op, lhs, rhs) => {
            match op {
                BinOp::Lt | BinOp::Le | BinOp::Gt | BinOp::Ge => {}
                _ => return None,
            }
            // lhs = Place(Local(id)), rhs = const  OR  reversed
            if let (Some(id), Some(val)) = (operand_local(lhs), operand_const_i64(rhs)) {
                Some((id, val, *op))
            } else if let (Some(val), Some(id)) = (operand_const_i64(lhs), operand_local(rhs)) {
                // Reverse the comparison direction.
                let flipped = match op {
                    BinOp::Lt => BinOp::Gt,
                    BinOp::Le => BinOp::Ge,
                    BinOp::Gt => BinOp::Lt,
                    BinOp::Ge => BinOp::Le,
                    other => *other,
                };
                Some((id, val, flipped))
            } else {
                None
            }
        }
        // Might be a place referencing a pre-computed boolean; not handled.
        _ => None,
    }
}

fn operand_local(op: &Operand) -> Option<u32> {
    match op {
        Operand::Place(Place::Local(id)) => Some(id.0),
        _ => None,
    }
}

fn operand_const_i64(op: &Operand) -> Option<i64> {
    match op {
        Operand::ConstI32(v) => Some(*v as i64),
        Operand::ConstI64(v) => Some(*v),
        Operand::ConstU32(v) => Some(*v as i64),
        Operand::ConstU64(v) => Some(*v as i64),
        Operand::ConstI8(v) => Some(*v as i64),
        Operand::ConstI16(v) => Some(*v as i64),
        Operand::ConstU8(v) => Some(*v as i64),
        Operand::ConstU16(v) => Some(*v as i64),
        _ => None,
    }
}

/// Look for `Assign(Local(ind), BinaryOp(Add, Place(Local(ind)), Const(step)))` in body.
fn find_induction_step(body: &BasicBlock, ind: u32) -> Option<i64> {
    for stmt in &body.stmts {
        if let MirStmt::Assign(Place::Local(dest), Rvalue::BinaryOp(BinOp::Add, lhs, rhs)) = stmt {
            if dest.0 == ind {
                if let (Some(l), Some(r)) = (operand_local(&lhs), operand_const_i64(&rhs)) {
                    if l == ind {
                        return Some(r);
                    }
                }
                if let (Some(l), Some(r)) = (operand_const_i64(&lhs), operand_local(&rhs)) {
                    if r == ind {
                        return Some(l);
                    }
                }
            }
        }
    }
    None
}

/// Find `Assign(Local(id), Use(Const(v)))` in a block and return the constant value.
fn find_const_assign_in_block(block: &BasicBlock, id: u32) -> Option<i64> {
    for stmt in &block.stmts {
        if let MirStmt::Assign(Place::Local(dest), Rvalue::Use(val)) = stmt {
            if dest.0 == id {
                return operand_const_i64(val);
            }
        }
    }
    None
}

/// Compute the number of loop iterations given init, bound, step, and
/// comparison operator. Returns `None` if non-positive or too large.
fn compute_trip_count(
    init: i64,
    bound: i64,
    step: i64,
    op: BinOp,
    body_is_then: bool,
) -> Option<usize> {
    if step <= 0 {
        return None;
    }

    // When body_is_then, the loop body executes while the condition is true.
    // When body is else, the body executes while the condition is false, so
    // we invert the comparison.
    let effective_op = if body_is_then {
        op
    } else {
        match op {
            BinOp::Lt => BinOp::Ge,
            BinOp::Le => BinOp::Gt,
            BinOp::Gt => BinOp::Le,
            BinOp::Ge => BinOp::Lt,
            other => other,
        }
    };

    // Compute range length.
    let range = match effective_op {
        BinOp::Lt => bound - init,         // while i < bound
        BinOp::Le => bound - init + 1,     // while i <= bound
        BinOp::Gt => init - bound,         // while i > bound  (counting down — not typical)
        BinOp::Ge => init - bound + 1,
        _ => return None,
    };

    if range <= 0 {
        return None;
    }

    // Round up: iterations = ceil(range / step)
    let iters = ((range as u64) + (step as u64) - 1) / (step as u64);
    let iters = iters as usize;

    if iters >= 1 && iters <= MAX_UNROLL_ITERATIONS {
        Some(iters)
    } else {
        None
    }
}

// ── Unrolling ───────────────────────────────────────────────────────────────

/// Perform the actual unroll: duplicate the body `trip_count` times and
/// replace the loop with straight-line code.
fn unroll_loop(function: &mut MirFunction, candidate: &LoopCandidate) -> bool {
    let trip = candidate.trip_count;
    let body_stmts = function.blocks[candidate.body_idx].stmts.clone();
    let header_source = function.blocks[candidate.header_idx].source;

    // Allocate fresh block ids for the unrolled copies.
    let mut next_id = next_available_block_id(function).0;
    let mut new_blocks: Vec<BasicBlock> = Vec::with_capacity(trip);

    for i in 0..trip {
        let bid = BlockId(next_id);
        next_id += 1;

        let terminator = if i + 1 < trip {
            // Chain to next unrolled copy.
            Terminator::Goto(BlockId(next_id))
        } else {
            // Last copy exits the loop.
            Terminator::Goto(candidate.exit_block)
        };

        new_blocks.push(BasicBlock {
            id: bid,
            stmts: body_stmts.clone(),
            terminator,
            source: header_source,
        });
    }

    // Rewrite the header to jump directly to the first unrolled block,
    // keeping any init assignments that were in the header.
    let first_unrolled = new_blocks[0].id;
    function.blocks[candidate.header_idx].terminator = Terminator::Goto(first_unrolled);

    // The original body block is now dead (it will be cleaned up by
    // dead_block_elim).  Redirect it to Unreachable so the validator
    // doesn't complain about dangling back-edges.
    function.blocks[candidate.body_idx].stmts.clear();
    function.blocks[candidate.body_idx].terminator = Terminator::Unreachable;

    // Append the new unrolled blocks.
    function.blocks.extend(new_blocks);

    true
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::mir::*;
    use ark_typecheck::types::Type;

    /// Helper: build a minimal MirFunction with the given blocks.
    fn make_function(blocks: Vec<BasicBlock>) -> MirFunction {
        let entry = blocks.first().map(|b| b.id).unwrap_or(BlockId(0));
        MirFunction {
            id: FnId(0),
            name: "test".into(),
            instance: InstanceKey {
                item: "test".into(),
                substitution: vec![],
                target_shape: String::new(),
            },
            params: vec![],
            return_ty: Type::Unit,
            locals: vec![
                MirLocal {
                    id: LocalId(0),
                    name: Some("i".into()),
                    ty: Type::I32,
                },
            ],
            blocks,
            entry,
            struct_typed_locals: Default::default(),
            enum_typed_locals: Default::default(),
            type_params: vec![],
            source: SourceInfo::unknown(),
            is_exported: false,
        }
    }

    #[test]
    fn unroll_simple_loop() {
        // header (bb0): i = 0; if i < 3 -> bb1, bb2
        // body   (bb1): i = i + 1; goto bb0
        // exit   (bb2): return
        let header = BasicBlock {
            id: BlockId(0),
            stmts: vec![MirStmt::Assign(
                Place::Local(LocalId(0)),
                Rvalue::Use(Operand::ConstI32(0)),
            )],
            terminator: Terminator::If {
                cond: Operand::BinOp(
                    BinOp::Lt,
                    Box::new(Operand::Place(Place::Local(LocalId(0)))),
                    Box::new(Operand::ConstI32(3)),
                ),
                then_block: BlockId(1),
                else_block: BlockId(2),
                hint: None,
            },
            source: SourceInfo::unknown(),
        };
        let body = BasicBlock {
            id: BlockId(1),
            stmts: vec![MirStmt::Assign(
                Place::Local(LocalId(0)),
                Rvalue::BinaryOp(
                    BinOp::Add,
                    Operand::Place(Place::Local(LocalId(0))),
                    Operand::ConstI32(1),
                ),
            )],
            terminator: Terminator::Goto(BlockId(0)),
            source: SourceInfo::unknown(),
        };
        let exit = BasicBlock {
            id: BlockId(2),
            stmts: vec![],
            terminator: Terminator::Return(None),
            source: SourceInfo::unknown(),
        };

        let mut func = make_function(vec![header, body, exit]);
        let summary = loop_unroll(&mut func);

        assert_eq!(summary.loops_unrolled, 1);
        // Header should now Goto the first unrolled block.
        assert!(matches!(func.blocks[0].terminator, Terminator::Goto(_)));
        // Original body (bb1) should be Unreachable.
        assert!(matches!(func.blocks[1].terminator, Terminator::Unreachable));
        // We should have 3 new blocks (trip_count = 3).
        assert_eq!(func.blocks.len(), 3 + 3); // original 3 + 3 unrolled
    }

    #[test]
    fn skip_large_body() {
        // Body has > MAX_BODY_STMTS statements — should not unroll.
        let stmts: Vec<MirStmt> = (0..=MAX_BODY_STMTS)
            .map(|_| {
                MirStmt::Assign(
                    Place::Local(LocalId(0)),
                    Rvalue::Use(Operand::ConstI32(0)),
                )
            })
            .collect();

        let header = BasicBlock {
            id: BlockId(0),
            stmts: vec![MirStmt::Assign(
                Place::Local(LocalId(0)),
                Rvalue::Use(Operand::ConstI32(0)),
            )],
            terminator: Terminator::If {
                cond: Operand::BinOp(
                    BinOp::Lt,
                    Box::new(Operand::Place(Place::Local(LocalId(0)))),
                    Box::new(Operand::ConstI32(2)),
                ),
                then_block: BlockId(1),
                else_block: BlockId(2),
                hint: None,
            },
            source: SourceInfo::unknown(),
        };
        let body = BasicBlock {
            id: BlockId(1),
            stmts,
            terminator: Terminator::Goto(BlockId(0)),
            source: SourceInfo::unknown(),
        };
        let exit = BasicBlock {
            id: BlockId(2),
            stmts: vec![],
            terminator: Terminator::Return(None),
            source: SourceInfo::unknown(),
        };

        let mut func = make_function(vec![header, body, exit]);
        let summary = loop_unroll(&mut func);
        assert_eq!(summary.loops_unrolled, 0);
    }

    #[test]
    fn skip_too_many_iterations() {
        // Trip count = 5 which exceeds MAX_UNROLL_ITERATIONS.
        let header = BasicBlock {
            id: BlockId(0),
            stmts: vec![MirStmt::Assign(
                Place::Local(LocalId(0)),
                Rvalue::Use(Operand::ConstI32(0)),
            )],
            terminator: Terminator::If {
                cond: Operand::BinOp(
                    BinOp::Lt,
                    Box::new(Operand::Place(Place::Local(LocalId(0)))),
                    Box::new(Operand::ConstI32(5)),
                ),
                then_block: BlockId(1),
                else_block: BlockId(2),
                hint: None,
            },
            source: SourceInfo::unknown(),
        };
        let body = BasicBlock {
            id: BlockId(1),
            stmts: vec![MirStmt::Assign(
                Place::Local(LocalId(0)),
                Rvalue::BinaryOp(
                    BinOp::Add,
                    Operand::Place(Place::Local(LocalId(0))),
                    Operand::ConstI32(1),
                ),
            )],
            terminator: Terminator::Goto(BlockId(0)),
            source: SourceInfo::unknown(),
        };
        let exit = BasicBlock {
            id: BlockId(2),
            stmts: vec![],
            terminator: Terminator::Return(None),
            source: SourceInfo::unknown(),
        };

        let mut func = make_function(vec![header, body, exit]);
        let summary = loop_unroll(&mut func);
        assert_eq!(summary.loops_unrolled, 0);
    }

    #[test]
    fn no_loops_is_noop() {
        let block = BasicBlock {
            id: BlockId(0),
            stmts: vec![],
            terminator: Terminator::Return(None),
            source: SourceInfo::unknown(),
        };

        let mut func = make_function(vec![block]);
        let summary = loop_unroll(&mut func);
        assert_eq!(summary.loops_unrolled, 0);
    }
}
