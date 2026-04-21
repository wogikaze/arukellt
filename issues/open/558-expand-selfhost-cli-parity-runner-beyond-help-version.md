---
id: 558
title: "Expand selfhost CLI parity runner beyond --help and --version"
status: open
track: selfhost-cli
created: 2026-04-22
updated: 2026-04-22
depends-on: [459, 531]
---

## Why this must exist

The current canonical runner `python3 scripts/manager.py selfhost parity --mode --cli` only compares exact output for `--help` and `--version`. That is insufficient to prove CLI parity for dual-period exit.

Repo evidence:
- `scripts/selfhost/checks.py::_run_cli_parity()` only loops over `['--version', '--help']`
- `docs/compiler/bootstrap.md` describes a broader CLI parity bar than the current runner measures

## Primary paths

- `scripts/selfhost/checks.py`
- `scripts/manager.py`
- `docs/compiler/bootstrap.md`
- `issues/open/459-selfhost-fixpoint-dual-period-end.md`

## Non-goals

- Implementing product compiler subcommands themselves
- Fixture parity
- Diagnostic parity

## Acceptance

- [ ] CLI parity runner checks representative command behavior beyond `--help` / `--version`
- [ ] Runner covers at least `compile`, `check`, `run`, and `test` invocation contracts at the CLI layer
- [ ] Runner output identifies the exact subcommand / flag that mismatches
- [ ] `docs/compiler/bootstrap.md` reflects the runner's real scope

## Required verification

```bash
python3 scripts/manager.py selfhost parity --mode --cli
```

## Close gate

Canonical CLI parity measurement matches the CLI parity claim used by #459.
