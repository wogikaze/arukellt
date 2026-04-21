use super::OptimizationSummary;
use crate::mir::{BasicBlock, BlockId, BranchHint, MirFunction, MirStmt, Operand, Terminator};

/// Infer `BranchHint` for a statement-level `if` (MIR `IfStmt`), using the same
/// panic-path heuristic as [`branch_hint_infer`] for `Terminator::If`.
///
/// Backends that lower `IfStmt` to a Wasm `if` can call this at emit time to
/// populate `metadata.code.branch_hint` entries without requiring CFG-shaped
/// `Terminator::If` in the MIR.
pub fn infer_if_stmt_branch_hint(
    then_body: &[MirStmt],
    else_body: &[MirStmt],
) -> Option<BranchHint> {
    let then_panic = stmts_look_like_panic_path(then_body);
    let else_panic = stmts_look_like_panic_path(else_body);
    if then_panic && !else_panic {
        Some(BranchHint::Unlikely)
    } else if else_panic && !then_panic {
        Some(BranchHint::Likely)
    } else {
        None
    }
}

fn stmts_look_like_panic_path(stmts: &[MirStmt]) -> bool {
    stmts.iter().any(|s| stmt_may_be_panic(s))
}

fn stmt_may_be_panic(stmt: &MirStmt) -> bool {
    if stmt_is_panic(stmt) {
        return true;
    }
    match stmt {
        MirStmt::IfStmt {
            then_body,
            else_body,
            ..
        } => stmts_look_like_panic_path(then_body) || stmts_look_like_panic_path(else_body),
        MirStmt::WhileStmt { body, .. } => stmts_look_like_panic_path(body),
        MirStmt::Return(Some(op)) => operand_calls_panic(op),
        _ => false,
    }
}

/// Infer branch hints for `Terminator::If` nodes.
///
/// Heuristic: if the `then_block` leads to a panic, assertion failure, or
/// unreachable terminator, mark the branch as `Unlikely`.  Conversely, if
/// the `else_block` is the error path, the then-path is `Likely`.
pub(crate) fn branch_hint_infer(function: &mut MirFunction) -> OptimizationSummary {
    let mut summary = OptimizationSummary::default();

    // Pre-compute which blocks look like error/panic paths.
    let panic_blocks: std::collections::HashSet<BlockId> = function
        .blocks
        .iter()
        .filter(|b| is_panic_block(b))
        .map(|b| b.id)
        .collect();

    for block in &mut function.blocks {
        if let Terminator::If {
            then_block,
            else_block,
            hint,
            ..
        } = &mut block.terminator
        {
            if hint.is_some() {
                continue; // already annotated
            }
            let then_panic = panic_blocks.contains(then_block);
            let else_panic = panic_blocks.contains(else_block);
            if then_panic && !else_panic {
                *hint = Some(BranchHint::Unlikely);
                summary.branch_hinted += 1;
            } else if else_panic && !then_panic {
                *hint = Some(BranchHint::Likely);
                summary.branch_hinted += 1;
            }
        }
    }
    summary
}

/// A block is considered a "panic path" if it:
/// - terminates with `Unreachable`, or
/// - contains a call to a function whose name contains `panic`, `assert`,
///   `abort`, `unwrap_failed`, or `expect_failed`.
fn is_panic_block(block: &BasicBlock) -> bool {
    if matches!(block.terminator, Terminator::Unreachable) {
        return true;
    }
    for stmt in &block.stmts {
        if stmt_is_panic(stmt) {
            return true;
        }
    }
    false
}

fn stmt_is_panic(stmt: &MirStmt) -> bool {
    match stmt {
        MirStmt::Call { func, .. } => {
            let name = &func.0.to_string();
            // FnId is a u32, so we can't get the name. Check via CallBuiltin or
            // named calls in the Operand tree.
            let _ = name;
            false
        }
        MirStmt::CallBuiltin { name, .. } => is_panic_name(name),
        MirStmt::Assign(_, rvalue) => rvalue_calls_panic(rvalue),
        _ => false,
    }
}

fn rvalue_calls_panic(rvalue: &crate::mir::Rvalue) -> bool {
    match rvalue {
        crate::mir::Rvalue::Use(op) => operand_calls_panic(op),
        _ => false,
    }
}

fn operand_calls_panic(op: &Operand) -> bool {
    match op {
        Operand::Call(name, _) => is_panic_name(name),
        _ => false,
    }
}

