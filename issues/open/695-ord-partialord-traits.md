---
Status: open
Created: 2026-06-26
Updated: 2026-06-26
ID: 695
Track: stdlib-api
Depends on: 688
Orchestration class: blocked-by-upstream
Orchestration upstream: "#688 trait method dispatch"
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

- [ ] Define `trait Ord: Eq { fn cmp(self: Ord, other: Ord) -> Ordering }`
      and `trait PartialOrd: Ord { fn partial_cmp(...) -> Option<Ordering> }`
      in `std::core::cmp`.
- [ ] Provide scalar impls: `impl Ord for i32/i64/f64/char/String/bool`.
- [ ] Implement `sort_by<T: Ord>` / `binary_search_by<T: Ord>` on `Vec<T>`
      (or via `Iterator` consumers once #691 lands).
- [ ] Implement `cmp::Reverse<T>` wrapper.
- [ ] Implement `min_by` / `max_by` / `min_by_key` / `max_by_key` generic
      helpers.
- [ ] Fixtures: sort a `Vec<String>` via `Ord`; binary search by key.
- [ ] Regenerate stdlib docs and manifest.
- [ ] `python3 scripts/manager.py verify quick` exits 0.

## Acceptance

- [ ] `Ord` / `PartialOrd` traits defined with scalar impls.
- [ ] `sort_by` / `binary_search_by` work generically on `Vec<T: Ord>`.
- [ ] Fixture sorts a non-`i32` collection (e.g. `Vec<String>`) via the trait.
- [ ] `python3 scripts/manager.py verify quick` exits 0.

## References

- Depends on: #688 (trait dispatch), #691 (iterator consumers for sort_by)
- Blocks: #697 (Vec ops extension uses sort_by)
- `std/core/cmp.ark`, `std/seq/mod.ark`
- Rust `std::cmp`: <https://doc.rust-lang.org/std/cmp/index.html>
