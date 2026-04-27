---
Status: done
Created: 2026-04-14
Updated: 2026-04-15
ID: 499
Track: selfhost
Depends on: none
Orchestration class: implementation-ready
Orchestration upstream: —
Blocks v5: True
Source: audit — issues/done/493-selfhost-mir-control-flow-coverage.md STOP_IF
for closure environments: the selfhost parser has no `|params| body` closure
# 499 — Selfhost compiler: closure capture environment lowering
---
# 499 — Selfhost compiler: closure capture environment lowering

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

- [x] Selfhost parser recognises `|params| body` closure syntax
- [x] Selfhost typechecker resolves captured variables in closure environments
- [x] Selfhost compiler lowers closures with captured environments to MIR
- [x] At least one positive selfhost fixture exercises closure capture
- [x] `cargo test` passes (pre-existing failures in ark-wasm/ark-llvm are unrelated)

## Required verification

- `python scripts/manager.py verify quick` passes

## Close gate

Acceptance items checked; selfhost fixture proves closure capture lowering.