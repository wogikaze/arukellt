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
//! - `textDocument/hover` — shows type information for identifiers
//! - `textDocument/completion` — suggests local variables, functions,
//!   and built-in names
//!
//! ## Not yet supported
//!
//! - Go-to-definition / references
//! - Rename / code actions
//! - Semantic tokens
//! - Workspace symbols
//! - Incremental parsing / caching

mod server;

pub use server::run_lsp;
