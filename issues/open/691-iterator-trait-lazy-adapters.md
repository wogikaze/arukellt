---
Status: open
Created: 2026-06-26
Updated: 2026-06-29
ID: 691
Track: stdlib-api
Depends on: "688, 707"
Orchestration class: blocked-by-upstream
Orchestration upstream: "#688 trait method dispatch"
Blocks v{N}: none
Priority: 1
Source: Stdlib abstraction gap audit 2026-06-26 — Rust parity comparison
---

# 691 — `Iterator` trait, lazy adapters, and `FromIterator` / `collect`

## Summary

`std::seq` is currently a set of **eager, monomorphic** functions over
`Vec<i32>` (`map_i32_i32`, `filter_i32`, `fold_i32_i32`, `take_i32`, `skip_i32`,
`unique`, `seq_reverse`, ...). The module header explicitly states "Lazy
generic `Seq<T>` pipelines are deferred to a future release." There is no
`Iterator` trait, no lazy adapter types, no `collect` / `FromIterator`, and no
`IntoIterator`. The `for x in v` syntax works via a structural `next() ->
Option<T>` protocol (`src/compiler/mir/lower/loop_struct_iter.ark`) but is not
formalized as a trait.

This is the single highest-impact stdlib abstraction gap: every collection
transformation currently allocates intermediate `Vec`s and requires
type-specific function variants.

## Rust baseline

`Iterator` trait with `next`; adapter methods `map`/`filter`/`take`/`skip`/
`zip`/`enumerate`/`chain`/`flat_map`/`rev`/`peekable`; consumers `collect`/
`fold`/`sum`/`count`/`any`/`all`/`find`/`for_each`; `IntoIterator` and
`FromIterator` for `Vec<T>` / `HashMap` / `HashSet`.

## Current state

- `std::seq` — eager `Vec<i32>` helpers, monomorphic.
- `std::collections::vec.ark` — 30-line stub (`new_i32` only).
- `for x in v` — structural protocol, not a trait.
- No `Iterator` trait, no `IntoIterator`, no `FromIterator`.

## Required work

- [ ] Define `trait Iterator { fn next(self: Iterator) -> Option<T> }` in
      `std::core::iter` (or `std::iter`).
- [ ] Define `IntoIterator` and `FromIterator` traits.
- [ ] Implement `impl Iterator for Vec<T>` (or a `VecIter` adapter) and
      `impl IntoIterator for Vec<T>`.
- [ ] Implement lazy adapter types: `Map`/`Filter`/`Take`/`Skip`/`Zip`/
      `Enumerate`/`Chain` (structs holding the upstream iterator + closure).
- [ ] Implement `collect` / `FromIterator for Vec<T>`.
- [ ] Wire the existing `for x in v` structural protocol to the `Iterator`
      trait (or document the relationship).
- [ ] Deprecate `std::seq` monomorphic helpers in favor of `iter().map()...`.
- [ ] Fixtures: lazy pipeline `v.iter().map(f).filter(g).take(3).collect()`
      with no intermediate allocation observable.
- [ ] Fixtures: `for x in hashmap_iter` over collection iterators.
- [ ] Regenerate stdlib docs and manifest.
- [ ] `python3 scripts/manager.py verify quick` exits 0.

## Acceptance

- [ ] `Iterator` / `IntoIterator` / `FromIterator` traits defined and
      implemented for `Vec<T>`.
- [ ] At least `map` / `filter` / `take` / `skip` / `fold` / `collect` work
      generically over any `T` through trait dispatch.
- [ ] A single lazy pipeline fixture replaces the equivalent
      `std::seq::map_i32_i32` + `filter_i32` eager chain.
- [ ] `std::seq` monomorphic helpers marked deprecated with migration path.
- [ ] `python3 scripts/manager.py verify quick` exits 0.

## References

- Depends on: #688 (trait method dispatch)
- Blocks: #697 (Vec ops extension depends on iterator adapters)
- `std/seq/mod.ark`, `std/collections/vec.ark`
- `src/compiler/mir/lower/loop_struct_iter.ark`
- `tests/fixtures/iterator/custom_iterator.ark`
- Rust `std::iter`: <https://doc.rust-lang.org/std/iter/index.html>
