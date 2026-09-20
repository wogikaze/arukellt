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

## Progress receipt — 2026-09-20 (generic lazy adapter `for` lowering)

- Test commit `6c92fa741` sends the composed
  `iter().map(double).filter(is_even).take(2)` adapter through a generic
  `sum_into_for<T: IntoIterator>` consumer. The monomorphized adapter state is
  preserved by identity `IntoIterator` before canonical `next` consumption.
- GC and linear-memory Wasm compile, validate, and run with `OK`. WAT confirms
  `sum_into_for__MapFilterTakeIter` uses the same conversion and `next` calls
  on both backends.
- This remains concrete `Vec<i32>` evidence. Arbitrary item representations,
  dynamic dispatch, broader adapter composition, manifest-level deprecation,
  and the final RFC-011 residual audit remain open. This is not a close
  receipt.

## Progress receipt — 2026-09-20 (generic range `for` lowering)

- Test commit `e9ee279b5` instantiates the generic
  `sum_into_for<T: IntoIterator>` consumer with `Range` and
  `RangeInclusive` direct expressions. The fixture preserves the half-open
  and closed range boundaries while using the shared `into_iter`/`next` path.
- GC and linear-memory Wasm compile, validate, and run successfully. WAT
  confirms separate monomorphized range functions and their corresponding
  canonical iterator calls on both backends.
- This remains static concrete `i32` evidence. Arbitrary item
  representations, dynamic dispatch, broader adapter composition,
  manifest-level deprecation, and the final RFC-011 residual audit remain
  open. This is not a close receipt.

## Progress receipt — 2026-09-20 (adapter identity conversion)

- Commit `ce6a83389` implements identity `IntoIterator` conversions for the
  concrete lazy adapters. `for` over an adapter therefore uses the same
  `IntoIterator::into_iter` boundary as `Vec` and range sources before calling
  the adapter's canonical `Iterator::next` method.
- `iterator_adapters.ark` verifies the composed
  `map/filter/take` source in a direct `for`; GC and linear-memory artifacts
  both compile, validate, and run with `OK`. WAT inspection shows
  `MapFilterTakeIter::into_iter` immediately before `next` on both backends.
- `verify lane` passes with the formatter baseline synchronized. This advances
  the concrete `Vec<i32>` adapter lane only; arbitrary item representations,
  additional compositions, manifest-level deprecation, and the final RFC-011
  residual audit remain open. This is not a close receipt.

## Progress receipt — 2026-09-20 (generic `IntoIterator` `for` lowering)

- Test commit `5492a9d81` extends the generic for-loop fixture with
  `sum_into_for<T: IntoIterator>`, instantiated with `Vec<i32>`. The
  monomorphized body calls `Vec::into_iter` and consumes the resulting
  `VecIter` through its canonical `next` method.
- GC and linear-memory Wasm compile, validate, and run successfully. WAT
  inspection confirms `sum_into_for__Vec_i32_` uses the same conversion and
  `next` sequence on both backends.
- This advances only the concrete monomorphized generic case. Arbitrary item
  representations, dynamic dispatch, broader adapter composition,
  manifest-level deprecation, and the final RFC-011 residual audit remain
  open. This is not a close receipt.

## Progress receipt — 2026-09-20 (map/take adapter composition)

- Commit `b0849dee4` adds `MapTakeIter` and `MapIter::take`, keeping the map
  callback and take limit in one lazy adapter state. The standard iterator
  consumers delegate to the same canonical traversal helpers.
- `iterator_adapters.ark` covers `iter().map(double).take(2).collect()`;
  GC and linear-memory Wasm compile, validate, and run with `OK`. WAT confirms
  the `MapIter::take` construction and `MapTakeIter::next` calls on both
  backends.
- This remains concrete `Vec<i32>` evidence. Other item representations,
  arbitrary adapter combinations, dynamic dispatch, manifest-level
  deprecation, and the final RFC-011 residual audit remain open. This is not a
  close receipt.

