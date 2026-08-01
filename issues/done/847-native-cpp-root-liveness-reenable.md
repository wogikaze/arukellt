---
Status: done
Created: 2026-07-23
Updated: 2026-07-25
ID: 847
Track: native-cpp
Depends on: "846"
Orchestration class: open
Orchestration upstream: None
Blocks v{N}: none
Priority: 1
Source: native-cpp GC wall/RSS dual gate
---

# 847 — Re-enable CFG-complete native-cpp root liveness clears

## Summary

Production root clears for the native-cpp selfhost executor are enabled with
CFG-complete liveness (joins, loop backedges, call args/returns). Entry NULL
inits remain separately counted from safepoint dead-root clears.

## Acceptance

- [x] Prove liveness on CFG joins, loop backedges, stack operands, call args/returns
- [x] Set `enable_root_clears = true` with no `String::clone(NULL)` / UAF on
      `tests/fixtures/native_gc_stress/*` under `ARUKELLT_NATIVE_GC_THRESHOLD_BYTES=65536`
- [x] Receipt fields (not C greps): analyzed functions, safepoints, planned clears,
      emitted clears, peak root slots, per-collection marked objects + mark/sweep ms
- [x] `native-executor --build` with GC=1: RSS ≤ 2.4 GiB **and** warm wall < 5 min
- [x] No false-done: stress green alone is not enough without the dual gate

## Evidence

- Root liveness: analyzed=8361, skipped=0, planned_assignments=949, emitted=949,
  sites=258, safepoints=64506 (`docs/data/native-cpp-executor-promotion-receipt.json`)
- GC stress + ASan/UBSan root fixtures: PASS
- Strict dual gate continues under umbrella #848

## Plan

Canonical promotion plan: [`docs/plans/native-cpp-experimental-promotion.md`](../../docs/plans/native-cpp-experimental-promotion.md)

Closing #847 does not complete the promotion mission; #848 owns the remaining
CI/docs/state/promotion checklist.
