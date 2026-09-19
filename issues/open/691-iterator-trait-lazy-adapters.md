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

## Progress receipt — 2026-09-20 (generic IntoIterator dispatch)

- Test commit `87222ad53` adds a generic `T: IntoIterator` consumer returning
  `Option<T::Item>` and instantiates it with `Range`.
- The fixture compiles and runs on the GC component path and on linear Wasm;
  associated-item projection and conversion dispatch both observe the first
  range value (`7`).

## Progress receipt — 2026-09-20 (RangeInclusive IntoIterator)

- Commit `997d19ffb` adds `RangeInclusiveIter` and `IntoIterator for
  RangeInclusive`, preserving the closed `[start, end]` boundary.
- The range fixture verifies the first item and the two remaining items after
  consuming `3` from `[3, 5]`; GC component and linear Wasm checks pass.

## Progress receipt — 2026-09-20 (range canonical collect)

- Test commit `01f548c9e` exercises `RangeInclusive::into_iter().collect()`
  through the existing `Iterator::collect` method and checks the closed-range
  result `[4, 5, 6]`.
- GC component execution and linear Wasm validation/runtime pass. A separate
  `FromIterator` impl for range adapters is deferred because adding it exposed
  an existing GC numeric-width lowering defect; no invalid path is retained.

## Progress receipt — 2026-09-20 (range FromIterator dispatch)

- Commit `b7baf92fb` adds concrete `FromIterator<RangeIter>` and
  `FromIterator<RangeInclusiveIter>` implementations for `Vec<i32>` and
  resolves `Vec::from_iter` by the source iterator type at MIR lowering. This
  preserves the trait-first associated-function surface while keeping static
  monomorphization and the canonical `Iterator::collect` body.
- `iterator_range_basic.ark` exercises both direct associated-function calls;
  `iterator_vec_iter_next.ark` preserves the existing `VecIter` path. The GC
  WAT contains separate source-qualified symbols, and both fixtures pass GC
  component plus linear-memory compile/validate/runtime checks.
- The earlier GC integer-width defect is independently fixed by `38cc7cc71`
  under #871; the range FromIterator path now has no invalid Wasm artifact.
  Formatter, `verify lane`, and docs check pass. Generic item projection,
  callback-bearing consumers, and the remaining eager/lazy migration are still
  open; this is not a close receipt.

## Progress receipt — 2026-09-20 (canonical fold and fallback callback ABI)

- Commit `3ab6f0f56` adds `Iterator::fold(self, init, f)` for the supported
  `i32` item lane and a shared `fold_iterator_i32<T: Iterator>` traversal.
  `VecIter`, both range iterators, and the existing lazy adapter types delegate
  to that canonical path; `iter_fold_i32` is now a compatibility entry point
  over `VecIter::fold` rather than a second traversal implementation.
- `iterator_vec_iter_next.ark` exercises a generic `fold_values<T: Iterator>`
  consumer with `VecIter`; `iterator_range_basic.ark` exercises the same
  consumer with both `RangeIter` and `RangeInclusiveIter`.
- GC uses the existing typed `ref.func`/`call_ref` callback ABI. The linear
  fallback now lowers the same callback MIR to an `i32` table slot and
  `call_indirect`, including function signatures and callback locals. This
  removes the previous linear validation/runtime boundary without changing
  the GC primary representation.
- `iterator_fold_sum.ark`, `iterator_fold_product.ark`,
  `iterator_fold_empty.ark`, `iterator_map_basic.ark`, and
  `iterator_filter_basic.ark` pass GC component and linear-memory compile,
  Wasm validation, and runtime checks. The generated linear WAT contains
  `call_indirect` and no typed callback parameter; the GC WAT contains
  `call_ref` and typed callback parameters.
- Targeted formatter checks and default `python3 scripts/manager.py verify lane`
  pass. Generic `Vec<T::Item>` collection, method-shaped lazy adapters, the
  remaining eager/lazy migration, and the final RFC item 37 residual audit
  remain open; no acceptance checkbox is marked complete.

## Progress receipt — 2026-09-20 (projected collection and memory64 callback ABI)

