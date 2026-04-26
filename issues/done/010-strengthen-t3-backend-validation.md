# Strengthen T3 backend validation

**Status**: done
**Created**: 2026-03-27
**Updated**: 2026-03-30
**ID**: 010
**Depends on**: 004, 005, 006, 007, 008, 009
**Track**: main
**Blocks v1 exit**: yes

## Summary

Make backend validation for WasmGC artifacts strong and deterministic enough that invalid T3 output never passes the build as a successful compile.

## Acceptance Criteria

- [x] Malformed T3/WasmGC modules fail the build reliably.
- [x] Validation diagnostics for T3 backend failures are stable and attributed to backend validation rather than generic typecheck errors.
- [x] T3 backend validation is enforced in the same hard-error way current policy describes for `W0004`.
- [x] Negative tests exist for broken GC type/layout/import scenarios.

## Goal

Turn T3 backend validation into a trustworthy final gate.

## Implementation

- Review `crates/ark-wasm/src/emit/mod.rs` validation flow for T3-specific WasmGC output.
- Expand negative coverage around invalid heap types, ref usage, broken subtype declarations, and broken layout/import combinations.
- Ensure the backend emits diagnostics that clearly identify validation as the failure phase.
- Keep `W0004` hard-error behavior intact.

## Dependencies

- Issues 004 through 009.

## Impact

- backend validation path
- diagnostics
- T3 negative tests

## Tests

- malformed WasmGC negative tests.
- diagnostic snapshot tests.
- full T3 compile smoke with validation enabled.

## Docs updates

- `docs/compiler/diagnostics.md`
- `docs/process/policy.md`
- `docs/current-state.md`

## Compatibility

- May reject outputs that previously slipped through.
- This is an intended correctness tightening.

## Notes

- Do not downgrade validation failures to warnings for T3.