## Progress receipt — 2026-09-20 (item-preserving String lane and consumer split)

- Commit `e6d474dad` adopts [ADR-076](../../docs/adr/ADR-076-language-v2-iterator-consumer-boundary.md)
  and removes fixed `sum`/`fold` methods from the producer `Iterator` trait.
  Numeric consumers are now represented by the concrete
  `IteratorSum<i32, i32>` and `IteratorFold<i32, i32>` capability impls,
  while `Iterator` retains item-preserving `next`, `collect`, and `count`.
- `StringIter` implements the producer contract, `Vec<String>` converts to it
  through `IntoIterator`, and `Vec<String>` reconstructs from it through
  `FromIterator`. `iterator_string.ark` covers generic producer/source calls,
  collection, counting, associated construction, and direct `for` iteration.
  GC component and linear-memory Wasm compilation, validation, and runtime
  assertions pass with the same source behavior.
- The iterator-loop compiler path now allocates the element local with the
  correct GC/linear reference value type and selects a typed `Option::Some`
  payload layout. The implementation avoids integer packing and keeps the
  String item identity intact on both backends.
- This advances the arbitrary-item boundary with one concrete String lane; it
  does not claim all adapter combinations, all collection types, manifest-level
  deprecation, or completion of #691/#860.

## Progress receipt — 2026-09-20 (generic String `IntoIterator` for lowering)

- Test commit `c91f76573` extends the String callback fixture with a generic
  `total_string_lengths<T: IntoIterator>` consumer. The source is the lazy
  `filter(...).map(...)` adapter, and its `for` loop is lowered through the
  adapter's identity `IntoIterator` conversion and canonical `next` method.
- GC component and linear-memory Wasm artifacts validate and run with the
  same result. The specialization is
  `total_string_lengths__StringFilterMapIter`; callback calls remain typed
  `call_ref` on GC and table-based `call_indirect` on linear memory, so the
  adapter chain retains one source-level meaning across both backends.
- This is bounded generic-consumer evidence for one concrete String adapter
  composition. Broader arbitrary-item adapter composition, HashMap iterator
  coverage, manifest-level deprecation, and the remaining RFC-011 audit stay
  open; #691 is not closed.

## Progress receipt — 2026-09-20 (String filter-map composition)

- Commit `38c326934` adds the reverse `StringFilterMapIter` path. Its predicate
  receives a clone of the source String; only accepted originals are passed to
  `map`, preserving ownership and source order without an intermediate vector.
- `iterator_string_callbacks.ark` verifies lazy `collect` and direct
  `Vec::from_iter` on both the GC component and linear-memory fallback paths.
  Typed GC `call_ref` and linear table `call_indirect` artifacts validate and
  run with matching results.
- This remains concrete String composition. Arbitrary item-generic adapter
  composition, dynamic dispatch, manifest-level deprecation, and the final
  RFC-011 residual audit remain open; #691 is not closed.

## Progress receipt — 2026-09-20 (String map-filter composition)

- Commit `4b0468228` adds `StringMapFilterIter` for the lazy
  `StringIter::map(...).filter(...)` path. The mapped String is cloned only for
  the owning predicate; an accepted mapped handle is yielded in source order.
- `iterator_string_callbacks.ark` verifies lazy `collect`, direct
  `Vec::from_iter`, `IntoIterator`, and the GC typed callback / linear table
  callback forms. GC component and linear-memory artifacts validate and run with
  matching results.
- This remains a concrete String composition. Arbitrary item-generic adapter
  composition, dynamic dispatch, manifest-level deprecation, and the final
  RFC-011 residual audit remain open; #691 is not closed.

## Progress receipt — 2026-09-20 (String adapter collection dispatch)

- Commit `3a0b3b3ee` extends the concrete String callback fixture through
  `Vec::from_iter` for both `StringMapIter` and `StringFilterIter`. The compiler
  now resolves the source-qualified `Vec_String::from_iter__<Source>` symbol
  for every concrete String callback adapter currently in the lane.
