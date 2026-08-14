---
Status: done
Created: 2026-06-16
Updated: 2026-08-14
Closed: 2026-08-14
ID: 667
Track: component-model
Depends on: 666
Priority: 2
---

# 667 — Library component routing: scalar emitter bypasses specialized / WIT-complete path

## Close summary

Library component emission is specialized-first again. `src/compiler/component/emit.ark` now invokes `emit_specialized_component` before generic scalar export sections, including the flattened bootstrap facade path. Existing string/record/list/option/result adapters therefore no longer lose their WIT shape merely because the module is classified as a library.

The scalar calculator path remains the fallback for shapes without a specialized adapter. The current bootstrap provenance already records the later #834 pin→s2→s3 byte-for-byte fixpoint, so the historical drift recorded when this issue was opened is no longer an outstanding exception.

## Acceptance closure

- [x] Library emit dispatch is specialized-first before scalar generic lowering.
- [x] String-greet and record-point selfhost scripts are part of the runtime close gate when a branch-built s2 is available.
- [x] Scalar calculator compile/invoke is retained in the same runtime close gate.
- [x] Bootstrap fixpoint is governed by `bootstrap/PROVENANCE.md` and records s2 == s3.
- [x] Component state documentation distinguishes routing support from unsupported aggregate shapes.

## Verification

- `scripts/check/gate-667-library-specialized-routing.py`
- `scripts/check/gate-component-wit-productization.py`
