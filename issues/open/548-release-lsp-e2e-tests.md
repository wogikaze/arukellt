# Release: LSP E2E Tests

> **Status:** open
> **Track:** release
> **Type:** Verification

## Scope

Ensure LSP E2E tests pass for release verification.

## Checklist Source

docs/release-checklist.md — Pre-release section

## Acceptance

- [ ] `cargo test -p ark-lsp --test lsp_e2e -- --test-threads=1` passes
- [ ] LSP protocol compliance verified (initialize, shutdown, completion, hover, definition)

## Required Verification

- Run LSP E2E test suite
- Verify all protocol operations work correctly
- Ensure single-threaded execution (--test-threads=1)

## Close Gate

All LSP E2E tests must pass with protocol compliance verified.

## Primary Paths

- `crates/ark-lsp/` (LSP implementation)
- LSP test fixtures
- Extension E2E test suite

## Non-Goals

- Performance optimization
- Feature completeness beyond protocol compliance
