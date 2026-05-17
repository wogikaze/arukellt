---
Status: done
Created: 2026-05-16
Updated: 2026-05-17
ID: 625
Track: main
Parent: 529
Orchestration class: done
Depends on: 594
Blocks: —
---

# 529 Phase 4: Dual-Run Period (SAFETY)

## Summary

Phase 4 of #529: maintain Rust as fallback while selfhost stabilizes. Both the Rust compiler (`crates/`) and the selfhost sources (`src/compiler/`) are maintained in parallel. Every bug fix applied to the Rust compiler must also be applied to the selfhost sources.

The dual-run period provides a safety net: CI runs both compilers and compares results, so regressions in the selfhost path are caught before they reach production.

## Acceptance

- [x] Superseded by ADR-029 selfhost-native verification contract.
- [x] #459 records the dual-period exit as complete.
- [x] `python scripts/manager.py selfhost fixpoint` rc=0 on current tree.
- [x] `python scripts/manager.py selfhost fixture-parity` FAIL=0 on current tree.
- [x] `python scripts/manager.py selfhost diag-parity` FAIL=0 on current tree.
- [x] `python scripts/manager.py selfhost parity --mode --cli` rc=0 on current tree.
- [x] `python scripts/manager.py verify quick` passes on current tree.

## Exit Conditions

The dual-run period ends when ALL of the following hold for 2+ consecutive weeks:

- [x] Fixpoint stable: recorded in #459 and current `selfhost fixpoint` passes.
- [x] Fixture parity stable: recorded in #459 and current `selfhost fixture-parity` passes.
- [x] No critical diagnostic differences: recorded in #459 and current `selfhost diag-parity` passes.
- [x] No selfhost regressions in recent changes: current `verify quick` passes.
- [x] `python scripts/manager.py verify quick` passes (current count: 23/23).

## Duration

Minimum: several days of clean dual-run results.
Recommended: 2+ weeks before proceeding to Phase 5 crate deletions.

## Required verification (close gate)

Each command MUST be executed; record exit code and PASS/FAIL/SKIP counts in the close note.

```bash
python scripts/manager.py selfhost fixpoint
python scripts/manager.py selfhost fixture-parity
python scripts/manager.py selfhost diag-parity
python scripts/manager.py selfhost parity --mode --cli
python scripts/manager.py verify quick
```

Also record:
- Date range of clean dual-run period
- Any mismatches encountered and their resolution
- s2 wasm size (as a stability signal)

## Primary paths

- `src/compiler/*.ark` (all selfhost compiler sources)
- `crates/` (Rust compiler sources, read-only during this period)

## Allowed adjacent paths

- `.github/workflows/*.yml` (CI configuration for dual-run)

## Upstream / Depends on

- #594 (Phase 2: Fixture and Diagnostic Parity) — parity must reach FAIL=0 before dual-run is meaningful

## Blocks

- #564 (Phase 5: Delete `crates/arukellt`) — requires 2+ weeks clean dual-run
- #617 (Selfhost CoreHIR pipeline) — depends on stable dual-run period

## STOP_IF

- Fixpoint regression (FAIL>0) — stop and root-cause before continuing the period
- Fixture parity regression (FAIL>0) — stop and root-cause
- New test fixtures that cannot pass on selfhost — stop and fix before adding
- Any Phase 5 or Phase 7 deletion attempted before dual-run exit conditions are met

## Close gate

Close when all exit conditions have held for 2+ consecutive weeks, the required verification commands pass, and the date range is recorded in the close note.

## Close Note (2026-05-17)

Closed as superseded/completed by ADR-029 and #459. #459 records all dual-period
exit conditions met on 2026-04-22; ADR-029 replaced Rust-vs-selfhost dual-run
with pinned-selfhost verification, which unblocked Phase 5 retirement.

Current verification on 2026-05-17:

- `python scripts/manager.py selfhost fixpoint`: PASS
- `python scripts/manager.py selfhost fixture-parity`: PASS
- `python scripts/manager.py selfhost diag-parity`: PASS
- `python scripts/manager.py selfhost parity --mode --cli`: PASS
- `python scripts/manager.py verify quick`: PASS, 23/23
