//! Lower typed AST and CoreHIR to MIR.

mod ctx;
pub(crate) use ctx::LowerCtx;

mod builders;
#[allow(unused_imports)]
pub(crate) use builders::{
    default_function_instance, fallback_block, fallback_function, finalize_block,
    finalize_function, finalize_function_blocks, finalize_function_metadata,
    finalize_lowered_module, infer_fn_id, push_function, type_to_sig_name,
};

mod expr;
mod facade;
mod func;
mod pattern;
mod stmt;
mod types;

pub use facade::*;

// Re-export the main lowering function from func submodule (deprecated, use CoreHIR path)
#[allow(deprecated)]
pub use func::lower_to_mir;

#[cfg(test)]
mod tests;
