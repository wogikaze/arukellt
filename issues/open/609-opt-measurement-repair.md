# Optimization Uplift: Measurement Truth Repair

**Status**: open
**Created**: 2026-04-22
**Updated**: 2026-04-22
**ID**: 609
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

## Blocks

- #610 (lowering work must see accurate baseline)
- #611 (T3 unlock must see accurate T3 size baseline)
- #612 (binary size work must see accurate size baseline)

---

## Acceptance

1. `docs/process/benchmark-results.md` contains an embedded cross-language table
   (not just "No cross-language table embedded yet")
2. `startup_ms` and `guest_ms` are surfaced in human-readable perf docs
3. Average phase costs are visible in a table or summary section
4. `docs/current-state.md`, `benchmark-results.md`, and `wasm-size-reduction.md`
   are consistent with `tests/baselines/perf/baselines.json`

---

## Required verification

```bash
python scripts/manager.py perf benchmarks --no-quick
bash scripts/compare-benchmarks.sh
python scripts/manager.py docs check
```

---

## STOP_IF

- Do not start implementing optimizations — this issue is measurement only
- Do not start parallel typecheck or arena rewrite (rejected issues)

---

## Close gate

Close when: cross-language doc has a real table, startup/guest/phase are visible in
docs, and all four perf/size docs are consistent with each other.
