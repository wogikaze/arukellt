# 543 — Benchmark: file I/O (I/O-heavy workloads)

**Status**: open
**Created**: 2026-04-21
**ID**: 543
**Depends on**: 076
**Track**: benchmark
**Orchestration class**: implementation-ready
**Orchestration upstream**: —
**Blocks v{N}**: none
**Source**: docs/benchmarks/feature-matrix.md coverage gap

## Summary

docs/benchmarks/feature-matrix.md identifies **I/O-heavy workloads** as a medium-severity coverage gap with only trivial `println` calls. This issue adds `bench_file_io.ark` to measure performance of file read/write, String, and I/O host calls.

## Primary paths

- `benchmarks/bench_io_file_io.ark`
- `benchmarks/bench_io_file_io.expected`
- `scripts/util/benchmark_runner.py` (register benchmark)

## Non-goals

- Full I/O semantics beyond file read/write
- I/O optimization (measurement only)

## Acceptance

- [ ] `benchmarks/bench_io_file_io.ark` created with file read/write and I/O host calls
- [ ] `benchmarks/bench_io_file_io.expected` created with correct stdout
- [ ] Benchmark registered in `scripts/util/benchmark_runner.py` `BENCHMARKS` tuple
- [ ] Tags: `io-bound`, `string-heavy`, `allocation-heavy`
- [ ] `mise bench` passes for the new benchmark
- [ ] `mise bench:update-baseline` includes the new benchmark
- [ ] docs/benchmarks/feature-matrix.md updated to mark I/O-heavy as covered

## Required verification

- `python scripts/manager.py verify quick` passes
- `mise bench` runs without errors

## Close gate

Benchmark fixture exists, is registered in the runner, and baseline is updated.
