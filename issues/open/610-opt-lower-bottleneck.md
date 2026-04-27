---
Status: open
Created: 2026-04-22
Updated: 2026-04-22
ID: 610
Track: main
Orchestration class: implementation-ready
Depends on: none
---
# Optimization Uplift: Lowering Bottleneck Reduction
**Parent**: #591
**Depends on**: 609
**Track**: compiler / selfhost
**Orchestration class**: blocked-by-upstream

---

## Summary

Child issue for #591 Phase 2 — Compile-Speed Uplift (Lowering First).

Current perf data shows `lower` is ~49% of measured compile-phase cost. This issue
attacks the measured bottleneck before investing in other compiler ideas. It must
produce a measurable improvement verified by the baselines established in #609.

---

## Scope

**In scope:**
- Instrument lowering with deterministic sub-phase timing (stable enough for baselines)
  Examples of acceptable cuts: expression lowering, CFG/block construction,
  local allocation/remap, function index/call planning, monomorph/specialization hookup
- Eliminate duplicated or allocation-heavy work in lowering:
  - remove duplicated AST/CoreHIR traversals
  - reduce clone-heavy intermediate materialization
  - avoid re-deriving stable symbol/literal/signature facts multiple times per session
  - prefer deterministic in-session memoization before any persistent cross-build cache
- Exit condition: average `lower` phase improves ≥15%, compile median improves ≥8%
  vs Phase 0 baseline

**Out of scope:**
- Parallel typecheck — explicitly rejected (#098)
- Arena allocator rewrite — explicitly rejected (#097)
- Incremental parse beyond current design (#099) — only escalate if new phase data proves
  lower is no longer the bottleneck after this issue
- T3 pass changes (that is #611)

---

## Primary paths

- Current selfhost lowering path: `src/compiler/corehir.ark`, `src/compiler/mir.ark`
- Adjacent instrumentation: `scripts/perf/` if new timers needed
- `tests/baselines/perf/baselines.json` (update after implementation)

## Allowed adjacent paths

- `src/compiler/typechecker.ark` (read-only — understand what is being passed to lowering)
- `crates/ark-mir/` (read-only — understand current MIR pass sequence)

---

## Upstream / Depends on

609 (measurement baselines must be established first)

## Blocks

- #611 (T3 unlock may benefit from knowing lowering bottleneck is reduced)
- #612 (binary size may interplay with lowering output)

---

## Acceptance

1. Sub-phase timing for lowering is instrumented and visible in perf reports
2. Average `lower` phase time improves ≥15% vs Phase 0 (#609) baseline
3. Benchmark-suite compile median improves ≥8% vs Phase 0 baseline
4. No determinism, selfhost, or correctness regressions

---

## Required verification

```bash
python scripts/manager.py perf benchmarks --no-quick
python scripts/manager.py perf gate
python scripts/manager.py verify --full
python scripts/manager.py selfhost parity
```

---

## STOP_IF

- Do not start parallel typecheck or arena allocator rewrite
- Do not escalate to incremental work unless new data proves lower is no longer the bottleneck
- Do not change MIR pass semantics or T3 reachability rules

---

## Close gate

Close when: `lower` phase improves ≥15%, compile median improves ≥8%, no regressions,
baselines.json is updated, and sub-phase timing is visible.