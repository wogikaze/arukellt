# 532 — Benchmark: closure map (higher-order functions)

**Status**: open
**Created**: 2026-04-21
**ID**: 532
**Depends on**: 499
**Track**: benchmark
**Orchestration class**: implementation-ready
**Orchestration upstream**: —
**Blocks v{N}**: none
**Source**: docs/benchmarks/feature-matrix.md coverage gap

## Summary

docs/benchmarks/feature-matrix.md identifies **closures / higher-order functions** as a high-severity coverage gap with zero dedicated benchmarks. This issue adds `bench_closure_map.ark` to measure performance of closure-based higher-order functions over Vec.

## Primary paths

- `benchmarks/bench_cpu_closure_map.ark`
- `benchmarks/bench_cpu_closure_map.expected`
- `scripts/util/benchmark_runner.py` (register benchmark)

## Non-goals

- Full closure semantics beyond lexical capture
- Closure performance optimization (measurement only)

## Acceptance

- [ ] `benchmarks/bench_cpu_closure_map.ark` created with closure-based map over Vec
- [ ] `benchmarks/bench_cpu_closure_map.expected` created with correct stdout
- [ ] Benchmark registered in `scripts/util/benchmark_runner.py` `BENCHMARKS` tuple
- [ ] Tags: `cpu-bound`, `closure-heavy`, `allocation-heavy`, `iteration`
- [ ] `mise bench` passes for the new benchmark
- [ ] `mise bench:update-baseline` includes the new benchmark
- [ ] docs/benchmarks/feature-matrix.md updated to mark closures as covered

## Required verification

- `python scripts/manager.py verify quick` passes
- `mise bench` runs without errors

## Close gate

Benchmark fixture exists, is registered in the runner, and baseline is updated.
