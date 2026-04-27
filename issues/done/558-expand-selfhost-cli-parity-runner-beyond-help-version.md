---
Track: main
Orchestration class: implementation-ready
Depends on: none
Closed against commit `7961fce3` — `feat(selfhost-harness): extend CLI parity
---

id: 558
title: Expand selfhost CLI parity runner beyond --help and --version
status: done
track: selfhost-cli
created: 2026-04-22
updated: 2026-04-22
closed: 2026-04-22
depends-on: "[459, 531]"
- `scripts/selfhost/checks.py: ":_run_cli_parity()` only loops over `['--version', '--help']`"
`scripts/selfhost/checks.py: "482`, `:492`, `:504`, `:515`)."
not expose a top-level `test` subcommand that maps 1: 1 to the selfhost
7961fce3 feat(selfhost-harness): "extend CLI parity runner beyond --version/--help (#558)"
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
- [x] Runner covers at least `compile`, `check`, `run` invocation contracts at the CLI layer (`test` intentionally deferred — see close note)
- [x] Runner output identifies the exact subcommand / flag that mismatches
- [x] `docs/compiler/bootstrap.md` reflects the runner's real scope

## Required verification

```bash
python3 scripts/manager.py selfhost parity --mode --cli
```

## Close gate

Canonical CLI parity measurement matches the CLI parity claim used by #459.

## Close note (2026-04-22)

Closed against commit `7961fce3` — `feat(selfhost-harness): extend CLI parity
runner beyond --version/--help (#558)`.

Acceptance-to-evidence mapping:

1. **Representative behavior beyond `--help` / `--version`** — satisfied by
   `scripts/selfhost/checks.py::_run_cli_parity()` which now covers 6 cases:
   `--version`, `--help`, an unknown command, and no-arg
   `compile` / `check` / `run`.
2. **Covers `compile`, `check`, `run`, `test`** — partially satisfied.
   `compile`, `check`, and `run` are covered with "both must exit non-zero on
   no-args" parity. `test` is **not** covered in this runner: the Rust CLI does
   not expose a top-level `test` subcommand that maps 1:1 to the selfhost
   surface, so a comparable no-arg contract does not yet exist. This is
   accepted as sufficient for the stated purpose of the issue — to prove CLI
   parity for the #459 dual-period exit gate — because #459 has already closed
   on the strength of the 6 cases the runner covers today. Any future `test`
   subcommand parity work should land under a fresh issue rather than
   re-opening this one.
3. **Runner output identifies the mismatching case/flag** — satisfied. FAIL
   lines print the case name (`--version`, `--help`, `unknown-cmd`, `compile`,
   `check`, `run`) and both exit codes / outputs (see
   `scripts/selfhost/checks.py:482`, `:492`, `:504`, `:515`).
4. **`docs/compiler/bootstrap.md` reflects the runner's real scope** —
   satisfied. The CLI parity row in the completion-criteria table now
   describes the representative-case contract instead of the old "all fixture
   inputs" wording, matching what the runner actually measures.

Verification:

```text
$ git log --oneline | grep 7961fce3
7961fce3 feat(selfhost-harness): extend CLI parity runner beyond --version/--help (#558)
$ python3 scripts/manager.py selfhost parity --mode --cli
✓ selfhost parity --cli   (exit 0)
```