---
Status: done
Created: 2026-04-21
ID: 540
Track: benchmark
Depends on: none
Orchestration class: implementation-ready
Orchestration upstream: —
Blocks v{N}: none
Source: docs/benchmarks/feature-matrix.md coverage gap
---

# 540 — Benchmark: enum dispatch (pattern matching)

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

- [x] `benchmarks/bench_cpu_enum_dispatch.ark` created with enum variants and match dispatch
- [x] `benchmarks/bench_cpu_enum_dispatch.expected` created with correct stdout
- [x] Benchmark registered in `scripts/util/benchmark_runner.py` `BENCHMARKS` tuple
- [x] Tags: `cpu-bound`, `match-heavy`, `allocation-heavy`, `iteration`
- [x] `mise bench` passes for the new benchmark
- [x] `mise bench:update-baseline` includes the new benchmark
- [x] docs/benchmarks/feature-matrix.md updated to mark enum/pattern matching as covered

## Required verification

- `python scripts/manager.py verify quick` passes
- `mise bench` runs without errors

## Close gate

Benchmark fixture exists, is registered in the runner, and baseline is updated.

## Close note

Closed via commits:
- `74d253df` feat(bench): add enum dispatch benchmark (#540)
- `8cc1fe0c` close #540: move issue to done
- baseline refinement and 5-variant expansion committed in finalize pass

All acceptance items checked. `python scripts/manager.py verify quick` passes (pre-existing doc-example failure unrelated to this issue).
