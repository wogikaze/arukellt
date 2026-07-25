---
Status: open
Created: 2026-07-25
Updated: 2026-07-25
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

#690 accepted ADR-039 and shipped Option `?` plus Result `?` with From
conversion. The living MIR path still selects the conversion callee as the
mangled associated method `E_target::from` via `ctx_has_fn_name`, because
`SignatureEntry.trait_id` / canonical `SemanticTraitId::From` are not yet
wired for language-syntax desugaring (RFC-004 §6).

## Required work

- [ ] Populate `SignatureEntry.trait_id` (or equivalent) for `impl From<E>
      for T` methods during signature registry build.
- [ ] Change `try_resolve_from_conversion` to resolve by canonical
      `SemanticTraitId::From` + `(E_source, E_target)`, not by inventing
      `E_target::from` from type-name strings.
- [ ] Keep import-independence (language syntax desugaring).
- [ ] Extend From conversion on `wasm32-gc` if still skipped after trait-id
      wiring.
- [ ] Regression: `tests/fixtures/question_mark/from_error.ark` remains green.

## Acceptance

- [ ] ADR-039 D2 living implementation matches the canonical trait-id path.
- [ ] No name-invented `concat(target, "::from")` lookup remains in
      `try_from_conversion.ark`.
- [ ] `python3 scripts/manager.py verify lane` (and fixture gate for
      `question_mark/`) passes.

## References

- ADR-039, RFC-004 §6, ADR-040 SignatureRegistry
- `src/compiler/mir/lower/try_from_conversion.ark`
