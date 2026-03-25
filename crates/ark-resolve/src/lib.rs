//! Name resolution for the Arukellt compiler.
//!
//! Resolves identifiers to their definitions, handles imports,
//! detects circular dependencies, and injects the prelude.

pub mod scope;
pub mod resolve;

pub use scope::{Scope, ScopeId, Symbol, SymbolKind, SymbolTable};
pub use resolve::{resolve_module, resolve_program_entry, ResolvedModule};
