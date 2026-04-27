---
Status: open
Created: 2026-04-21
ID: 544
Track: benchmark
Depends on: none
Orchestration class: implementation-ready
Orchestration upstream: —
Blocks v{N}: none
Source: benchmarks/ directory cleanup request
---

# 544 — Benchmark suite reorganization and docs integration

## Summary

The `benchmarks/` directory needs better organization and clearer result presentation in `docs/`. This issue reorganizes the benchmark suite structure and improves docs integration for better visibility of benchmark results.

## Primary paths

- `benchmarks/` (directory structure)
- `docs/process/benchmark-results.md` (result presentation)
- `scripts/util/benchmark_runner.py` (runner updates)

## Goals

1. **Directory organization**
   - Move legacy fixtures to `benchmarks/legacy/` subdirectory
   - Organize benchmarks by category: `benchmarks/cpu/`, `benchmarks/memory/`, `benchmarks/io/`, `benchmarks/parse/`
   - Update naming convention to `bench_<suite>_<name>.ark` for all new benchmarks

2. **Docs integration**
   - Improve `docs/process/benchmark-results.md` presentation with better tables and charts
   - Add trend visualization (if feasible with static docs)
   - Add per-benchmark detail pages with historical trends
   - Integrate with `docs/current-state.md` for performance snapshot

3. **Runner improvements**
   - Update `scripts/util/benchmark_runner.py` to handle new directory structure
   - Add category-based filtering (e.g., `--category cpu`)
   - Improve markdown report generation

## Acceptance

- [x] Legacy fixtures moved to `benchmarks/legacy/`
- [x] New benchmark category directories created (`cpu/`, `memory/`, `io/`, `parse/`)
- [x] All benchmarks follow `bench_<suite>_<name>.ark` naming convention
- [x] `scripts/util/benchmark_runner.py` updated to handle new structure
- [x] `docs/process/benchmark-results.md` improved with better presentation
- [x] `docs/current-state.md` includes performance snapshot section
- [x] `mise bench` works with new structure
- [x] `mise bench:update-baseline` works with new structure

## Required verification

- `python scripts/manager.py verify quick` passes
- `mise bench` runs without errors
- `mise bench:compare` runs without errors

## Close gate

Directory structure reorganized, docs integration improved, and all benchmark modes work correctly.

## Closed

- Status: done
- Date: 2026-04-22
- Commit: b531b7eb
- Acceptance: all YES
