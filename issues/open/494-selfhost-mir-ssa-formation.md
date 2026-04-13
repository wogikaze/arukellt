# 494 — Selfhost MIR: SSA formation pass

**Track:** selfhost
**Status:** open
**Created:** 2026-04-14
**Updated:** 2026-04-14
**Source:** audit — issues/done/211-selfhost-mir-lower-fn-bodies.md "Out of scope (deferred)"

## Summary

Issue #211 defers "full MIR SSA form" to future work. No open issue tracks SSA
formation (phi-node insertion, dominance frontier computation) for the selfhost
MIR pipeline.

## Depends on

- #493 (selfhost MIR control-flow coverage)

## Primary paths

- `src/compiler/`

## Non-goals

- MIR optimization passes that consume SSA (separate issues)
- Rust-side MIR SSA changes

## Acceptance

- [ ] Selfhost MIR pipeline produces SSA-form IR with phi nodes at join points
- [ ] At least one fixture demonstrates SSA phi elimination for a simple branch
- [ ] `cargo test` passes

## Required verification

- `bash scripts/run/verify-harness.sh --quick` passes

## Close gate

Acceptance items checked; MIR dump shows phi nodes at join points.
