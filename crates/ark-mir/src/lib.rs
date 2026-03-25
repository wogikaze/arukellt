//! Mid-level intermediate representation (MIR) for Arukellt.
//!
//! Provides a CFG-based IR between the typed AST and Wasm codegen.
//! Handles monomorphization of generic functions.

pub mod mir;
pub mod lower;

pub use mir::*;
