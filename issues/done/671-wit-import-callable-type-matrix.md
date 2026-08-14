---
Status: done
Created: 2026-06-17
Updated: 2026-08-14
Closed: 2026-08-14
ID: 671
Track: language-design
Parent: 124
Depends on: 653, 654
Priority: 2
---

# 671 — WIT import callable type matrix (fixtures + gates)

## Close summary

The WIT import type mapping now preserves compound callable types instead of collapsing typechecker bindings to scalar placeholders. `list<T>`, `option<T>`, `result<T,E>`, tuples and named record/enum/variant bindings are materialized as recursive `TypeInfo` structures; scalar f32/f64/i64/string/bool remain explicit.

The fixture matrix covers bool, i64, f32, f64, string, list<s32>, option<s32>, result<s32,string>, tuple<s32,s32>, record result and payload variant. `stream<T>` and `future<T>` callable compiler imports are explicitly rejected with E0402 until async backend lowering is a supported contract.

## Acceptance closure

- [x] Positive fixture pairs exist for every requested callable shape.
- [x] Record result and variant parameter/result surfaces are represented by named bindings.
- [x] Negative stream/future fixtures are E0402 contracts.
- [x] Type arguments survive into typechecker `TypeInfo`.
- [x] Dedicated matrix close gate is present.

## Verification

- `scripts/check/gate-671-wit-import-type-matrix.py`
- `scripts/check/gate-component-wit-productization.py`
