---
Status: open
Created: 2026-06-26
Updated: 2026-09-20
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
- Commit `9a1241bcc` preserves concrete signatures for qualified imported
  non-generic calls. The iterator fixture now leaves `iter::vec_iter(...)`
  unannotated before passing it to a generic `Iterator` function, covering the
  module-alias inference path on GC and linear-memory targets.

This is a partial implementation receipt, not a close receipt. Lazy adapter
methods, generic collection implementations, structural `for` integration,
and the existing `verify quick` backlog remain open. The canonical
`Iterator::collect` path is now present for the supported `i32` item lane:
`collect_iterator_i32<T: Iterator>` owns the traversal, every current adapter
impl delegates to it, and the legacy `collect_*` helpers plus
`FromIterator<VecIter>` delegate through the trait method. The fixture's
generic `collect_values<T: Iterator>` proves this path on GC and linear Wasm.

The `Vec<T::Item>` generalization is intentionally not enabled: the current
aggregate emitter still produces invalid Wasm for that projection. #860 owns
removal of this boundary once generic aggregate projection lowering validates
on both targets. The generic imported non-generic constructor inference gap is
also intentionally outside this unit; the regression fixture now exercises
the unannotated qualified constructor path that is supported.

The existing linear-memory function-reference ABI gap for callback-bearing
adapters remains outside this receipt (#807); this unit does not claim the
full adapter fixture matrix as a fallback parity gate.

## Progress receipt — 2026-09-20 (canonical collect)

- Commit `cfd072391` adds `Iterator::collect(self) -> Vec<i32>` and routes
  the adapter-specific collection helpers and `FromIterator<VecIter>` through
  that canonical trait method. Commit `fde82c993` records the generic
  aggregate-lowering removal condition in the source boundary comment and
  updates the formatter baseline.
- `tests/fixtures/stdlib_trait/iterator_vec_iter_next.ark` adds a generic
  `collect_values<T: iter::Iterator>` caller. It compiles and runs on
  `wasm32-gc` as a component and on `wasm32` after Wasm validation, with both
  runtimes passing.
- The current implementation deliberately returns `Vec<i32>`; `Vec<T::Item>`
  remains a tracked follow-up under #860 because the existing aggregate ABI is
  not yet valid for that projection.

## Progress receipt — 2026-09-20 (iterator consumers)

- Commit `d901883d0` adds canonical `Iterator::count` and `Iterator::sum`
  methods for the current `i32` item lane. Their generic traversals are shared
  by the current adapter implementations, with no callback or projected Vec
  ABI involved.
- The same iterator fixture calls both consumers through generic
  `count_values<T: iter::Iterator>` and `sum_values<T: iter::Iterator>`
  functions. GC component execution and linear Wasm validation/runtime pass.
- `fold` remains open: its generic callback path still exposes the existing
  function-reference ABI gap, so this receipt does not claim the full consumer
  acceptance condition.

## Progress receipt — 2026-09-20 (Vec IntoIterator)

- Commit `30a5ec717` adds `IntoIterator for Vec<i32>`, producing the canonical
  `VecIter` adapter while retaining the identity conversion for `VecIter`.
- The iterator fixture now exercises `vector_into_source.into_iter()` directly.
  GC component execution and linear Wasm validation/runtime pass.

## Progress receipt — 2026-09-20 (for equivalence)

- Test commit `67b4c876c` adds a same-input comparison between structural
  `for x in iter::vec_iter(v)` and generic `sum_iter<T: Iterator>`. Both
  observe `60` on GC and linear Wasm, documenting the current concrete
  lowering equivalence without changing the structural lowering implementation.

## Progress receipt — 2026-09-20 (Range IntoIterator)

- Commit `089ebf819` adds `IntoIterator for Range`, producing the existing
  `RangeIter` adapter through the same trait conversion boundary as `Vec<i32>`.
- `iterator_range_basic.ark` exercises `source.into_iter().next()`; GC
  component compilation/execution and linear Wasm validation/execution pass.
- Generic item collection, callback-bearing consumers, and structural
  `for` desugaring remain open, so this is an incremental receipt rather than
  a close receipt.
