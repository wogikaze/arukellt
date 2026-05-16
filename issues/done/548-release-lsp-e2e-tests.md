# Release: LSP E2E Tests

> **Status:** done
> **Track:** release
> **Type:** Verification

## Scope

Ensure LSP E2E tests pass for release verification.

## Checklist Source

docs/release-checklist.md — Pre-release section

## Acceptance

- [x] `cargo test -p ark-lsp --test lsp_e2e -- --test-threads=1` passes
- [x] LSP protocol compliance verified (initialize, shutdown, completion, hover, definition)

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

## Recheck — 2026-05-14

The older evidence above refers to the retired Rust `crates/ark-lsp` test
suite. In the current repository state, `crates/ark-lsp` has been removed and
the active LSP implementation is the selfhost server in `src/compiler/lsp.ark`
exposed through `arukellt lsp`.

Current evidence:

- `python3 scripts/manager.py verify quick` passes 22/22 and includes the
  selfhost LSP lifecycle gate (#569).
- `extensions/arukellt-all-in-one/src/test/extension.test.js` directly drives
  `arukellt lsp` for initialize, hover, and definition coverage.
- At this recheck point, `src/compiler/lsp.ark` advertised `hoverProvider` and
  `definitionProvider` but did not advertise or handle
  `textDocument/completion`.
- Rebuilding a fresh selfhost `s2` wasm from the pinned bootstrap wasm still
  fails in this workspace, so changing `src/compiler/lsp.ark` cannot yet be
  honestly verified through the selfhost lifecycle gate.

Updated verdict: close-candidate `no`. The current release checklist asks for
initialize, shutdown, completion, hover, and definition protocol compliance.
Completion coverage was not implemented in the active selfhost LSP surface at
the time of this recheck, and the old
`cargo test -p ark-lsp --test lsp_e2e -- --test-threads=1` command is no longer
valid after the crate retirement.

## Progress — 2026-05-14

Implemented the missing completion surface in `src/compiler/lsp.ark`:

- Initialize capabilities now advertise `completionProvider` with
  `resolveProvider: false` and trigger characters `"."` / `":"`.
- `textDocument/completion` is routed in the selfhost LSP dispatcher.
- Added a minimal static `CompletionList` response containing language keywords,
  common std modules, and common builtins.

Verification status:

- Source inspection confirms `completionProvider`, `textDocument/completion`,
  `build_completion_response`, and `handle_completion` are present.
- `target/release/arukellt check src/compiler/main.ark` still fails before it can
  validate the updated source because the current selfhost/bootstrap compiler
  traps with an out-of-bounds memory access. This is the same class of blocker
  noted above for rebuilding fresh selfhost `s2` wasm.

Updated verdict remains close-candidate `no` until the active selfhost wasm can
be rebuilt from source and an LSP E2E/golden test covers initialize, shutdown,
completion, hover, and definition together.

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
