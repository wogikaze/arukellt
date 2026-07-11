---
Status: done
Created: 2026-07-11
Updated: 2026-07-11
ID: 771
Track: docs-audit
Depends on: 768
Orchestration class: implementation-ready
Orchestration upstream: None
Blocks v{N}: none
Priority: 1
Source: Docs re-audit follow-up (P0-3 honesty vs selfhost CLI)
Blocks: none
---

# 771 — Docs: deny-clock/random honesty vs selfhost CLI

## Summary

Phase 1 #768 removed `future enforcement` and claimed compile-time MIR deny was
**implemented**. That matched Rust-era #291 / stale current-state prose, but the
selfhost CLI has **no** `--deny-clock` / `--deny-random` flags. Related fixtures
remain in `DIAG_PARITY_SKIP` (#459).

## Acceptance

- [x] `capabilities.toml` marks deny enforcement `unimplemented` with intended `compile_time_mir`
- [x] `process/policy.md`, `current-state.md`, platform capability tables agree
- [x] Generated `capability-surface.md` regenerated
- [x] No current doc claims deny flags are implemented on selfhost

## Completion

Completed 2026-07-11.
