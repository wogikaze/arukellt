---
Status: open
Created: 2026-06-26
Updated: 2026-06-26
ID: 696
Track: stdlib-api
Depends on: "688, 692"
Orchestration class: blocked-by-upstream
Orchestration upstream: "#688 trait dispatch, #692 Display/From"
Blocks v{N}: none
Priority: 3
Source: Stdlib abstraction gap audit 2026-06-26 — Rust parity comparison
---

# 696 — `Debug` trait and `format!` / `write!` formatting ecosystem

## Summary

Arukellt has a `Display` trait (`std::core::convert`) but no `Debug` trait, no
`format!` / `write!` macros, and `std::text::fmt` is a 55-line minimal stub.
Struct debugging is ad-hoc: users must hand-write `concat` chains. There is no
standard way to print a struct for diagnostics, and no `{:?}` equivalent.

Rust's `Debug` trait + `format!` / `write!` / `println!` formatting macros +
`fmt::Formatter` / `fmt::Arguments` provide a unified formatting layer used by
`assert_eq!` diagnostics, `{:?}` printing, and error messages.

## Current state

- `Display` trait with scalar impls (`std::core::convert`).
- `std::text::fmt` — 55-line stub.
- `std::text::builder` — `StringBuilder` utility.
- No `Debug`, no `format!`, no `Formatter`, no `Arguments`.
- `assert_eq!` in `std::test` compares values but cannot print them on
  failure (no `Debug`).

## Required work

- [ ] Define `trait Debug { fn fmt(self: Debug, f: Formatter) -> () }` in
      `std::fmt` (or `std::core::fmt`).
- [ ] Define a `Formatter` type and `fmt::Arguments` representation.
- [ ] Provide scalar `impl Debug` for all built-in types (mirroring `Display`
      for scalars, with `Debug`-specific output for strings — quoted).
- [ ] Provide a derive-style or convention-based `Debug` for structs (or
      document manual impl pattern).
- [ ] Implement `format!` / `write!` macros (or intrinsic-backed equivalents)
      consuming `Display` / `Debug`.
- [ ] Wire `assert_eq!` to print `Debug` on failure.
- [ ] Fixtures: `format!("{:?}", struct_value)` output; `assert_eq!` failure
      message showing both sides.
- [ ] Regenerate stdlib docs and manifest.
- [ ] `python3 scripts/manager.py verify quick` exits 0.

## Acceptance

- [ ] `Debug` trait defined with scalar impls.
- [ ] `format!` (or equivalent) produces a string from `Display` / `Debug`
      values.
- [ ] `assert_eq!` failure output includes `Debug` rendering of both values.
- [ ] `python3 scripts/manager.py verify quick` exits 0.

## References

- Depends on: #688 (trait dispatch), #692 (`Display` generic dispatch)
- `std/core/convert.ark`, `std/text/fmt.ark`, `std/text/builder.ark`,
  `std/test/mod.ark`
- Rust `std::fmt`: <https://doc.rust-lang.org/std/fmt/index.html>
