# Docs Domain Migration to manager.py

> **Status:** Implementation-ready (Phase 1 of #531 complete)
> **Track:** tooling
> **Parent:** #531
> **Type:** Implementation

## Scope

Migrate docs scripts into `scripts/manager.py docs` subcommands:

- `docs check` (check-docs-consistency.py, check-docs-freshness.py, check-doc-examples.py)
- `docs regenerate` (generate-docs.py)
- Remove `verify docs` stub (exit 0 / skip) introduced in Phase 1

## Acceptance Criteria

- [ ] `scripts/manager.py docs` domain added
- [ ] All docs subcommands pass behavioral contract tests
- [ ] `verify docs` stub replaced with delegation to `docs check`
- [ ] CI updated (dual-run period)
