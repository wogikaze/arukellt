---
Status: done
Track: main
Orchestration class: done
Depends on: none
Sub-issues: 624, 594, 625, 564, 626, 627, 628, 571, 631, 574, 575, 576, 577, 578, 579, 580, 581, 582
Updated: 2026-05-17
---

# 529 — 100% Selfhost Transition Plan

## Summary

The selfhost transition lane is complete. The compiler entrypoint is selfhost
native, the former Rust package workspace has been retired, and the final
workspace deletion is recorded in #582.

## Sub-Issues

- [x] #624 Phase 1 fixpoint achievement: `issues/done/529-phase1-fixpoint-achievement.md`
- [x] #594 Phase 2+3 fixture and diagnostic parity: `issues/done/594-selfhost-phase2-fixture-diag-parity.md`
- [x] #625 Phase 4 dual-run period: `issues/done/529-phase4-dual-run-period.md`
- [x] #564 Phase 5 delete former CLI entrypoint: `issues/done/564-phase5-delete-arukellt.md`
- [x] #626 Phase 6/A IDE-ready frontend: `issues/done/529-phase6a-ide-ready-frontend.md`
- [x] #627 Phase 6/B analysis API: `issues/done/529-phase6b-analysis-api.md`
- [x] #628 Phase 6/C LSP minimum viable: `issues/done/529-phase6c-lsp-minimum-viable.md`
- [x] #571 Phase 6/D DAP scaffold: `issues/done/571-phase6-debug-adapter-scaffold-deferred-priority.md`
- [x] #631 Phase 7 delete playground wasm package: `issues/done/631-phase7-delete-ark-playground-wasm.md`
- [x] #574 Phase 7 delete lexer package: `issues/done/574-phase7-delete-ark-lexer.md`
- [x] #575 Phase 7 delete parser package: `issues/done/575-phase7-delete-ark-parser.md`
- [x] #576 Phase 7 delete resolver package: `issues/done/576-phase7-delete-ark-resolve.md`
- [x] #577 Phase 7 delete typecheck package: `issues/done/577-phase7-delete-ark-typecheck.md`
- [x] #578 Phase 7 delete HIR package: `issues/done/578-phase7-delete-ark-hir.md`
- [x] #579 Phase 7 delete diagnostics package: `issues/done/579-phase7-delete-ark-diagnostics.md`
- [x] #580 Phase 7 delete manifest package: `issues/done/580-phase7-delete-ark-manifest.md`
- [x] #581 Phase 7 delete target package: `issues/done/581-phase7-delete-ark-target.md`
- [x] #582 Phase 7 remove Cargo workspace: `issues/done/582-phase7-remove-cargo-workspace.md`

## Close Evidence

- [x] `python scripts/manager.py verify` rc=0: 23 passed, 0 skipped, 0 failed.
- [x] `python scripts/manager.py selfhost fixpoint` rc=0.
- [x] `python scripts/manager.py selfhost fixture-parity` rc=0.
- [x] `python scripts/manager.py selfhost parity --mode --cli` rc=0.
- [x] `python scripts/manager.py selfhost diag-parity` rc=0.
- [x] `test ! -e Cargo.toml && test ! -e Cargo.lock && test ! -d crates` rc=0.
