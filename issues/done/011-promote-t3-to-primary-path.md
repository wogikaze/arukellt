# Promote T3 to primary path

**Status**: done
**Created**: 2026-03-27
**Updated**: 2026-03-30
**ID**: 011
**Depends on**: 009, 010
**Track**: main
**Blocks v1 exit**: yes

## Summary

Once T3 compile/run/validation are complete, update the target tables, help text, verification policy, and user guidance so T3 is the canonical v1 path.

## Acceptance Criteria

- [x] Current-first docs no longer describe T3 as experimental fallback.
- [x] Verification policy treats T3 as a primary correctness gate.
- [x] CLI help and target tables describe T1 as compatibility/stable legacy path and T3 as canonical v1 path.
- [x] The branch can explain clearly whether the default CLI target remains T1 for compatibility or changes to T3, with docs and code aligned.

## Goal

Finish the social/operational side of making T3 the real path, not just the technically complete one.

## Implementation

- Update target help/status in `crates/ark-target/src/lib.rs`.
- Update `docs/current-state.md`, `docs/platform/wasm-features.md`, `docs/quickstart.md`, and `docs/migration/t1-to-t3.md` to reflect T3-first reality.
- Update verify scripts/CI so T3 compile/run is a normal gate.
- Decide and implement whether the CLI default target changes now or whether T3 is only the canonical documented path while T1 remains default for compatibility.

## Dependencies

- Issues 009 and 010.

## Impact

- CLI help text
- docs
- CI policy
- possible target default behavior

## Tests

- Help snapshot tests.
- Verify-harness smoke.
- Docs consistency checks.

## Docs updates

- `docs/current-state.md`
- `docs/platform/wasm-features.md`
- `docs/quickstart.md`
- `docs/migration/t1-to-t3.md`
- `docs/process/policy.md`

## Compatibility

- If the default target changes, this is user-visible and must be documented explicitly.
- If the default target does not change, documentation must state why T3 is still the canonical path.

## Notes

- Do not leave “experimental” wording in any current-first page after this issue is complete.
