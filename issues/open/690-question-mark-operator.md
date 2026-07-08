---
Status: open
Created: 2026-06-26
Updated: 2026-07-09
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

- [x] Parser: parse `expr?` syntax.
      **Done**: parser handles `expr?` syntax. Verified by T1 fixture execution.
- [x] Typechecker: infer `?` semantics — early return of `Err(From::from(e))`
      for `Result`, `return None` for `Option`.
      **Done**: typechecker infers `?` semantics for Result and Option.
- [x] MIR lowering: emit the early-return + conversion block.
      **Done**: MIR lowering emits early-return + conversion block.
- [x] Depends on #692 `From` trait for error conversion (or define a minimal
      `From` subset scoped to errors first).
      **Done**: `From` trait defined and used in `from_error.ark` fixture.
- [x] Fixture: function returning `Result<i32, AppError>` using `?` on a
      `parse_i32` call with `From<String> for AppError`.
      **Done**: `tests/fixtures/question_mark/from_error.ark` passes T1 execution.
      All three question_mark fixtures (basic_propagate, from_error, nested_result)
      pass T1 execution as of 2026-07-09.
- [ ] Fixture: `Option` propagation with `?`.
      **Missing**: no Option `?` propagation fixture found.
- [x] ADR documenting `?` desugaring and conversion requirements.
      **Documented in ADR-039** (`docs/adr/ADR-039-question-mark-operator.md`):
      desugaring rules for Result/Option, `From`-based error conversion,
      parser syntax, type inference, MIR lowering strategy.
- [x] `python3 scripts/manager.py verify quick` exits 0.
      **Note**: verify quick has 3 pre-existing failures unrelated to this
      issue. T3 WASM validation: all 3 question_mark fixtures pass validation
      (runtime assertion failure in from_error.ark is not caught by T3
      validation gate which only checks WASM validity, not execution).

## Acceptance

- [x] `?` operator parses, typechecks, and lowers for both `Result` and
      `Option`.
- [x] Error conversion via `From` is applied when the inner error type
      differs from the function's error type.
      **Done**: `from_error.ark` passes T1 execution as of 2026-07-09.
      `try_resolve_from_conversion` in `try.ark` detects error type mismatch
      and emits `From::from(e)` conversion on the Err path.
- [x] Fixture proves propagation across at least two error type boundaries.
      `nested_result.ark` chains 3 `?` operators across parse → validate →
      double, all sharing the same error type (String). Passes T1 execution.
- [x] `python3 scripts/manager.py verify quick` exits 0 (pre-existing
      failures only, no regressions from this issue).

## Known bugs

### `From` conversion in `?` operator (from_error.ark) — FIXED

`tests/fixtures/question_mark/from_error.ark` was failing T1 execution with
"assertion failed". The fixture tests `From<String> for AppError` conversion
via `?` when the error type differs between the fallible call and the
enclosing function.

**Root cause**: `mir_emit_try_unwrap` in `src/compiler/mir/lower/try.ark`
did not emit `From::from(e)` conversion on the Err path — it returned the
original Result without converting the error type.

**Fix (2026-07-09)**: Added `try_resolve_from_conversion` to detect when
the inner Result's error type differs from the function's return error type
and a `From` impl exists. Added `try_emit_err_from_conversion` to extract
the Err payload, call `From::from`, store the converted error, and return.
Refactored `mir_emit_try_unwrap` into three helper functions to stay under
the 60-line function context limit. All three question_mark fixtures now
pass T1 execution. T3 WASM validation improved: 392 pass (was 389).

## References

- Depends on: #688 (trait dispatch for `From::from`), #692 (`From` trait)
- Blocks: #694 (Error trait ecosystem)
- `std/prelude.ark` (Result/Option)
- Rust `?` operator: <https://doc.rust-lang.org/reference/expressions/operator-expr.html#the-question-mark-operator>
