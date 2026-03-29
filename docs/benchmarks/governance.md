# Benchmark Governance

This document defines the naming conventions, result schema, execution modes,
directory layout, and regression-detection methodology for the Arukellt
benchmark suite.

## 1. Naming Conventions

### Fixture files

```
bench_<category>_<name>.ark
```

| Segment      | Rule                                         | Examples                        |
|--------------|----------------------------------------------|---------------------------------|
| `category`   | Lowercase workload category (`cpu`, `alloc`, `string`, `recurse`, `parse`) | `cpu`, `alloc`       |
| `name`       | Descriptive snake\_case identifier            | `fib`, `binary_tree`, `vec_ops` |

Legacy fixtures that predate this convention (e.g. `fib.ark`, `vec_ops.ark`)
are grandfathered; new benchmarks **must** follow the naming scheme.

Each fixture **must** have a matching `.expected` file containing the correct
stdout so the runner can verify correctness.

### Workload taxonomy tags

Every benchmark declares a `tags` tuple used for grouped analysis.
Use lowercase hyphenated tokens from the following vocabulary:

| Tag                  | Meaning                                     |
|----------------------|---------------------------------------------|
| `cpu-bound`          | Dominated by scalar arithmetic / branching   |
| `loop`               | Hot loop is the primary construct             |
| `scalar`             | No heap allocation in hot path                |
| `recursion-heavy`    | Deep or wide recursive call tree              |
| `call-heavy`         | Many function calls per unit of work          |
| `allocation-heavy`   | Significant heap allocation pressure          |
| `container`          | Exercises Vec / Map / Set APIs                |
| `iteration`          | Iterates over a collection                    |
| `string-heavy`       | String creation / concatenation workload      |
| `gc-pressure`        | Generates garbage that exercises the allocator |
| `parse`              | Parsing-oriented workload                     |

New tags may be added by updating this table and the runner's tag vocabulary.

## 2. Result Schema

All benchmark output conforms to **`schema_version: "arukellt-bench-v1"`**.
The authoritative JSON Schema is [`benchmarks/schema.json`](../../benchmarks/schema.json).

### Top-level envelope

| Field            | Type     | Description                                     |
|------------------|----------|-------------------------------------------------|
| `schema_version` | string   | Always `"arukellt-bench-v1"`                    |
| `generated_at`   | string   | ISO 8601 UTC timestamp                          |
| `mode`           | string   | Execution mode (`quick`, `full`, `compare`, `ci`, `update-baseline`) |
| `mode_description` | string | Human-readable mode summary                     |
| `target`         | string   | Compilation target (e.g. `wasm32-wasi-p1`)      |
| `thresholds`     | object   | Regression thresholds (% allowed increase)      |
| `compiler`       | object   | `{ "path": "<relative path>" }`                |
| `environment`    | object   | Platform, Python version, machine, kernel        |
| `tooling`        | object   | Availability of `wasmtime`, `hyperfine`, `/usr/bin/time` |
| `benchmarks`     | array    | Array of benchmark result objects                |

### Benchmark result object

| Field         | Type     | Description                                    |
|---------------|----------|------------------------------------------------|
| `name`        | string   | Benchmark identifier (e.g. `"fib"`)            |
| `source`      | string   | Relative path to `.ark` source                 |
| `expected`    | string   | Relative path to `.expected` file              |
| `description` | string   | Human-readable description                     |
| `tags`        | string[] | Workload taxonomy tags                         |
| `metrics`     | string[] | Metric categories collected (`compile`, `runtime`, `size`, `memory`) |
| `compile`     | object   | Compile-phase measurements                     |
| `runtime`     | object   | Runtime-phase measurements                     |

### Compile metrics

| Field          | Type       | Unit   | Description                         |
|----------------|------------|--------|-------------------------------------|
| `status`       | string     | —      | `"ok"` or `"skipped"`              |
| `iterations`   | integer    | —      | Number of samples taken              |
| `samples_ms`   | number[]   | ms     | Raw wall-clock samples               |
| `median_ms`    | number     | ms     | Median compile time                  |
| `max_rss_kb`   | number\|null | KiB  | Peak resident memory (null if `/usr/bin/time` unavailable) |
| `binary_bytes` | integer    | bytes  | Output binary size                   |

### Runtime metrics

