# 496 — Selfhost typechecker: match exhaustiveness checking

**Track:** selfhost
**Status:** open
**Created:** 2026-04-14
**Updated:** 2026-04-14
**Source:** audit — issues/done/210-selfhost-typechecker-typed-fns.md "Out of scope (deferred)"

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

- [ ] Selfhost typechecker reports a diagnostic when a match is non-exhaustive
- [ ] Selfhost typechecker accepts exhaustive matches without false positives
- [ ] At least one positive and one negative fixture exercise exhaustiveness
- [ ] `cargo test` passes

## Required verification

- `bash scripts/run/verify-harness.sh --quick` passes

## Close gate

Acceptance items checked; fixtures prove exhaustiveness acceptance and rejection.
