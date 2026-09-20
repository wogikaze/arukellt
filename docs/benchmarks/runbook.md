# Benchmark Runbook

Operational commands for measuring and comparing Arukellt benchmarks.
**Normative rules** (schema, thresholds, naming) live in
[`governance.md`](governance.md) — do not restate them here.

## Quick commands

```bash
python3 scripts/util/benchmark_runner.py --mode full                  # full local benchmark
python3 scripts/util/benchmark_runner.py --mode quick                 # single-sample smoke
python3 scripts/util/benchmark_runner.py --mode compare               # measure + compare baseline
python3 scripts/util/benchmark_runner.py --mode update-baseline       # replace baseline
python3 scripts/util/benchmark_runner.py --mode ci                    # regression gate
bash scripts/compare-benchmarks.sh                                    # cross-language table
```

When the default selfhost wrapper is selected, the runner uses the available
current s2/s3 artifact under `.build/selfhost` unless
`ARUKELLT_SELFHOST_WASM` is already set.  Compile inputs and temporary outputs
are repository-relative because the selfhost Wasm process sees the repository
through its preopen; this also keeps the benchmark artifact path usable by
Wasmtime.

`ci` is fail-closed: a compile error, runtime error, correctness failure, or
missing baseline metric is a gate failure.  A report containing only skipped
comparisons is not performance evidence, even when no numeric threshold was
exceeded.

## Output

Generated Markdown lands in [`../history/benchmarks/benchmark-results.md`](../history/benchmarks/benchmark-results.md).
If that file’s Current Run is all-skipped or uses a deprecated target id, treat it as
**INVALID** — do not cite it as current performance evidence (#765).

## See also

- [`governance.md`](governance.md)
- [`../process/benchmark-plan.md`](../process/benchmark-plan.md) (process-side plan; prefer governance for schema)
