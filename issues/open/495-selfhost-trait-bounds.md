# 495 — Selfhost typechecker: trait bounds and constraint solving

**Status**: blocked-by-upstream
**Created**: 2026-04-14
**Updated**: 2026-04-15
**ID**: 495
**Depends on**: 312, 504
**Track**: selfhost
**Blocks v1 exit**: no
**Source**: audit — issues/done/210-selfhost-typechecker-typed-fns.md "Out of scope (deferred)"

## Summary

Issue #210 shipped basic typed-function typechecking but explicitly deferred
trait bounds and constraint solving. No open issue tracks this for the selfhost
compiler.

## Depends on

- #312 (selfhost generic monomorphization)
- #504 (trait/interface syntax and impl-block infrastructure)

## Primary paths

- `src/compiler/`

## Non-goals

- Higher-kinded types
- Trait coherence / orphan rules (language-level design, not selfhost scope)

## Acceptance

- [ ] Selfhost typechecker resolves trait bounds on generic parameters
- [ ] Selfhost typechecker rejects programs with unsatisfied trait bounds
- [ ] At least one positive and one negative fixture exercise trait-bound checking
- [ ] `cargo test` passes

## Required verification

- `bash scripts/run/verify-harness.sh --quick` passes

## Close gate

Acceptance items checked; fixtures prove trait-bound acceptance and rejection.
