---
Status: open
Created: 2026-06-26
Updated: 2026-06-26
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

Arukellt has `Result<T, E>` and `Option<T>` in the prelude, but no `?`
operator and no `From<E1> for E2` conversion trait. Error types are
fragmented: `parse_i32` returns `Result<i32, String>`, `std::io` returns
`Result<_, IoError>`, and there is no way to propagate an error while
converting between error types in one expression.

Rust's `?` operator desugars to `match expr { Ok(v) => v, Err(e) => return
Err(From::from(e)) }`, relying on the `From` trait for error conversion. This
is the foundation of ergonomic error handling and the `Error` trait ecosystem
(#694).

## Current state

- `Result` / `Option` exist; `Ok` / `Err` / `Some` / `None` constructors work.
- No `?` operator in the parser or typechecker.
- No `From` / `Into` trait (tracked in #692).
- Manual `match` propagation is required everywhere.

## Required work

- [ ] Parser: parse `expr?` syntax.
- [ ] Typechecker: infer `?` semantics — early return of `Err(From::from(e))`
      for `Result`, `return None` for `Option`.
- [ ] MIR lowering: emit the early-return + conversion block.
- [ ] Depends on #692 `From` trait for error conversion (or define a minimal
      `From` subset scoped to errors first).
- [ ] Fixture: function returning `Result<i32, AppError>` using `?` on a
      `parse_i32` call with `From<String> for AppError`.
- [ ] Fixture: `Option` propagation with `?`.
- [x] ADR documenting `?` desugaring and conversion requirements.
      **Documented in ADR-039** (`docs/adr/ADR-039-question-mark-operator.md`):
      desugaring rules for Result/Option, `From`-based error conversion,
      parser syntax, type inference, MIR lowering strategy.
- [ ] `python3 scripts/manager.py verify quick` exits 0.
      **Blocked**: pinned bootstrap wasm refresh required (same as #688).

## Acceptance

- [ ] `?` operator parses, typechecks, and lowers for both `Result` and
      `Option`.
- [ ] Error conversion via `From` is applied when the inner error type
      differs from the function's error type.
- [ ] Fixture proves propagation across at least two error type boundaries.
- [ ] `python3 scripts/manager.py verify quick` exits 0.

## References

- Depends on: #688 (trait dispatch for `From::from`), #692 (`From` trait)
- Blocks: #694 (Error trait ecosystem)
- `std/prelude.ark` (Result/Option)
- Rust `?` operator: <https://doc.rust-lang.org/reference/expressions/operator-expr.html#the-question-mark-operator>
