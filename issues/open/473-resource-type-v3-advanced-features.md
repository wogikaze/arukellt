---
Status: open
Created: 2026-04-03
Updated: 2026-06-11
ID: 473
Track: wasm-feature
Depends on: "032 (resource-type, done)"
Orchestration class: implementation-ready
Orchestration upstream: None
---

# Resource type v3+: 継承・async drops・クロスコンポーネント転送・ハンドル GC

## Summary

Issue 032 completed v2 basic resource support (own/borrow handle passing). This issue
tracks the advanced resource features deferred to v3+:

1. **Resource inheritance** — subtyping of resource types across WIT interfaces
2. **Async resource drops** — `resource.drop` in async context without deadlock
3. **Cross-component resource forwarding** — transferring handles across component boundaries
4. **Handle GC** — automatic garbage collection of leaked handles (current: bounded i32 counter, no GC)

## Progress (2026-06-11) — foundation slice

Selfhost compiler pipeline now parses `resource` declarations end-to-end and emits WIT
`resource { ... }` text. Five `tests/fixtures/component/resource_v3/*.ark` fixtures
compile on `wasm32-wasi-p2`. WIT import binding for `own`/`borrow` remains rejected (E0402)
until canonical ABI handle-table lifting is implemented.

## Non-goals

- Basic resource own/borrow (done in 032)
- WASI P2 capability resources that don't require inheritance (done in 032)
- v4 or v5 feature additions beyond resource type system

## Primary paths

- `src/compiler/component/wit_type_defs.ark`, `wit_decl.ark` — WIT resource emission
- `src/compiler/parser/decl_resource*.ark` — `resource` declaration parsing
- `src/compiler/resolver/`, `src/compiler/typechecker/` — `SYM_RESOURCE` / `TY_RESOURCE`
- `tests/fixtures/component/resource_v3/` — compile fixtures

## Acceptance

- [x] Resource subtype declarations compile without error in `.ark` source (basic `resource Foo { ... }` only; WIT subtyping not yet)
- [ ] Async resource drop emits correct WASM GC instructions
- [ ] Cross-component resource handle forwarding works in a smoke test
- [ ] Handle table GC releases unused handles (no leak after N allocations)
- [x] `python scripts/manager.py verify quick` passes

## Required verification

- [x] New fixtures in `tests/fixtures/component/resource_v3/`
- [ ] Runtime smoke for async drop, forwarding, and GC (future slices)

## Close gate

All acceptance items checked with repo-internal evidence; no deferred items remain.
