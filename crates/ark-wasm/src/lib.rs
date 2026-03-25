//! Wasm code generator for Arukellt.
//!
//! Translates MIR to Wasm binary using wasm-encoder.
//! Targets wasm32-wasi with WASI Preview 1.

pub mod emit;

pub use emit::emit;
