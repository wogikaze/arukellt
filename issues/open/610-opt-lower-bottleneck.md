---
Status: open
Created: 2026-04-22
Updated: 2026-05-16
ID: 610
Track: compiler / selfhost
Orchestration class: implementation-ready
Depends on: 609 (DONE)
Dependency status: 609 is complete. Issue is TECHNICALLY UNBLOCKED but needs path updates before implementation can proceed.
Assessment date: 2026-05-16
---

# Optimization Uplift: Lowering Bottleneck Reduction

---

## Assessment (2026-05-16)

### Blocking check: 609 DONE -- NOT truly blocked

Issue #609 (Measurement Truth Repair) was completed on 2026-04-23 with all five
acceptance criteria met. Phase 0 baselines exist at `tests/baselines/perf/baselines.json`
(generated 2026-05-14, 14 benchmarks, 5 compile iterations each). Cross-language
comparison, startup/guest ms, and phase-cost tables are all in place in the docs.

**However, three issues prevent straightforward implementation:**

1. **Sub-phase timing is not currently captured.** The benchmark runner
   (`scripts/util/benchmark_runner.py`) does not pass `--time` to the compile command
   (line 494), so the current baselines have no phase-level timing data. The claim that
   `lower` is ~49% of compile-phase cost comes from a previous baseline. To measure
   improvement against the current code, either `--time` support must be restored in the
   benchmark runner or instrumented via a different mechanism.

2. **Stale file paths.** The issue references `src/compiler/corehir.ark` and
   `src/compiler/mir.ark`. The current tree has `src/compiler/hir.ark` (CoreHIR) and
   `src/compiler/mir_ir.ark` / `src/compiler/mir_lower.ark` (MIR lowering).
   The `crates/ark-mir/` crate (listed as read-only adjacent path) was removed in #561;
   all MIR handling is now in `src/compiler/`.

3. **Current compile medians are ~25-35ms** (fib 29ms, binary_tree 28ms, vec_ops 29ms,
   log_processor 35ms). An 8% compile-median improvement against these baselines means
   ~2-3ms -- a tight target for a phase that already runs quickly. Several benchmarks
   (enum_dispatch, error_chain, closure_map, http_parser, config_loader, data_pipeline)
   fail to compile, which means the effective benchmark suite for measuring improvement
   is ~8 benchmarks, not 14.

### Recommendation

This issue is **not truly blocked by 609**. The orchestration class should be reconsidered
(paths need updating and sub-phase timing needs fixing before meaningful implementation
can begin, but the upstream dependency is satisfied). The issue remains actionable once
paths are corrected and phase timing is observable in the baseline runner.

---

## Summary

Child issue for #591 Phase 2 -- Compile-Speed Uplift (Lowering First).

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

- Current selfhost lowering path: `src/compiler/hir.ark` (CoreHIR), `src/compiler/mir_lower.ark` (MIR lowering)
- Adjacent instrumentation: `scripts/perf/` if new timers needed
- `tests/baselines/perf/baselines.json` (update after implementation)

## Allowed adjacent paths

- `src/compiler/typechecker.ark` (read-only -- understand what is being passed to lowering)
- `src/compiler/mir_ir.ark` (read-only -- MIR IR definitions)
- Note: `crates/ark-mir/` was removed in #561; MIR pass sequence is now fully in `src/compiler/`

---

## Upstream / Depends on

609 (measurement baselines must be established first) -- DONE
**Note:** Current benchmark runner does not pass `--time` to compile command (see
`scripts/util/benchmark_runner.py` line 494). Phase-level timing needs restoration
before sub-phase improvement can be measured against the current baselines.

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
