# Selfhost Domain Migration to manager.py

> **Status:** Implementation-ready (Phase 1 of #531 complete)
> **Track:** tooling
> **Parent:** #531
> **Type:** Implementation

## Scope

Migrate selfhost scripts into `scripts/manager.py selfhost` subcommands:

- `selfhost fixpoint` (check-selfhost-fixpoint.sh)
- `selfhost fixture-parity` (check-selfhost-fixture-parity.sh)
- `selfhost diag-parity` (check-selfhost-diagnostic-parity.sh)
- `selfhost parity` (check-selfhost-parity.sh)

## Acceptance Criteria

- [ ] `scripts/manager.py selfhost` domain added
- [ ] All selfhost subcommands pass behavioral contract tests
- [ ] Existing selfhost .sh scripts converted to thin wrappers
- [ ] CI updated (dual-run period)
