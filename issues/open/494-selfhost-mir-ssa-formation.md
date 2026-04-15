# 494 — Selfhost MIR: SSA formation pass

**Status**: blocked-by-upstream
**Created**: 2026-04-14
**Updated**: 2026-04-15
**ID**: 494
**Depends on**: 493, 503
**Track**: selfhost
**Blocks v1 exit**: no
**Source**: audit — issues/done/211-selfhost-mir-lower-fn-bodies.md "Out of scope (deferred)"

## Summary

Issue #211 defers "full MIR SSA form" to future work. No open issue tracks SSA
formation (phi-node insertion, dominance frontier computation) for the selfhost
MIR pipeline.

## Depends on

- #493 (selfhost MIR control-flow coverage)
- **#503 (selfhost MIR CFG + dominance-frontier infrastructure) — BLOCKING**

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
