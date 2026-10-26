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
      adapters over `Vec<T>` (via #691 `Iterator`). **Partial:** eager
      `Vec<Vec<T>>` helpers landed; lazy cursors are blocked by generic-struct
      return lowering on `wasm32-gc`.
- [x] Implement `retain<T>(v: Vec<T>, f: fn(T) -> bool)` (in-place filter).
- [x] Implement `truncate` / `resize` / `extend` / `append`.
- [x] Implement `drain` / `splice` (range removal / replacement).
- [ ] Implement `sort` / `sort_by` / `sort_by_key` (via #695 `Ord`).
      **Partial:** comparator / key forms (`sort_by`, `sort_by_key`, the
      `sort_unstable*` pair) landed; the no-argument `sort` is blocked by
      `Ord`-bounded `Vec<T>` impl lowering.
- [ ] Implement `dedup` / `dedup_by`. **Partial:** `dedup_by` landed with a
      comparator contract; `dedup` is blocked with the other `Ord` methods.
- [ ] Implement `binary_search_by` / `binary_search_by_key` (via #695).
      **Partial:** `binary_search_by_key` landed; `binary_search_by` is
      blocked with the other `Ord` methods.
- [x] Migrate `std::seq` `Vec<i32>` helpers to generic `Vec<T: Ord>` /
      `Vec<T>` versions, deprecate monomorphic variants. (already landed)
- [x] Fixtures covering each new operation.
- [x] Regenerate stdlib docs and manifest.
- [ ] `python3 scripts/manager.py verify quick` exits 0.

## Acceptance

- [x] `windows` / `chunks` / `retain` / `sort_by` / `drain` / `splice`
      available on `Vec<T>`. (`windows` / `chunks` are eager `Vec<Vec<T>>`
      helpers, not lazy cursors.)
- [ ] `sort_by` / `binary_search_by` work generically via `Ord` (#695).
      `sort_by` uses a comparator; `binary_search_by` is still blocked.
- [x] `std::seq` monomorphic helpers deprecated with generic replacements.
      (already landed before this wave)
- [ ] `python3 scripts/manager.py verify quick` exits 0.

## References

- Depends on: #691 (Iterator adapters for windows/chunks/drain),
  #695 (Ord for sort_by/binary_search_by)
- `std/collections/vec.ark`, `std/seq/mod.ark`, `std/prelude.ark`
- Rust `Vec`: <https://doc.rust-lang.org/std/vec/struct.Vec.html>

## Progress (wave/697-vec, 2026-10-26)

Landed in `std/collections/vec.ark` (module-gated by
`use std::collections::vec`):

- Eager `windows` / `chunks` / `chunks_exact` returning `Vec<Vec<T>>`.
- `retain`, `truncate`, `resize`, `extend`, `append`, `drain`, `splice`.
- `sort_by`, `sort_by_key`, `sort_unstable_by`, `sort_unstable_by_key`,
  `dedup_by`, `partition`, `is_sorted_by`, `binary_search_by_key`.
- `resize` / `extend` copy element handles and therefore do not require a
  `Clone` bound under the current value model.

Fixtures: `tests/fixtures/stdlib_vec/vec_windows_chunks.ark`,
`vec_in_place.ark`, `vec_algorithms.ark`, each with `.expected` and registered
as `run`, `t3-compile`, and `t3-run`. Verified on both `wasm32-gc` (compile,
`wasm-tools validate`, component run) and `--target wasm32` (compile, validate,
run), with stdout matching `.expected`.

Residual: the `Ord`-shortcut methods (`sort`, `is_sorted`, `dedup`,
`binary_search_by`, `select_nth`) are not shipped. Both an `Ord`-bounded
inherent `impl<T: Ord> Vec<T>` and a concrete `impl Vec<i32>` currently lower to
an unreachable body/call site in the selfhost compiler, so they need a compiler
fix rather than a stdlib workaround. Lazy window/chunk cursors are likewise
blocked: a `Vec<T>` method returning a generic cursor struct emits an
uninstantiated layout on `wasm32-gc`.
