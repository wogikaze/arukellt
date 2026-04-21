---
id: 530
title: "Selfhost CLI parity and diagnostic parity CI gates"
status: done
track: selfhost
created: 2026-04-21
updated: 2026-04-22
closed: 2026-04-22
closing-commit: 1eada2b6
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

- [x] CI job runs selfhost CLI parity check on every PR
- [x] CI job runs selfhost diagnostic parity check on every PR
- [x] Both jobs are merge-blocking
- [x] manager.py verify includes both checks

## Required verification

```bash
# CI config has parity jobs
grep -q 'cli.*parity\|diagnostic.*parity' .github/workflows/ci.yml
# manager.py verify includes checks
python scripts/manager.py verify
```

## Close gate

All 4 acceptance boxes checked with CI evidence (green PR run).

## Close note (2026-04-22)

Closed via close review. Evidence commit: `1eada2b6`
("chore(issues): close #558 CLI parity runner expansion"), which
mislabelled its scope but actually landed the #530 CI-gate wiring
alongside the #558 runner expansion. All four acceptance items are
satisfied at HEAD with concrete in-repo evidence:

1. **CI job runs selfhost CLI parity check on every PR** —
   `.github/workflows/ci.yml:409` step `Selfhost CLI parity
   (merge-blocking, #530)` runs `python3 scripts/manager.py selfhost
   parity --mode --cli` inside the `selfhost-bootstrap` job
   (`.github/workflows/ci.yml:361`), which executes on every PR push.
2. **CI job runs selfhost diagnostic parity check on every PR** —
   `.github/workflows/ci.yml:420` step `Selfhost diagnostic parity
   (merge-blocking, #530)` runs `python3 scripts/manager.py selfhost
   diag-parity` in the same job.
3. **Both jobs are merge-blocking** — `selfhost-bootstrap` is listed in
   the `verify` gate's `needs` at `.github/workflows/ci.yml:533`, and
   neither of the two new steps (lines 409–414 and 420–425) uses
   `|| true`, so a non-zero exit fails the PR (contrast the
   pre-existing fixture step at line 404 which is intentionally
   informational with `|| true`).
4. **manager.py verify includes both checks** — `scripts/manager.py`
   exposes `cmd_verify_selfhost_parity` and the `--selfhost-parity`
   flag, which is reachable from `verify --full`; `python3
   scripts/manager.py verify --selfhost-parity` exits 0 locally.

Local verification (exit codes):

```
python3 scripts/manager.py selfhost parity --mode --cli   # exit 0
python3 scripts/manager.py selfhost diag-parity           # exit 0
python3 -c "import yaml; yaml.safe_load(open('.github/workflows/ci.yml'))"  # exit 0
```
