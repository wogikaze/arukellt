# Release: Formatter CLI-LSP Parity

> **Status:** open
> **Track:** release
> **Type:** Verification

## Scope

Ensure formatter output matches between CLI and LSP for release verification.

## Checklist Source

docs/release-checklist.md — Extension distribution section

## Acceptance

- [ ] Formatter output matches between CLI and LSP (shared `format_source()`)

## Evidence

- CLI entrypoint: `crates/arukellt/src/main.rs` is a thin wasm-runner shell and does not expose a formatter command or call `format_source()`.
- LSP entrypoints: `crates/ark-lsp/src/server.rs` routes both `textDocument/formatting` and `textDocument/rangeFormatting` through `ark_parser::fmt::format_source()`.
- Test gap: no dedicated CLI-vs-LSP formatter parity test was found in `crates/ark-lsp`, `crates/arukellt`, or `tests/`; the only nearby formatter test is the LSP idempotence check in `formatter_and_fix_all_produce_consistent_output`.

## Blocker

- The close gate is not provable from current code because the repo slice has no CLI formatter entrypoint to compare against LSP output. Keep this issue open until a real CLI formatter surface exists or the release checklist is revised.

## Required Verification

- Format same code using CLI formatter
- Format same code using LSP formatter
- Compare outputs for byte-for-byte identity
- Verify both use shared `format_source()` function

## Close Gate

CLI and LSP formatter outputs must be identical.

## Primary Paths

- CLI formatter implementation
- LSP formatter implementation
- Shared `format_source()` function
- Test fixtures for formatting

## Non-Goals

- Performance comparison
- Feature differences beyond output identity
