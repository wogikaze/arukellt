---
id: 530
title: "Selfhost CLI parity and diagnostic parity CI gates"
status: open
track: selfhost
created: 2026-04-21
updated: 2026-04-21
depends-on: [268, 288, 289]
---

## Why this must exist

Issue #268 (selfhost-parity-ci-verification) was closed with 2/4 acceptance criteria unchecked:
CLI parity scripts and diagnostic parity scripts exist locally but are NOT wired into CI.
Only fixture parity is verified in CI. This issue tracks adding the missing CI gates.

## Evidence source

- issues/done/268-selfhost-parity-ci-verification.md: 2 of 4 checkboxes unchecked
- .github/workflows/ci.yml: no cli-parity or diagnostic-parity job

## Primary paths

- .github/workflows/ci.yml
- `python scripts/manager.py selfhost parity --mode --cli` (or equivalent)
- `python scripts/manager.py selfhost diag-parity` (or equivalent)

## Non-goals

- Implementing new parity scripts (they already exist)
- Changing selfhost compiler behavior

## Acceptance

- [ ] CI job runs selfhost CLI parity check on every PR
- [ ] CI job runs selfhost diagnostic parity check on every PR
- [ ] Both jobs are merge-blocking
- [ ] manager.py verify includes both checks

## Required verification

```bash
# CI config has parity jobs
grep -q 'cli.*parity\|diagnostic.*parity' .github/workflows/ci.yml
# manager.py verify includes checks
python scripts/manager.py verify
```

## Close gate

All 4 acceptance boxes checked with CI evidence (green PR run).
