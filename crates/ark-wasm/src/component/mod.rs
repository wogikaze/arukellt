//! WIT / Component Model generation for T3 targets.
//!
//! Generates WIT (Wasm Interface Type) descriptions from the compiler's
//! public API surface. Component wrapping uses external `wasm-tools`.

mod wit;

pub use wit::generate_wit;
