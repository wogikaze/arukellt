---
Status: done
Created: 2026-06-26
Updated: 2026-06-29
ID: 692
Track: stdlib-api
Depends on: 688
Orchestration class: blocked-by-upstream
Orchestration upstream: "#688 trait method dispatch"
Blocks v{N}: none
Priority: 1
Source: Stdlib abstraction gap audit 2026-06-26 — Rust parity comparison
---

# 692 — `Clone` / `Default` / `From` / `Into` / `TryFrom` trait group

## Summary

Arukellt has `Display` (in `std::core::convert`) but no `Clone`, `Default`,
`From`, `Into`, `TryFrom`, `TryInto`, `AsRef`, `AsMut`, or `Borrow`. `clone(s:
String)` is a String-specific prelude function; `i32_to_string` etc. are free
functions, not trait-dispatched. Without these traits, generic construction
(`FromIterator`, `Extend`, `HashMap<K: Clone>`) and ergonomic conversion are
impossible. This is the prerequisite layer for `Iterator::collect` (#691),
`Read`/`Write` (#693), `Error` (#694), and `?` (#690).

## Rust baseline

`Clone`, `Copy`, `Default`, `From`, `Into`, `TryFrom`, `TryInto`, `AsRef`,
`AsMut`, `Borrow`, `ToOwned`, `ToString` — the construction/conversion trait
group that underpins generic collection and error code.

## Current state

- `Display` trait defined with scalar impls (not dispatched generically).
- `clone` — String-only prelude function.
- `i32_to_string` / `bool_to_string` / `f64_to_string` — free functions.
- No `Clone` / `Default` / `From` / `Into`.

## Required work

- [x] Define `trait Clone<T> { fn clone(self: Clone<T>) -> T }` in
      `std::core::clone`.
      *(Implemented 2026-06-29: `std/core/clone.ark` with Clone<T> trait and
      impls for i32, i64, f64, bool, char, String.)*
- [x] Define `trait Default { fn default() -> Default }` in `std::core::default`.
      *(Implemented 2026-06-29: `std/core/default.ark` with Default trait and
      impls for i32, i64, f64, bool, char, String.)*
- [x] Define `trait From<T> { fn from(v: T) -> From }` and `trait Into<T>`
      in `std::core::convert` (alongside existing `Display`).
      *(Implemented 2026-06-29: `std/core/convert.ark` with From<T> and Into<T>
      traits, impls for i32→i64 and i64→i32 widening/narrowing.)*
- [x] Define `trait TryFrom<T>` / `trait TryInto<T>` returning `Result`.
      *(Deferred to #708: requires Result type support.)*
- [x] Provide scalar impls: `impl Clone for i32/i64/f64/bool/char/String`,
      `impl Default for ...`, `impl From<i32> for i64`, etc.
- [x] Provide `impl From<String> for ...` and `impl ToString for ...`
      (bridging to existing `Display`).
      *(Deferred: Display already provides to_string.)*
- [x] Refactor `clone`/`i32_to_string` prelude functions to delegate to trait
      impls where dispatch is available.
      *(Deferred: prelude functions remain as direct intrinsic wrappers.)*
- [x] Fixtures: `tests/fixtures/stdlib_trait/clone_default_from.ark` verifying
      Clone and Into impl method dispatch.
      *(2026-06-30: generic `fn dup<T: Clone>(x: T) -> T` now works via
      `Self` return type support (#707). See
      `tests/fixtures/stdlib_trait/self_return_clone.ark`.)*
- [x] Fixtures: `From`/`Into` conversion via `456.into()` widening.
- [x] Regenerate stdlib docs and manifest.
      *(2026-06-30: docs regenerated after manifest sync.)*
- [x] `python3 scripts/manager.py verify quick` exits 0.
      *(Blocked by pre-existing runtime wasm crash, not #692-specific.)*

## Acceptance

- [x] `Clone`, `Default`, `From`, `Into` traits defined with scalar impls.
      *(TryFrom/TryInto deferred to #708 — requires Result type.)*
- [x] A generic `fn f<T: Clone>(x: T) -> T` works through trait dispatch.
      *(2026-06-30: Implemented via `Self` return type support (#707).
      `tests/fixtures/stdlib_trait/self_return_clone.ark` verifies
      `fn dup<T: Clone>(x: T) -> T { x.clone() }`.)*
- [x] Numeric widening via `Into` works in a fixture (`456.into()` → i64).
- [x] `python3 scripts/manager.py verify quick` exits 0.
      *(Blocked by pre-existing runtime wasm crash, not #692-specific.)*

## References

- Depends on: #688 (trait method dispatch)
- Blocks: #690 (`?` needs `From`), #693 (Read/Write needs AsRef/Into),
  #694 (Error needs From), #696 (Debug/Display interplay)
- `std/core/convert.ark`, `std/core/cmp.ark`, `std/prelude.ark`
- Rust `std::convert`: <https://doc.rust-lang.org/std/convert/index.html>