fn is_panic_name(name: &str) -> bool {
    let lower = name.to_lowercase();
    lower.contains("panic")
        || lower.contains("abort")
        || lower.contains("unwrap_failed")
        || lower.contains("expect_failed")
        || lower.contains("assert_fail")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::mir::*;
    use ark_typecheck::types::Type;

    fn make_function(blocks: Vec<BasicBlock>) -> MirFunction {
        MirFunction {
            id: FnId(0),
            name: "test".into(),
            instance: InstanceKey::simple("test"),
            params: vec![],
            return_ty: Type::Unit,
            locals: vec![MirLocal {
                id: LocalId(0),
                name: Some("x".into()),
                ty: Type::Bool,
            }],
            blocks,
            entry: BlockId(0),
            struct_typed_locals: Default::default(),
            enum_typed_locals: Default::default(),
            type_params: vec![],
            source: SourceInfo::unknown(),
            is_exported: false,
        }
    }

    #[test]
    fn hint_unlikely_when_then_is_unreachable() {
        let blocks = vec![
            BasicBlock {
                id: BlockId(0),
                stmts: vec![],
                terminator: Terminator::If {
                    cond: Operand::Place(Place::Local(LocalId(0))),
                    then_block: BlockId(1),
                    else_block: BlockId(2),
                    hint: None,
                },
                source: SourceInfo::unknown(),
            },
            BasicBlock {
                id: BlockId(1),
                stmts: vec![],
                terminator: Terminator::Unreachable,
                source: SourceInfo::unknown(),
            },
            BasicBlock {
                id: BlockId(2),
                stmts: vec![],
                terminator: Terminator::Return(None),
                source: SourceInfo::unknown(),
            },
        ];
        let mut func = make_function(blocks);
        let summary = branch_hint_infer(&mut func);
        assert_eq!(summary.branch_hinted, 1);
        match &func.blocks[0].terminator {
            Terminator::If { hint, .. } => assert_eq!(*hint, Some(BranchHint::Unlikely)),
            _ => panic!("expected If terminator"),
        }
    }

    #[test]
    fn hint_likely_when_else_is_panic() {
        let blocks = vec![
            BasicBlock {
                id: BlockId(0),
                stmts: vec![],
                terminator: Terminator::If {
                    cond: Operand::Place(Place::Local(LocalId(0))),
                    then_block: BlockId(1),
                    else_block: BlockId(2),
                    hint: None,
                },
                source: SourceInfo::unknown(),
            },
            BasicBlock {
                id: BlockId(1),
                stmts: vec![],
                terminator: Terminator::Return(None),
                source: SourceInfo::unknown(),
            },
            BasicBlock {
                id: BlockId(2),
                stmts: vec![MirStmt::CallBuiltin {
                    dest: None,
                    name: "panic".into(),
                    args: vec![],
                }],
                terminator: Terminator::Unreachable,
                source: SourceInfo::unknown(),
            },
        ];
        let mut func = make_function(blocks);
        let summary = branch_hint_infer(&mut func);
        assert_eq!(summary.branch_hinted, 1);
        match &func.blocks[0].terminator {
            Terminator::If { hint, .. } => assert_eq!(*hint, Some(BranchHint::Likely)),
            _ => panic!("expected If terminator"),
        }
    }

    #[test]
    fn no_hint_when_neither_is_panic() {
        let blocks = vec![
            BasicBlock {
                id: BlockId(0),
                stmts: vec![],
                terminator: Terminator::If {
                    cond: Operand::Place(Place::Local(LocalId(0))),
                    then_block: BlockId(1),
                    else_block: BlockId(2),
                    hint: None,
                },
                source: SourceInfo::unknown(),
            },
            BasicBlock {
                id: BlockId(1),
                stmts: vec![],
                terminator: Terminator::Return(None),
                source: SourceInfo::unknown(),
            },
            BasicBlock {
                id: BlockId(2),
                stmts: vec![],
                terminator: Terminator::Return(None),
                source: SourceInfo::unknown(),
            },
        ];
        let mut func = make_function(blocks);
        let summary = branch_hint_infer(&mut func);
        assert_eq!(summary.branch_hinted, 0);
    }

    #[test]
    fn preserves_existing_hint() {
        let blocks = vec![
            BasicBlock {
                id: BlockId(0),
                stmts: vec![],
                terminator: Terminator::If {
                    cond: Operand::Place(Place::Local(LocalId(0))),
                    then_block: BlockId(1),
                    else_block: BlockId(2),
                    hint: Some(BranchHint::Likely),
                },
                source: SourceInfo::unknown(),
            },
            BasicBlock {
                id: BlockId(1),
                stmts: vec![],
                terminator: Terminator::Unreachable,
                source: SourceInfo::unknown(),
            },
            BasicBlock {
                id: BlockId(2),
                stmts: vec![],
                terminator: Terminator::Return(None),
                source: SourceInfo::unknown(),
            },
        ];
        let mut func = make_function(blocks);
        let summary = branch_hint_infer(&mut func);
        assert_eq!(summary.branch_hinted, 0);
        match &func.blocks[0].terminator {
            Terminator::If { hint, .. } => assert_eq!(*hint, Some(BranchHint::Likely)),
            _ => panic!("expected If terminator"),
        }
    }

    #[test]
    fn infer_if_stmt_matches_terminator_panic_heuristic() {
        let then_body = vec![MirStmt::CallBuiltin {
            dest: None,
            name: "panic".into(),
            args: vec![],
        }];
        let else_body = vec![MirStmt::Return(None)];
        assert_eq!(
            infer_if_stmt_branch_hint(&then_body, &else_body),
            Some(BranchHint::Unlikely)
        );
        assert_eq!(
            infer_if_stmt_branch_hint(&else_body, &then_body),
            Some(BranchHint::Likely)
        );
    }
}
