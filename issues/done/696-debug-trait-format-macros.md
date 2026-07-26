---
Status: done
Created: 2026-06-26
Updated: 2026-07-26
ID: 696
Track: stdlib-api
Depends on: none
Orchestration class: implementation
Orchestration upstream: none
Blocks v{N}: none
Priority: 3
Source: Stdlib abstraction gap audit 2026-06-26 — Rust parity comparison
---

# 696 — `Debug` trait and `format!` / `write!` equivalent formatting

## Summary

Arukellt has a `Display` trait (`std::core::convert`) and a `Debug` trait
with scalar impls. The `format!` / `write!` / `println!` / `assert_eq!`
functionality is **not** delivered as Rust-style `!` macros. Instead:

- generic `Display`/`Debug`-bounded formatting (`fmt::format_debug`,
  `fmt::write_debug_to`),
- f-string interpolation `f"value = {x}"` (`format!("{}", x)` equivalent),
- f-string debug specifier `f"{x:?}"` (`format!("{:?}", x)` equivalent),
- `std::test` typed `assert_eq_*` helpers and generic `assert_eq_debug`
  with `Debug` failure messages.

Struct debugging remains manual: users write `impl Debug for T`.

## Current state

- `Display` / `Debug` traits with scalar impls (`std::core::convert`).
- `std::text::fmt`: `format_debug`, `write_debug_to`, container helpers.
- F-string `f"{expr}"` and `f"{expr:?}"` (desugars to `.fmt_debug()`).
- `std::test::assert_eq_debug<T: Eq + Debug>` plus typed helpers with Debug
  failure text.
- Prelude bare `assert_eq` Ark body updated for Debug messages, but **call
  sites remain CoreOp-inlined** (see Residual). Prefer `test::assert_eq_*` /
  `test::assert_eq_debug`.

## Required work

- [x] Define `trait Debug { fn fmt_debug(self: Debug) -> String }` in
      `std::core::convert`.
- [x] Provide scalar `impl Debug` for all built-in types (mirroring `Display`
      for scalars, with `Debug`-specific output for strings — quoted).
- [x] Provide generic `fmt::format_debug` / `fmt::write_debug_to` functions.
- [x] Provide monomorphic `Debug` helpers for common container types
      (`Vec<T>`, `Option<T>`, `Result<T, E>`).
- [x] Document the manual `impl Debug for Struct` pattern in `std::core::convert`.
- [x] Extend the f-string parser to support a debug format specifier
      (`f"{x:?}"`) and desugar it to a `fmt_debug` method call (equivalent to
      `fmt::format_debug(x)` without requiring `fmt` in scope).
- [x] Add a generic `assert_eq_debug<T: Eq + Debug>` to `std::test`, and update
      typed `assert_eq_*` helpers to print `Debug` renderings on failure.
- [x] Add/extend fixtures for:
  - `fmt::format_debug` / `fmt::write_debug_to` on scalars and structs,
  - f-string debug specifier output,
  - assertion failure messages showing both `Debug` values.
- [x] Regenerate stdlib docs and manifest.
- [x] `python3 scripts/manager.py verify quick` exits 0.

## Acceptance

- [x] `Debug` trait defined with scalar impls.
- [x] `format!`-equivalent mechanism (`fmt::format_debug` and/or `f"{x:?}"`)
      produces a string from `Display` / `Debug` values.
- [x] `assert_eq` failure output includes `Debug` rendering of both values
      (`test::assert_eq_*` / `test::assert_eq_debug`; see Residual for bare
      prelude `assert_eq` call sites).
- [x] `python3 scripts/manager.py verify quick` exits 0.

## Residual (documented, not false-done)

Bare name `assert_eq` cannot host the generic API:

1. Call sites to bare `assert_eq` are still rewritten/inlined via CoreOp
   (`runtime.assert_eq`), so Debug text in the prelude Ark body is not what
   user call sites execute today.
2. A `std::test` generic named exactly `assert_eq` mis-lowers scalar
   monomorphizations (`structref` vs `i32`).

Shipped alternative that meets acceptance: `test::assert_eq_debug` + Debug
messages on typed `test::assert_eq_*`. Fixture evidence:
`stdlib_test/assert_eq_generic_fail.ark`, `stdlib_test/assert_eq_fail.ark`.

Follow-up (out of scope): retire CoreOp call-site rewrite for `assert_eq` and
rename/alias `assert_eq_debug` → `assert_eq` once lowering is safe.

## Evidence

- Commit: `92310c77` (implementation) + close tranche on this branch
- Fixtures: `string_interp/fstring_debug_specifier.ark`,
  `stdlib_test/assert_eq_generic.ark`, `stdlib_test/assert_eq_generic_fail.ark`,
  `stdlib_trait/format_debug_point.ark`, existing `stdlib_text/debug_format_*`
- Lane: `verify lane` PASS; close gate: `verify quick` PASS (147/147) on 2026-07-26

## References

- Was blocked by: #688 (trait dispatch, done), #692 (`Display`/`From`/`Into`, done)
- `std/core/convert.ark`, `std/text/fmt.ark`, `std/text/builder.ark`,
  `std/test/mod.ark`, `std/prelude.ark`
- `src/compiler/parser/fstring*.ark`, `src/compiler/parser/type_params.ark`
- ADR-044, ADR-046, ADR-039
- Rust `std::fmt`: <https://doc.rust-lang.org/std/fmt/index.html>
