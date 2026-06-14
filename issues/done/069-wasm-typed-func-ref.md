---
Status: done
Created: 2026-03-28
Updated: 2026-06-14
Closed: 2026-06-14
ID: 069
Track: wasm-feature
Depends on: none
Orchestration class: design-ready
Blocks v4 exit: none
Blocks v5 exit: none
Source: docs-to-issues audit — revived from issues/reject/; docs/current-state.md documents call_indirect HOF dispatch
Status note: Design slice closed — ADR-033 records phased call_ref migration; emitter implementation deferred to Phase A–C.
---

## Closed — 2026-06-14

Design acceptance slice: [ADR-033](../../docs/adr/ADR-033-call-ref-hof-migration.md) decides phased
`call_ref` migration while `call_indirect` remains baseline. `docs/current-state.md` Closures row
updated with ADR link. Full emitter/benchmark acceptance (#069 items 1–5) remains future work per ADR Phase A–C.

# 069 — Wasm Typed Function References: ref.func / call_ref フル活用

## Summary

WebAssembly Typed Function References 提案 (`docs/spec/spec-3.0.0/proposals/function-references/Overview.md`) の
`ref.func`・`call_ref`・`br_on_null`・`br_on_non_null` を Arukellt のクロージャ実装に完全活用する。
現在のクロージャ実装は `call_indirect` による HOF dispatch を使用している（`docs/current-state.md`）。

## Evidence source

- `docs/current-state.md` — Closures row: `call_indirect` for HOF dispatch
- `docs/spec/spec-3.0.0/proposals/function-references/Overview.md`
- `src/compiler/emitter.ark` — closure lowering
- `issues/reject/069-wasm-typed-func-ref.md` — prior tracking (revived 2026-06-12)

## Primary paths

- `src/compiler/emitter.ark`
- `tests/fixtures/` (closure / HOF fixtures)
- `docs/current-state.md` (GC-Native Data Model table)

## Non-goals

- v4 exit blocker (Blocks v4 exit: none)
- Eliminating all `call_indirect` before v5 (incremental migration acceptable)
- `return_call_ref` tail-call work beyond #492 scope

## Acceptance

- [x] `docs/current-state.md` Closures row updated to reflect phased `call_ref` adoption (ADR-033)
- Deferred (ADR-033 Phase A–C): typed `call_ref` emission, `ref.func` audit, nullable-ref branches, benchmark gate

## Required verification

```bash
python3 scripts/manager.py verify quick
python3 scripts/manager.py verify fixtures
```

## Close gate

At least one HOF fixture proves `call_ref` emission; current-state docs no longer claim exclusive `call_indirect` dispatch without qualification.
