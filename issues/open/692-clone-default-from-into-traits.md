---
Status: open
Created: 2026-06-26
Updated: 2026-06-26
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

- [ ] Define `trait Clone { fn clone(self: Clone) -> Self }` in
      `std::core::clone`.
- [ ] Define `trait Default { fn default() -> Default }` in `std::core::default`.
- [ ] Define `trait From<T> { fn from(v: T) -> From }` and `trait Into<T>`
      in `std::core::convert` (alongside existing `Display`).
- [ ] Define `trait TryFrom<T>` / `trait TryInto<T>` returning `Result`.
- [ ] Provide scalar impls: `impl Clone for i32/i64/f64/bool/char/String`,
      `impl Default for ...`, `impl From<i32> for i64`, etc.
- [ ] Provide `impl From<String> for ...` and `impl ToString for ...`
      (bridging to existing `Display`).
- [ ] Refactor `clone`/`i32_to_string` prelude functions to delegate to trait
      impls where dispatch is available.
- [ ] Fixtures: generic `fn dup<T: Clone>(x: T) -> T` returning `x.clone()`.
- [ ] Fixtures: `From` conversion chain across numeric types.
- [ ] Regenerate stdlib docs and manifest.
- [ ] `python3 scripts/manager.py verify quick` exits 0.

## Acceptance

- [ ] `Clone`, `Default`, `From`, `Into`, `TryFrom`, `TryInto` traits defined
      with scalar impls.
- [ ] A generic `fn f<T: Clone>(x: T) -> T` works through trait dispatch.
- [ ] Numeric widening via `From` works in a fixture.
- [ ] `python3 scripts/manager.py verify quick` exits 0.

## References

- Depends on: #688 (trait method dispatch)
- Blocks: #690 (`?` needs `From`), #693 (Read/Write needs AsRef/Into),
  #694 (Error needs From), #696 (Debug/Display interplay)
- `std/core/convert.ark`, `std/core/cmp.ark`, `std/prelude.ark`
- Rust `std::convert`: <https://doc.rust-lang.org/std/convert/index.html>
