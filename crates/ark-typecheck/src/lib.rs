//! Type checker for the Arukellt compiler.
//!
//! Bidirectional type inference with monomorphization support.

mod checker;
mod typed_ast;
pub mod types;

pub use checker::{TypeChecker, SemanticModel};
pub use typed_ast::{ExprId, StmtId, TypedAstMap, TypedExprInfo};
pub use types::Type;
