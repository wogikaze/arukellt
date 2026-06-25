---
Status: open
Created: 2026-06-26
Updated: 2026-06-26
ID: 697
Track: stdlib-api
Depends on: "691, 695"
Orchestration class: blocked-by-upstream
Orchestration upstream: "#691 Iterator, #695 Ord"
Blocks v{N}: none
Priority: 3
Source: Stdlib abstraction gap audit 2026-06-26 — Rust parity comparison
---

# 697 — `Vec<T>` operation extension (windows / chunks / retain / sort_by / drain / splice)

## Summary

`std::collections::vec.ark` is a 30-line stub exposing only `new_i32`. The
real `Vec` surface is split between prelude intrinsics (`push`, `pop`, `get`,
`set`, `len`) and `std::seq` eager helpers. Compared to Rust's `Vec<T>`,
Arukellt is missing `windows` / `chunks` / `split` / `retain` / `truncate` /
`resize` / `extend` / `drain` / `splice` / `sort` / `sort_by` /
`binary_search_by` / `dedup_by` / `into_iter` / `from_iter`.

Once `Iterator` (#691) and `Ord` (#695) land, most of these become composable;
this issue tracks the remaining `Vec`-specific surface that needs direct
methods or intrinsics.

## Current state

- `std::collections::vec.ark` — `new_i32` only.
- Prelude: `push` / `pop` / `get` / `set` / `len` / `get_unchecked`.
- `std::seq` — `unique`, `seq_reverse`, `count_eq`, `binary_search`,
  `min_i32`, `max_i32`, `sum_i32` (all `Vec<i32>`-specific).
- No `windows` / `chunks` / `retain` / `truncate` / `resize` / `extend` /
  `drain` / `splice` / `sort_by` / `dedup_by`.

## Required work

- [ ] Implement `windows(n)` / `chunks(n)` / `chunk_exact` as iterator
      adapters over `Vec<T>` (via #691 `Iterator`).
- [ ] Implement `retain<T>(v: Vec<T>, f: fn(T) -> bool)` (in-place filter).
- [ ] Implement `truncate` / `resize` / `extend` / `append`.
- [ ] Implement `drain` / `splice` (range removal / replacement).
- [ ] Implement `sort` / `sort_by` / `sort_by_key` (via #695 `Ord`).
- [ ] Implement `dedup` / `dedup_by`.
- [ ] Implement `binary_search_by` / `binary_search_by_key` (via #695).
- [ ] Migrate `std::seq` `Vec<i32>` helpers to generic `Vec<T: Ord>` /
      `Vec<T>` versions, deprecate monomorphic variants.
- [ ] Fixtures covering each new operation.
- [ ] Regenerate stdlib docs and manifest.
- [ ] `python3 scripts/manager.py verify quick` exits 0.

## Acceptance

- [ ] `windows` / `chunks` / `retain` / `sort_by` / `drain` / `splice`
      available on `Vec<T>`.
- [ ] `sort_by` / `binary_search_by` work generically via `Ord` (#695).
- [ ] `std::seq` monomorphic helpers deprecated with generic replacements.
- [ ] `python3 scripts/manager.py verify quick` exits 0.

## References

- Depends on: #691 (Iterator adapters for windows/chunks/drain),
  #695 (Ord for sort_by/binary_search_by)
- `std/collections/vec.ark`, `std/seq/mod.ark`, `std/prelude.ark`
- Rust `Vec`: <https://doc.rust-lang.org/std/vec/struct.Vec.html>
