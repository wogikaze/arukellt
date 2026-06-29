//! Type checker for the Arukellt compiler.
//!
//! Bidirectional type inference with monomorphization support.

mod build_corehir;
mod checker;
mod selection;
mod typed_ast;
pub mod types;

pub use build_corehir::CoreHirBundle;
pub use checker::{CheckOutput, SemanticModel, TypeChecker};
pub use typed_ast::{ExprId, StmtId, TypedAstMap, TypedExprInfo};
pub use types::Type;
