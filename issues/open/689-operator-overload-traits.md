---
Status: open
Created: 2026-06-26
Updated: 2026-06-26
ID: 689
Track: language-design
Depends on: 688
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

- [ ] Language: define operator-to-trait mapping in the typechecker
      (`+` → `Add::add`, `v[i]` → `Index::index`, etc.) with fallback to
      built-in scalar semantics when no user impl exists.
- [ ] Language: resolve operator calls against trait impls (depends on #688
      trait method dispatch).
- [ ] Stdlib: define `std::core::ops` module with `Add`/`Sub`/`Mul`/`Div`/
      `Neg`/`Index`/`IndexMut`/`Deref`/`DerefMut` trait declarations.
- [ ] Stdlib: provide built-in impls for scalars (`impl Add for i32`, ...).
- [ ] Fixture: user type (e.g. `struct Vec2 { x: i32, y: i32 }`) with
      `impl Add for Vec2` and `v1 + v2` syntax.
- [ ] Fixture: `Index` on a user collection type.
- [ ] ADR documenting operator overloading semantics and precedence over
      built-in scalar behavior.
- [ ] `python3 scripts/manager.py verify quick` exits 0.

## Acceptance

- [ ] At least `Add`, `Sub`, `Mul`, `Index`, `Deref` are defined in
      `std::core::ops` with scalar impls.
- [ ] A user-defined type can overload `+` and `[]` and the operator syntax
      resolves to the trait method.
- [ ] Built-in scalar operators continue to work without explicit impl
      lookup regression.
- [ ] `python3 scripts/manager.py verify quick` exits 0.

## References

- Depends on: #688 (trait method dispatch)
- `src/compiler/typechecker/` (operator resolution)
- `src/compiler/parser/` (operator AST)
- Rust `std::ops`: <https://doc.rust-lang.org/std/ops/index.html>
