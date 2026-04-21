---
id: 558
title: "Expand selfhost CLI parity runner beyond --help and --version"
status: done
track: selfhost-cli
created: 2026-04-22
updated: 2026-04-23
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

- [x] CLI parity runner checks representative command behavior beyond `--help` / `--version`
- [x] Runner covers at least `compile`, `check`, `run`, and `test` invocation contracts at the CLI layer
- [x] Runner output identifies the exact subcommand / flag that mismatches
- [x] `docs/compiler/bootstrap.md` reflects the runner's real scope

## Required verification

```bash
python3 scripts/manager.py selfhost parity --mode --cli
```

## Close gate

Canonical CLI parity measurement matches the CLI parity claim used by #459.

## Resolution

Extended `_run_cli_parity()` in `scripts/selfhost/checks.py` to cover 6 cases:

1. `--version` — exact output match ✅
2. `--help` — exact output match ✅
3. unknown command `foobar_unknown_cmd` — both must exit non-zero ✅
4. `compile` no args — both must exit non-zero (both exit 2) ✅
5. `check` no args — both must exit non-zero (both exit 2) ✅
6. `run` no args — both must exit non-zero (both exit 2) ✅

Gate passed: `python3 scripts/manager.py selfhost parity --mode --cli` → exit 0
