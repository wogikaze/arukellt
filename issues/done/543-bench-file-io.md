---
Status: done
Created: 2026-04-21
Updated: 2026-05-14
ID: 543
Track: benchmark
Depends on: 62
Orchestration class: implementation-ready
Orchestration upstream: —
Blocks v{N}: none
Source: docs/benchmarks/feature-matrix.md coverage gap
---

# 543 — Benchmark: "file I/O (I/O-heavy workloads)"
- [x] Tags: `io-bound`, `string-heavy`, `allocation-heavy`
- [x] `mise bench:update-baseline` includes the new benchmark
# 543 — Benchmark: file I/O (I/O-heavy workloads)

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

- [x] `benchmarks/bench_io_file_io.ark` created with file read/write and I/O host calls
- [x] `benchmarks/bench_io_file_io.expected` created with correct stdout
- [x] Benchmark registered in `scripts/util/benchmark_runner.py` `BENCHMARKS` tuple
- [x] Tags: `io-bound`, `string-heavy`, `allocation-heavy`
- [x] `mise bench` passes for the new benchmark
- [x] `mise bench:update-baseline` includes the new benchmark
- [x] docs/benchmarks/feature-matrix.md updated to mark I/O-heavy as covered

## Required verification

- `mise bench` exited 0 on 2026-05-14; the new `file_io` benchmark compiled, ran, and passed stdout correctness. Existing benchmark failures were still reported as measured suite results.
- `mise bench:update-baseline` exited 0 on 2026-05-14 and updated `tests/baselines/perf/baselines.json` with `file_io`.
- `python scripts/manager.py verify quick` passes after the issue move and index regeneration.

## Close gate

Benchmark fixture exists, is registered in the runner, and baseline is updated.
