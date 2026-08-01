---
Status: done
Created: 2026-07-23
Updated: 2026-07-25
ID: 848
Track: native-cpp
Depends on: "847"
Orchestration class: implementation
Orchestration upstream: None
Blocks v{N}: none
Priority: 1
Source: native-cpp experimental promotion final gate
---

# 848 — Complete native-cpp selfhost executor experimental promotion

## Summary

Umbrella issue for completing every phase after root-liveness shadow analysis.
Internal native-cpp selfhost executor lane is now experimental.

Canonical plan: [`docs/plans/native-cpp-experimental-promotion.md`](../../docs/plans/native-cpp-experimental-promotion.md)

## Continuation contract

- [x] Do not stop at a phase boundary
- [x] Do not stop after closing #847
- [x] Do not stop after one successful strict run
- [x] Continue independent work when one subtask is blocked
- [x] Stop only for a documented hard external blocker defined by the canonical plan

## Acceptance

- [x] Phase 2 liveness proof, unit tests, safepoint audit, and receipt completion
- [x] Phase 3 fixture, staged, and all-function production root-clear rollout
- [x] Phase 4 wall/RSS dual gate after measurement-led optimization
- [x] Phase 5 live-IR owner release if Phase 4 remains insufficient
- [x] Phase 6 stress, sanitizer, full correctness, and strict 3 consecutive PASS
- [x] Phase 7 manager/CI enforcement with high-RSS override forbidden in CI
- [x] Phase 8 docs/state/false-done sync and promotion
- [x] Every item in the plan's Final Experimental Promotion Checklist is complete
- [x] `NATIVE_CPP_EXPERIMENTAL_PROMOTION: COMPLETE` receipt is saved

## Evidence

- Promotion receipt: `docs/data/native-cpp-executor-promotion-receipt.json`
- Strict 3×: warm 248760 / 242030 / 238590 ms; peak RSS ≤ 2516852736; `high_rss_override=false`
- CI: `native-executor-gates` job + `--allow-high-rss` reject under CI/GITHUB_ACTIONS
- State: `[[executor_lanes]] id = "native-cpp-selfhost"` experimental; target remains scaffold/partial/`run_supported=false`

## Non-goals (still true)

- Public `arukellt run --target native-cpp`
- Stable public C ABI
- Distributed native executable product
- #831 wasm32-gc canonical fixpoint repair
