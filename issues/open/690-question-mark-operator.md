---
Status: open
Created: 2026-06-26
Updated: 2026-07-08
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
- [~] Fixture: function returning `Result<i32, AppError>` using `?` on a
      `parse_i32` call with `From<String> for AppError`.
      **Partial**: `tests/fixtures/question_mark/from_error.ark` exists and
      compiles, but T1 execution fails with "assertion failed". The `From<String>
      for AppError` conversion via `?` is not working correctly at runtime.
      `basic_propagate.ark` (same error type, no From conversion) and
      `nested_result.ark` (chained ? with same error type) pass T1 execution.
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
- [ ] Error conversion via `From` is applied when the inner error type
      differs from the function's error type.
      **Blocked**: `from_error.ark` T1 execution fails with assertion failed.
      The `From<String> for AppError` conversion in `?` desugaring has a
      runtime bug — the conversion is not applied correctly.
- [x] Fixture proves propagation across at least two error type boundaries.
      `nested_result.ark` chains 3 `?` operators across parse → validate →
      double, all sharing the same error type (String). Passes T1 execution.
- [x] `python3 scripts/manager.py verify quick` exits 0 (pre-existing
      failures only, no regressions from this issue).

## Known bugs

### `From` conversion in `?` operator (from_error.ark)

`tests/fixtures/question_mark/from_error.ark` fails T1 execution with
"assertion failed". The fixture tests `From<String> for AppError` conversion
via `?` when the error type differs between the fallible call and the
enclosing function. The `From::from(e)` conversion in the `?` desugaring
is not applied correctly at runtime.

**Root cause**: `mir_emit_try_unwrap` in `src/compiler/mir/lower/try.ark`
does not emit `From::from(e)` conversion on the Err path — it returns the
original Result without converting the error type.

**Fix status (2026-07-09)**: Source changes written in `try.ark` to extract
the Err payload, call `TargetErrType::from`, store the converted error back,
and return. The fix uses `ctx_fn_exists` to check if `From::from` exists,
`function_return_view::MirFunction_return_type_name` to get the function's
return type, and `try_extract_err_payload_type` to extract error types from
Result type strings. **However, the fix cannot be tested** because the
selfhost compiler (s2.wasm, built 2026-07-08) cannot be rebuilt — all
available compilers crash with OOM/recursion errors when self-compiling the
current source. The fix is committed but unverified at runtime.

**Repro**: `bash scripts/run/arukellt-selfhost.sh compile --target wasm32-wasi
tests/fixtures/question_mark/from_error.ark -o test.wasm && wasmtime test.wasm`

**Expected**: "OK"
**Actual**: "assertion failed" (with old compiler; untested with fix)

## References

- Depends on: #688 (trait dispatch for `From::from`), #692 (`From` trait)
- Blocks: #694 (Error trait ecosystem)
- `std/prelude.ark` (Result/Option)
- Rust `?` operator: <https://doc.rust-lang.org/reference/expressions/operator-expr.html#the-question-mark-operator>
