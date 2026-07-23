---
Status: open
Created: 2026-07-23
Updated: 2026-07-23
ID: 833
Track: native-cpp
Depends on: "832"
Orchestration class: open
Orchestration upstream: None
Blocks v{N}: none
Priority: 1
Source: native-cpp GC wall/RSS dual gate
---

# 833 — Re-enable CFG-complete native-cpp root liveness clears

## Summary

`src/compiler/native_c/root_liveness.ark` has a liveness planner skeleton, but
production clears are **explicitly disabled**:

```text
let enable_root_clears = false
```

Incorrect earlier clears caused `String::clone(NULL)` during native S3. Until
CFG-complete liveness (join, loop backedge, stack operands, call args/returns)
is proven, every reference local stays rooted for the whole function.

That keeps the GC live graph huge (~13.8M objects on full S3), so `ARUKELLT_NATIVE_GC=1`
meets the RSS gate (~1.55 GiB) but misses the warm wall gate (~8 min). Arena
path stays operational only via `--allow-high-rss` (~12.3 GiB).

Generated C `= NULL` counts are dominated by **function-entry reference local
initialization**, not safepoint dead-root clears. Do not treat grep counts as
liveness evidence.

## Acceptance

- [ ] Prove liveness on CFG joins, loop backedges, stack operands, call args/returns
- [ ] Set `enable_root_clears = true` with no `String::clone(NULL)` / UAF on
      `tests/fixtures/native_gc_stress/*` under `ARUKELLT_NATIVE_GC_THRESHOLD_BYTES=65536`
- [ ] Receipt fields (not C greps): analyzed functions, safepoints, planned clears,
      emitted clears, peak root slots, per-collection marked objects + mark/sweep ms
- [ ] `native-executor --build` with GC=1: RSS ≤ 2.4 GiB **and** warm wall < 5 min
- [ ] No false-done: stress green alone is not enough without the dual gate

## Plan

Canonical promotion plan: [`docs/plans/native-cpp-experimental-promotion.md`](../../docs/plans/native-cpp-experimental-promotion.md)

Phase order: receipt → emitter effects → CFG liveness → shadow → fixture clears →
full-S3 clears → measure → (only if needed) collector/threshold → 3× strict → docs.

## Non-goals

- `#831` wasm32-gc fixpoint validation (separate)
- Full typed exact heap scan / iterative mark stack (follow-up after live-set shrink)
