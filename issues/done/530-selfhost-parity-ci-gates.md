---
---

id: 530
title: Selfhost CLI parity and diagnostic parity CI gates
status: done
track: selfhost
created: 2026-04-21
updated: 2026-04-22
depends-on: "[268, 288, 289]"
closed-by: feat/530-ci-parity-gates
Track: main
Orchestration class: implementation-ready
Depends on: none
- issues/done/268-selfhost-parity-ci-verification.md: 2 of 4 checkboxes unchecked
- .github/workflows/ci.yml: no cli-parity or diagnostic-parity job
- name: Selfhost diagnostic parity check
needs: "[..., selfhost-cli-parity, selfhost-diag-parity, ...]"
- `selfhost-cli-parity`: runs `python3 scripts/manager.py selfhost parity --mode --cli`, no continue-on-error, depends on selfhost-bootstrap
- `selfhost-diag-parity`: runs `python3 scripts/manager.py selfhost diag-parity`, no continue-on-error, depends on selfhost-bootstrap
Both jobs are in the final `verify` gate `needs: ` list, making them merge-blocking.
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

## Verification evidence

```
$ grep 'cli.*parity\|parity.*cli\|diag.*parity\|parity.*diag' .github/workflows/ci.yml
#  selfhost-parity         selfhost-cli-parity           yes           selfhost CLI parity gate (#530)
#  selfhost-parity         selfhost-diag-parity          yes           selfhost diagnostic parity gate (#530)
          python3 scripts/manager.py selfhost parity --mode --cli 2>&1 \
  selfhost-cli-parity:
          python3 scripts/manager.py selfhost parity --mode --cli 2>&1 \
  selfhost-diag-parity:
      - name: Selfhost diagnostic parity check
          python3 scripts/manager.py selfhost diag-parity 2>&1 \
    needs: [..., selfhost-cli-parity, selfhost-diag-parity, ...]

$ python -c "import yaml; yaml.safe_load(open('.github/workflows/ci.yml')); print('YAML valid')"
YAML valid
```

Two new jobs added to .github/workflows/ci.yml:
- `selfhost-cli-parity`: runs `python3 scripts/manager.py selfhost parity --mode --cli`, no continue-on-error, depends on selfhost-bootstrap
- `selfhost-diag-parity`: runs `python3 scripts/manager.py selfhost diag-parity`, no continue-on-error, depends on selfhost-bootstrap

Both jobs are in the final `verify` gate `needs:` list, making them merge-blocking.

manager.py already had `cmd_verify_selfhost_parity` wired into `verify --selfhost-parity` and `verify --full` (added before this PR).