- GC and linear-memory Wasm artifacts validate and run with the same collection
  assertions. The existing typed GC callback signatures and linear table
  `call_indirect` path remain unchanged; this unit only closes the associated
  collection dispatch boundary.
- Arbitrary item-generic adapter composition, dynamic dispatch, manifest-level
  deprecation, and the final RFC-011 residual audit remain open. This is not a
  close receipt.

## Progress receipt — 2026-09-20 (String callback adapters)

- Commit `2b862af04` adds `StringMapIter` and `StringFilterIter` with
  `Iterator`, identity `IntoIterator`, and concrete `FromIterator` impls for
  `Vec<String>`. `StringIter::map` accepts `fn(String) -> String`, and
  `StringIter::filter` accepts `fn(String) -> bool`; the filter path clones the
  callback argument so the accepted source item remains available to yield.
- `tests/fixtures/stdlib_trait/iterator_string_callbacks.ark` exercises both
  lazy adapters and their canonical `collect` traversal. The same source
  compiles and validates on `wasm32-gc` and linear-memory `wasm32`, and both
  runtime paths pass. The existing `iterator_string.ark` and
  `iterator_adapters.ark` regression paths also pass after the callback ABI
  change.
- GC WAT records `String -> String` as typed callback index 46 and the
  Memory64 `String -> bool` representation as index 48; fields, callback locals,
  and `call_ref` use the same index. Linear WAT retains the table-based
  `call_indirect` ABI. This is the same source-level item/order semantics, not
  an integer-packed String representation.
- `Vec::from_iter(source.iter().map(...))` is intentionally not included in
  this receipt: the existing generic associated-function lowering still emits
  `drop; unreachable` for that projected source. The concrete adapter
  `FromIterator` impls remain tracked until that lowering emits valid Wasm on
  both targets. Arbitrary adapter composition, manifest deprecation, and the
  final RFC-011 residual audit remain open; #691 is not closed.

## Progress receipt — 2026-09-20 (Memory64 GC callback field parity)

- Commit `51e41360a` fixes the GC type-section emitter so callback fields in
  user structs and `A_fnref` arrays select the same unary/binary callback type
  as function parameters for the active memory width. Previously, struct
  fields always used the `i32 -> i32` callback index while Memory64 function
  parameters used `i64 -> i64`, producing an invalid GC module for the
  existing i32 lazy adapter chain.
- `iterator_adapters.ark` now compiles as a GC P2 component and as linear P1
  Wasm; both artifacts pass `wasm-tools validate` and run successfully under
  Wasmtime (`OK`). `iterator_string.ark`'s existing no-callback String lane
  also passes the same GC component and linear-memory compile, validation, and
  runtime checks.
- `python3 scripts/manager.py verify lane` and
  `python3 scripts/manager.py verify lane --gate component-interop` pass,
  including formatter, local lint, quality ratchet, and component interop.
- This receipt does not add String callback adapters. `fn(String) -> String` /
  `fn(String) -> bool` remains outside the current callback ABI contract, and
  broader item-generic adapters, eager/lazy migration, manifest deprecation,
  and the final RFC-011 audit remain open. #691 is not closed.

## Progress receipt — 2026-09-20 (`Vec<String>::iter()` producer entry point)

- `Vec<String>` now exposes the same item-preserving `StringIter` producer via
  `iter()` as well as `IntoIterator::into_iter`. The fixture uses
  `method_source.iter().collect()`, and WAT confirms the GC lowering calls
  `Vec_String::iter` before `StringIter::collect`.
- The current stage-2 artifacts validate on both targets. The linear-memory
  core runs successfully; the Wasm GC component validates and runs with the
  supported component runner. This keeps the source-level result and String
  payload identity equal across the primary and fallback paths.
- The evidence is still one concrete non-numeric lane. Generic adapters,
  broader item representations, manifest-level deprecation, and the remaining
  RFC-011 audit are not complete, so #691 remains open.

