---
Status: open
Created: 2026-06-26
Updated: 2026-06-26
ID: 694
Track: stdlib-api
Depends on: "690, 692"
Orchestration class: blocked-by-upstream
Orchestration upstream: "#690 ? operator + From, #692 From trait"
Blocks v{N}: none
Priority: 2
Source: Stdlib abstraction gap audit 2026-06-26 — Rust parity comparison
---

# 694 — `Error` trait and unified error type ecosystem

## Summary

Arukellt error types are fragmented: `parse_i32` returns `Result<i32, String>`,
`std::io` returns `Result<_, IoError>`, JSON/TOML/CSV each have their own
error shape, and there is no unifying `Error` trait or `From<E>` conversion.
Combined with the missing `?` operator (#690), error propagation across
library boundaries requires manual `match` and ad-hoc conversion at every
call site.

Rust's `std::error::Error` trait + `From<E1> for E2` + `?` provides a uniform
error ecosystem, enabling `anyhow`-style aggregation and library
interoperability.

## Current state

- `IoError` enum in `std::io` (concrete).
- `Result<i32, String>` from parse functions.
- No `Error` trait, no `Display`-for-error requirement, no `source()` chaining.
- No `From<String> for IoError` or vice versa.

## Required work

- [ ] Define `trait Error: Display { fn source(self: Error) -> Option<Error> }`
      in `std::error` (or `std::core::error`).
- [ ] Require `Display` as a supertrait (depends on existing `Display` trait
      becoming generically dispatched — #688).
- [ ] Implement `impl Error for IoError`, `impl Error for String` (or a
      wrapper), and per-library error types.
- [ ] Implement `From` conversions between error types (depends on #692).
- [ ] Document a recommended top-level application error pattern (enum
      wrapping library errors, like Rust's `anyhow::Error` / `thiserror`).
- [ ] Fixtures: a function returning `Result<T, AppError>` that uses `?` to
      propagate `IoError` and `parse` errors via `From`.
- [ ] Regenerate stdlib docs and manifest.
- [ ] `python3 scripts/manager.py verify quick` exits 0.

## Acceptance

- [ ] `Error` trait defined with `Display` supertrait and `source` method.
- [ ] At least `IoError` and one parse error type implement `Error`.
- [ ] A fixture demonstrates cross-library error propagation via `?` + `From`.
- [ ] `python3 scripts/manager.py verify quick` exits 0.

## References

- Depends on: #690 (`?` operator), #692 (`From` trait), #688 (trait dispatch)
- `std/core/error.ark`, `std/io/mod.ark`, `std/core/convert.ark`
- Rust `std::error`: <https://doc.rust-lang.org/std/error/index.html>
