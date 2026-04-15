//! Dead basic-block elimination MIR pass.
//!
//! Removes basic blocks that are unreachable from the function entry, shrinking
//! the CFG and enabling downstream passes to skip dead code.
//!
//! **Minimum opt-level**: O1  
//! **Depends on**: nothing  
//! **Safe to run multiple times**: yes (idempotent — already-removed blocks stay gone)

use super::PassStats;
use crate::mir::MirModule;
use crate::opt::{OptimizationPass, run_single_pass};
use crate::opt_level::OptLevel;

/// Minimum optimization level required to run this pass.
pub const MIN_LEVEL: OptLevel = OptLevel::O1;

/// Run the dead-block-elimination pass over every function in `module`.
///
/// Returns immediately (no-op) when `level < MIN_LEVEL`.
pub fn run(module: &mut MirModule, level: OptLevel) -> PassStats {
    if !level.at_least(MIN_LEVEL) {
        return PassStats::default();
    }
    let summary = run_single_pass(module, OptimizationPass::DeadBlockElim).unwrap_or_default();
    PassStats {
        name: "dead_block_elim",
        changed: summary.dead_blocks_removed,
    }
}
