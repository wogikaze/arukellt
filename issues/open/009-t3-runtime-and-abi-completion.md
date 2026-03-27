# T3 runtime and ABI completion

**Status**: open
**Created**: 2026-03-27
**Updated**: 2026-03-27
**ID**: 009
**Depends on**: 003, 004, 005, 006, 007, 008
**Track**: main
**Blocks v1 exit**: yes

## Summary
Replace the current GC-enabled but Preview-1-backed T3 run path with a runtime/ABI model that truthfully matches completed T3 behavior.

## Acceptance Criteria
- [ ] `run --target wasm32-wasi-p2` no longer uses the P1 internal fallback path documented today.
- [ ] `RuntimeModel::T3FallbackToT1` is no longer the runtime model used for completed T3 execution.
- [ ] Compile artifact import/export expectations match the runtime implementation.
- [ ] Current-first docs no longer describe T3 runtime as Preview 1 internally.

## Goal
Ensure T3 is not only compile-complete but also runtime-true.

## Implementation
- Rework `crates/arukellt/src/runtime.rs` so `run_wasm_gc()` does not remain a P1-linked compatibility mode for the completed T3 path.
- Introduce and wire the true completed T3 runtime model from `crates/ark-target/src/plan.rs` into runtime dispatch.
- Align import/export contracts between backend plan, Wasm emitter, and runtime loader.
- Keep filesystem/clock/random policy current-first and explicit; if capability filtering is still incomplete, document exactly what remains and keep it out of the v1 exit gate only if it is already policy-approved.

## Dependencies
- Issues 003 through 008.

## Impact
- `crates/arukellt/src/runtime.rs`
- `crates/arukellt/src/commands.rs`
- `crates/ark-target/src/plan.rs`
- possible target/help docs

## Tests
- T3 run e2e tests.
- Runtime import/export smoke tests.
- Capability/host-bridge smoke tests.

## Docs updates
- `docs/current-state.md`
- `docs/platform/abi.md`
- `docs/platform/wasm-features.md`
- `docs/migration/t1-to-t3.md`

## Compatibility
- T3 runtime behavior changes materially.
- T1 remains intact.

## Notes
- A GC-enabled Wasmtime config linked against Preview 1 is not sufficient to claim T3 completion.
