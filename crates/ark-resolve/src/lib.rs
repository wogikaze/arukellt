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
mod unused;

#[allow(deprecated)]
pub use resolve::{
    ResolvedEnum, ResolvedFunction, ResolvedModule, ResolvedProgram, ResolvedStruct, Visibility,
    bind_program, merge_prelude, resolve_module, resolve_module_default, resolve_module_for_tests,
    resolve_module_legacy, resolve_module_public, resolve_module_stdlib,
    resolve_module_with_intrinsic_prelude, resolve_program, resolve_program_entry,
    resolved_program_entry, resolved_program_to_module,
};
pub use scope::{Scope, ScopeId, Symbol, SymbolKind, SymbolTable};
pub use symbols::{Scope as SymbolScope, ScopeId as SymbolScopeId, Symbol as ResolvedSymbol};
pub use unused::check_unused_imports;
