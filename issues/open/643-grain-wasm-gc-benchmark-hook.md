---
Status: open
Created: 2026-06-12
Updated: 2026-06-12
ID: 643
Track: benchmark
Depends on: 112
Orchestration class: implementation-ready
Blocks v1 exit: none
Source: docs-to-issues audit — docs/process/docs-gap-inventory-2026-06-12.md
---

# 643 — Grain Wasm GC benchmark comparison hook

## Summary

docs/process/benchmark-results.md notes Grain (Wasm GC) is not in the benchmark runner. Issue #112 context defers Grain comparison.

## Evidence source

docs/process/benchmark-results.md L26, docs/process/roadmap-v4.md, scripts/run/compare-benchmarks.sh

## Primary paths

benchmarks/, scripts/run/compare-benchmarks.sh, docs/process/benchmark-results.md

## Non-goals

Grain language feature parity, CI blocking on Grain availability

## Acceptance

- [ ] Benchmark runner documents Grain hook point (script flag or README section)
- [ ] Optional benchmarks/*.grain sources or documented skip when grain CLI absent
- [ ] docs/process/benchmark-results.md updated with Grain comparison status

## Required verification

```bash
bash scripts/run/compare-benchmarks.sh --help
python3 scripts/manager.py verify quick
```

## Close gate

Grain path documented; runner skips gracefully without grain CLI; no false claims of Grain numbers in docs.
