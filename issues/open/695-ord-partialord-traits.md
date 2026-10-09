---
Status: open
Created: 2026-06-26
Updated: 2026-10-10
ID: 695
Track: stdlib-api
Depends on: "908"
Orchestration class: blocked-by-upstream
Orchestration upstream: "#908 generic mono-instance emit mis-bind"
Blocks v{N}: none
Priority: 2
Source: Stdlib abstraction gap audit 2026-06-26 — Rust parity comparison
---

# 695 — `Ord` / `PartialOrd` traits and comparison-based algorithms

## Summary

Arukellt has `Ordering` and `cmp_i32` in `std::core`, and an `Eq` trait in
`std::core::cmp`, but no `Ord` / `PartialOrd` trait. Sorting, binary search
by key, ordered maps, and min/max-by-key all currently hardcode `i32`
comparison. `std::seq::binary_search` and `std::seq::min_i32` / `max_i32` are
`Vec<i32>`-specific.

Rust's `Ord` / `PartialOrd` traits underpin `sort`, `sort_by`,
`binary_search_by`, `BTreeMap`, `min`/`max` on iterators, and `cmp::Reverse`.

## Current state

- `Eq` trait defined (`std::core::cmp`), scalar impls present.
- `Ordering` enum, `cmp_i32` / `min` / `max` / `clamp` free functions.
- No `Ord` / `PartialOrd` trait.
- `std::seq::binary_search` / `min_i32` / `max_i32` — `Vec<i32>` only.

## Required work

- [x] Define `trait Ord { fn cmp(self: Ord, other: Ord) -> Ordering }`
      and `trait PartialOrd { fn partial_cmp(...) -> Option<Ordering> }`
      in `std::core::cmp` (supertrait bounds are not part of the trait
      syntax; Ord/PartialOrd stand alone).
- [x] Provide scalar impls: `impl Ord for i32/i64/f64/char/String/bool`.
- [x] Implement `sort_by<T: Ord>` / `binary_search_by<T: Ord>` on `Vec<T>`
      (or via `Iterator` consumers once #691 lands).
- [x] Implement `cmp::Reverse<T>` wrapper.
- [x] Implement `min_by` / `max_by` / `min_by_key` / `max_by_key` generic
      helpers.
- [x] Fixtures: sort a `Vec<String>` via `Ord` (`ord_sort_vec_string.ark`);
      binary search by key (`seq_ord_generic_algorithms.ark`).
- [x] Regenerate stdlib docs and manifest.
- [x] `python3 scripts/manager.py verify quick` exits 0.

## Status note (2026-10-10 close audit)

The trait surface and generic algorithms are fully landed and verified:
`Ord`/`PartialOrd` traits with scalar impls and `Reverse<T>` in
`std/core/cmp.ark`; generic `sort_by`/`binary_search_by`/
`binary_search_by_key`/`lower_bound`/`upper_bound`/`min_by`/
`max_by`/`min_by_key`/`max_by_key` in `std/seq/mod.ark`; fixtures
`ord_sort_*`, `partial_ord_*`, `seq_ord_generic_algorithms`, and new
`ord_sort_vec_string` (Vec<String> + Vec<Reverse<i32>>, proven on
wasm32 and wasm32-gc).

One acceptance item stays open: calls needing a **second** instantiation
of the same generic fn mis-bind to the `__i32` instance —
`binary_search_by(Vec<String>)` and `min_by(Vec<String>)` produce
invalid wasm32-gc and silently miscompile on wasm32. Filed as #908;
this issue is blocked on it.

## Acceptance

- [x] `Ord` / `PartialOrd` traits defined with scalar impls.
- [ ] `sort_by` / `binary_search_by` work generically on `Vec<T: Ord>`
      — sort_by proven on Vec<String>/Vec<Reverse<i32>> on both targets;
      binary_search_by/min_by at T=String blocked by #908.
- [x] Fixture sorts a non-`i32` collection (e.g. `Vec<String>`) via the trait.
- [x] `python3 scripts/manager.py verify quick` exits 0.

## References

- Depends on: #688 (trait dispatch), #691 (iterator consumers for sort_by)
- Blocks: #697 (Vec ops extension uses sort_by)
- `std/core/cmp.ark`, `std/seq/mod.ark`
- Rust `std::cmp`: <https://doc.rust-lang.org/std/cmp/index.html>
