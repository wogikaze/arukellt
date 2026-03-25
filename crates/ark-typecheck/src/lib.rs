//! Type checker for the Arukellt compiler.
//!
//! Bidirectional type inference with monomorphization support.

pub mod checker;
pub mod types;

pub use checker::TypeChecker;
pub use types::Type;
