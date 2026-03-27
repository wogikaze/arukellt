# Complete T3 emitter compile correctness

**Status**: open
**Created**: 2026-03-27
**Updated**: 2026-03-27
**ID**: 004
**Depends on**: 002, 003
**Track**: main
**Blocks v1 exit**: yes

## Summary
Finish `crates/ark-wasm/src/emit/t3_wasm_gc.rs` so T3 compile succeeds across the language surface covered by current v1 fixtures and representative samples.

## Acceptance Criteria
- [ ] `arukellt compile --target wasm32-wasi-p2` succeeds for the full T3 fixture matrix defined in issue 002.
- [ ] T3 compile no longer depends on hidden delegation to T1 behavior for major frontend constructs.
- [ ] Unsupported or malformed T3 output fails with backend diagnostics instead of silently succeeding.
- [ ] Emitter comments and docs no longer overstate T3 completeness.

## Goal
Make T3 compile itself correct and broad before attempting to declare T3 primary.

## Implementation
- Audit `crates/ark-wasm/src/emit/t3_wasm_gc.rs` for partial implementations, bridge-only behavior, and places where heap values are still effectively lowered as T1-compatible linear-memory pointers.
- Ensure compile support for:
  - primitives
  - function calls and selected calls
  - methods and operators selected in frontend
  - control flow / loops / `?`
  - structs / enums / match
  - closures
- Align T3 intrinsic handling with T1 semantics through `normalize_intrinsic` and related dispatch.
- Remove misleading comments such as “real Wasm GC types” where bridge/fallback behavior still exists; keep comments truthful until later tasks complete the full GC-native path.

## Dependencies
- Issues 002 and 003.

## Impact
- `crates/ark-wasm/src/emit/t3_wasm_gc.rs`
- `crates/ark-wasm/src/emit/mod.rs`
- possible MIR/type-table touch points if missing information blocks correctness

## Tests
- T3 compile fixtures.
- Targeted emitter unit tests.
- Negative tests for malformed backend output.

## Docs updates
- `docs/platform/wasm-features.md`
- `docs/current-state.md`

## Compatibility
- T3 generated code changes substantially.
- Source language semantics must remain aligned with T1.

## Notes
- This issue is about correctness, not yet about removing every linear-memory bridge. Those are handled by the data-model issues below.
