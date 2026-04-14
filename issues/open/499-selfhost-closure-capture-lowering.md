# 499 — Selfhost compiler: closure capture environment lowering

**Track:** selfhost
**Status:** open
**Created:** 2026-04-14
**Updated:** 2026-04-14
**Source:** audit — issues/done/493-selfhost-mir-control-flow-coverage.md STOP_IF

## Summary

Issue #493 implemented MIR lowering for match/loop/for but triggered STOP_IF
for closure environments: the selfhost parser has no `|params| body` closure
syntax and the selfhost typechecker has no closure-environment support.

This issue tracks completing closure lowering once the parser and typechecker
prerequisites land.

## Primary paths

- `src/compiler/`

## Non-goals

- Rust-side closure changes
- Higher-order function calling conventions beyond lexical capture

## Acceptance

- [ ] Selfhost parser recognises `|params| body` closure syntax
- [ ] Selfhost typechecker resolves captured variables in closure environments
- [ ] Selfhost compiler lowers closures with captured environments to MIR
- [ ] At least one positive selfhost fixture exercises closure capture
- [ ] `cargo test` passes

## Required verification

- `bash scripts/run/verify-harness.sh --quick` passes

## Close gate

Acceptance items checked; selfhost fixture proves closure capture lowering.
