---
Status: done
Created: 2026-04-21
Track: main
Orchestration class: implementation-ready
Depends on: none
---
# 539 — Benchmark: closure map (higher-order functions)
**Closed**: 2026-04-22
**ID**: 539
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

- [x] `benchmarks/bench_cpu_closure_map.ark` created with closure-based map over Vec
- [x] `benchmarks/bench_cpu_closure_map.expected` created with correct stdout
- [x] Benchmark registered in `scripts/util/benchmark_runner.py` `BENCHMARKS` tuple
- [x] Tags: `cpu-bound`, `closure-heavy`, `allocation-heavy`, `iteration`
- [x] `mise bench` passes for the new benchmark
- [x] `mise bench:update-baseline` includes the new benchmark
- [x] docs/benchmarks/feature-matrix.md updated to mark closures as covered

## Required verification

- `python scripts/manager.py verify quick` passes
- `mise bench` runs without errors

## Close gate

Benchmark fixture exists, is registered in the runner, and baseline is updated.

## Close note

Implemented in commit `698c8e8a` on branch `feat/539-bench-closure-map`. Benchmark compiles and produces
deterministic output `6017000` (map: `x * 3 + 7` over 2000 elements, reduce: sum). Closures
with lexical captures work as expected via the production Rust compiler. Feature matrix updated
to mark closures/higher-order functions as covered.