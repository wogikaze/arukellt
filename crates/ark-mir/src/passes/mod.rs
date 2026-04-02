//! Individual MIR optimization passes.
//!
//! Each pass is a standalone module that operates on `MirModule` or `MirFunction`.
//! The pipeline orchestrator (`opt/pipeline.rs`) drives pass ordering and iteration.
//!
//! ## Pass contract
//!
//! Every pass function has the signature:
//! ```ignore
//! fn pass_name(function: &mut MirFunction) -> OptimizationSummary
//! ```
//!
//! Passes must be:
//! - **Idempotent**: running a pass twice should not break the MIR.
//! - **Sound**: the MIR must remain semantically equivalent.
//! - **Bounded**: each pass should terminate in O(n) or O(n²) at worst.
//!
//! ## Opt-level mapping
//!
//! | Level | Passes |
//! |-------|--------|
//! | O0    | None |
//! | O1    | const_fold, branch_fold, cfg_simplify, copy_prop, const_prop, dead_local_elim, dead_block_elim, unreachable_cleanup |
//! | O2    | All O1 + inline_small_leaf, string_concat_opt, aggregate_simplify, algebraic_simplify, strength_reduction |

