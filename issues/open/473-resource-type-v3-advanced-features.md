# Resource type v3+: 継承・async drops・クロスコンポーネント転送・ハンドル GC

**Status**: open
**Created**: 2026-04-03
**Updated**: 2026-04-03
**ID**: 473
**Depends on**: 032 (resource-type, done)
**Track**: wasm-feature
**Orchestration class**: blocked-by-upstream
**Orchestration upstream**: #32
**Blocks v4 exit**: no

---

## Reopened by audit — 2026-04-03

**Source**: Future-work gap extracted from `issues/done/032-resource-type.md`.

**Quote**: "Advanced features (resource inheritance, async resource drops,
cross-component resource forwarding) are deferred to v3+." and
"No garbage collection of handles is implemented in v2; leaked handles are a known limitation."

**Action**: New open issue created per audit rule — deferred items in done issues must
have a corresponding open issue.

---

## Summary

Issue 032 completed v2 basic resource support (own/borrow handle passing). This issue
tracks the advanced resource features deferred to v3+:

1. **Resource inheritance** — subtyping of resource types across WIT interfaces
2. **Async resource drops** — `resource.drop` in async context without deadlock
3. **Cross-component resource forwarding** — transferring handles across component boundaries
4. **Handle GC** — automatic garbage collection of leaked handles (current: bounded i32 counter, no GC)

## Non-goals

- Basic resource own/borrow (done in 032)
- WASI P2 capability resources that don't require inheritance (done in 032)
- v4 or v5 feature additions beyond resource type system

## Primary paths

- `crates/ark-wasm/src/emit/t3/cabi_adapters.rs` — current handle table implementation
- `crates/ark-wasm/src/emit/t3/` — T3 emitter
- `crates/ark-mir/src/lib.rs` — MIR resource representation
- `tests/fixtures/component/` — component model fixtures

## Acceptance

- [ ] Resource subtype declarations compile without error in `.ark` source
- [ ] Async resource drop emits correct WASM GC instructions
- [ ] Cross-component resource handle forwarding works in a smoke test
- [ ] Handle table GC releases unused handles (no leak after N allocations)
- [ ] `python scripts/manager.py verify` passes

## Required verification

- New fixtures in `tests/fixtures/component/resource_v3/`
- `cargo test -p ark-wasm` passes new resource v3 tests

## Close gate

All 5 acceptance items checked with repo-internal evidence; no deferred items remain.