| Field          | Type       | Unit   | Description                         |
|----------------|------------|--------|-------------------------------------|
| `status`       | string     | —      | `"ok"` or `"skipped"`              |
| `iterations`   | integer    | —      | Number of timed iterations           |
| `warmups`      | integer    | —      | Warmup iterations (discarded)        |
| `samples_ms`   | number[]   | ms     | Raw wall-clock samples               |
| `median_ms`    | number     | ms     | Median runtime                       |
| `max_rss_kb`   | number\|null | KiB  | Peak resident memory                 |
| `correctness`  | string     | —      | `"pass"` or `"fail"`               |

## 3. Execution Modes

All modes are invoked through `mise` tasks backed by `scripts/benchmark_runner.py`.

| Mode               | Command                    | Compile iters | Runtime iters | Warmups | Purpose                                           |
|--------------------|----------------------------|:---:|:---:|:---:|---------------------------------------------------|
| **quick**          | `mise bench:quick`         | 1   | 1   | 0   | Single-sample smoke test during local development  |
| **full**           | `mise bench`               | 5   | 5   | 1   | Default local benchmark with statistical sampling  |
| **compare**        | `mise bench:compare`       | 5   | 5   | 1   | Measure and compare against stored baseline        |
| **ci**             | `mise bench:ci`            | 5   | 5   | 1   | Compare + fail on threshold regression (CI gate)   |
| **update-baseline**| `mise bench:update-baseline` | 5 | 5   | 1   | Replace baseline with current measurements         |

### Threshold policy (CI gate)

| Metric        | Max allowed regression |
|---------------|:----------------------:|
| Compile time  | +20 %                  |
| Runtime       | +10 %                  |
| Binary size   | +15 %                  |

If any benchmark exceeds its threshold in `ci` mode the runner exits non-zero.

## 4. Directory Structure

```
benchmarks/
├── README.md               # How to run benchmarks
├── schema.json             # JSON Schema for result files
├── fib.ark                 # Benchmark fixture
├── fib.expected            # Expected stdout
├── binary_tree.ark
├── binary_tree.expected
├── vec_ops.ark
├── vec_ops.expected
├── string_concat.ark
├── string_concat.expected
├── run_benchmarks.sh       # Thin wrapper around benchmark_runner.py
├── parity-check.sh         # T1 vs T3 parity verification
├── size-compare.sh         # Wasm binary size comparison
├── results/                # (gitignored) Local run output
└── baselines/              # (gitignored) Local baseline snapshots

tests/baselines/perf/
├── baselines.json          # Committed baseline (update via mise bench:update-baseline)
├── current.json            # Latest run output (overwritten each run)
└── hello-wasm-size.json    # Standalone size check

docs/benchmarks/
└── governance.md           # This document

docs/sample/
└── parser.ark              # Complex benchmark (full parser workload)

scripts/
├── benchmark_runner.py     # Canonical runner (all modes)
├── compare-benchmarks.sh   # Baseline comparison wrapper
└── perf-gate.sh            # CI gate wrapper
```

## 5. Comparison Methodology

### Across commits

1. Check out the **base** commit and run `mise bench:update-baseline`.
2. Check out the **head** commit and run `mise bench:compare`.
3. The runner loads `tests/baselines/perf/baselines.json`, measures current
   performance, and prints a per-benchmark delta table.

### Regression detection

For each benchmark and metric the runner computes:

```
delta_pct = ((current_median - baseline_median) / baseline_median) * 100
```

A regression is flagged when `delta_pct` exceeds the threshold for that metric
(see §3). In `ci` mode the process exits non-zero on any flagged regression.

### Best practices

- Always build with `--release` before benchmarking (the `mise bench*` tasks
  handle this automatically).
- Close CPU-intensive programs during local runs to reduce noise.
- Use `full` mode (5 iterations + 1 warmup) for any numbers you intend to
  share or commit.
- `quick` mode is for fast feedback only; do not use its numbers for
  regression decisions.

## 6. Adding a New Benchmark

1. Create `benchmarks/bench_<category>_<name>.ark` following the naming
   convention.
2. Create `benchmarks/bench_<category>_<name>.expected` with correct stdout.
3. Register the benchmark in `scripts/benchmark_runner.py` by adding a
   `BenchmarkCase` entry to the `BENCHMARKS` tuple.
4. Run `mise bench` to verify it compiles, runs, and produces correct output.
5. Run `mise bench:update-baseline` to include the new benchmark in the
   baseline.
6. Commit the fixture, expected file, and updated baseline together.
