//! LSP server for the Arukellt language.
//!
//! Provides editor support via the Language Server Protocol using
//! `tower-lsp`.  Reuses the compiler frontend (lexer → parser →
//! resolve → typecheck) to produce diagnostics, hover, and
//! completion information.
//!
//! ## Supported capabilities
//!
//! - `textDocument/didOpen`, `textDocument/didChange` — triggers
//!   diagnostics refresh via `publishDiagnostics`
//! - `textDocument/hover` — shows type-aware information for
//!   identifiers (function signatures, struct fields, enum variants)
//! - `textDocument/completion` — suggests local variables, functions,
//!   and built-in names
//! - `textDocument/definition` — go-to-definition for functions,
//!   structs, enums, traits, and let bindings
//! - `textDocument/references` — find all uses of an identifier
//! - `textDocument/documentSymbol` — outline view of top-level items
//! - `textDocument/semanticTokens/full` — semantic token
//!   classification for richer syntax highlighting
//!
//! ## Not yet supported
//!
//! - Rename / code actions
//! - Workspace symbols
//! - Incremental parsing / caching

mod server;

pub use server::run_lsp;
