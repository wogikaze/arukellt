# 493 — Selfhost MIR lowering: control-flow coverage (match/loop/closure)

**Track:** selfhost
**Status:** open
**Created:** 2026-04-14
**Updated:** 2026-04-14
**Source:** audit — issues/done/211-selfhost-mir-lower-fn-bodies.md "Out of scope (deferred)"

## Summary

Issue #211 shipped basic fn-body MIR lowering but explicitly deferred match
expressions, loop constructs, and closure environments.
No open issue tracks these deferred lowering targets for the selfhost compiler.

## Depends on

- #309 (selfhost module import resolution)

## Primary paths

- `src/compiler/`

## Non-goals

- MIR optimization passes (separate issues)
- SSA formation (tracked in #494)

## Acceptance

- [ ] Selfhost compiler lowers match expressions to MIR branch/switch
- [ ] Selfhost compiler lowers loop/while/for to MIR loop headers and back-edges
- [ ] Selfhost compiler lowers closures with captured environment to MIR
- [ ] Selfhost fixtures exercise each construct

## Required verification

- `bash scripts/run/verify-harness.sh --quick` passes

## Close gate

Acceptance items checked; selfhost fixtures prove lowering for all three constructs.
