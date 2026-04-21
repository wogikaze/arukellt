# Perf Domain Migration to manager.py

> **Status:** Implementation-ready (Phase 1 of #531 complete)
> **Track:** tooling
> **Parent:** #531
> **Type:** Implementation

## Scope

Migrate perf scripts into `scripts/manager.py perf` subcommands:

- `perf baseline` (collect-baseline.py, run-benchmarks.sh)
- `perf gate` (perf-gate.sh)

## Acceptance Criteria

- [ ] `scripts/manager.py perf` domain added
- [ ] All perf subcommands pass behavioral contract tests
- [ ] Existing perf .sh scripts converted to thin wrappers
- [ ] CI updated (dual-run period)
