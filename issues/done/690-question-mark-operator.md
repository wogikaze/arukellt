---
Status: done
Created: 2026-06-26
Updated: 2026-07-25
ID: 690
Track: language-design
Depends on: 688
Orchestration class: design-required
Orchestration upstream: None
Blocks v{N}: none
Priority: 2
Source: Stdlib abstraction gap audit 2026-06-26 — Rust parity comparison
---

# 690 — `?` operator and `From<E>` error conversion

## Summary

Arukellt has `Result<T, E>` and `Option<T>` in the prelude. This issue tracks
the `?` operator (Result + Option) and `From`-based error conversion for
heterogeneous `Err` types (ADR-039).

## Current state (2026-07-25)

- Parser / typecheck / MIR lowering for `expr?` are implemented.
- Result identity `?`, Result + `From` conversion, and Option `?` compile and
  execute (wasm32 + golden).
- `From` trait is available (#692).
- ADR-039 is **ACCEPTED**.
- Living gap: From callee selection still uses registered `E_target::from`
  mangling until `SemanticTraitId::From` / `SignatureEntry.trait_id` (#839).
- `arukellt run` WASI P2 adapter failures are #686 / #810, out of scope here.

## Required work

- [x] Parser: parse `expr?` syntax.
- [x] Typechecker: Result and Option `?` inference.
- [x] MIR lowering: early-return + From conversion block.
- [x] Depends on #692 `From` trait.
- [x] Fixture: `tests/fixtures/question_mark/from_error.ark` (Result + From).
- [x] Fixture: `tests/fixtures/question_mark/option_propagate.ark` (Option `?`).
- [x] ADR-039 ACCEPTED (Option `?` + From conversion decisions).
- [x] Docs: `docs/language/spec.md` §3.9, `error-handling.md`, guide mention.
- [x] Manifest: `run` / `t3-compile` / `t3-run` entries for `option_propagate`.

## Acceptance

- [x] `?` parses, typechecks, and lowers for both `Result` and `Option`.
- [x] Error conversion via `From` when inner error type differs
      (`from_error.ark`).
- [x] Propagation across multiple `?` sites (`nested_result.ark`).
- [x] Option propagation fixture with golden output.
- [x] ADR-039 ACCEPTED; normative docs updated.

## Follow-up

- #839 — resolve From conversion via canonical `SemanticTraitId::From`.

## References

- ADR-039, RFC-004 §6
- #688 / #692 / #694 / #839
- `src/compiler/mir/lower/try.ark`, `try_from_conversion.ark`
- Rust `?`: <https://doc.rust-lang.org/reference/expressions/operator-expr.html#the-question-mark-operator>
