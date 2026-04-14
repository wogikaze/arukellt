//! Individual MIR optimization passes.
//!
//! Each pass is a standalone module that operates on [`MirModule`].
//! The pipeline orchestrator (`opt/pipeline.rs`) drives pass ordering and iteration
//! for batch compilation; individual passes can also be invoked directly through
//! the unified interface below.
//!
//! ## Pass contract
//!
//! Every pass module exposes:
//! ```ignore
//! pub const MIN_LEVEL: OptLevel;
//! pub fn run(module: &mut MirModule, level: OptLevel) -> PassStats;
//! ```
//!
//! Passes must be:
//! - **Idempotent**: running a pass twice must not change the MIR.
//! - **Sound**: the MIR must remain semantically equivalent after the pass.
//! - **Bounded**: each pass must terminate in O(n) or O(n²) at worst.
//!
//! ## Opt-level mapping
//!
//! | Level | Passes enabled |
//! |-------|----------------|
//! | None  | (none) |
//! | O1    | const_fold, branch_fold, cfg_simplify, copy_prop, const_prop, dead_local_elim, dead_block_elim, unreachable_cleanup, cse |
//! | O2    | All O1 + inline_small_leaf, string_concat_opt, aggregate_simplify, algebraic_simplify, strength_reduction, loop_unroll, licm, bounds_check_elim, escape_analysis, type_narrowing |
//! | O3    | All O2 + gc_hint, branch_hint_infer |

use crate::mir::MirModule;
pub use crate::opt_level::OptLevel;

pub mod const_fold;
pub mod dead_block_elim;

/// Summary returned by a single pass invocation.
///
/// `name` is a static string identifying the pass.
/// `changed` counts the number of rewrites performed (0 means the pass was a no-op).
#[derive(Debug, Clone, Default)]
pub struct PassStats {
    /// Name of the pass that produced this result.
    pub name: &'static str,
    /// Number of rewrites / transformations applied.
    pub changed: usize,
}

impl PassStats {
    /// Return `true` if the pass made any changes.
    pub fn did_change(&self) -> bool {
        self.changed > 0
    }
}

/// Run all registered `passes` in order over `module` at the given `level`.
///
/// Passes whose `MIN_LEVEL` exceeds `level` are silently skipped.
pub fn run_all(module: &mut MirModule, level: OptLevel) -> Vec<PassStats> {
    vec![
        const_fold::run(module, level),
        dead_block_elim::run(module, level),
    ]
}
