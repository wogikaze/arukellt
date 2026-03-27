# Align target profile and backend plan

**Status**: open
**Created**: 2026-03-27
**Updated**: 2026-03-27
**ID**: 003
**Depends on**: 001
**Track**: main
**Blocks v1 exit**: yes

## Summary

Resolve the semantic drift between `TargetProfile` and `BackendPlan` for T3 so current shipped behavior and future target intent are represented separately and consistently.

## Acceptance Criteria

- [ ] `TargetId::Wasm32WasiP2` profile, target help text, and docs no longer contradict the current fallback implementation.
- [ ] `crates/ark-target/src/plan.rs` distinguishes current fallback runtime from the true completed T3 runtime model.
- [ ] Plan/profile matching logic does not claim full T3 completion when the runtime model is still fallback-based.
- [ ] Docs and code use the same terms for fallback, experimental, primary, and canonical.

## Goal

Fix the core semantic mismatch where T3 is described as WasmGC + P2 + Component Model in one place and as an experimental fallback in another.

## Implementation

- Update `crates/ark-target/src/lib.rs` so target help/status text reflects current shipped behavior and/or clearly distinguishes ideal target profile from current runtime implementation.
- Update `crates/ark-target/src/plan.rs` to introduce a non-fallback T3 runtime model (e.g. `T3WasmGcP2`) while retaining `T3FallbackToT1` as the current transitional state.
- Update `build_backend_plan()` so the returned runtime model is a truthful representation of current behavior.
- Update `plan_matches_target_profile()` so it no longer silently equates fallback runtime with completed T3 semantics.
- Reconcile wording in `docs/current-state.md`, `docs/platform/wasm-features.md`, and `docs/process/policy.md` with the new code terminology.

## Dependencies

- Issue 001.

## Impact

- `crates/ark-target/src/lib.rs`
- `crates/ark-target/src/plan.rs`
- CLI help and target docs

## Tests

- Target parse tests.
- Backend plan tests.
- Help text snapshot tests.

## Docs updates

- `docs/current-state.md`
- `docs/platform/wasm-features.md`
- `docs/migration/t1-to-t3.md`

## Compatibility

- No target names change.
- Target status/help strings may change.

## Notes

- Treat `TargetProfile` as capability intent and `BackendPlan` as executable reality unless and until both become identical.
