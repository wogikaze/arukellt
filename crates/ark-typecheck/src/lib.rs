//! Type checker for the Arukellt compiler.
//!
//! Bidirectional type inference with monomorphization support.

pub mod types;
pub mod checker;

pub use types::Type;
pub use checker::TypeChecker;
