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
//!   built-in names, and importable module aliases with relevance ordering
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
//! - Incremental parsing
//!
//! ## Architecture
//!
//! Document analysis (lex → parse → resolve → typecheck) is performed
//! once on `didOpen`/`didChange` and cached per-URI.  All features
//! (hover, completion, definition, etc.) read from the shared cache,
//! avoiding redundant compiler passes.

pub mod config;
mod server;

pub use config::LspConfig;
pub use server::run_lsp;
