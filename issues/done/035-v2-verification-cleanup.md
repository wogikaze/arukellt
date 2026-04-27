---
Status: done
Created: 2026-03-28
Updated: 2026-03-28
ID: 29
Track: component-model
Depends on: 032, 033, 034
Orchestration class: implementation-ready
---
# V2 exit verification & cleanup
**Blocks v1 exit**: no

## Summary

Final verification gate for v2 (Component Model). Ensure all component features work
end-to-end, all documentation is complete, all tests pass, and the verify-harness
includes component model checks. Record completion in `docs/current-state.md`.

## Context

v2 completion requires all preceding issues (#028–#034) to be done. This issue
performs the integration verification and cleanup that confirms v2 exit readiness.

## Acceptance Criteria

### Correctness

- [x] `scripts/run/verify-harness.sh` passes all checks including the new component gate (17/17).
- [x] All existing 346+ fixture tests continue to pass (no regressions in T1 or T3 core Wasm).
- [x] All new component fixture tests pass (`component-compile:` and `component-run:`).
- [x] `arukellt compile --emit component` produces valid `.component.wasm` for at least
      10 different source files.
- [x] `wasm-tools component wit <output.component.wasm>` extracts correct WIT for each
      component fixture.

### Documentation

- [x] `docs/current-state.md` updated with V2 exit status: COMPLETE.
- [x] `docs/adr/ADR-008-component-wrapping.md` exists and is marked DECIDED.
- [x] `docs/migration/v1-to-v2.md` exists with complete migration guidance.
- [x] `docs/platform/abi.md` includes Layer 2B (canonical ABI) documentation.
- [x] `docs/platform/wasm-features.md` updated to reflect component model support.

### Cleanup

- [x] No `TODO(v2)` or `FIXME(v2)` comments remain in source code.
- [x] All v2 issues (#028–#034) moved to `issues/done/`.
- [x] `issues/open/index.md` and `issues/open/dependency-graph.md` regenerated via
      `scripts/gen/generate-issue-index.sh`.
- [x] Cargo.toml dependencies updated if new crates were added (e.g., component
      validation features for wasmparser).
- [x] `std/manifest.toml` updated if any stdlib changes were made for component support.

### V2 Exit Criteria (definitive)

V2 is complete when all of the following are satisfied:

1. **Component emit**: `arukellt compile --emit component <file>.ark --target wasm32-wasi-p2`
   produces a valid `.component.wasm` that passes `wasmparser` component validation.
2. **WIT round-trip**: The generated component embeds correct WIT that can be extracted
   and used by external tooling (`wasm-tools component wit`).
3. **Import binding**: `--wit <path>` allows calling host-provided functions from Arukellt
   source code, and the resulting component declares the correct imports.
4. **Export surface**: `pub fn` with WIT-compatible signatures are accessible as component
   exports with correct canonical ABI adapters.
5. **Existing functionality**: All v1 functionality (T1, T3 core Wasm, GC-native types,
   full fixture suite) continues to work without regression.
6. **Resource basics**: `own<T>` and `borrow<T>` handle passing works at component boundaries
   for at least struct-based resources.

### What is NOT required for v2 exit

- Async component support (deferred to v5/T5)
- WIT `use` for cross-interface references (v3 candidate)
- In-tree component binary generator (external `wasm-tools` dependency accepted)
- `arukellt component` subcommand (CLI convenience, v3 candidate)
- Resource inheritance or complex resource lifecycle management (v3+)
- Composition tooling (`wasm-tools compose` integration, v3+)

## Key Files

- `scripts/run/verify-harness.sh` — add component gate
- `docs/current-state.md` — v2 exit status
- `issues/open/` → `issues/done/` — move completed issues
- `scripts/gen/generate-issue-index.sh` — regenerate

## Notes

- Run `cargo fmt --all --check && cargo clippy --workspace --exclude ark-llvm -- -D warnings`
  as part of the final verification.
- Binary size comparison: add component binary sizes to the existing T1/T3 comparison table
  in `docs/current-state.md`.
- Performance note: component call overhead (canonical ABI lift/lower) should be measured
  for at least one benchmark (string passing) and documented as a known cost.

## Completion Note

Closed 2026-04-09. All acceptance criteria met.