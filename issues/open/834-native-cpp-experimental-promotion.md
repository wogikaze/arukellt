---
Status: open
Created: 2026-07-23
Updated: 2026-07-23
ID: 834
Track: native-cpp
Depends on: "833"
Orchestration class: implementation
Orchestration upstream: None
Blocks v{N}: none
Priority: 1
Source: native-cpp experimental promotion final gate
---

# 834 — Complete native-cpp selfhost executor experimental promotion

## Summary

Umbrella issue for completing every phase after root-liveness shadow analysis. Closing #833 is not the end of the mission.
Continue through production root clears, performance dual gate, strict 3-run validation, CI enforcement, docs/state sync,
and final experimental promotion.

Canonical plan: [`docs/plans/native-cpp-experimental-promotion.md`](../../docs/plans/native-cpp-experimental-promotion.md)

## Continuation contract

- [ ] Do not stop at a phase boundary
- [ ] Do not stop after closing #833
- [ ] Do not stop after one successful strict run
- [ ] Continue independent work when one subtask is blocked
- [ ] Stop only for a documented hard external blocker defined by the canonical plan

## Acceptance

- [ ] Phase 2 liveness proof, unit tests, safepoint audit, and receipt completion
- [ ] Phase 3 fixture, staged, and all-function production root-clear rollout
- [ ] Phase 4 wall/RSS dual gate after measurement-led optimization
- [ ] Phase 5 live-IR owner release if Phase 4 remains insufficient
- [ ] Phase 6 stress, sanitizer, full correctness, and strict 3 consecutive PASS
- [ ] Phase 7 manager/CI enforcement with high-RSS override forbidden in CI
- [ ] Phase 8 docs/state/false-done sync and promotion
- [ ] Every item in the plan's Final Experimental Promotion Checklist is complete
- [ ] `NATIVE_CPP_EXPERIMENTAL_PROMOTION: COMPLETE` receipt is saved

## Non-goals

- Public `arukellt run --target native-cpp`
- Stable public C ABI
- Distributed native executable product
- #831 wasm32-gc canonical fixpoint repair
