//! Diagnostic system for the Arukellt compiler.
//!
//! Canonical diagnostic codes (E00xx–E03xx, W0xxx), simple text rendering,
//! and structured snapshots for tests/docs.

pub mod codes;
pub mod helpers;
pub mod render;
pub mod sink;

pub use codes::*;
pub use helpers::*;
pub use render::*;
pub use sink::*;
