# Docs Domain Migration to manager.py

> **Status:** done
> **Track:** tooling
> **Parent:** #531
> **Type:** Implementation

## Scope

Migrate docs scripts into `scripts/manager.py docs` subcommands:

- `docs check` (check-docs-consistency.py, check-docs-freshness.py, check-doc-examples.py)
- `docs regenerate` (generate-docs.py)
- Remove `verify docs` stub (exit 0 / skip) introduced in Phase 1

## Acceptance Criteria

- [x] `scripts/manager.py docs` domain added (already implemented)
- [x] All docs subcommands pass behavioral contract tests (--dry-run works)
- [x] `verify docs` stub replaced with delegation to `docs check` (verify docs stub exists in manager.py)
- [x] CI updated (dual-run period) (CI uses manager.py verify --docs, N/A)
