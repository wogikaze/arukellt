# Benchmark Plan — Governance, Run Modes, and Baseline Rules

This document is the authoritative process reference for the Arukellt benchmark
suite.  It defines run modes, baseline update rules, and the governance contract
that all benchmark-related issues must follow.

**Companion documents**:
- [`benchmarks/README.md`](../../benchmarks/README.md) — how to run, result schema, fixture list
- [`docs/benchmarks/governance.md`](../benchmarks/governance.md) — naming conventions, full schema reference, comparison methodology
- [`benchmarks/schema.json`](../../benchmarks/schema.json) — machine-readable JSON Schema (`arukellt-bench-v1`)
- [`benchmarks/workload-taxonomy.md`](../../benchmarks/workload-taxonomy.md) — canonical tag vocabulary

---

## 1. Run Modes

All five modes are invoked through `mise` tasks backed by
`scripts/util/benchmark_runner.py`.  The table below is the normative
definition; downstream tooling **must** honour these semantics.

| Mode               | Command                      | Compile iters | Runtime iters | Warmups | Wall-clock target | Purpose |
|--------------------|------------------------------|:---:|:---:|:---:|:-----------:|---------|
| `quick`            | `mise bench:quick`           | 1   | 1   | 0   | < 30 s      | Single-sample smoke test.  Fast feedback during local edits.  Numbers are **not** suitable for regression decisions. |
| `full`             | `mise bench`                 | 5   | 5   | 1   | 2–5 min     | Complete suite with statistical sampling.  Default for local performance work. |
| `compare`          | `mise bench:compare`         | 5   | 5   | 1   | 2–5 min     | Measure then diff against the committed baseline (`tests/baselines/perf/baselines.json`).  Prints per-benchmark delta table. Does **not** fail on regression. |
| `ci`               | `mise bench:ci`              | 5   | 5   | 1   | 2–5 min     | Like `compare`, but exits non-zero if any metric exceeds its regression threshold.  Used as the PR gate. |
| `update-baseline`  | `mise bench:update-baseline` | 5   | 5   | 1   | 2–5 min     | Overwrites `tests/baselines/perf/baselines.json` with current measurements.  Must be committed intentionally (see §3). |

### Mode invariants

- `quick` **must not** write to `baselines.json`.
- `ci` **must** exit non-zero on threshold regression (see §2).
- `update-baseline` **must** write `baselines.json` and **must not** compare against the old baseline.
- All modes write `tests/baselines/perf/current.json` and overwrite
  `docs/process/benchmark-results.md`.

### CI gate wiring

The perf gate is invoked via `python scripts/manager.py perf gate`, which calls
`benchmark_runner.py --mode ci`.  The gate runs on every PR that touches:

- `src/` (compiler source)
- `std/` (stdlib)
- `benchmarks/` (fixtures or runner)
- `scripts/util/benchmark_runner.py`

The gate does **not** run on documentation-only PRs.

---

## 2. Regression Thresholds

These thresholds govern the `ci` mode gate:

| Metric                           | Max allowed regression |
|----------------------------------|:----------------------:|
| Compile time (`compile_time_ms`) | +20 %                  |
| Runtime (`runtime_ms`)           | +10 %                  |
| Binary size (`wasm_size_bytes`)  | +15 %                  |

Threshold changes **must** go through the same approval process as baseline
updates (see §3).

---

## 3. Baseline Update Rules

### When to update

A baseline update is legitimate in exactly these cases:

| Scenario | Allowed? | Notes |
|----------|:--------:|-------|
| Intentional performance improvement merged to `main` | ✅ | Update in the same PR or a follow-up. |
| Compiler or runtime refactor with neutral/better perf | ✅ | Update baseline in the same PR. |
| New benchmark fixture added | ✅ | Baseline must include the new fixture before CI can gate it. |
| Acceptable temporary regression (documented trade-off) | ✅ | Requires reviewer sign-off and an issue linking the trade-off. |
| Masking an unintentional regression | ❌ | File an issue instead; do not update the baseline to hide the regression. |
| Noise-driven update (run-to-run variance without code change) | ❌ | Investigate variance first; see `docs/benchmarks/variance-control.md`. |

### Who approves

- **Routine updates** (improvement, refactor, new fixture): one reviewer with
  knowledge of the changed code path is sufficient.
- **Regression-accepting updates**: two reviewers required; the PR description
  must link the tracking issue that records the trade-off rationale.
- **Threshold changes**: the same two-reviewer bar as regression-accepting
  updates, plus a note in this document's changelog section.

### How to record a baseline update

1. Run `mise bench:update-baseline` on the target commit (release build).
2. Verify `tests/baselines/perf/baselines.json` changed as expected.
3. Run `mise bench:compare` to confirm the delta is as intended.
4. Commit `baselines.json` together with the triggering code change (or as an
   immediate follow-up if the PR is already merged).
5. Add an entry to the PR description or commit message stating _why_ the
   baseline changed (e.g. "compile_time_ms improved ~15 % due to MIR
   inlining; baseline updated").

### Baseline file location

```
tests/baselines/perf/baselines.json   ← committed, source of truth
tests/baselines/perf/current.json     ← generated each run, not committed
```

`baselines.json` is the only file that `mise bench:ci` gates against.
`current.json` is ephemeral output and should be `.gitignore`d.

---

## 4. Cross-Language Comparison (`compare` mode details)

The `compare` mode is primarily used for **cross-commit** comparison within the
Arukellt compiler.  It is **not** a cross-language (Rust/C/Go) comparison tool.
Use it to answer: "Did this PR make things faster or slower?"

Workflow:

```bash
# 1. On the base commit, record the baseline
git checkout <base-sha>
cargo build --release
mise bench:update-baseline

# 2. On the head commit, compare
git checkout <head-sha>
cargo build --release
mise bench:compare
```

The runner prints a per-benchmark delta table showing `Δ compile_time_ms`,
`Δ runtime_ms`, `Δ wasm_size_bytes`, and a pass/fail verdict per metric.

---

## 5. Result Schema Summary

Full schema: [`benchmarks/schema.json`](../../benchmarks/schema.json).  
Conceptual field aliases (from the README field reference):

| Conceptual name        | JSON path                                              | Unit    |
|------------------------|--------------------------------------------------------|---------|
| `compile_time_ms`      | `benchmarks[].compile.median_ms`                       | ms      |
| `runtime_ms`           | `benchmarks[].runtime.median_ms`                       | ms      |
| `wasm_size_bytes`      | `benchmarks[].compile.binary_bytes`                    | bytes   |
| `peak_memory_bytes`    | `benchmarks[].compile.max_rss_kb * 1024` (or runtime)  | bytes   |
| `metadata.mode`        | `mode`                                                 | enum    |
| `metadata.generated_at`| `generated_at`                                        | ISO 8601|
| `metadata.target`      | `target`                                               | string  |
| `metadata.environment` | `environment`                                          | object  |

---

## 6. Adding a New Benchmark

1. Create `benchmarks/bench_<suite>_<name>.ark` (naming convention §1 of
   [`governance.md`](../benchmarks/governance.md)).
2. Create `benchmarks/bench_<suite>_<name>.expected` with correct stdout.
3. Register in `scripts/util/benchmark_runner.py` (`BENCHMARKS` tuple).
4. Run `mise bench` to verify compilation, execution, and correctness.
5. Run `mise bench:update-baseline` to include the new fixture.
6. Commit fixture, expected file, and updated `baselines.json` together.

---

## 7. Changelog

| Date       | Change |
|------------|--------|
| 2026-04-03 | Initial creation — governance schema, run-mode definitions, baseline rules (#149) |
