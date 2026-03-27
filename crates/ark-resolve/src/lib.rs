//! Name resolution for the Arukellt compiler.
//!
//! Resolves identifiers to their definitions, handles imports,
//! detects circular dependencies, and injects the prelude.

mod resolve;
mod scope;

#[allow(deprecated)]
pub use resolve::{
    ResolvedModule, ResolvedProgram, merge_prelude, resolve_module, resolve_program,
    resolve_program_entry, resolved_program_to_module,
};
pub use scope::{Scope, ScopeId, Symbol, SymbolKind, SymbolTable};
