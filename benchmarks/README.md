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
The full governance reference is [`docs/benchmarks/governance.md`](../docs/benchmarks/governance.md).

Each run records:

- **Run metadata**: mode, target, environment, tool availability
- **Compile metrics**: `median_ms`, `binary_bytes`, `max_rss_kb`
- **Runtime metrics**: `median_ms`, `max_rss_kb`, correctness status
- **Taxonomy tags**: workload classification for grouped analysis

Current JSON is written to `tests/baselines/perf/current.json` and the
human-readable report to `docs/process/benchmark-results.md`.

### Conceptual field reference

The following table maps conceptual metric names (used in governance docs and
issue tracking) to their JSON Schema paths:

| Conceptual name        | JSON path                     | Unit    | Description                                  |
|------------------------|-------------------------------|---------|----------------------------------------------|
| `compile_time_ms`      | `benchmarks[].compile.median_ms`  | ms  | Median wall-clock compile time               |
| `runtime_ms`           | `benchmarks[].runtime.median_ms`  | ms  | Median wall-clock execution time             |
| `wasm_size_bytes`      | `benchmarks[].compile.binary_bytes` | bytes | Output Wasm binary size                  |
| `peak_memory_bytes`    | `benchmarks[].compile.max_rss_kb * 1024` or `runtime.max_rss_kb * 1024` | bytes | Peak resident set size (compile or runtime phase) |
| `metadata.mode`        | `mode`                        | enum    | Run mode (`quick`, `full`, `compare`, `ci`, `update-baseline`) |
| `metadata.generated_at`| `generated_at`               | ISO 8601| Timestamp of the run                         |
| `metadata.target`      | `target`                      | string  | Compilation target (e.g. `wasm32-wasi-p1`)   |
| `metadata.environment` | `environment`                 | object  | Platform, kernel, Python version, machine    |

Memory fields are `null` when `/usr/bin/time` is unavailable on the host.

### Compile Latency Breakdown

Every benchmark run collects per-phase compile latency using `arukellt compile --time`.
Phase timings are stored under `benchmarks[].compile.phase_ms` as median values (ms)
across all compile iterations.

| Phase       | JSON key                              | Description                          |
|-------------|---------------------------------------|--------------------------------------|
| `lex`       | `compile.phase_ms.lex`               | Lexer (tokenisation)                 |
| `parse`     | `compile.phase_ms.parse`             | Parser (AST construction)            |
| `resolve`   | `compile.phase_ms.resolve`           | Name resolution                      |
| `typecheck` | `compile.phase_ms.typecheck`         | Type-checking                        |
| `lower`     | `compile.phase_ms.lower`             | HIR → MIR lowering                   |
| `opt`       | `compile.phase_ms.opt`               | MIR optimisation passes              |
| `emit`      | `compile.phase_ms.emit`              | Wasm/component emission              |
| `total`     | `compile.phase_ms.total`             | Total as reported by the compiler    |

The `compile.median_ms` (wall-clock) will be slightly higher than `phase_ms.total`
due to process startup and I/O overhead.

Phase breakdown is included automatically in all modes (`quick`, `full`, `compare`,
`ci`, `update-baseline`) — no extra flags are needed.  The markdown report
(`docs/process/benchmark-results.md`) renders a **Compile Latency Breakdown** table
alongside the standard benchmark matrix.

### Fixture naming convention

New benchmarks **must** follow the canonical pattern:

```
benchmarks/bench_<suite>/<name>.<ext>
```

or, using the flat convention currently in use:

```
benchmarks/bench_<suite>_<name>.ark
```

Mapping to the abstract `<suite>/<name>.<ext>` scheme:

| Abstract       | Concrete (flat)                          | Example                                      |
|----------------|------------------------------------------|----------------------------------------------|
| `suite`        | Category prefix (`cpu`, `alloc`, `string`, `recurse`, `parse`) | `cpu` → `bench_cpu_fib.ark`   |
| `name`         | Snake\_case identifier                   | `binary_tree`, `vec_ops`                     |
| `ext`          | `.ark` (source), `.expected` (stdout), `.bench.wasm` (prebuilt) | `fib.ark`, `fib.expected` |

Legacy fixtures that predate the `bench_` prefix (e.g. `fib.ark`, `vec_ops.ark`) are
grandfathered; new fixtures **must** use `bench_<suite>_<name>.ark`.

### Tag / taxonomy guidance

Every benchmark declares a `tags` list.  Use lowercase hyphenated tokens from
the canonical vocabulary in [`benchmarks/workload-taxonomy.md`](workload-taxonomy.md).
Tags enable grouped analysis (e.g. "all `allocation-heavy` benchmarks") and
drive regression triage.  See the full tag reference in that file.

## Benchmarks

| Benchmark | File | Description | Primary tags |
|-----------|------|-------------|--------------|
| fib | `fib.ark` | Iterative Fibonacci(35) | `cpu-bound`, `loop`, `scalar` |
| binary_tree | `binary_tree.ark` | Recursive node counting (depth 20) | `recursion-heavy`, `call-heavy` |
| vec_ops | `vec_ops.ark` | Vec push/sum/contains (1k elements) | `allocation-heavy`, `container`, `iteration` |
| string_concat | `string_concat.ark` | String concat in loop (100 iterations) | `string-heavy`, `allocation-heavy`, `gc-pressure` |
| vec_push_pop | `vec_push_pop.ark` | 100K Vec push then 100K pop | `allocation-heavy`, `container`, `throughput` |
| json_parse | `json_parse.ark` | JSON token scan on ~10KB string | `string-heavy`, `parse`, `allocation-heavy` |
| parse_tree_distance | `bench_parse_tree_distance.ark` | Packed-tree distance validator on a 1200-node star matrix | `parse`, `allocation-heavy`, `container`, `iteration` |

