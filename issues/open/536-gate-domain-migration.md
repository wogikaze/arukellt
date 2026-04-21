# Gate Domain Migration to manager.py

> **Status:** Implementation-ready (Phase 1 of #531 complete)
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

- [ ] `scripts/manager.py gate` domain added
- [ ] All gate subcommands pass behavioral contract tests
- [ ] Existing gate .sh scripts converted to thin wrappers
- [ ] CI updated (dual-run period)
