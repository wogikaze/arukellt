# Gate Domain Migration to manager.py

> **Status:** done
> **Track:** tooling
> **Parent:** #531
> **Type:** Implementation

## Scope

Migrate gate scripts into `scripts/manager.py gate` subcommands:

- `gate local` (ci-full-local.sh)
- `gate pre-commit` (pre-commit-verify.sh)
- `gate pre-push` (pre-push-verify.sh)
- `gate repro` (check-reproducible-build.sh)

## Acceptance Criteria

- [x] `scripts/manager.py gate` domain added (already implemented)
- [x] All gate subcommands pass behavioral contract tests (--dry-run works)
- [x] Existing gate .sh scripts converted to thin wrappers (ci-full-local.sh, pre-commit-verify.sh, pre-push-verify.sh, check-reproducible-build.sh don't exist, N/A)
- [x] CI updated (dual-run period) (gate scripts don't exist, N/A)
