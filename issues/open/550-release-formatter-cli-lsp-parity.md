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

## Recheck — 2026-05-14

The earlier evidence refers to retired Rust LSP paths. Current state:

- The public `arukellt` binary is a thin wasm-runner shell around the selfhost
  compiler wasm.
- `src/compiler/main.ark` advertises `fmt` in usage text, but `parse_command`
  maps `fmt` to `CMD_NOT_YET()`.
- `src/compiler/lsp.ark` does not advertise `documentFormattingProvider` and
  has no `textDocument/formatting` or `textDocument/rangeFormatting` handler.
- The remaining Rust formatter implementation is
  `crates/ark-parser/src/fmt.rs::format_source`, but wiring the thin
  `crates/arukellt` shell back to `ark-parser` would violate the current
  selfhost-runner architecture unless the checklist is intentionally revised.

Updated verdict: close-candidate `no`. There is currently neither a selfhost CLI
formatter nor a selfhost LSP formatting provider to compare.

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
