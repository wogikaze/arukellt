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

## Verification Evidence

- 2026-04-22: `cargo test -p ark-lsp --test lsp_e2e -- --test-threads=1`
  - Result: PASS
  - Summary: `34 passed; 0 failed; 1 ignored; 0 measured; 0 filtered out`
  - Notes: `snapshot_definition_shadowed_let_points_to_inner` remains ignored in the suite; no LSP E2E failures were observed. `ark-wasm` emitted two pre-existing `unused_assignments` warnings during the test build, but they did not affect the result.
- 2026-04-22: reran `cargo test -p ark-lsp --test lsp_e2e -- --test-threads=1`
  - Result: PASS
  - Summary: `34 passed; 0 failed; 1 ignored; 0 measured; 0 filtered out`
  - Notes: the ignored `snapshot_definition_shadowed_let_points_to_inner` test is a known shadowing-resolution gap tracked by issue #450; it does not block the required initialize/shutdown/completion/hover/definition protocol coverage for this release check.

## Verdict

- Command validity: PASS
- LSP protocol compliance evidence: PASS from the E2E suite
- Close gate status: YES
- Reviewer verdict: close-candidate yes
- Blockers: None for the requested verification command; the ignored shadowing snapshot is expected and non-blocking for this release slice
- DONE_WHEN: no

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
