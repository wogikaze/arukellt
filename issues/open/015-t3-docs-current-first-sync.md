# T3 docs current-first sync

**Status**: open
**Created**: 2026-03-27
**Updated**: 2026-03-27
**ID**: 015
**Depends on**: 003, 009, 010, 011
**Track**: parallel
**Blocks v1 exit**: no

## Summary
Synchronize all current-first docs with the actual completed T3 implementation and explicitly archive or disclaim older aspirational descriptions.

## Acceptance Criteria
- [ ] No current-first doc still claims T3 is an experimental fallback once the implementation has graduated.
- [ ] No current-first doc claims component emit is implemented unless it actually is.
- [ ] Target status, fixture count, validation policy, and runtime model are consistent across docs.
- [ ] Archive/disclaimer notes remain on older aspirational documents instead of silently deleting historical context.

## Goal
Make the documentation truthful and non-contradictory once T3 is complete enough to end v1.

## Implementation
- Update all current-first docs that mention targets, runtime, validation, or migration.
- Add archive/current-first notes where old design docs would otherwise mislead readers.
- Ensure docs consistency tooling checks T3 status and not only the older T1 status.

## Dependencies
- Issues 003, 009, 010, and 011.

## Impact
- docs tree
- docs consistency script

## Tests
- Docs consistency checks.
- Link checks.
- Sample command smoke tests.

## Docs updates
- `docs/current-state.md`
- `docs/platform/wasm-features.md`
- `docs/migration/t1-to-t3.md`
- `docs/platform/abi.md`
- `docs/quickstart.md`
- `docs/process/policy.md`

## Compatibility
- No compiler behavior change.

## Notes
- This is separate from the implementation issues so docs work can proceed in parallel once code reality is stable.
