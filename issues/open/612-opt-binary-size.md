# Optimization Uplift: Binary Size Squeeze

**Status**: open
**Created**: 2026-04-22
**Updated**: 2026-04-22
**ID**: 612
**Parent**: #591
**Depends on**: 609, 611
**Track**: compiler / runtime-perf
**Orchestration class**: blocked-by-upstream

---

## Summary

Child issue for #591 Phase 4 — Binary Size / Wasm Layout.

After measurement repair (#609) and T3 dead-function elimination (#611), this issue
addresses the remaining T3 size overhead opportunities documented in
`docs/process/wasm-size-reduction.md`:

- Dead type elimination (~70 B)
- Merge identical code functions (~59 B)
- Remove unused element section (~40 B)

These are late-stage output compaction mechanisms applied after reachability is known.

---

## Scope

**In scope:**
- Dead type elimination pass (section compaction after reachability)
- Merge identical / duplicate code functions
- Remove unused element section entries
- Verify improvements against Phase 0 (#609) T1/T3 hello.ark baselines
- Update `docs/process/wasm-size-reduction.md` with final measurements

**Out of scope:**
- New optimization algorithms
- LTO-style whole-program decisions
- Anything that regresses determinism or correctness
- Binary size on T1 (only T3 targets unless T1 has documented overhead)

---

## Primary paths

- `crates/ark-mir/src/passes/` (dead type, duplicate code passes)
- `crates/ark-wasm/` (element section, output compaction)
- `docs/process/wasm-size-reduction.md`
- `tests/fixtures/` (size regression fixtures)

## Allowed adjacent paths

- `tests/baselines/perf/baselines.json` (update after improvements)

---

## Upstream / Depends on

609 (size baselines), 611 (dead-function elimination must be safe before dead-type elimination)

## Blocks

None (this closes the #591 umbrella with #609+#610)

---

## Acceptance

1. T3 hello.ark binary is smaller by ≥100 B compared to Phase 0 baseline
2. At least two of the three documented opportunities are addressed
3. `docs/process/wasm-size-reduction.md` is updated with current measurements
4. No determinism or correctness regressions

---

## Required verification

```bash
python scripts/manager.py verify --full
python scripts/manager.py perf benchmarks --no-quick
python scripts/manager.py perf gate
```

Measure T1/T3 hello.ark sizes manually:
```bash
./target/release/arukellt compile tests/fixtures/hello/hello.ark \
  --target wasm32-wasi-p1 --opt-level 2 -o /tmp/hello-t1.wasm
./target/release/arukellt compile tests/fixtures/hello/hello.ark \
  --target wasm32-wasi-p2 --opt-level 2 -o /tmp/hello-t3.wasm
wc -c /tmp/hello-t1.wasm /tmp/hello-t3.wasm
```

---

## STOP_IF

- Do not regress determinism
- Do not implement LTO-style cross-module decisions
- Do not change public ABI or export surface

---

## Close gate

Close when: T3 binary is ≥100 B smaller than Phase 0 baseline, `wasm-size-reduction.md`
is updated with current numbers, and no correctness regressions.
