# LLVM scaffold follow-up after T3 completion

**Status**: open
**Created**: 2026-03-27
**Updated**: 2026-03-27
**ID**: 018
**Depends on**: none
**Track**: parallel
**Blocks v1 exit**: no

## Summary
Bring `ark-llvm` and related optional paths back into structural compatibility after the T3 completion work, without letting T4 scope derail v1 exit.

## Acceptance Criteria
- [ ] Optional LLVM feature builds are structurally compatible with the final MIR/backend contracts.
- [ ] T4 remains subordinate to Wasm semantics and does not add native-only semantic divergence.
- [ ] Workspace checks do not fail purely because the optional LLVM scaffold has drifted away from current MIR shape.
- [ ] This work does not expand T4 into a v1 exit dependency.

## Goal
Prevent optional backend drift from obscuring or undermining the completed T3 world.

## Implementation
- Update `crates/ark-llvm/**` to match the final MIR structures after T3 completion.
- Keep T4 behavior explicitly scaffold/subordinate; do not add native-only optimizations or semantics.
- Adjust optional-build checks so the workspace remains coherent.

## Dependencies
- Any MIR/ABI work needed by completed T3.
- Not part of the v1 exit gate itself.

## Impact
- `crates/ark-llvm/**`
- optional CI jobs

## Tests
- `--features llvm` build smoke.
- Optional target smoke tests.

## Docs updates
- `docs/adr/ADR-005-llvm-scope.md`
- `docs/current-state.md`

## Compatibility
- Optional path only.

## Notes
- This issue is deliberately parallel/non-blocking for v1 exit.
