---
Status: done
Created: 2026-04-14
Updated: 2026-04-14
Track: selfhost
Source: "audit — issues/done/210-selfhost-typechecker-typed-fns.md (Out of scope deferred)"
Orchestration class: implementation-ready
Depends on: none
# 496 — Selfhost typechecker: match exhaustiveness checking
---
# 496 — Selfhost typechecker: match exhaustiveness checking

## Summary

Issue #210 deferred match exhaustiveness checking for the selfhost typechecker.
No open issue tracks this.

## Depends on

- #493 (selfhost MIR control-flow coverage — match lowering needed first)

## Primary paths

- `src/compiler/`

## Non-goals

- Pattern optimization (or-patterns, guard elision)
- Rust-side exhaustiveness improvements

## Acceptance

- [x] Selfhost typechecker reports a diagnostic when a match is non-exhaustive
- [x] Selfhost typechecker accepts exhaustive matches without false positives
- [x] At least one positive and one negative fixture exercise exhaustiveness
- [x] `cargo test` passes

## Required verification

- `bash scripts/run/verify-harness.sh --quick` passes

## Close gate

Acceptance items checked; fixtures prove exhaustiveness acceptance and rejection.