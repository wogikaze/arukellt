//! Constant-folding MIR pass.
//!
//! Evaluates constant binary operations at compile time, replacing them with
//! their result operand so later passes see fewer instructions.
//!
//! **Minimum opt-level**: O1  
//! **Depends on**: nothing  
//! **Safe to run multiple times**: yes (idempotent)

use crate::mir::MirModule;
use crate::opt_level::OptLevel;
use super::PassStats;

/// Minimum optimization level required to run this pass.
pub const MIN_LEVEL: OptLevel = OptLevel::O1;

/// Run the constant-folding pass over every function in `module`.
///
/// Returns immediately (no-op) when `level < MIN_LEVEL`.
pub fn run(module: &mut MirModule, level: OptLevel) -> PassStats {
    if !level.at_least(MIN_LEVEL) {
        return PassStats::default();
    }
    let mut total = 0usize;
    for function in &mut module.functions {
        let summary = crate::opt::const_fold::const_fold(function);
        total += summary.const_folded;
    }
    PassStats {
        name: "const_fold",
        changed: total,
    }
}