## Progress receipt — 2026-09-20 (String item-preserving `take` adapter)

- `StringIter::take(n)` now produces a concrete `StringTakeIter` while
  preserving `Iterator::Item = String`. The fixture checks the bounded
  `source.iter().take(1).collect()` pipeline and its one-item result.
- The GC component and linear-memory core artifacts both validate and run.
  WAT shows `Vec_String::iter`, `StringIter::take`, and
  `StringTakeIter::collect` in sequence, so the adapter does not reinterpret
  String references through the numeric i32 item path.
- This receipt covers a no-callback adapter only. Generic callback adapters,
  broader item composition, manifest-level deprecation, and the remaining
  RFC-011 audit are still open; #691 remains open.

## Progress receipt — 2026-09-20 (String `skip` / `skip().take()` composition)

- `StringIter::skip(n)` and `StringSkipIter::take(m)` now form the concrete
  `StringSkipTakeIter` composition. The fixture checks skip-before-take order
  through the canonical `Iterator::collect` consumer.
- GC component and linear-memory artifacts both validate and run. The lowering
  keeps the String reference item representation through
  `Vec_String::iter`, `StringIter::skip`, `StringSkipIter::take`, and
  `StringSkipTakeIter::collect`.
- Callback-bearing adapters, broader generic item composition,
  manifest-level deprecation, and the remaining RFC-011 audit are not complete;
  #691 remains open.

## Progress receipt — 2026-09-20 (for-loop fixture migration boundary)

- Commit `4b8d19657` updates `for_loops/for_values.ark` to import
  `std::core::iter`, matching the v2 requirement that the concrete
  `IntoIterator` implementation be visible to type checking. The fixture had
  retained the pre-IntoIterator form and was the source of an avoidable E0200
  failure in the current-only fixture gate.
- The migrated fixture compiles and passes `wasm-tools validate` for both
  `wasm32-gc` and linear-memory `wasm32`. The filtered `for_loops` gate now
  reports 2/3 passes; the remaining `for_range` failure occurs after compile
  and core validation in the existing P2 `realloc` component-packaging path.
- `python3 scripts/manager.py fmt --check` passes (1971 checked, 0 failed).
  This receipt records fixture alignment only and does not close #691.

## Progress receipt — 2026-09-20 (skip/take adapter composition)

- Commit `317583de9` adds `SkipTakeIter` and `SkipIter::take`, preserving the
  skip-before-take state in one lazy adapter. The standard iterator consumers
  delegate to the same canonical traversal helpers.
- `iterator_adapters.ark` covers `iter().skip(2).take(2).collect()` and a
  generic `T: IntoIterator` consumer. GC and linear-memory Wasm compile,
  validate, and run with `OK`; WAT confirms `SkipIter::take`,
  `SkipTakeIter::into_iter`, and `SkipTakeIter::next` on both backends.
- This remains concrete `Vec<i32>` evidence. Other item representations,
  arbitrary adapter combinations, dynamic dispatch, manifest-level
  deprecation, and the final RFC-011 residual audit remain open. This is not a
  close receipt.

## Progress receipt — 2026-09-20 (eager helper delegation)

- Commit `8384f8115` routes `collect_map_filter_i32` and
  `collect_map_filter_take_i32` through the concrete lazy
  `map/filter[/take]` adapter chain and canonical `Iterator::collect`. The
  compatibility helpers no longer carry separate eager source loops.
- `iterator_adapters.ark` covers both the unbounded map/filter result and the
  bounded map/filter/take result. GC and linear-memory artifacts compile,
  validate, and run with `OK`; WAT confirms the adapter method calls are used
  inside both compatibility entries.
- `verify lane` passes. Generic item representations, broader adapter
  composition, manifest-level deprecation, and the final RFC-011 residual
  audit remain open. This is not a close receipt.

## Progress receipt — 2026-09-20 (generic `Iterator` `for` lowering)

