---
Status: done
Created: 2026-07-25
Updated: 2026-07-26
ID: 839
Track: language-design
Depends on: 690
Orchestration class: implementation
Orchestration upstream: 690
Blocks v{N}: none
Priority: 3
Source: #690 follow-up — living From lookup vs ADR-039 D2 SemanticTraitId
---

# 839 — Resolve `?` From conversion via SemanticTraitId

## Summary

Wire canonical `SemanticTraitId::From` into SignatureRegistry so `?` error
conversion does not invent `E_target::from` from type-name strings.

## Required work

- [x] Populate `SignatureEntry.trait_id` for `impl From<E> for T` methods.
- [x] Resolve via `SemanticTraitId::From` + `(E_source, E_target)` type names
      on SignatureEntry params/returns (`signature_registry_from.ark`).
- [x] Keep import-independence (language syntax desugaring).
- [x] wasm32-gc Err rewrite deferred to #840 (trait-id path complete on wasm32).
- [x] Regression: `from_error.ark` green; inherent `Type::from` not selected
      (`from_trait_not_inherent.ark`).

## Acceptance

- [x] ADR-039 D2 living path uses canonical trait id.
- [x] No `concat(target, "::from")` name invention in `try_from_conversion.ark`.
- [x] `verify lane` + question_mark fixtures pass.

## References

- ADR-039, RFC-004 §6, ADR-040 SignatureRegistry
- `src/compiler/corehir/semantic_trait_id.ark`
- `src/compiler/corehir/signature_registry_from.ark`
- `src/compiler/mir/lower/try_from_conversion.ark`
