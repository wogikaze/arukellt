//! Strength reduction pass.
//!
//! Replaces expensive operations with cheaper equivalents:
//! - Multiplication by power-of-2 → left shift
//! - Division by power-of-2 → right shift
//!
//! See `opt/pipeline.rs::strength_reduction` for the current implementation.

// The implementation lives in opt/pipeline.rs alongside the other passes.
// This module exists to establish the passes/ directory convention.
// Future work: move the implementation here.