- Commit `00cecec42` changes the canonical `Iterator::collect` surface to
  `Vec<Self::Item>` and makes the shared traversal `Vec<T::Item>`. Exact
  associated-type projection is retained through monomorphized `Vec`
  construction, so the current `i32`-backed iterator implementations no
  longer require an `i32`-only aggregate helper.
- The compiler reserves memory64 GC unary/binary callback signatures ahead of
  user function types and keeps callback parameter, local, and `call_ref`
  type indexes consistent. The linear-memory fallback keeps the same source
  callback meaning through table-index `call_indirect`.
- `iterator_vec_iter_next.ark`, `iterator_range_basic.ark`,
  `iterator_fold_product.ark`, and
  `iterator_map_filter_take_pipeline.ark` pass GC component and linear Wasm
  compile, validation, and runtime checks. `verify lane` also passes.
- This is still partial #691 evidence: only the currently implemented
  `i32`-backed item lane is proven; arbitrary item implementations, the
  method-shaped lazy adapter API, full eager/lazy migration, structural
  trait-driven `for`, and the final RFC-011 residual audit remain open.

## Progress receipt — 2026-09-20 (concrete IntoIterator for lowering)

- Commit `33a604056` adds a shared concrete-source resolver to both legacy AST
  and CoreHir `for` lowering. Concrete sources with a registered
  `IntoIterator::into_iter` conversion are converted before the canonical
  iterator `next` call; the existing compatibility and structural fallbacks
  remain for sources outside this boundary.
- `iterator_vec_iter_for_loop.ark` and `iterator_range_basic.ark` cover direct
  `for` over `Vec<i32>`, `Range`, and `RangeInclusive`. GC P2/Memory64
  components and linear P1 modules compile, validate, and run successfully on
  both target paths.
- WAT inspection confirms the source-specific `into_iter` and iterator
  `next` symbols in both artifacts. `verify lane` and `docs check` pass.
- This remains partial #691 evidence: arbitrary item representations, the
  method-shaped lazy adapter API, full eager/lazy migration, generic source
  expressions, and the final RFC-011 residual audit remain open.

## Progress receipt — 2026-09-20 (method-shaped lazy pipeline)

- Commit `0a6cb7b81` adds the callable `Vec<i32>` method surface
  `iter().map/filter/take/skip(...)` and routes the legacy `vec_iter`,
  `map_iter`, `filter_iter`, `take_iter`, and `skip_iter` constructors through
  those methods. `MapIter::filter` and `MapFilterIter::take` provide the
  verified composed path `v.iter().map(f).filter(g).take(n).collect()` through
  a single `MapFilterTakeIter` traversal without intermediate `Vec` stages.
- `tests/fixtures/stdlib_trait/iterator_adapters.ark` checks the four
  one-step method consumers and the composed map/filter/take pipeline. The
  fixture compiles and runs as a `wasm32-gc` P2 component and as `wasm32`
  linear Wasm; both artifacts pass `wasm-tools validate` and print `OK` under
  Wasmtime. WAT inspection confirms the adapter chain and the linear
  table-index callback representation.
- The same commit fixes nested method lowering in both AST and CoreHir paths:
  CoreHir child indexes are materialized before shape predicates, and results
  stored in a nested method local are reloaded before the outer call. This
  keeps the source meaning identical across GC and linear backends.
- `docs/language/spec.md` and `docs/stdlib/migration-guidance.md` now state
  the verified method status. Other item representations, additional adapter
  compositions, manifest-level deprecation, and the final RFC-011 residual
  audit remain open; this is not a close receipt.

## Progress receipt — 2026-09-20 (lazy adapter source in `for`)

- Commit `dacac461e` adds a direct `for` consumer to
  `iterator_adapters.ark`: `v.iter().map(double).filter(is_even).take(2)`.
  The concrete `map/filter/take` chain is consumed through
  `MapFilterTakeIter::next` and produces the expected total without an
  intermediate collection.
- The GC component and linear-memory Wasm fixture both compile, validate, and
  run successfully, printing `OK`. The generated WAT confirms the same
  adapter construction and `next` sequence on both backends, with typed GC
  calls and the linear callback/table fallback preserving the source meaning.
- `verify lane` passes. This advances only the implemented `Vec<i32>` adapter
  lane; arbitrary item representations, additional adapter compositions,
  manifest-level deprecation, and the final RFC-011 residual audit remain
  open. This is not a close receipt.
