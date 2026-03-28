//! Algebraic simplification pass.
//!
//! Eliminates identity elements and absorbing elements from binary/unary operations.
//! See `opt/pipeline.rs::algebraic_simplify` for the current implementation.
//!
//! This module re-exports the pass for use by the pipeline.

// The implementation lives in opt/pipeline.rs alongside the other passes.
// This module exists to establish the passes/ directory convention.
// Future work: move the implementation here.
