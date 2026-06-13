# Tooling Feature Matrix

This document defines which tooling features are available in the CLI, LSP, and VS Code extension, and identifies the source of truth for each.

## Feature Matrix

| Feature | CLI | LSP | VS Code Extension | Source of Truth |
|---------|-----|-----|-------------------|-----------------|
| **Format** (`fmt`) | `arukellt fmt` / `--check` | `textDocument/formatting`, `rangeFormatting` | Format Document / Selection (`editor.defaultFormatter`) | `src/compiler/fmt` (`format_source`) |
| **Check** (`check`) | `arukellt check` | diagnostics on open/change | Problems panel | `selfhost typechecker` + `selfhost resolver` (shared) |
| **Lint** | `arukellt lint` | diagnostics (W-codes) | Problems panel | `selfhost diagnostics` (shared) |
| **Go to Definition** | — | `textDocument/definition` | Ctrl+Click / F12 | LSP `goto_definition()` with symbol index |
| **Go to Type Definition** | — | `textDocument/typeDefinition` | — | LSP `goto_type_definition()` |
| **Find References** | — | `textDocument/references` | Shift+F12 | LSP `references()` scope-aware |
| **Rename** | — | `textDocument/rename` | F2 | LSP `rename()` scope-aware |
| **Hover** | — | `textDocument/hover` | Mouse hover | LSP `hover()` + stdlib manifest |
| **Completion** | — | `textDocument/completion` | IntelliSense | LSP `completion()` + manifest |
| **Signature Help** | — | `textDocument/signatureHelp` | Parameter hints | LSP + manifest + symbol index |
| **Document Symbols** | — | `textDocument/documentSymbol` | Outline view | LSP `document_symbol()` |
| **Workspace Symbols** | — | `workspace/symbol` | Ctrl+T | LSP `symbol()` + symbol index |
| **Document Highlight** | — | `textDocument/documentHighlight` | Word highlight | LSP `document_highlight()` scope-aware |
| **Code Actions** | — | `textDocument/codeAction` | Lightbulb | LSP `code_action()` |
| **Auto-import** | — | code action (quickfix) | Lightbulb | LSP + manifest `import_candidates()` |
| **Organize Imports** | — | `source.organizeImports` | Organize Imports | `fmt::sort_imports` only (#346) |
| **Fix All** | — | `source.fixAll` | Source Action | LSP (formatter + lint fixes) |
| **Diagnostics** | exit code + stderr | `textDocument/publishDiagnostics` | Problems panel | `selfhost diagnostics` (shared) |

## Source of Truth Details

### Shared Code Paths (CLI = LSP)

- **Formatter**: CLI `arukellt fmt`, LSP formatting/range formatting, and the
  playground formatter delegate to `fmt::format_source()`. See
  [language/formatter.md](language/formatter.md).
- **Diagnostics**: Both CLI `arukellt check` and LSP produce diagnostics via `selfhost typechecker` and `selfhost resolver` using the same error codes.
- **Lint**: Both CLI `arukellt lint` and LSP lint diagnostics use the same warning codes (W0001–W0006+).

### LSP-Only Features

- Navigation: go-to-definition, references, rename, hover, completion, signature help
- Document/workspace symbols
- Code actions (auto-import, organize imports, fix-all)
- Document highlight

### CLI-Only Features

- `arukellt run` — compile and execute
- `arukellt build` — compile to Wasm
- `arukellt test` — run test suite
- `arukellt new` / `arukellt init` — project scaffolding

## Test Coverage

| Feature | Test ID | Type |
|---------|---------|------|
| Format goldens | `python3 scripts/manager.py selfhost fmt-parity` | selfhost gate |
| Format LSP smoke | `tests/fixtures/selfhost/lsp_formatting.lsp-script` | LSP lifecycle |
| Format idempotency | `fmt-parity` (second `fmt` pass) | selfhost gate |
| Diagnostics | `lint_diagnostics_have_arukellt_lint_source` | LSP unit |
| Go to Definition | `definition_resolves_local_symbol` | LSP E2E |
| Completion | `completion_returns_results` | LSP E2E |
| Hover | `hover_returns_info` | LSP E2E |
| Symbol Index | `symbol_index_extracts_all_item_kinds` | LSP unit |
| Cross-file | `symbol_index_cross_file_lookup` | LSP unit |
| Scope-aware | `is_top_level_symbol_detects_fn_and_struct` | LSP unit |
| Keyword rejection | `is_keyword_rejects_language_keywords` | LSP unit |
| Builtin rejection | `is_builtin_name_rejects_stdlib_builtins` | LSP unit |
| LSP protocol | `initialize_returns_capabilities` | LSP E2E |
| LSP shutdown | `shutdown_sequence_works` | LSP E2E |
