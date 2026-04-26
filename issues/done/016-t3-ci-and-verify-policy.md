# T3 CI and verify policy

**Status**: done
**Created**: 2026-03-27
**Updated**: 2026-03-30
**ID**: 016
**Depends on**: 002, 009, 010, 011
**Track**: parallel
**Blocks v1 exit**: no

## Summary

Update verification and CI so T3 compile/run regressions are treated as primary failures and heavy perf work remains isolated.

## Acceptance Criteria

- [x] Normal correctness CI includes T3 compile coverage and representative T3 run coverage.
- [x] Heavy perf comparisons remain on a separate job and do not destabilize normal correctness verification.
- [x] Docs consistency checks include T3 target status drift.
- [x] Verification policy text matches the implemented workflow.

## Goal

Turn T3 completion into an enforced policy, not a one-time milestone.

## Implementation

- Update `scripts/run/verify-harness.sh` to run the T3 fixture matrix and representative T3 smoke tests.
- Update CI workflows to separate normal correctness from heavy perf telemetry.
- Update docs consistency checks so drift on T3 target status/runtime model fails verification.
- If `cargo check --workspace` still includes optional paths that are intentionally unstable, document exactly how they are handled in CI.

## Dependencies

- Issues 002, 009, 010, and 011.

## Impact

- `scripts/run/verify-harness.sh`
- `.github/workflows/ci.yml`
- docs consistency scripts

## Tests

- Workflow smoke/dry-run.
- Verify script smoke tests.

## Docs updates

- `docs/process/policy.md`
- `docs/contributing.md`

## Compatibility

- No user-facing behavior change.
- Developer workflow becomes stricter.

## Notes

- T3 should be treated as a normal correctness gate after this issue, not as an opt-in experimental check.
