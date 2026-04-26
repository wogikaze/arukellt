# Changelog

All notable changes to the Arukellt VS Code extension will be documented here.

## [Unreleased]

### Added

- Language support: syntax highlighting, snippets, bracket matching for `.ark` files
- LSP integration: diagnostics, completions, hover, go-to-definition, references
- LSP navigation: go-to-type-definition, go-to-implementation, call hierarchy
- LSP editing: inlay hints (parameter names, type annotations), selection ranges
- LSP code actions: quick fix (auto-import), organize imports, extract variable
- AST-based formatting via `textDocument/formatting`
- Debug adapter (DAP): breakpoints, stepping, stack traces, variable inspection
- Task provider: check, compile, run, test, fmt, watch (background) tasks
- Problem matchers for compiler diagnostics
- Test controller: discover and run `.ark` test files
- Project tree view: sidebar with Modules/Scripts/Targets categories
- Language status item: LSP server state (starting/ready/error)
- Output channel taxonomy: Language Server, Compiler, Tests
- Setup Doctor: binary discovery, environment inspection
- Binary discovery: PATH, `~/.ark/bin`, `~/.cargo/bin`, custom `server.path`
- Context menu integration: Run/Check/Compile from editor and explorer
- Commands: restart LSP, toggle verbose logging, show output, run script
