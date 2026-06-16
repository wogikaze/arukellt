---
Status: open
Created: 2026-06-17
Updated: 2026-06-17
ID: 671
Track: language-design
Parent: 124
Depends on: "653, 654 (done)"
Orchestration class: implementation-ready
Orchestration upstream: None
Blocks v{N}: none
Priority: 2
Source: P0 WIT import resolver checklist audit 2026-06-17 — per-type fixture gaps
---

# 671 — WIT import callable type matrix (fixtures + gates)

## Summary

Callable scalar WIT imports work for `add(s32,s32)->s32` and basic record/enum
fixtures (#653/#664). The audit checklist expects explicit compile/run (or
component-compile) fixtures per WIT type shape. Most shapes lack dedicated fixtures
and close-gate coverage.

Negative fixtures for `flags` and `resource` are **obsolete** (#651, #473 closed);
keep negative coverage for `stream<T>` / `future<T>` only.

## Acceptance

Each row needs a `tests/fixtures/wit_import/types/<shape>/` fixture pair (`.ark` +
`.wit`), manifest entry, and gate assertion:

- [ ] `bool` parameter and result
- [ ] `i64` parameter and result
- [ ] `f32` parameter and result
- [ ] `f64` parameter and result
- [ ] `string` parameter and result
- [ ] `list<s32>` parameter and result
- [ ] `option<s32>` parameter and result
- [ ] `result<s32, string>` result
- [ ] `tuple<s32, s32>` parameter
- [ ] `record` result (parameter covered by `record_field.ark`)
- [ ] `variant` parameter and result
- [ ] Negative: `stream<T>` import rejected (`E0402`)
- [ ] Negative: `future<T>` import rejected (`compile-error` manifest)
- [ ] `mir-dump` or `backend-plan` snapshot for at least one new shape
- [ ] Close gate `scripts/check/gate-671-wit-import-type-matrix.py`
- [ ] `python3 scripts/manager.py verify quick` exits 0

## Out of scope

- WIT binding **code generation** (#672)
- Full component round-trip for every shape (covered incrementally; compose E2E
      remains #665 baseline)

## References

- `tests/fixtures/wit_import/check/call_add.ark`
- `issues/done/653-wit-import-resolver-mir.md`
- `issues/done/654-wit-import-component-emit.md`
