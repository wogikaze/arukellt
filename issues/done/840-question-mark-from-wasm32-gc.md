---
Status: done
Created: 2026-07-26
Updated: 2026-07-26
ID: 840
Track: language-design
Depends on: 839
Orchestration class: implementation
Orchestration upstream: 839
Blocks v{N}: none
Priority: 3
Source: #839 follow-up — From conversion on wasm32-gc Err path
---

# 840 — Enable `?` From conversion on wasm32-gc

## Summary

#839 wired `SemanticTraitId::From` resolution for `?`. On `wasm32-gc`,
`try_resolve_from_conversion` still returns empty so the Err path is identity
early-return only (no payload rewrite via `From::from`).

## Required work

- [x] Emit GC-safe Err payload extract → `From::from` call → store → return
      for `Result` on wasm32-gc.
- [x] Keep Option `?` and same-type Result `?` behavior unchanged.
- [x] Regression: `from_error.ark` under `--target wasm32-gc` (execute or
      validate+equivalent MIR assert once runtime allows).

## Acceptance

- [x] `try_from_conversion.ark` no longer early-returns empty solely because
      `is_gc`.
- [x] `python3 scripts/manager.py verify lane` passes.

## Close evidence (2026-07-26)

- GC Err rewrite in `try_emit_err_from_conversion_gc` (`try_from_conversion.ark`).
- `question_mark/from_error.ark`: host-run prints `OK`; fixture-parity note
  "pinned wasm invalid, current OK".
- Full fixture-parity after L5: FAIL=294 (from 299); #807 remains open.
- Follow-up (not this issue): `from_trait_not_inherent` still traps on GC when
  no `From` impl exists (typed Err identity).

## References

- ADR-039 D2, #839
- `src/compiler/mir/lower/try_from_conversion.ark`