- Test commit `c50043518` adds a generic `sum_for<T: Iterator>` consumer to
  `iterator_vec_iter_for_loop.ark`. Its `for` body is instantiated with
  `VecIter`, proving that desugared iteration and a handwritten generic
  `Iterator` consumer share the same monomorphized source path.
- GC component and linear-memory Wasm compile, validate, and run successfully.
  WAT inspection shows `sum_for__VecIter` calling the identity
  `VecIter::into_iter` before the canonical `VecIter::next` loop on both
  backends.
- This is evidence for the checked `T: Iterator` case, not dynamic dispatch or
  arbitrary item support. Generic obligations outside that bound, broader
  adapter composition, manifest-level deprecation, and the final RFC-011
  residual audit remain open. This is not a close receipt.

## Progress receipt — 2026-09-20 (filter/take adapter composition)

- Commit `5a1408aac` adds `FilterTakeIter` and `FilterIter::take`, keeping the
  predicate and take limit in one lazy adapter state. The standard iterator
  consumers delegate to the same canonical traversal helpers.
- `iterator_adapters.ark` covers `iter().filter(is_even).take(1).collect()`
  and a generic `T: IntoIterator` consumer. GC and linear-memory Wasm compile,
  validate, and run with `OK`; WAT confirms `FilterIter::take`,
  `FilterTakeIter::into_iter`, and `FilterTakeIter::next` on both backends.
- This remains concrete `Vec<i32>` evidence. Other item representations,
  arbitrary adapter combinations, dynamic dispatch, manifest-level
  deprecation, and the final RFC-011 residual audit remain open. This is not a
  close receipt.

## Progress receipt — 2026-09-20 (std::seq compatibility delegation)

- Commit `d31cb9ec3` changes the concrete `std::seq` map/filter/take/skip/fold
  compatibility helpers to delegate to `std::core::iter` consumers. The
  qualified `std::seq` map/filter/fold entries are kept as ordinary Ark bodies
  so they no longer enter the legacy eager HOF lowering shim.
- `stdlib_seq/seq_map_filter.ark` and `stdlib_seq/seq_take_skip_fold.ark`
  compile and pass `wasm-tools validate` on both targets. Both GC components
  and linear-memory Wasm run with identical output; WAT shows calls to
  `collect_map_i32`, `collect_filter_i32`, `collect_take_i32`,
  `collect_skip_i32`, and `iter_fold_i32` on both backends.
- This is concrete `Vec<i32>` migration evidence only. Arbitrary item
  representations, arbitrary adapter combinations, manifest-level deprecation,
  and the final RFC-011 residual audit remain open. This is not a close receipt.

## Progress receipt — 2026-09-20 (item-preserving String lane and consumer split)

- Commit `e6d474dad` adopts [ADR-076](../../docs/adr/ADR-076-language-v2-iterator-consumer-boundary.md)
  and removes fixed `sum`/`fold` methods from the producer `Iterator` trait.
  Numeric consumers are now represented by the concrete
  `IteratorSum<i32, i32>` and `IteratorFold<i32, i32>` capability impls,
  while `Iterator` retains item-preserving `next`, `collect`, and `count`.
- `StringIter` implements the producer contract, `Vec<String>` converts to it
  through `IntoIterator`, and `Vec<String>` reconstructs from it through
  `FromIterator`. `iterator_string.ark` covers generic producer/source calls,
  collection, counting, associated construction, and direct `for` iteration.
  GC component and linear-memory Wasm compilation, validation, and runtime
  assertions pass with the same source behavior.
- The iterator-loop compiler path now allocates the element local with the
  correct GC/linear reference value type and selects a typed `Option::Some`
  payload layout. The implementation avoids integer packing and keeps the
  String item identity intact on both backends.
- This advances the arbitrary-item boundary with one concrete String lane; it
  does not claim all adapter combinations, all collection types, manifest-level
  deprecation, or completion of #691/#860.
