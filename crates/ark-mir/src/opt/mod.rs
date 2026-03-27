pub mod pipeline;

pub use pipeline::{
    OptimizationPass, OptimizationSummary, default_pass_order, find_pass, optimize_module,
    optimize_module_named, optimize_module_named_only, optimize_module_named_until,
    optimize_module_named_without, optimize_module_none, optimization_pass_catalog,
    optimization_trace_snapshot, pass_pipeline_snapshot, run_single_pass,
};
