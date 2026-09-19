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

## Progress receipt — 2026-09-20

- Commit `4ec1085c7` adds the associated-type projection path needed by the
  iterator redesign. `Iterator` now exposes `Item`; `IntoIterator` exposes
  `Item` and `IntoIter`; and the existing `FromIterator` surface is public in
  `std/core/iter.ark`.
- Generic calls and methods resolve nested projections such as
  `Option<T::Item>`, retain the concrete return type in monomorphized
  instances, and accept imported trait/type paths such as `iter::Iterator`.
- `tests/fixtures/stdlib_trait/iterator_vec_iter_next.ark` covers an imported
  generic `Iterator` function on both targets. `trait_core_v2.ark` covers the
  associated-type method path; its GC and linear executions produce `79`.
- Evidence: `selfhost build-compiler` PASS; `verify lane` PASS including the
  touched-code quality ratchet; `docs check` PASS; GC P2 component compile and
  run PASS; linear P1 Wasm validate and run PASS.
- Follow-up commit `c8707c13e` removes the duplicate inherent `next` methods
  from all current adapters. The trait implementation is now the sole stdlib
  `next` definition while the existing structural `for` lowering remains
  compatible on GC and linear-memory targets.
- Commit `099863a45` fixes `FromIterator<T>` so its source type is the trait
  parameter, registers `FromIterator<VecIter> for Vec<i32>`, and adds a
  `Vec::from_iter(iter::vec_iter(...))` fixture. GC component and linear Wasm
  compile, validation, and runtime checks pass.
- Test commit `18885e48e` extends the same fixture through
  `iter::vec_iter(...).into_iter()` and verifies the returned iterator on both
  target paths.

This is a partial implementation receipt, not a close receipt. Lazy adapter
methods, generic collection implementations, canonical `collect`, structural
`for` integration, and the existing `verify quick` backlog remain open. The
generic imported non-generic constructor inference gap is also intentionally
outside this unit; the regression fixture uses an explicit `iter::VecIter`
annotation until that inference path is addressed.
