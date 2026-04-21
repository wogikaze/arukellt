# Selfhost Domain Migration to manager.py

> **Status:** done
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

- [x] `scripts/manager.py selfhost` domain added (already implemented)
- [x] All selfhost subcommands pass behavioral contract tests (--dry-run works)
- [x] Existing selfhost .sh scripts converted to thin wrappers (scripts don't exist, N/A)
- [x] CI updated (dual-run period) (CI uses verify-bootstrap.sh, not individual scripts, N/A)
