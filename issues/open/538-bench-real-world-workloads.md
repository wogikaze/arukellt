---
Status: open
Created: 2026-04-21
ID: 538
Track: benchmark
Depends on: none
Orchestration class: implementation-ready
Orchestration upstream: —
Blocks v{N}: none
Source: request for comprehensive real-world benchmarks
---

# 538 — Benchmark: real-world workloads

## Summary

The current benchmark suite consists of microbenchmarks (fib, vec_ops, string
_concat). This issue adds comprehensive real-world workloads that better represent actual usage patterns and stress multiple language features together.

## Primary paths

- `benchmarks/bench_application_*.ark` (multiple real-world workloads)
- `benchmarks/bench_application_*.expected`
- `scripts/util/benchmark_runner.py` (register benchmarks)

## Suggested workloads

1. **HTTP request parser** (`bench_application_http_parser.ark`)
   - String parsing, struct allocation, enum dispatch
   - Simulates parsing HTTP headers and body
   - Tags: `parse`, `string-heavy`, `struct-heavy`, `enum-heavy`

2. **Log processor** (`bench_application_log_processor.ark`)
   - String operations, Vec iteration, pattern matching
   - Simulates processing log lines with different formats
   - Tags: `string-heavy`, `iteration`, `match-heavy`, `io-bound`

3. **Configuration loader** (`bench_application_config_loader.ark`)
   - JSON/TOML parsing, nested structs, error handling
   - Simulates loading and validating configuration
   - Tags: `parse`, `struct-heavy`, `error-heavy`, `allocation-heavy`

4. **Data pipeline** (`bench_application_data_pipeline.ark`)
   - Vec operations, closures, transformations
   - Simulates ETL-style data processing
   - Tags: `allocation-heavy`, `closure-heavy`, `iteration`, `container`

5. **Template engine** (`bench_application_template_engine.ark`)
   - String interpolation, HashMap lookups, recursion
   - Simulates rendering templates with variables
   - Tags: `string-heavy`, `container`, `recursion-heavy`, `gc-pressure`

## Non-goals

- Full application implementation (simplified workloads only)
- External dependencies (all workloads self-contained)

## Acceptance

- [ ] At least 3 real-world workload benchmarks created
- [ ] Each benchmark has `.expected` file
- [ ] Benchmarks registered in `scripts/util/benchmark_runner.py`
- [ ] Appropriate tags assigned to each benchmark
- [ ] `mise bench` passes for all new benchmarks
- [ ] `mise bench:update-baseline` includes new benchmarks
- [ ] docs/benchmarks/feature-matrix.md updated with new benchmarks

## Required verification

- `python scripts/manager.py verify quick` passes
- `mise bench` runs without errors

## Close gate

At least 3 real-world workload benchmarks exist, are registered, and baseline
 is updated.
