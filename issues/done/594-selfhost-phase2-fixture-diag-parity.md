---
Status: done
Created: 2026-04-22
Updated: 2026-05-16
ID: 594
Track: selfhost
Orchestration class: blocked-by-upstream
Depends on: 593
Parent: None
In scope:
Out of scope:
# Selfhost Phase 2: Fixture and Diagnostic Parity

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

593 (done — closed 2026-04-28, fixpoint PASS, merged via `e0f419f3`)

## Blocks

- #563 (pre-deletion invariants include `FAIL=0` across parity gates)

---

## Current status (2026-05-16 assessment)

### Dep #593

- **Fixpoint:** PASS (exit 0, sha256 identity)
- **Close note:** `selfhost fixture-parity` = PASS; 17/22 verify quick (5 pre-existing failures)

### `selfhost fixpoint` — SKIPPED (exit 2)

- No current wasm build; fixpoint not yet reached in this worktree.

### `selfhost fixture-parity` — PASS (exit 0)

- `FIXTURE_PARITY_SKIP` = 2 fixtures (baseline):
  - `stdlib_sort/sort_f64.ark` — f64_to_string digit extraction precision
  - `functions/higher_order.ark` — funcref table / call_indirect not yet emitted

### `selfhost diag-parity` — FAIL (1 check failed)

- **PASS=20, SKIP=27, FAIL=2**
- SKIP breakdown: 23 in `DIAG_PARITY_SKIP` + 4 with no `.diag` golden file
- **FAIL=2:**
  1. `selfhost/ret_stmt_mismatch.ark` — selfhost compiler does not produce the expected diagnostic:
     - Golden: `return type mismatch: 'bad' declared to return i32 but body returns String`
     - Selfhost: compiles successfully (no error emitted)
     - Root cause: selfhost typechecker does not detect return type mismatches in explicit return statements.
  2. `selfhost/match_non_exhaustive_enum.ark` — selfhost compiler does not produce the expected diagnostic:
     - Golden: `non-exhaustive match: missing Direction::West`
     - Selfhost: compiles successfully (no error emitted)
     - Root cause: selfhost exhaustiveness checker does not detect non-exhaustive matches on user-defined enums.

### `verify quick` — 19/22 pass, 0 skip, **3 fail** (all pre-existing hygiene/docs)

1. docs consistency (generated docs out of date)
2. doc example check (3 blocks in `docs/design/lang-uplift-gap-ledger.md`)
3. broken internal links
   - (Up from 5 failures at #593 close; #568 and #569 gates now pass)

---

## Acceptance

1. `python scripts/manager.py selfhost fixture-parity` reports FAIL=0 — **DONE**
2. `python scripts/manager.py selfhost diag-parity` reports FAIL=0 — **NOT DONE** (2 FAIL remain)
3. SKIP count does not increase compared to the baseline recorded in #593 — **NEEDS VERIFICATION** (no explicit SKIP count was recorded in #593 close note)

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
