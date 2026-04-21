# 542 — Benchmark: error chain (Result / error propagation)

**Status**: open
**Created**: 2026-04-21
**ID**: 542
**Depends on**: 515
**Track**: benchmark
**Orchestration class**: implementation-ready
**Orchestration upstream**: —
**Blocks v{N}**: none
**Source**: docs/benchmarks/feature-matrix.md coverage gap

## Summary

docs/benchmarks/feature-matrix.md identifies **error paths / Result handling** as a medium-severity coverage gap with zero dedicated benchmarks. This issue adds `bench_error_chain.ark` to measure performance of Result type, error propagation, and match on error paths.

## Primary paths

- `benchmarks/bench_compute_error_chain.ark`
- `benchmarks/bench_compute_error_chain.expected`
- `scripts/util/benchmark_runner.py` (register benchmark)

## Non-goals

- Full error handling semantics beyond Result propagation
- Error handling optimization (measurement only)

## Acceptance

- [ ] `benchmarks/bench_compute_error_chain.ark` created with Result and error propagation
- [ ] `benchmarks/bench_compute_error_chain.expected` created with correct stdout
- [ ] Benchmark registered in `scripts/util/benchmark_runner.py` `BENCHMARKS` tuple
- [ ] Tags: `cpu-bound`, `error-heavy`, `match-heavy`, `iteration`
- [ ] `mise bench` passes for the new benchmark
- [ ] `mise bench:update-baseline` includes the new benchmark
- [ ] docs/benchmarks/feature-matrix.md updated to mark error paths as covered

## Required verification

- `python scripts/manager.py verify quick` passes
- `mise bench` runs without errors

## Close gate

Benchmark fixture exists, is registered in the runner, and baseline is updated.
