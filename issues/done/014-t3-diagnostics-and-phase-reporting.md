---
Status: done
Created: 2026-03-27
Updated: 2026-03-30
ID: 12
Track: parallel
Depends on: 004, 005, 006, 007, 008, 009, 010
Orchestration class: implementation-ready
---
# T3 diagnostics and phase reporting
**Blocks v1 exit**: no

## Summary

Make T3-specific failures diagnosable by giving them stable codes/messages and accurate phase origin rather than generic fallback diagnostics.

## Acceptance Criteria

- [x] T3 backend/runtime failures are distinguishable from typecheck or generic target errors.
- [x] Negative tests snapshot T3-specific diagnostics.
- [x] Phase-aware reporting covers at least compile, backend validation, and runtime bridge failures for T3.
- [x] Current-first diagnostics docs mention the relevant T3 failure categories.

## Goal

Prevent T3 work from degenerating into opaque backend failures.

## Implementation

- Extend diagnostics in `crates/ark-diagnostics` and T3 backend/runtime call sites to carry clear phase origin and stable wording for T3-specific failures.
- Add snapshots for invalid GC layout, invalid import/export bridge, runtime ABI mismatch, and backend validation failures.
- Ensure backend-validate and runtime failures are not collapsed into unrelated E02xx diagnostics.

## Dependencies

- Issues 004 through 010.

## Impact

- diagnostics crate
- T3 backend/runtime emit paths
- negative fixture coverage

## Tests

- Diagnostic snapshot tests.
- Negative compile/runtime tests.

## Docs updates

- `docs/compiler/diagnostics.md`

## Compatibility

- Message wording may change; failure classification becomes more precise.

## Notes

- This issue improves developer velocity and triage quality during the T3 completion push.