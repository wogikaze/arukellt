//! Mid-level intermediate representation (MIR) for Arukellt.
//!
//! Provides a CFG-based IR between the typed AST and Wasm codegen.
//! Handles monomorphization of generic functions.

pub mod escape;
pub mod lower;
pub mod mir;
pub mod opt;
pub mod opt_level;
pub mod passes;
pub mod validate;

#[allow(deprecated)]
pub use lower::{compare_lowering_paths, lower_check_output_to_mir, lower_legacy_only};
pub use mir::*;
pub use opt::{
    OptimizationPass, OptimizationSummary, default_pass_order, eliminate_dead_functions, find_pass,
    infer_if_stmt_branch_hint, optimization_pass_catalog, optimization_trace_snapshot,
    optimize_module, optimize_module_named, optimize_module_named_only,
    optimize_module_named_until, optimize_module_named_without, optimize_module_none,
    pass_pipeline_snapshot, run_single_pass,
};
pub use opt_level::OptLevel;
pub use passes::PassStats;
pub use validate::{MirValidationError, validate_backend_legal_module, validate_module};
