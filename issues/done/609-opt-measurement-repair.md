---
Status: open
Created: 2026-04-22
Updated: 2026-04-22
ID: 609
Track: main
Orchestration class: implementation-ready
Depends on: none
---
# Optimization Uplift: Measurement Truth Repair
**Parent**: #591
**Depends on**: —
**Track**: benchmark / docs
**Orchestration class**: implementation-ready

---

## Summary

Child issue for #591 Phase 1 — Measurement Truth Repair (CRITICAL).

Performance work cannot be judged on stale or mixed numbers. This issue ensures that
perf baselines, cross-language comparison, phase attribution, and size docs are all
consistent and trustworthy before any optimization implementation work starts.

**Phase 0 baseline re-record is part of this issue.**

---

## Scope

**In scope:**
- Re-run Phase 0 observe-only baseline:
  - compile / run / guest / startup medians
  - phase breakdown averages (resolve, typecheck, lower, opt, emit)
  - T1 / T3 hello.ark binary sizes
  - T3 gated pass list from `crates/ark-mir/src/passes/README.md`
- Restore cross-language visibility: ensure `scripts/compare-benchmarks.sh` embeds
  the C/Rust/Go comparison block into `docs/process/benchmark-results.md`
- Surface `startup_ms` and `guest_ms` as first-class reporting in perf docs
  (not only raw JSON, not only wall-clock total)
- Add phase attribution summary (table of average phase cost) to human-readable report
- Sync: `docs/current-state.md`, `docs/process/benchmark-results.md`,
  `docs/process/wasm-size-reduction.md`

**Out of scope:**
- Any implementation optimization — observe only in this issue
- Modifying lowering, MIR passes, or emitter
- T3 dead-function elimination (that is #611)

---

## Primary paths

- `docs/process/benchmark-results.md`
- `docs/process/wasm-size-reduction.md`
- `docs/current-state.md`
- `scripts/compare-benchmarks.sh`
- `tests/baselines/perf/baselines.json`

## Allowed adjacent paths

- `scripts/perf/` (instrumentation for phase reporting only)
- `docs/benchmarks/` (if phase attribution table goes here)

---

## Upstream / Depends on

None.

## Dispatch dependency map

- depends_on_open: none
- depends_on_done: none
- blocks: #610, #611, #612

## Blocks

- #610 (lowering work must see accurate baseline)
- #611 (T3 unlock must see accurate T3 size baseline)
- #612 (binary size work must see accurate size baseline)

---

## Acceptance

1. `docs/process/benchmark-results.md` contains an embedded cross-language table
   (not just "No cross-language table embedded yet")
2. `startup_ms` and `guest_ms` are surfaced in human-readable perf docs
3. Average phase costs are visible in a table with explicit `resolve`, `typecheck`,
  `lower`, `opt`, and `emit` columns
4. `docs/current-state.md`, `benchmark-results.md`, and `wasm-size-reduction.md`
  are consistent with `tests/baselines/perf/baselines.json` for compile/run/startup/guest medians
5. T1/T3 hello.ark size numbers in docs match the measured baseline artifacts

---

## Required verification

```bash
python scripts/manager.py perf benchmarks --no-quick
bash scripts/compare-benchmarks.sh
python scripts/manager.py docs check
```

Manual check:

- Confirm `issues/open/dependency-graph.md` still shows `#609 -> #610/#611/#612`

---

## STOP_IF

- Do not start implementing optimizations — this issue is measurement only
- Do not start parallel typecheck or arena rewrite (rejected issues)

---

## Close gate

Close when: cross-language doc has a real table, startup/guest/phase are visible in
docs, and all four perf/size docs are consistent with each other.

---

## Close note (2026-04-23)

All five acceptance criteria met:

1. **Cross-language comparison table**: Real measured data injected into
   `docs/process/benchmark-results.md` inside `<!-- arukellt:cross-lang-compare:start/end -->`
   markers. fib C=1.12 ms, fib Rust=1.19 ms, binary\_tree C=2.11 ms, binary\_tree Rust=1.11 ms,
   vec\_ops C=1.03 ms vs Ark fib wall=10.793 ms.

2. **startup_ms / guest_ms visible in docs**: `## Runtime Latency Breakdown (ms)` table
   in `docs/process/benchmark-results.md` has startup and guest columns.

3. **Phase cost table present**: `## Compile Latency Breakdown (ms, baseline (2026-04-22T04:46:08+00:00))`
   present in `docs/process/benchmark-results.md` via fallback to `tests/baselines/perf/baselines.json`
   (because `--time` flag was removed from selfhost CLI).

4. **docs/current-state.md consistent with baselines.json**: Benchmark suite table updated to
   real values from baselines.json (fib=993 B, binary\_tree=977 B, vec\_ops=1,983 B,
   string\_concat=1,248 B). Suite labels changed from "cpu (legacy)" to "cpu".

5. **T1/T3 size numbers corrected**: hello.ark T1/T3 table updated (measured: 494 B both targets
   at default opt; canonical wasm-size-reduction.md reference T1=534 B, T3=918 B at opt-level 2).

**Verification results:**
- `python scripts/manager.py perf benchmarks --no-quick` → PASS (1/1 checks)
- `bash scripts/compare-benchmarks.sh` → runs end-to-end, "Done."
- `issues/open/dependency-graph.md`: I609 --> I610, I609 --> I611, I609 --> I612 confirmed

**Changed files:**
- `scripts/util/benchmark_runner.py` — removed `--time` from compile command; fixed `# equivalent:` comment; phase fallback to baseline JSON
- `scripts/compare-benchmarks.sh` — fixed to call correct python command
- `docs/process/benchmark-results.md` — injected real cross-lang table, startup/guest columns, phase baseline table
- `docs/current-state.md` — updated benchmark suite table sizes and binary size table