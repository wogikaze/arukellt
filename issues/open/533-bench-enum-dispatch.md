# 533 — Benchmark: enum dispatch (pattern matching)

**Status**: open
**Created**: 2026-04-21
**ID**: 533
**Depends on**: none
**Track**: benchmark
**Orchestration class**: implementation-ready
**Orchestration upstream**: —
**Blocks v{N}**: none
**Source**: docs/benchmarks/feature-matrix.md coverage gap

## Summary

docs/benchmarks/feature-matrix.md identifies **enum / pattern matching** as a high-severity coverage gap with only legacy `vec-ops.ark` (Option match) exercising it. This issue adds `bench_enum_dispatch.ark` to measure performance of enum variant dispatch and pattern matching.

## Primary paths

- `benchmarks/bench_cpu_enum_dispatch.ark`
- `benchmarks/bench_cpu_enum_dispatch.expected`
- `scripts/util/benchmark_runner.py` (register benchmark)

## Non-goals

- Full enum semantics beyond variant dispatch
- Pattern matching optimization (measurement only)

## Acceptance

- [ ] `benchmarks/bench_cpu_enum_dispatch.ark` created with enum variants and match dispatch
- [ ] `benchmarks/bench_cpu_enum_dispatch.expected` created with correct stdout
- [ ] Benchmark registered in `scripts/util/benchmark_runner.py` `BENCHMARKS` tuple
- [ ] Tags: `cpu-bound`, `match-heavy`, `allocation-heavy`, `iteration`
- [ ] `mise bench` passes for the new benchmark
- [ ] `mise bench:update-baseline` includes the new benchmark
- [ ] docs/benchmarks/feature-matrix.md updated to mark enum/pattern matching as covered

## Required verification

- `python scripts/manager.py verify quick` passes
- `mise bench` runs without errors

## Close gate

Benchmark fixture exists, is registered in the runner, and baseline is updated.
