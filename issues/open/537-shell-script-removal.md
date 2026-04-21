# Shell Script Removal After Deprecation

> **Status:** Pending (blocked until #533–#536 complete and stable)
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

- [ ] All domains (#533–#536) migrated and stable
- [ ] CI dual-run period ended for all domains
- [ ] Shell scripts removed
- [ ] docs/process/agent-harness.md updated to reference manager.py exclusively
