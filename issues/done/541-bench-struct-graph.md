# 541 — Benchmark: struct graph (nested structs / recursive types)

**Status**: done

**Closed**: 2026-04-22
**Close commit**: 80bd25e4
**Created**: 2026-04-21
**ID**: 541
**Depends on**: none
**Track**: benchmark
**Orchestration class**: implementation-ready
**Orchestration upstream**: —
**Blocks v{N}**: none
**Source**: docs/benchmarks/feature-matrix.md coverage gap

## Summary

docs/benchmarks/feature-matrix.md identifies **struct-heavy allocation** and **nested structs / composite types** as medium-severity coverage gaps with only trivial `struct-create.ark` (legacy) and `parser.ark` (not in bench suite). This issue adds `bench_struct_graph.ark` to measure performance of nested struct allocation and recursive types.

## Primary paths

- `benchmarks/bench_memory_struct_graph.ark`
- `benchmarks/bench_memory_struct_graph.expected`
- `scripts/util/benchmark_runner.py` (register benchmark)

## Non-goals

- Full struct semantics beyond nested allocation
- Struct layout optimization (measurement only)

## Acceptance

- [x] `benchmarks/bench_memory_struct_graph.ark` created with nested structs and recursive types
- [x] `benchmarks/bench_memory_struct_graph.expected` created with correct stdout
- [x] Benchmark registered in `scripts/util/benchmark_runner.py` `BENCHMARKS` tuple
- [x] Tags: `allocation-heavy`, `struct-heavy`, `recursion-heavy`, `container`
- [x] `mise bench` passes for the new benchmark
- [x] `mise bench:update-baseline` includes the new benchmark
- [x] docs/benchmarks/feature-matrix.md updated to mark struct-heavy as covered

## Required verification

- `python scripts/manager.py verify quick` passes
- `mise bench` runs without errors

## Close gate

Benchmark fixture exists, is registered in the runner, and baseline is updated.
