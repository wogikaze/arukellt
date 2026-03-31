# Benchmarks

Benchmark suite for Arukellt language performance measurement.

> **Governance**: See [`docs/benchmarks/governance.md`](../docs/benchmarks/governance.md)
> for naming conventions, result schema, comparison methodology, and how to
> add new benchmarks.
>
> **Result schema**: See [`benchmarks/schema.json`](schema.json) for the
> machine-readable JSON Schema (`arukellt-bench-v1`).

## Quick Start

```bash
# Default local workflow: build release compiler + compile/runtime/size/memory metrics
mise bench

# Quick smoke benchmark: single-sample run
mise bench:quick

# Compare current results against baseline
mise bench:compare

# Update baseline from current measurements
mise bench:update-baseline

# CI regression gate
mise bench:ci
```

`bash benchmarks/run_benchmarks.sh` remains as a thin wrapper around the same Python runner.

## Standard Modes

| Mode | Command | Compile iters | Runtime iters | Warmups | Purpose |
|------|---------|:---:|:---:|:---:|---------|
| `quick` | `mise bench:quick` | 1 | 1 | 0 | Single-sample smoke run for local edits |
| `full` | `mise bench` | 5 | 5 | 1 | Default local benchmark with statistical sampling |
| `compare` | `mise bench:compare` | 5 | 5 | 1 | Measure and compare against stored baseline |
| `ci` | `mise bench:ci` | 5 | 5 | 1 | Compare + fail on threshold regression (CI gate) |
| `update-baseline` | `mise bench:update-baseline` | 5 | 5 | 1 | Replace baseline with current measurements |

## Result Schema

Benchmark output conforms to `schema_version = "arukellt-bench-v1"`.
The authoritative JSON Schema is [`schema.json`](schema.json).

Each run records:

- **Run metadata**: mode, target, environment, tool availability
- **Compile metrics**: `median_ms`, `binary_bytes`, `max_rss_kb`
- **Runtime metrics**: `median_ms`, `max_rss_kb`, correctness status
- **Taxonomy tags**: workload classification for grouped analysis

Current JSON is written to `tests/baselines/perf/current.json` and the
human-readable report to `docs/process/benchmark-results.md`.

## Benchmarks

| Benchmark | File | Description | Primary tags |
|-----------|------|-------------|--------------|
| fib | `fib.ark` | Iterative Fibonacci(35) | `cpu-bound`, `loop`, `scalar` |
| binary_tree | `binary_tree.ark` | Recursive node counting (depth 20) | `recursion-heavy`, `call-heavy` |
| vec_ops | `vec_ops.ark` | Vec push/sum/contains (1k elements) | `allocation-heavy`, `container`, `iteration` |
| string_concat | `string_concat.ark` | String concat in loop (100 iterations) | `string-heavy`, `allocation-heavy`, `gc-pressure` |
| parse_tree_distance | `bench_parse_tree_distance.ark` | Packed-tree distance validator on a 1200-node star matrix | `parse`, `allocation-heavy`, `container`, `iteration` |

A complex parser workload lives at [`docs/sample/parser.ark`](../docs/sample/parser.ark)
and serves as a real-world stress test for the compiler and runtime.

### Legacy Fixtures

| Fixture | File | Description |
|---------|------|-------------|

| string_ops | `string_ops.ark` | Basic string concat |
| struct_create | `struct_create.ark` | Struct field creation |

## Tooling Notes

- **`scripts/benchmark_runner.py`** — canonical runner used by `mise bench`, compare, baseline update, and CI gate.
- **`benchmarks/run_benchmarks.sh`** — wrapper for the canonical runner.
- **`scripts/perf-gate.sh`** — CI-oriented wrapper (`--mode ci` / `--mode update-baseline`).
- **`scripts/compare-benchmarks.sh`** — baseline comparison wrapper.
- **`parity-check.sh`** — Verify T1 vs T3 produce identical output for all `.ark` fixtures.
- **`size-compare.sh`** — Compare Wasm binary sizes between T1 and T3.

## Optional Tools and Skip Policy

- `wasmtime`: required for runtime timing and output verification. If missing, runtime metrics are marked skipped.
- `/usr/bin/time`: used to capture max RSS for compile/run memory telemetry. If missing, memory fields are `null`.
- `hyperfine`: optional. Availability is recorded, but the canonical runner currently uses an internal timer so the workflow still works without it.

## Threshold Policy

`mise bench:ci` and `scripts/perf-gate.sh` enforce these baseline regressions:

| Metric       | Max allowed regression |
|--------------|:----------------------:|
| Compile time | +20 %                  |
| Runtime      | +10 %                  |
| Binary size  | +15 %                  |

## Comparing Across Commits

```bash
# 1. On the base commit, capture the baseline
git checkout <base>
mise bench:update-baseline

# 2. On the head commit, compare
git checkout <head>
mise bench:compare
```

The runner loads `tests/baselines/perf/baselines.json`, measures current
performance, and prints a per-benchmark delta table. See the
[governance doc](../docs/benchmarks/governance.md#5-comparison-methodology)
for full details on regression detection.

## Expected Output

Each benchmark has a `.expected` file containing the correct stdout.
When `wasmtime` is available, runtime execution verifies output against these files.
Benchmarks that read checked-in files may declare extra wasmtime args in `scripts/benchmark_runner.py` (for example `run --dir=.`) so runtime verification can access benchmark-local inputs.

## Adding a New Benchmark

1. Create `benchmarks/bench_<category>_<name>.ark` (see [naming conventions](../docs/benchmarks/governance.md#1-naming-conventions)).
2. Create a matching `.expected` file with correct stdout.
3. Register the benchmark in `scripts/benchmark_runner.py` (`BENCHMARKS` tuple).
4. Run `mise bench` to verify, then `mise bench:update-baseline`.
5. Commit the fixture, expected file, and updated baseline.

## Environment Variables

| Variable | Default | Description |
|----------|---------|-------------|
| `ARUKELLT` | `target/release/arukellt` | Legacy variable. Prefer `mise bench`, or pass `--arukellt` to `scripts/benchmark_runner.py`. |
