//! Dead basic-block elimination MIR pass.
//!
//! Removes basic blocks that are unreachable from the function entry, shrinking
//! the CFG and enabling downstream passes to skip dead code.
//!
//! **Minimum opt-level**: O1  
//! **Depends on**: nothing  
//! **Safe to run multiple times**: yes (idempotent — already-removed blocks stay gone)

use crate::mir::MirModule;
use crate::opt_level::OptLevel;
use super::PassStats;

/// Minimum optimization level required to run this pass.
pub const MIN_LEVEL: OptLevel = OptLevel::O1;

/// Run the dead-block-elimination pass over every function in `module`.
///
/// Returns immediately (no-op) when `level < MIN_LEVEL`.
pub fn run(module: &mut MirModule, level: OptLevel) -> PassStats {
    if !level.at_least(MIN_LEVEL) {
        return PassStats::default();
    }
    let mut total = 0usize;
    for function in &mut module.functions {
        let summary = crate::opt::dead_block_elim::dead_block_elim(function);
        total += summary.dead_blocks_removed;
    }
    PassStats {
        name: "dead_block_elim",
        changed: total,
    }
}
