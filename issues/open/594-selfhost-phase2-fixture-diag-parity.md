---
Status: open
Created: 2026-04-22
Updated: 2026-04-22
ID: 594
Track: main
Orchestration class: implementation-ready
Depends on: none
---
# Selfhost Phase 2: Fixture and Diagnostic Parity
**Parent**: #529
**Depends on**: 593
**Track**: selfhost
**Orchestration class**: blocked-by-upstream

---

## Summary

This is the child issue for #529 Phase 2 — Fixture and Diagnostic Parity.

Once fixpoint is achieved (#593), the selfhost compiler must demonstrate semantic parity
with the Rust-backed compiler path on the existing fixture suite. This issue tracks
closing the parity gap: all `FAIL` counts in `selfhost fixture-parity` and
`selfhost diag-parity` must reach zero.

---

## Scope

**In scope:**
- Identify every fixture that currently FAILs under the selfhost path but PASSes under Rust
- Fix each failing fixture by correcting selfhost behavior (not by deleting fixtures or
  relaxing expectations)
- Improve selfhost diagnostic output to reach parity with the reference diagnostics

**Out of scope:**
- Adding new fixtures beyond what is needed to close current failures
- Implementing new language features not already in the Rust path
- Architectural changes to the selfhost compiler beyond parity fixes

---

## Primary paths

- `src/compiler/*.ark` (any file contributing to parity failures)
- `tests/fixtures/selfhost/`
- `tests/fixtures/` (positive and negative, as needed for parity comparison)

## Allowed adjacent paths

- `scripts/check/` (parity checker scripts)

---

## Upstream / Depends on

593 (fixpoint must be stable before parity work can produce reliable baselines)

## Blocks

- #563 (pre-deletion invariants include `FAIL=0` across parity gates)

---

## Acceptance

1. `python scripts/manager.py selfhost fixture-parity` reports FAIL=0
2. `python scripts/manager.py selfhost diag-parity` reports FAIL=0
3. SKIP count does not increase compared to the baseline recorded in #593

---

## Required verification

```bash
python scripts/manager.py selfhost fixpoint
python scripts/manager.py selfhost fixture-parity
python scripts/manager.py selfhost diag-parity
python scripts/manager.py selfhost parity --mode --cli
python scripts/manager.py verify quick
```

Record all counts in the close note.

---

## STOP_IF

- Do not add new language features to close parity — only fix existing gaps
- Do not relax fixture expectations to make FAIL count drop
- Do not touch any Rust crates for deletion — that is Phase 5/7

---

## Close gate

Close when `selfhost fixture-parity` FAIL=0 and `selfhost diag-parity` FAIL=0,
with no increase in SKIP count versus the #593 baseline.