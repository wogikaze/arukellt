//! Name resolution for the Arukellt compiler.
//!
//! Resolves identifiers to their definitions, handles imports,
//! detects circular dependencies, and injects the prelude.

mod analyze;
mod bind;
mod load;
mod module_graph;
mod resolve;
mod scope;
mod symbols;

#[allow(deprecated)]
pub use resolve::{
    ResolvedEnum, ResolvedFunction, ResolvedModule, ResolvedProgram, ResolvedStruct, Visibility,
    merge_prelude, resolve_module, resolve_program, resolve_program_entry,
    resolved_program_to_module,
};
pub use scope::{Scope, ScopeId, Symbol, SymbolKind, SymbolTable};
pub use symbols::{Scope as SymbolScope, ScopeId as SymbolScopeId, Symbol as ResolvedSymbol};
