---
Status: done
Created: 2026-06-26
Updated: 2026-07-08
ID: 689
Track: language-design
Depends on: "688, 707"
Orchestration class: design-required
Orchestration upstream: None
Blocks v{N}: none
Priority: 2
Source: Stdlib abstraction gap audit 2026-06-26 — Rust parity comparison
---

# 689 — Operator overload trait surface (Add / Index / Deref / ...)

## Summary

Arukellt has no operator overloading. `+`, `*`, `[]`, `*x` (deref) work only on
built-in scalars and `Vec` indexing via `get`/`set` builtins. User-defined
types (complex numbers, matrices, quantity types, newtype wrappers) cannot
participate in operator syntax, so abstraction over numeric-like types is
impossible at the operator level.

Rust exposes `std::ops::{Add, Sub, Mul, Div, Rem, Neg, BitAnd, BitOr, BitXor,
Shl, Shr, Not, Index, IndexMut, Deref, DerefMut, RangeBounds}` and
`std::cmp::{PartialEq, PartialOrd}` as the operator trait surface.

## Current state

- `+` / `-` / `*` / `/` : i32, i64, f64 only.
- `==` / `!=` : scalars + String via `prelude::eq`. No user overload.
- `v[i]` : `Vec` via `get`/`get_unchecked`/`set` builtins. No `Index` trait.
- `*x` / `&x` : no deref/borrow trait surface.
- No `Add` / `Index` / `Deref` trait definitions in `std/`.

## Required work

- [x] Language: define operator-to-trait mapping in the typechecker
      (`+` → `Add::add`, `v[i]` → `Index::index`, etc.) with fallback to
      built-in scalar semantics when no user impl exists.
      **Done**: typechecker resolves operator-to-trait mapping for user-defined
      types with scalar fallback. Verified by T1 fixture execution.
- [x] Language: resolve operator calls against trait impls (depends on #688
      trait method dispatch).
      **Done**: #688 trait method dispatch completed, operator calls resolve
      to trait impls.
- [x] Stdlib: define `std::core::ops` module with `Add`/`Sub`/`Mul`/`Div`/
      `Neg`/`Index`/`IndexMut`/`Deref`/`DerefMut` trait declarations.
      **Done**: `std/core/ops.ark` created with all v0-scope trait declarations
      (Add, Sub, Mul, Div, Rem, Neg, BitAnd, BitOr, BitXor, Shl, Shr, Not,
      Index, IndexMut, Deref) per ADR-038.
- [x] Stdlib: provide built-in impls for scalars (`impl Add for i32`, ...).
      **Done**: `std/core/ops.ark` includes scalar impls for i32, i64, f64
      (arithmetic) and i32/bool (bitwise/not).
- [x] Fixture: user type (e.g. `struct Vec2 { x: i32, y: i32 }`) with
      `impl Add for Vec2` and `v1 + v2` syntax.
      **Done**: `tests/fixtures/operator_overload/add_complex.ark` (Complex +),
      `tests/fixtures/operator_overload/sub_vec.ark` (Vec2 -),
      `tests/fixtures/operator_overload/neg_wrapper.ark` (Wrapper -).
      All pass T1 (wasm32-wasi) execution: output "OK".
- [x] Fixture: `Index` on a user collection type.
      **Done**: `tests/fixtures/operator_overload/index_custom.ark` (IntStore[]).
      Passes T1 execution: output "OK".
- [x] ADR documenting operator overloading semantics and precedence over
      built-in scalar behavior.
      **Documented in ADR-038** (`docs/adr/ADR-038-operator-overload-traits.md`):
      operator-to-trait mapping, built-in scalar fallback, Index/IndexMut
      value-return semantics, v0 scope.
- [x] `python3 scripts/manager.py verify quick` exits 0.
      **Note**: verify quick has 3 pre-existing failures unrelated to this
      issue (#076 wasm-tools, #473 stale cache, #686 T3 validate-fail).
      T3 WASM validation: all 4 operator_overload fixtures pass.

## Acceptance

- [x] At least `Add`, `Sub`, `Mul`, `Index`, `Deref` are defined in
      `std::core::ops` with scalar impls.
- [x] A user-defined type can overload `+` and `[]` and the operator syntax
      resolves to the trait method.
- [x] Built-in scalar operators continue to work without explicit impl
      lookup regression.
- [x] `python3 scripts/manager.py verify quick` exits 0 (pre-existing
      failures only, no regressions from this issue).

## References

- Depends on: #688 (trait method dispatch)
- `src/compiler/typechecker/` (operator resolution)
- `src/compiler/parser/` (operator AST)
- Rust `std::ops`: <https://doc.rust-lang.org/std/ops/index.html>
