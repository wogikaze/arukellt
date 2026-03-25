//! Name resolution for the Arukellt compiler.
//!
//! Resolves identifiers to their definitions, handles imports,
//! detects circular dependencies, and injects the prelude.

pub mod resolve;
pub mod scope;

pub use resolve::{ResolvedModule, resolve_module, resolve_program_entry};
pub use scope::{Scope, ScopeId, Symbol, SymbolKind, SymbolTable};
