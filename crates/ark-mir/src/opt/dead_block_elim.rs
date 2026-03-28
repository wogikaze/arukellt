use crate::mir::{BlockId, MirFunction, Terminator};
use super::OptimizationSummary;

pub(crate) fn dead_block_elim(function: &mut MirFunction) -> OptimizationSummary {
    let reachable = reachable_blocks(function);
    let before = function.blocks.len();
    function
        .blocks
        .retain(|block| reachable.contains(&block.id));
    OptimizationSummary {
        dead_blocks_removed: before.saturating_sub(function.blocks.len()),
        ..OptimizationSummary::default()
    }
}

fn reachable_blocks(function: &MirFunction) -> std::collections::HashSet<BlockId> {
    let mut reachable = std::collections::HashSet::new();
    let mut worklist = vec![function.entry];
    while let Some(block_id) = worklist.pop() {
        if !reachable.insert(block_id) {
            continue;
        }
        let Some(block) = function.blocks.iter().find(|block| block.id == block_id) else {
            continue;
        };
        match &block.terminator {
            Terminator::Goto(target) => worklist.push(*target),
            Terminator::If {
                then_block,
                else_block,
                ..
            } => {
                worklist.push(*then_block);
                worklist.push(*else_block);
            }
            Terminator::Switch { arms, default, .. } => {
                for (_, block) in arms {
                    worklist.push(*block);
                }
                worklist.push(*default);
            }
            Terminator::Return(_) | Terminator::Unreachable => {}
        }
    }
    reachable
}
