//! Wasm code generator for Arukellt.
//!
//! Translates MIR to Wasm binary via per-target backends.
//! T1 (`wasm32-wasi-p1`): linear memory + WASI Preview 1.
//! T3 (`wasm32-wasi-p2`): Wasm GC + WASI Preview 2 (planned).

mod backend_ir;
pub mod component;
pub mod emit;

pub use emit::emit;
