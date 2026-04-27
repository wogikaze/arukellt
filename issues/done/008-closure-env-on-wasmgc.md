---
Status: done
Created: 2026-03-27
Updated: 2026-03-30
ID: 008
Track: main
Depends on: 004, 007
Orchestration class: implementation-ready
---
# Closure environment on WasmGC
**Blocks v1 exit**: yes

## Summary

Complete closure/function-value support for T3 by representing capture environments and call paths in a GC-native way.

## Acceptance Criteria

- [x] T3 compile/run works for representative closure fixtures.
- [x] Closure environments use a stable T3 representation rather than a T1 pointer assumption.
- [x] `SharedRef` vs `ValueCopy` capture semantics remain intact through backend lowering.
- [x] Closure implementation details are documented for downstream MIR/backend maintainers.

## Goal

Remove closures as a blocker for claiming T3 compile completeness.

## Implementation

- Implement GC-native closure env representation in the T3 backend.
- Ensure closure lowering, function-value passing, and captured locals agree on layout/ABI.
- Preserve frontend value-mode metadata (`SharedRef` vs `ValueCopy`) when lowering captures.
- Keep correctness first; non-escaping optimization can remain a later concern.

## Dependencies

- Issues 004 and 007.

## Impact

- T3 backend
- MIR closure lowering assumptions
- closure fixtures

## Tests

- Basic closure fixtures.
- Nested closure fixtures.
- Shared-reference capture regression tests.

## Docs updates

- `INTERFACE-COREHIR.md` if contract details need extension.
- `docs/language/memory-model.md`

## Compatibility

- T3 closure representation changes.
- Source semantics must not change.

## Notes

- Do not silently convert shared captures into copied captures.