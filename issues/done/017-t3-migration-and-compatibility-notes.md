---
Status: done
Created: 2026-03-27
Updated: 2026-03-30
ID: 15
Track: parallel
Depends on: 011, 015
Orchestration class: implementation-ready
---
# T3 migration and compatibility notes
**Blocks v1 exit**: no

## Summary

Document how users and internal maintainers move from the T1-first world to the T3-primary world without ambiguity.

## Acceptance Criteria

- [x] Migration docs explain what changed, what did not change, and what remains out of scope for v1.
- [x] Internal notes map old assumptions (T1 default, T3 fallback) to the new T3-primary reality.
- [x] Component emit remains explicitly out of scope unless later implemented.
- [x] No migration doc contradicts current-first status pages.

## Goal

Ensure the shift from T1-primary to T3-primary is documented both externally and internally.

## Implementation

- Update `docs/migration/t1-to-t3.md` with the completed-state migration guidance.
- Update `docs/process/internal-api-migration.md` with any internal API assumptions that changed during the T3 transition.
- Explicitly list behavior changes, non-changes, and out-of-scope items.

## Dependencies

- Issues 011 and 015.

## Impact

- migration docs
- internal notes

## Tests

- Docs consistency checks.
- Example command smoke tests.

## Docs updates

- `docs/migration/t1-to-t3.md`
- `docs/process/internal-api-migration.md`

## Compatibility

- Documentation-only, but critical for preventing downstream confusion.

## Notes

- State clearly whether the CLI default target changed or stayed the same.