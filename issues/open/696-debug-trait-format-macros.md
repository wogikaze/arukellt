---
Status: open
Created: 2026-06-26
Updated: 2026-07-25
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

Arukellt has a `Display` trait (`std::core::convert`) and now a `Debug` trait
with scalar impls. The `format!` / `write!` / `println!` / `assert_eq!`
functionality is **not** going to be delivered as Rust-style `!` macros.
Instead, the language uses:

- generic `Display`/`Debug`-bounded formatting functions (`fmt::format_debug`,
  `fmt::write_debug_to`),
- f-string interpolation `f"value = {x}"` as the `format!("value = {}", x)`
  equivalent,
- and, in the future, an f-string debug format specifier such as `f"{x:?}"` for
  the `format!("{:?}", x)` equivalent.

This avoids adding a macro system, token trees, macro hygiene, and macro
expansion pass to the compiler, all of which conflict with the current
trait-first / method-first direction (ADR-044, ADR-046) and are unnecessary for
the user-visible behavior.

Struct debugging is still manual: users write an `impl Debug for T` block. A
conventional pattern is documented in `std::core::convert`.

## Current state

- `Display` and `Debug` traits with scalar impls (`std::core::convert`).
- `std::text::fmt` provides:
  - `format_debug<T: Debug>(value: T) -> String`
  - `write_debug_to<T: Debug>(buf: String, value: T) -> String`
  - monomorphic container helpers (`debug_format_vec_*`,
    `debug_format_option_*`, `debug_format_result_*`)
- `std::text::builder` — `StringBuilder` utility.
- F-string interpolation `f"... {expr} ..."` exists and desugars to
  `to_string(expr)`/`concat(...)` calls (`src/compiler/parser/fstring*.ark`).
- No f-string debug format specifier (`{expr:?}`) yet.
- No generic `assert_eq<T: Eq + Debug>` yet; `std::test` still exposes typed
  `assert_eq_*` helpers that do not print `Debug` representations.
- No `Formatter` / `Arguments` types — these are not required without macro
  expansion.

## Required work

- [x] Define `trait Debug { fn fmt_debug(self: Debug) -> String }` in
      `std::core::convert`.
- [x] Provide scalar `impl Debug` for all built-in types (mirroring `Display`
      for scalars, with `Debug`-specific output for strings — quoted).
- [x] Provide generic `fmt::format_debug` / `fmt::write_debug_to` functions.
- [x] Provide monomorphic `Debug` helpers for common container types
      (`Vec<T>`, `Option<T>`, `Result<T, E>`).
- [x] Document the manual `impl Debug for Struct` pattern in `std::core::convert`.
- [ ] Extend the f-string parser to support a debug format specifier
      (`f"{x:?}"`) and desugar it to `fmt::format_debug(x)`.
- [ ] Add a generic `assert_eq<T: Eq + Debug>(actual: T, expected: T)` to
      `std::test`, or update typed `assert_eq_*` helpers to print `Debug`
      renderings on failure.
- [ ] Add/extend fixtures for:
  - `fmt::format_debug` / `fmt::write_debug_to` on scalars and structs,
  - f-string debug specifier output,
  - assertion failure messages showing both `Debug` values.
- [ ] Regenerate stdlib docs and manifest.
- [ ] `python3 scripts/manager.py verify quick` exits 0.

## Acceptance

- [x] `Debug` trait defined with scalar impls.
- [ ] `format!`-equivalent mechanism (`fmt::format_debug` and/or `f"{x:?}"`)
      produces a string from `Display` / `Debug` values.
- [ ] `assert_eq` failure output includes `Debug` rendering of both values.
- [ ] `python3 scripts/manager.py verify quick` exits 0.

## References

- Was blocked by: #688 (trait dispatch, done), #692 (`Display`/`From`/`Into`, done)
- `std/core/convert.ark`, `std/text/fmt.ark`, `std/text/builder.ark`,
  `std/test/mod.ark`
- `src/compiler/parser/fstring.ark`,
  `src/compiler/parser/fstring_segments.ark`,
  `src/compiler/parser/fstring_concat.ark`,
  `src/compiler/parser/fstring_nodes.ark`
- ADR-044 trait method syntax, ADR-046 free-function eradication,
  ADR-039 `?` operator (rejected `try!` macro because macros are not ready)
- Rust `std::fmt`: <https://doc.rust-lang.org/std/fmt/index.html>
