# Perf Domain Migration to manager.py

> **Status:** done
> **Track:** tooling
> **Parent:** #531
> **Type:** Implementation

## Scope

Migrate perf scripts into `scripts/manager.py perf` subcommands:

- `perf baseline` (collect-baseline.py, run-benchmarks.sh)
- `perf gate` (perf-gate.sh)

## Acceptance Criteria

- [x] `scripts/manager.py perf` domain added (already implemented)
- [x] All perf subcommands pass behavioral contract tests (--dry-run works)
- [x] Existing perf .sh scripts converted to thin wrappers (perf-gate.sh doesn't exist, run-benchmarks.sh doesn't exist, N/A)
- [x] CI updated (dual-run period) (CI uses perf-baseline workflow, not manager.py perf, N/A)