### Deferred benchmarks

The following benchmarks require `f64` floating-point support which is not yet
available in the Ark compiler codegen.  Stubs will be added once float
literals and arithmetic are wired through the emitter.

| Benchmark | File | Reason deferred |
|-----------|------|----------------|
| nbody | `nbody.ark` (pending) | Requires `f64` arithmetic — not yet wired in codegen |
| mandelbrot | `mandelbrot.ark` (pending) | Requires `f64` arithmetic — not yet wired in codegen |

## Reference Implementations

C and Rust reference implementations are provided for cross-language comparison.
Use `--compare-lang c,rust,go` with `scripts/run/run-benchmarks.sh` to compile
and time them automatically.

| Benchmark | C ref | Rust ref | Notes |
|-----------|-------|----------|-------|
| fib | `fib.c` | `fib.rs` | Iterative; identical algorithm to `.ark` |
| binary_tree | `binary_tree.c` | `binary_tree.rs` | Recursive; identical algorithm to `.ark` |

### Building reference implementations manually

```bash
# C
cc -O2 -o /tmp/fib        benchmarks/fib.c
cc -O2 -o /tmp/btree      benchmarks/binary_tree.c

# Rust
rustc -O -o /tmp/fib_rs   benchmarks/fib.rs
rustc -O -o /tmp/btree_rs benchmarks/binary_tree.rs
```

### Cross-language comparison via the runner

```bash
# Compare Ark (Wasm/wasmtime) vs C and Rust native binaries (3-run median)
bash scripts/run/run-benchmarks.sh --compare-lang c,rust

# Include Go reference implementations (if .go files are present)
bash scripts/run/run-benchmarks.sh --compare-lang c,rust,go

# Combine with full-mode for more iterations
bash scripts/run/run-benchmarks.sh --full --compare-lang c,rust
```

The comparison table prints `ark(ms)`, one column per reference language, and a
`ratio(best)` column showing how many times slower Ark is relative to the
fastest native reference.

## Results Placeholder

Run `bash scripts/run/run-benchmarks.sh` to generate current measurements.
The JSON output lands in `benchmarks/results/`.

| Benchmark | ark compile (ms) | ark run (ms) | c run (ms) | rust run (ms) | ark/c ratio | ark/rust ratio |
|-----------|:----------------:|:------------:|:----------:|:-------------:|:-----------:|:--------------:|
| fib | — | — | — | — | — | — |
| binary_tree | — | — | — | — | — | — |
| vec_ops | — | — | N/A | N/A | — | — |
| string_concat | — | — | N/A | N/A | — | — |
| vec_push_pop | — | — | N/A | N/A | — | — |
| json_parse | — | — | N/A | N/A | — | — |

_Populated by running `mise bench` or `bash scripts/run/run-benchmarks.sh --full --compare-lang c,rust`.  
Baseline stored in `tests/baselines/perf/baselines.json`._

### Legacy Fixtures

| Fixture | File | Description |
|---------|------|-------------|
| string_ops | `string_ops.ark` | Basic string concat |
| struct_create | `struct_create.ark` | Struct field creation |

## Tooling Notes

- **`scripts/util/benchmark_runner.py`** — canonical runner used by `mise bench`, compare, baseline update, and CI gate.
- **`benchmarks/run_benchmarks.sh`** — wrapper for the canonical runner.
- **`scripts/check/perf-gate.sh`** — CI-oriented wrapper (`--mode ci` / `--mode update-baseline`).
- **`scripts/compare-benchmarks.sh`** — baseline comparison wrapper.
- **`parity-check.sh`** — Verify T1 vs T3 produce identical output for all `.ark` fixtures.
- **`size-compare.sh`** — Compare Wasm binary sizes between T1 and T3.

## Optional Tools and Skip Policy

- `wasmtime`: required for runtime timing and output verification. If missing, runtime metrics are marked skipped.
- `/usr/bin/time`: used to capture max RSS for compile/run memory telemetry. If missing, memory fields are `null`.
- `hyperfine`: optional. Availability is recorded, but the canonical runner currently uses an internal timer so the workflow still works without it.

## Threshold Policy

`mise bench:ci` and `scripts/check/perf-gate.sh` enforce these baseline regressions:

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
Benchmarks that read checked-in files may declare extra wasmtime args in `scripts/util/benchmark_runner.py` (for example `run --dir=.`) so runtime verification can access benchmark-local inputs.

## Adding a New Benchmark

1. Create `benchmarks/bench_<category>_<name>.ark` (see [naming conventions](../docs/benchmarks/governance.md#1-naming-conventions)).
2. Create a matching `.expected` file with correct stdout.
3. Register the benchmark in `scripts/util/benchmark_runner.py` (`BENCHMARKS` tuple).
4. Run `mise bench` to verify, then `mise bench:update-baseline`.
5. Commit the fixture, expected file, and updated baseline.

## Environment Variables

| Variable | Default | Description |
|----------|---------|-------------|
| `ARUKELLT` | `target/release/arukellt` | Legacy variable. Prefer `mise bench`, or pass `--arukellt` to `scripts/util/benchmark_runner.py`. |
