# Shell Script Removal After Deprecation

> **Status:** done
> **Track:** tooling
> **Parent:** #531
> **Type:** Cleanup

## Scope

Remove all shell scripts that have been replaced by manager.py wrappers,
after the deprecation period (all domains migrated + CI dual-run period ends).

Scripts to remove:
- scripts/run/verify-harness.sh (thin wrapper — remove after CI dual-run complete)
- scripts/check/check-selfhost-*.sh (after #533)
- Docs check scripts replaced by manager.py docs (after #534)
- scripts/check/perf-gate.sh, scripts/run/run-benchmarks.sh (after #535)
- scripts/gate/*.sh (after #536)

## Acceptance Criteria

- [x] All domains (#533–#536) migrated and stable
- [x] CI dual-run period ended for all domains (scripts don't exist, N/A)
- [x] Shell scripts removed (scripts don't exist, N/A)
- [x] docs/process/agent-harness.md updated to reference manager.py exclusively
