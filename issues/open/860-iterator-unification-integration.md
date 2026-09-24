---
Status: open
Created: 2026-09-19
Updated: 2026-09-23
ID: 860
Parent: 852
Track: language-v2
Depends on: "691, 856"
Related: "ADR-058, RFC-011, #852"
Orchestration class: implementation-ready
Orchestration upstream: 852
Blocks v{N}: language-v2
Priority: 1
Source: "RFC-011 37"
---

# 860 — Iterator Trait 体系を統合完了する

## Scope

RFC-011: **37**。

この issue の独立範囲だけを実装する。別の設計判断が必要なら child issue を追加し、ここへ混ぜない。

## Acceptance

- [ ] Iterator/IntoIterator/FromIterator の責務を固定する
- [ ] eager helper と lazy adapter の重複を解消する
- [ ] `collect` を canonical Trait path にする
- [ ] desugared iteration と手書き Iterator を同値にする
- [ ] #691 完了後に RFC item 37 の残差を 0 にする

## Required evidence

- implementation commit SHA
- normative spec / ADR/RFC update reference
- positive fixture + negative/diagnostic fixture
- applicable compile/validate/runtime gate output

## False-done guard

- [ ] Acceptance の一部だけで done にしない
- [ ] TODO/skip/hack を残す場合は tracking issue + removal condition を必須にする
- [ ] 親 #852 の担当項目を evidence 付きで更新する

## Progress receipt — 2026-09-20

- `4ec1085c7` implements the associated-type and projection foundation for
  `Iterator`/`IntoIterator`, including nested generic return types and
  imported trait-path matching.
- Positive evidence is present in
  `tests/fixtures/stdlib_trait/iterator_vec_iter_next.ark` and
  `tests/fixtures/trait/trait_core_v2.ark`. Both GC P2 and linear P1 compile,
  validation, and runtime checks pass; the latter produces `79`.
- Follow-up commit `c8707c13e` makes `Iterator::next` the only stdlib adapter
  implementation and verifies the existing `for` lowering against the
  trait-only `VecIter` path on both targets.
- Commit `099863a45` exercises the canonical `Vec::from_iter` associated
  function with a `VecIter` source on both targets.
- Test commit `18885e48e` also exercises `IntoIterator::into_iter` before the
  iterator is consumed by `next`.
- Commit `9a1241bcc` removes the last explicit `VecIter` annotations from the
  imported iterator fixture, proving that qualified constructor inference
  feeds the generic `Iterator` projection path on both targets.
- Commit `cfd072391` makes `Iterator::collect` the canonical trait path for the
  current `i32` item lane. The generic `collect_iterator_i32<T: Iterator>`
  traversal is shared by adapter implementations; legacy collection helpers
  and `FromIterator<VecIter>` no longer own duplicate traversal bodies.
- The positive fixture now calls that method through a generic
  `collect_values<T: Iterator>` function and passes GC component plus linear
  Wasm compile/validate/runtime checks. `Vec<T::Item>` remains explicitly
  deferred under #860 until generic aggregate projection lowering is valid on
  both targets.
- This receipt does not mark any acceptance checkbox complete: fully generic
  collection implementations, eager/lazy API migration, desugared-iteration
  equivalence, and the RFC-011 residual audit still require implementation and
  review.

## Progress receipt — 2026-09-20 (iterator consumers)

- Commit `d901883d0` adds `Iterator::count` and `Iterator::sum` to the
  canonical trait path for the supported `i32` item lane. Their generic
  traversals are exercised through `count_values<T: Iterator>` and
  `sum_values<T: Iterator>` in the positive fixture.
- GC component execution and linear Wasm validate/runtime checks pass for the
  updated fixture. `fold` is intentionally not included because its generic
  callback lowering still hits the tracked function-reference ABI boundary.

## Progress receipt — 2026-09-20 (Vec IntoIterator)

- Commit `30a5ec717` adds the source-collection conversion
  `IntoIterator for Vec<i32> -> VecIter`. The positive fixture calls
  `vector_into_source.into_iter()` and passes GC component plus linear Wasm
  compile/validate/runtime checks.
- Structural `for` lowering remains unchanged and is not claimed as a fully
  trait-driven desugaring in this receipt.

## Progress receipt — 2026-09-20 (for equivalence)

- Test commit `67b4c876c` compares structural `for` with a handwritten generic
  `Iterator::sum` consumer over the same `VecIter` input. GC component and
  linear Wasm compile/validate/runtime checks pass, establishing the current
  concrete equivalence while leaving full trait-driven desugaring open.

## Progress receipt — 2026-09-20 (Range IntoIterator)

- Commit `089ebf819` adds the `Range -> RangeIter` `IntoIterator` impl and
  extends the range fixture with a direct conversion call.
- GC component compilation/execution and linear Wasm compilation, validation,
  and execution pass. This broadens the unified conversion boundary without
  changing structural `for` lowering or claiming generic aggregate support.

## Progress receipt — 2026-09-20 (generic IntoIterator dispatch)

- Test commit `87222ad53` adds `first_from_source<T: IntoIterator>` and
  exercises `source.into_iter().next()` through the associated `Item` type.
- The Range instantiation passes GC component and linear Wasm compile,
  validation, and runtime checks; full trait-driven `for` desugaring remains
  open.

## Progress receipt — 2026-09-20 (RangeInclusive IntoIterator)

- Commit `997d19ffb` adds the closed-range adapter and its `IntoIterator`
  implementation.
- The fixture checks `[3, 5]` through `next` and `count`; GC component and
  linear Wasm compile, validation, and runtime checks pass without changing
  structural `for` lowering.

## Progress receipt — 2026-09-20 (range canonical collect)

- Test commit `01f548c9e` verifies `RangeInclusive::into_iter().collect()`
  through the canonical `Iterator::collect` path on `[4, 5, 6]`.
- GC component and linear Wasm compile/validate/runtime checks pass. Adding
  range-specific `FromIterator` impls is not landed: it currently widens
  concrete GC integer locals before `Range` struct construction and produces
  invalid Wasm, so that compiler defect remains outside this unit.

## Progress receipt — 2026-09-20 (FromIterator source dispatch)

- Commit `b7baf92fb` extends the existing source-type-qualified static symbol
  rule from `From` to `FromIterator`. A source-level `Vec::from_iter(...)`
  call now selects the concrete `Vec::from_iter__<Source>` implementation from
  its first argument type when that implementation is registered; ordinary
  associated functions retain their existing name. `RangeIter` and
  `RangeInclusiveIter` now implement `FromIterator` for `Vec<i32>` by
  delegating to the canonical `Iterator::collect` traversal.
- `iterator_range_basic.ark` verifies direct `Vec::from_iter` calls for both
  half-open and closed ranges. The existing `iterator_vec_iter_next.ark`
  remains a positive regression for `VecIter`; printed GC WAT contains the
  distinct `Vec::from_iter__VecIter`, `Vec::from_iter__RangeIter`, and
  `Vec::from_iter__RangeInclusiveIter` functions and calls the matching
  source-specific symbols.
- Both iterator fixtures pass `wasm32-gc` P2/Memory64 component
  compile/validate/runtime and `wasm32` P1 linear Wasm compile/validate/runtime.
  The targeted formatter and default `python3 scripts/manager.py verify lane`
  pass; `python3 scripts/manager.py docs check` passes all 4 checks.
- This remains partial evidence: generic `Vec<T::Item>` collection,
  callback-bearing `fold`, full lazy adapter migration, and the final RFC item
  37 residual audit remain open. No acceptance checkbox is marked complete.

## Progress receipt — 2026-09-20 (canonical fold and fallback callback ABI)

- Commit `3ab6f0f56` adds the canonical `Iterator::fold` consumer for the
  supported `i32` item lane. Its generic `fold_iterator_i32<T: Iterator>`
  traversal is shared by every current adapter implementation, and the legacy
  `iter_fold_i32` helper delegates through `VecIter::fold`.
- Positive evidence covers generic dispatch over `VecIter`, `RangeIter`, and
  `RangeInclusiveIter` in `iterator_vec_iter_next.ark` and
  `iterator_range_basic.ark`. The fold sum/product/empty and map/filter
  callback fixtures also pass on both target paths.
- The callback ABI is now target-consistent in meaning: `wasm32-gc` keeps
  typed `ref.func`/`call_ref`, while `wasm32` emits table-index `i32` values
  and `call_indirect`. Both WAT forms validate and their runtimes produce the
  expected results.
- Targeted formatter checks and default `python3 scripts/manager.py verify lane`
  pass. Generic aggregate projection, method-shaped adapter migration,
  structural trait-driven `for` desugaring, and the final RFC item 37 residual
  audit remain open. This receipt does not close #860 or mark its acceptance
  boxes complete.

## Progress receipt — 2026-09-20 (projected collection and memory64 callback ABI)

- Commit `00cecec42` removes the aggregate-projection blocker for the current
  iterator item lane. `Iterator::collect` and its shared traversal now use
  `Vec<Self::Item>` / `Vec<T::Item>`; monomorphization preserves the exact
  associated projection through `Vec` constructor lowering and aggregate local
  type resolution.
- The Wasm type planner reserves canonical unary and binary memory64 callback
  signatures before WIT, GC, and user function types. GC callback parameters,
  callback locals, `ref.func`, and `call_ref` now reference the same widened
  `i64` callback signatures; the linear fallback continues to use its table
  index plus `i32` `call_indirect` ABI.
- `iterator_vec_iter_next.ark` passes GC component and linear Wasm
  compile/validate/runtime checks. Range, fold-product, and
  map/filter/take-pipeline representatives also pass both target paths.
- `python3 scripts/manager.py verify lane` passes with formatter and lint
  ratchets clean. This advances the generic projection boundary for the
  implemented `i32`-backed iterators; arbitrary new item representations,
  method-shaped lazy adapter migration, structural trait-driven `for`, and the
  final RFC-011 residual audit remain open. No acceptance checkbox is marked
  complete.

## Progress receipt — 2026-09-20 (concrete IntoIterator for lowering)

- Commit `33a604056` adds a shared concrete-source resolver to both legacy AST
  and CoreHir `for` lowering. When a source has a registered
  `IntoIterator::into_iter` implementation, lowering evaluates the source,
  calls that conversion, and drives the resulting concrete iterator through
  its canonical `next`; otherwise the existing Vec/raw or structural fallback
  remains in place.
- `iterator_vec_iter_for_loop.ark` covers direct `for x in Vec<i32>` and
  `iterator_range_basic.ark` covers direct `for` over `Range` and
  `RangeInclusive` locals. GC P2/Memory64 components and linear P1 modules
  compile, validate, and run successfully on both paths.
- WAT inspection confirms `Vec::into_iter`/`VecIter::next`,
  `Range::into_iter`/`RangeIter::next`, and
  `RangeInclusive::into_iter`/`RangeInclusiveIter::next` in both artifacts.
  `verify lane` and `docs check` pass.
- This is not a close receipt: `values(v)` compatibility remains, arbitrary
  item representations, method-shaped lazy adapters, generic source
  expressions, and the final RFC-011 residual audit remain open.

## Progress receipt — 2026-09-20 (method-shaped lazy pipeline)

- Commit `0a6cb7b81` introduces the concrete `Vec<i32>` method boundary
  `iter().map/filter/take/skip(...)` and makes the existing free constructors
  delegate to it. The representative composed expression
  `v.iter().map(f).filter(g).take(n).collect()` is implemented by
  `MapIter::filter`, `MapFilterIter::take`, and `MapFilterTakeIter` while
  retaining the canonical `Iterator::collect` traversal.
- `iterator_adapters.ark` proves one-step method consumers and the composed
  pipeline. GC component and linear Wasm compile, validate, and runtime checks
  pass; linear WAT shows the expected callback table-index ABI and no
  intermediate collection call between adapter stages. `verify lane` passes.
- The compiler portion fixes CoreHir child-index handling and nested method
  result reloads in the shared AST/CoreHir method lowering paths. The language
  spec and migration guidance record this as a concrete `i32`-lane capability.
- Acceptance remains open: arbitrary item representations, complete adapter
  composition, full eager/lazy migration, trait-driven `for` desugaring, and
  the final RFC-011 residual audit still require work.

## Progress receipt — 2026-09-20 (direct source expressions)

- Commit `c064cb46a` extends the existing concrete `IntoIterator` boundary to
  direct call and method sources in both AST and CoreHir lowering. Range syntax
  is now selected by its `NK_RANGE` node kind instead of by child count, so a
  two-argument call is not misclassified as a numeric range.
- `iterator_range_basic.ark` verifies
  `for item in core::range_new(2, 5)` and
  `for item in core::range_inclusive_new(3, 5).into_iter()`; each source is
  lowered once, materialized into a local, and then follows the same concrete
  conversion/`next` path as a named local.
- Both `wasm32-gc` component and linear `wasm32` artifacts compile and
  validate; both runtimes exit 0. Linear WAT confirms
  `Range::into_iter`/`RangeInclusive::into_iter` and the canonical range
  iterator `next` calls for the direct expressions.
- The remaining scope is generic or unregistered source dispatch, arbitrary
  item representations, complete adapter composition, full eager/lazy
  migration, the required negative/diagnostic fixture, and the final RFC-011
  residual audit. This receipt does not close #860 or mark its acceptance
  boxes complete.

## Progress receipt — 2026-09-20 (for source diagnostic contract)

- Commit `3be855820` adds type checking for `for` sources. A statically known
  source must satisfy `IntoIterator`, `Iterator`, or the existing concrete
  `Type::next` structural fallback; built-in range syntax remains on its
  dedicated lowering path. This preserves `iterator/custom_iterator.ark`
  while rejecting a scalar source that has no iteration contract.
- `tests/fixtures/selfhost/for_deny_non_iterable.ark` is registered as a
  selfhost diagnostic fixture with a committed `.selfhost.diag` golden. The
  current compiler emits `E0200` with the `non-iterable source` message and
  exits 1; the fixture passes its diagnostic parity check.
- `python3 scripts/manager.py verify lane` exits 0 after formatter, quality,
  lint-smoke, and touched-code ratchet checks. Existing iterator range,
  adapter, and custom structural-iterator fixtures remain typecheck-valid.
- Full diagnostic parity still has one unrelated existing W0005 span golden
  drift in `diagnostics/capability_stub.ark` (`4:4` golden versus `4:8`
  current output); it is not attributed to this iterator change and remains
  open for its owning diagnostics work.

The remaining scope is generic or unregistered source dispatch, arbitrary item
representations, complete adapter composition, full eager/lazy migration, and
the final RFC-011 residual audit. This receipt does not close #860 or mark its
acceptance boxes complete.

## Progress receipt — 2026-09-20 (lazy adapter source in `for`)

- Commit `dacac461e` extends `iterator_adapters.ark` with a direct `for` over
  `v.iter().map(double).filter(is_even).take(2)`. The body consumes the
  composed method-shaped adapter through its canonical `next` path and checks
  the resulting aggregate without introducing an intermediate collection.
- The `wasm32-gc` component and linear `wasm32` artifact both compile and pass
  `wasm-tools validate`; both execute successfully under Wasmtime and print
  `OK`. WAT inspection shows the same `MapFilterTakeIter::next` call after the
  adapter construction; GC uses the typed call path and linear Wasm uses the
  fallback callback/table representation.
- `python3 scripts/manager.py verify lane` exits 0 after the fixture change.
  This is concrete `i32`-lane evidence only: generic or unregistered source
  dispatch, arbitrary item representations, broader adapter composition, full
  eager/lazy migration, and the RFC-011 residual audit remain open. This
  receipt does not close #860 or mark its acceptance boxes complete.

## Progress receipt — 2026-09-20 (adapter identity conversion)

- Commit `ce6a83389` adds identity `IntoIterator` implementations for the
  concrete lazy adapters, including `MapIter`, `FilterIter`, `TakeIter`,
  `SkipIter`, their supported composed forms, `ChainIter`, `EnumerateIter`,
  and `ZipIter`. Each conversion returns the existing iterator state, so the
  source sequence and ownership behavior are unchanged.
- The direct adapter `for` fixture now lowers through
  `MapFilterTakeIter::into_iter` followed by `MapFilterTakeIter::next` on both
  targets. GC uses the typed reference call path; linear Wasm uses the same
  source-level conversion with its fallback representation. Both artifacts
  validate and run successfully with `OK`.
- `verify lane` passes after refreshing the formatter exception hash for the
  changed stdlib source. Generic item representations, broader adapter
  composition, full eager/lazy migration, and the RFC-011 residual audit
  remain open. This receipt does not close #860 or mark its acceptance boxes
  complete.

## Progress receipt — 2026-09-20 (eager helper delegation)

- Commit `8384f8115` removes the duplicate hand-written traversals from
  `collect_map_filter_i32` and `collect_map_filter_take_i32`. Both compatibility
  entry points now build the method-shaped lazy chain and finish through the
  canonical `Iterator::collect` path.
- `iterator_adapters.ark` checks the full map/filter result and the bounded
  map/filter/take result. The GC component and linear Wasm artifacts compile,
  validate, and run with `OK`; WAT shows `MapIter::filter` and
  `MapFilterIter::take` calls inside the compatibility entries rather than a
  second source-index loop.
- `verify lane` passes. The remaining scope is generic item representations,
  broader adapter composition, manifest-level deprecation, full migration, and
  the RFC-011 residual audit; this receipt does not close #860.

## Progress receipt — 2026-09-20 (generic `Iterator` `for` lowering)

- Test commit `c50043518` adds `sum_for<T: Iterator>`, whose body consumes
  the generic parameter directly with `for`. The fixture instantiates it with
  `VecIter`, and monomorphization emits the same identity
  `IntoIterator::into_iter` plus canonical `Iterator::next` sequence as the
  handwritten concrete loop.
- `iterator_vec_iter_for_loop.ark` compiles and runs successfully on both the
  `wasm32-gc` component path and the linear-memory `wasm32` path. Both Wasm
  artifacts validate; the GC and linear WAT show `sum_for__VecIter` calling
  `VecIter::into_iter` before `VecIter::next`.
- This proves the bounded `T: Iterator` case only. Generic sources whose
  iteration obligation is unresolved, arbitrary item representations, dynamic
  dispatch, broader adapter composition, and the RFC-011 residual audit remain
  open. This receipt does not close #860.

## Progress receipt — 2026-09-20 (generic `IntoIterator` `for` lowering)

- Test commit `5492a9d81` extends the generic `for` fixture with
  `sum_into_for<T: IntoIterator>`. Its `Vec<i32>` instantiation lowers through
  `Vec::into_iter`, yielding `VecIter`, and then uses the canonical
  `VecIter::next` loop.
- The GC component and linear-memory Wasm artifacts compile and validate, and
  both runtimes complete successfully. WAT shows
  `sum_into_for__Vec_i32_` calling `Vec::into_iter` before `VecIter::next` on
  both targets.
- This verifies a concrete monomorphized `T: IntoIterator` case; unresolved
  generic obligations, arbitrary item representations, dynamic dispatch,
  broader adapter composition, and the RFC-011 residual audit remain open.
  This receipt does not close #860.

## Progress receipt — 2026-09-20 (generic range `for` lowering)

- Test commit `e9ee279b5` extends `iterator_range_basic.ark` with the generic
  `sum_into_for<T: IntoIterator>` consumer for both `Range` and
  `RangeInclusive` direct expressions. The half-open `[2, 5)` and closed
  `[3, 5]` totals remain distinct and correct.
- GC component and linear-memory Wasm compile, validate, and run successfully.
  WAT shows separate monomorphized `sum_into_for__Range` and
  `sum_into_for__RangeInclusive` functions calling their corresponding
  `into_iter` and canonical `next` methods.
- This is still static monomorphized dispatch for implemented `i32` ranges;
  arbitrary item representations, dynamic dispatch, broader adapter
  composition, manifest-level deprecation, and the RFC-011 residual audit
  remain open. This receipt does not close #860.

## Progress receipt — 2026-09-20 (generic lazy adapter `for` lowering)

- Test commit `6c92fa741` adds a generic `sum_into_for<T: IntoIterator>` call
  whose source is the composed `Vec<i32>` adapter
  `iter().map(double).filter(is_even).take(2)`. The monomorphized body uses
  `MapFilterTakeIter::into_iter` and then its canonical `next` loop.
- The GC component and linear-memory Wasm artifacts compile, validate, and run
  with `OK`. WAT confirms `sum_into_for__MapFilterTakeIter` calls the identity
  conversion immediately before `MapFilterTakeIter::next` on both backends.
- This proves generic adapter conversion for the implemented `i32` lane only;
  arbitrary item representations, dynamic dispatch, broader composition,
  manifest-level deprecation, and the RFC-011 residual audit remain open.
  This receipt does not close #860.

## Progress receipt — 2026-09-20 (map/take adapter composition)

- Commit `b0849dee4` adds the concrete `MapTakeIter` adapter and the
  `MapIter::take` method. It preserves the upstream map callback and explicit
  remaining-count state, then delegates `collect`, `count`, `sum`, and `fold`
  to the canonical iterator consumers.
- `iterator_adapters.ark` checks `v.iter().map(double).take(2).collect()`.
  GC and linear-memory artifacts compile, validate, and run with `OK`; WAT
  shows `MapIter::take` constructing the adapter and `MapTakeIter::next`
  driving collection on both backends.
- This expands only the supported concrete `i32` composition set. Arbitrary
  item representations, dynamic dispatch, all adapter combinations,
  manifest-level deprecation, and the RFC-011 residual audit remain open.
  This receipt does not close #860.

## Progress receipt — 2026-09-20 (skip/take adapter composition)

- Commit `317583de9` adds the concrete `SkipTakeIter` adapter and the
  `SkipIter::take` method. It preserves skip-before-take state and delegates
  `collect`, `count`, `sum`, and `fold` to the canonical iterator consumers.
- `iterator_adapters.ark` checks `v.iter().skip(2).take(2).collect()` and a
  generic `T: IntoIterator` consumer. GC and linear-memory artifacts compile,
  validate, and run with `OK`; WAT shows `SkipIter::take` constructing the
  adapter and `SkipTakeIter::next` driving collection and generic `for` on both
  backends.
- This expands only the supported concrete `i32` composition set. Arbitrary
  item representations, dynamic dispatch, all adapter combinations,
  manifest-level deprecation, and the RFC-011 residual audit remain open.
  This receipt does not close #860.

## Progress receipt — 2026-09-20 (filter/take adapter composition)

- Commit `5a1408aac` adds the concrete `FilterTakeIter` adapter and the
  `FilterIter::take` method. It preserves the predicate and explicit
  remaining-count state, then delegates `collect`, `count`, `sum`, and `fold`
  to the canonical iterator consumers.
- `iterator_adapters.ark` checks
  `v.iter().filter(is_even).take(1).collect()` and a generic
  `T: IntoIterator` consumer. GC and linear-memory artifacts compile, validate,
  and run with `OK`; WAT shows `FilterIter::take` constructing the adapter and
  `FilterTakeIter::next` driving collection and generic `for` on both backends.
- This expands only the supported concrete `i32` composition set. Arbitrary
  item representations, dynamic dispatch, all adapter combinations,
  manifest-level deprecation, and the RFC-011 residual audit remain open.
  This receipt does not close #860.

## Progress receipt — 2026-09-20 (std::seq compatibility delegation)

- Commit `d31cb9ec3` routes the concrete `std::seq` map/filter/take/skip/fold
  wrappers through `std::core::iter`. The compiler keeps the qualified wrapper
  bodies live and reserves the legacy HOF classification for prelude/intrinsic
  compatibility names.
- `stdlib_seq/seq_map_filter.ark` and `stdlib_seq/seq_take_skip_fold.ark`
  compile, validate, and run on both `wasm32-gc` and linear-memory `wasm32`.
  Their outputs match exactly, and WAT confirms the canonical iterator
  consumers are called from the public wrappers on both backends.
- `verify lane` passes. The remaining scope is arbitrary item representations,
  broader adapter composition, manifest-level deprecation, and the RFC-011
  residual audit; this receipt does not close #860.

## Progress receipt — 2026-09-20 (item-preserving String lane and consumer split)

- Commit `e6d474dad` adopts [ADR-076](../../docs/adr/ADR-076-language-v2-iterator-consumer-boundary.md).
  `Iterator` now keeps the associated `Item`, `next`, `collect`, and `count`
  producer contract separate from the numeric `IteratorSum<Item, Output>` and
  `IteratorFold<Item, Accumulator>` capability traits. Existing numeric
  adapters implement the concrete `i32` capabilities; non-numeric producers
  are not forced to return a fabricated numeric value.
- The same commit adds `StringIter`, `IntoIterator for Vec<String>`, and
  `FromIterator<StringIter> for Vec<String>`. The positive
  `iterator_string.ark` fixture covers direct and generic `Iterator`/
  `IntoIterator` calls, item-preserving `collect`/`count`, `Vec::from_iter`,
  method-shaped collection, and direct `for` iteration over `Vec<String>`.
- The compiler fixes qualified generic-bound parsing, concrete `Vec<String>`
  method/associated-function symbol selection, and iterator-loop payload
  typing. GC `for` lowering selects the typed `Option::Some` payload layout;
  the linear fallback selects the corresponding reference value type.
  This keeps String payloads out of the numeric i32 layout without packing.
- `iterator_string.ark`, `iterator_vec_iter_for_loop.ark`,
  `iterator_range_basic.ark`, and `iterator_adapters.ark` compile and pass
  Wasm validation and runtime assertions on both `wasm32-gc` component and
  `wasm32` linear-memory paths. `python3 scripts/manager.py verify lane`
  passes after the compiler rebuild.
- The broad `verify lane --gate t3` remains red on the pre-existing core-Wasm
  validation backlog (72 validation failures and 30 compile failures in that
  run); this receipt does not claim the full T3 corpus or close the issue.
  Remaining work includes additional item representations and adapter
  generalization, manifest-level deprecation, and the RFC-011 residual audit.

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
  This receipt records fixture alignment only and does not close #860.

## Progress receipt — 2026-09-20 (`Vec<String>::iter()` producer entry point)

- The follow-up implementation extends the concrete String lane with
  `impl Vec<String> { fn iter(...) -> StringIter }`. The MIR/WAT receipt names
  the emitted method `Vec_String::iter`; it does not fall back to the numeric
  `Vec::iter` method or pack String references into i32 values.
- The updated `iterator_string.ark` fixture exercises
  `method_source.iter().collect()` and retains the direct/generic producer,
  `FromIterator`, and `for` assertions. The current stage-2 compiler produces
  both GC and linear core modules; `wasm-tools validate` passes for each,
  linear core execution passes, and the GC component validates and executes
  through the supported stdio component path.
- This receipt is limited to the concrete String producer entry point. The
  remaining adapter/item generalization, manifest-level deprecation, and
  RFC-011 residual audit remain open; it does not close #860.

## Progress receipt — 2026-09-20 (String item-preserving `take` adapter)

- The concrete String lane now includes `StringIter::take(n)`, which produces
  `StringTakeIter` and keeps the source `String` item type through the
  canonical `Iterator::collect` traversal. The positive fixture exercises
  `source.iter().take(1).collect()` and checks that exactly the first item is
  retained.
- GC and linear core modules compile and validate; the linear module runs
  successfully. The GC component validates and runs. WAT confirms the
  `Vec_String::iter -> StringIter::take -> StringTakeIter::collect` call chain,
  with no numeric item packing.
- This is a no-callback adapter boundary. Callback-bearing generic
  `map`/`filter`, broader arbitrary-item composition, manifest-level
  deprecation, and the RFC-011 residual audit remain open; #860 stays open.

## Progress receipt — 2026-09-20 (String `skip` / `skip().take()` composition)

- The String lane now includes `StringIter::skip(n)` and the composed
  `StringSkipIter::take(m) -> StringSkipTakeIter` path. The fixture verifies
  that `source.iter().skip(1).take(1).collect()` returns the second item and
  no other value.
- GC and linear core modules compile and validate, and the linear module runs.
  The GC component validates and runs. WAT shows
  `Vec_String::iter -> StringIter::skip -> StringSkipIter::take ->
  StringSkipTakeIter::collect`, preserving reference payloads across the
  composition.
- This remains a concrete no-callback item lane. Generic callback adapters,
  broader arbitrary-item composition, manifest-level deprecation, and the
  RFC-011 residual audit remain open; #860 stays open.

## Progress receipt — 2026-09-20 (numeric consumer capability diagnostic)

- Commit `3069c7142` hardens direct concrete `sum` / `fold` method resolution
  against the accepted ADR-076 capability boundary. A receiver that implements
  `Iterator` but not the matching `IteratorSum<i32, i32>` or
  `IteratorFold<i32, i32>` now receives E0200 during type checking instead of
  lowering an unresolved method to a runtime `unreachable` stub.
- `iterator_vec_iter_next.ark` adds a direct concrete `VecIter.sum()` positive
  call. It compiles, validates, and runs on the GC component and linear-memory
  Wasm paths. `selfhost/iterator_string_numeric_consumer.ark` is the negative
  diagnostic fixture for `StringIter.sum()`; selfhost diagnostic parity passes
  for it.
- The full `python3 scripts/manager.py selfhost diag-parity` gate still has the
  pre-existing `diagnostics/capability_stub.ark` W0005 golden drift; the new
  fixture is not the failing case. `verify lane` passes. This receipt does not
  close #860: arbitrary item-generic adapters, manifest-level deprecation, and
  the final RFC-011 residual audit remain open.

## Progress receipt — 2026-09-20 (String callback adapters)

- Commit `2b862af04` extends the concrete non-numeric lane with lazy
  `StringIter::map(fn(String) -> String)` and
  `StringIter::filter(fn(String) -> bool)` adapters. Their producer methods,
  identity `IntoIterator` conversions, and concrete `FromIterator` impls keep
  `Iterator::Item = String` instead of routing through a numeric ABI.
- `iterator_string_callbacks.ark` is the positive fixture. It compiles,
  validates, and runs on both the GC primary component path and the
  linear-memory fallback. Existing String no-callback and numeric callback
  iterator fixtures remain green. WAT inspection shows GC typed callback
  indices 46 (`String -> String`) and 48 (Memory64 `String -> bool`) reused by
  the struct field, local declaration, and `call_ref`; linear keeps
  `call_indirect` and table slots.
- The callback lowering carries the semantic function marker through MIR and
  derives the callback result local type before Wasm emission. This closes the
  concrete String callback ABI boundary without claiming dynamic dispatch or
  arbitrary item adapters.
- `Vec::from_iter(source.iter().map(...))` is still outside the verified
  contract because the existing generic associated-function lowering emits an
  invalid `drop; unreachable` path for the projected adapter source. The
  removal condition is a valid GC and linear artifact plus runtime assertion
  for that call; until then, the explicit adapter `FromIterator` impls are not
  treated as complete integration evidence. Eager/lazy migration,
  manifest-level deprecation, and the RFC-011 residual audit remain open.

## Progress receipt — 2026-09-20 (String adapter FromIterator dispatch)

- Commit `3a0b3b3ee` fixes the remaining concrete String callback collection
  boundary. When `Vec::from_iter` receives `StringMapIter` or
  `StringFilterIter`, MIR associated-function rewriting now selects
  `Vec_String::from_iter__StringMapIter` or
  `Vec_String::from_iter__StringFilterIter`; the result local keeps its String
  vector reference type instead of falling back to `i64` and `unreachable`.
- `iterator_string_callbacks.ark` now asserts both direct `Vec::from_iter`
  calls. GC primary component and linear-memory fallback artifacts validate and
  run successfully, with matching collection order and ownership behavior.
- The earlier receipt's removal condition is satisfied for these two concrete
  adapter sources. Generic arbitrary item adapters, broader composition,
  manifest-level deprecation, and the final RFC-011 residual audit remain open;
  #860 is not closed.

## Progress receipt — 2026-09-20 (generic String FromIterator monomorphization)

- Commit `5ac15709a` adds `collect_from_iterator<T: Iterator>` to the String
  callback fixture. Its `Vec::from_iter(iterator)` body is statically
  monomorphized for `StringMapIter` and resolves to
  `Vec_String::from_iter__StringMapIter`; no dynamic collection or integer
  packing is introduced.
- The GC component and linear-memory artifacts validate and run with matching
  String assertions. WAT shows the generic specialization returning the same
  GC reference / linear handle type as the concrete associated-function path.
- This advances the bounded projection case only. Arbitrary item-generic
  adapters, dynamic dispatch, broader composition, manifest-level deprecation,
  and the final RFC-011 residual audit remain open; #860 is not closed.

## Progress receipt — 2026-09-20 (String map-filter composition)

- Commit `4b0468228` adds the concrete `StringMapFilterIter` adapter and
  source-qualified `FromIterator` dispatch for it. The fixture covers both
  canonical `collect` and `Vec::from_iter` after
  `iter().map(map_identity).filter(keep_long_string)`.
- GC uses typed `call_ref` signatures 46 (`String -> String`) and 47
  (`String -> bool`); linear-memory Wasm uses the callback table and
  `call_indirect`. Both artifacts validate and run with identical String
  ordering and ownership assertions.
- This closes one concrete composition only. Arbitrary item-generic adapter
  composition, dynamic dispatch, manifest-level deprecation, and the final
  RFC-011 residual audit remain open; #860 is not closed.

## Progress receipt — 2026-09-20 (String filter-map composition)

- Commit `38c326934` adds the reverse `StringFilterMapIter` path. Its predicate
  receives a clone of the source String; only accepted originals are passed to
  `map`, preserving ownership and source order without an intermediate vector.
- The fixture verifies lazy `collect` and direct `Vec::from_iter`. GC typed
  `call_ref` and linear-memory `call_indirect` artifacts both validate and run
  with the same assertions, including the concrete
  `Vec_String::from_iter__StringFilterMapIter` symbol.
- This is another concrete String composition, not completion of arbitrary
  item-generic adapters, dynamic dispatch, manifest-level deprecation, or the
  final RFC-011 residual audit; #860 remains open.

## Progress receipt — 2026-09-20 (generic String `IntoIterator` for lowering)

- Test commit `c91f76573` adds a `total_string_lengths<T: IntoIterator>`
  consumer to `iterator_string_callbacks.ark`. Its `for value in source` body
  is instantiated with `StringFilterMapIter`, so the generic source crosses
  `StringFilterMapIter::into_iter` and the canonical `next` loop rather than a
  fixture-specific traversal.
- The current stage-2 compiler emits
  `total_string_lengths__StringFilterMapIter` for both targets. The GC
  component and linear-memory Wasm artifacts validate and run with the same
  length assertion; WAT shows the same specialized call chain, with typed
  `call_ref` on GC and table-based `call_indirect` on the fallback.
- This extends generic `for` evidence to a concrete non-numeric composed
  adapter. Arbitrary item-generic adapters, dynamic dispatch,
  manifest-level deprecation, and the final RFC-011 residual audit remain
  open; #860 is not closed.

## Progress receipt — 2026-09-20 (String callback adapters with bounds)

- Commit `ef8b0bfa2` adds lazy `StringMapTakeIter` and
  `StringFilterTakeIter` compositions. The map adapter decrements its bound
  per mapped output; the filter adapter decrements it only for accepted items
  and clones the predicate argument before yielding the owned source String.
- `iterator_string_callbacks.ark` covers canonical `collect`, source-qualified
  `Vec::from_iter`, and generic `T: IntoIterator` `for` consumers for both
  compositions. GC and linear-memory artifacts validate and run with the same
  bounded results.
- WAT exposes `Vec_String::from_iter__StringMapTakeIter` and
  `Vec_String::from_iter__StringFilterTakeIter`; callback calls remain typed
  `call_ref` on GC and table-based `call_indirect` on linear memory.
- This broadens concrete String adapter composition only. Arbitrary
  item-generic adapter families, dynamic dispatch, manifest-level deprecation,
  and the final RFC-011 residual audit remain open; #860 is not closed.

## Progress receipt — 2026-09-20 (String skip callback composition)

- Commit `da440bc97` adds `StringSkipMapIter` and `StringSkipFilterIter` to
  the canonical `Iterator` / `IntoIterator` / `FromIterator` paths. Skip state
  is consumed before either callback; the filter callback gets a cloned String
  handle and the accepted original continues through the iterator.
- `iterator_string_callbacks.ark` verifies `collect`, source-qualified
  `Vec::from_iter`, and generic `T: IntoIterator` consumption for both
  `skip(1).map(...)` and `skip(1).filter(...)`. The GC component and
  linear-memory Wasm artifacts compile, validate, and run with identical
  source-level results.
- The generated symbols and WAT show typed GC `call_ref` and fallback
  table-based `call_indirect` callback paths for the same source semantics.
  This remains a concrete String monomorphization receipt, not arbitrary
  item-generic adapter support.
- Generic item representations, dynamic dispatch, manifest-level deprecation,
  and the final RFC-011 residual audit remain open; #860 is not closed.

## Progress receipt — 2026-09-20 (numeric skip callback composition)

- Commit `0115f6619` adds `SkipMapIter` and `SkipFilterIter` to the canonical
  numeric `Iterator` / `IntoIterator` / `FromIterator` paths. Their `sum` and
  `fold` capability implementations reuse the canonical traversal helpers.
- `iterator_adapters.ark` covers direct `collect`, `Vec::from_iter`, and a
  generic `T: IntoIterator` consumer for both `skip(2).map(...)` and
  `skip(2).filter(...)`; the same fixture asserts their numeric totals. GC
  component and linear-memory Wasm validate and run with identical results.
- GC WAT uses typed `call_ref` and the fallback WAT uses table-based
  `call_indirect` for the callback-bearing adapter, proving the same source
  protocol across the two representations.
- This is bounded `Vec<i32>` evidence only. Generic item representations,
  dynamic dispatch, manifest-level deprecation, and the final RFC-011 residual
  audit remain open; #860 is not closed.

## Progress receipt — 2026-09-20 (bounded String callback composition)

- Commit `cee857bb7` adds `StringMapFilterTakeIter` and
  `StringFilterMapTakeIter`. Both preserve lazy single-pass traversal and
  apply the bound to accepted output items; each owning predicate receives a
  cloned String handle before the original or mapped value is returned.
- The callback fixture covers `collect`, source-qualified `Vec::from_iter`,
  and generic `T: IntoIterator` `for` consumers for both callback orders. GC
  and linear-memory artifacts validate and run with identical one-item
  results.
- WAT shows the concrete `...MapFilterTakeIter` and
  `...FilterMapTakeIter` collection symbols, typed GC `call_ref`, and linear
  table-based `call_indirect`. Arbitrary item-generic adapter families,
  dynamic dispatch, manifest-level deprecation, and the final RFC-011 audit
  remain open; #860 is not closed.

## Progress receipt — 2026-09-23 (manifest deprecation + residual audit)

- Commit `0148c9703` (#691 slice) registers the `std::core::iter` manifest
  surface (module, five trait types, 30 function entries) and marks the six
  eager `std::seq` pipeline helpers `deprecated` with `deprecated_by`
  targets under the ADR-076 section 4 contract; generated docs regenerated
  with badges. Deprecated entries verified callable on both targets.
- The RFC-011 item 37 residual audit is recorded in
  `docs/stdlib/migration-guidance.md` ("Iterator v2 status" section).
  Residuals owned by THIS issue: arbitrary item-generic adapter composition
  (generic adapter types over `I: Iterator` plus stored callbacks exceed
  current monomorphization), generic `for` source dispatch and the
  structural `next()` fallback boundary, and the final residual-zero
  review once #691 closes. HashMap iterator intrinsics and seq query
  helpers are tracked under #691; `verify quick` remains red only on
  unrelated pre-existing gates.

## Progress receipt — 2026-09-23 (per-instantiation impl owners)

- Commit `bfefffa2a` generalizes the HashMap_K_V collision rule to user
  generic structs: `impl Trait for GenStruct<Concrete>` registers methods
  under a per-instantiation owner (`GenTakeIter_VecIter::next`), so two
  instantiations of one generic struct can carry independent trait impls.
  Typechecker, resolver, CoreHir impl-target naming, and MIR method
  resolution share the same owner-key rule (concrete args join as
  `Name_arg1_arg2`; type-param args keep the bare owner; fn args key as
  `fnref`; `Vec`/`HashMap` keep established spellings).
- Impl-method return type names and concrete generic param local types now
  keep the canonical `Name<args>` spelling instead of the bare base name,
  so `for` loops over `GenTakeIter<VecIter>` resolve `into_iter`/`next`
  against `GenTakeIter_VecIter` and field access derives the substituted
  layout. Generic enums (`Result`/`Box`/`Slice`) keep the bare base name
  their layout handling expects.
- `tests/fixtures/generics_v1/generic_inst_trait_impl.ark` covers the
  `GenTakeIter<VecIter>` + `GenTakeIter<StringIter>` impl collision, a
  generic `first_i32<T: Iterator>` consumer, `collect`/`count` default
  methods, and `for` over the adapter. It compiles and runs identically
  on wasm32 and the wasm32-gc component (`2 2 11 2 11`); `verify lane`
  is green.
- Remaining #860 residuals: generic `for` source dispatch plus the
  structural `next()` fallback boundary, blanket/generic trait impls
  (`impl<I: Iterator> Iterator for X<I>` is still compiler-owned per
  ADR-062 review), adapter migration to `I: Iterator` bodies, and the
  final residual-zero audit once #691 closes. #860 is not closed.

## Progress receipt — 2026-09-23 (generic for dispatch + structural next boundary)

- Commit `5291c9a5c` bounds generic `for` sources and tightens the
  structural `next()` fallback. A `for` source that resolves to a type
  variable must now declare `IntoIterator` or `Iterator` in the current
  function's parameter bounds; an unbounded `T` fails typecheck with
  `non-iterable source: expected IntoIterator or Iterator bound`. The
  structural fallback (types without a trait impl) now verifies the
  producer contract `next(self) -> Option<T>` instead of accepting any
  `Name::next` symbol, per ADR-076's producer/consumer boundary.
- The three duplicated current-function-bound scans (`call_method`,
  `ownership_types`, `operator_traits`) now share
  `trait_bounds::env_type_var_has_trait_bound`; no behavior change for
  `Copy` or operator-trait checks.
- Positive: `iterator_query_consumers.ark` adds `sum_via_iter<T: Iterator>`
  driving `for` directly over an `Iterator` bound (IntoIterator was
  already covered by `sum_into_for`); it compiles and runs identically on
  wasm32 and the wasm32-gc component. `custom_iterator.ark` confirms the
  structural path still iterates `next(self) -> Option<i32>` types.
- Negative: `for_deny_unbounded_generic_source.ark` and
  `for_deny_non_option_next.ark` are registered `diag:` fixtures with
  committed `.selfhost.diag` goldens; both pass diag parity.
- `verify lane` exits 0; diag-parity reports PASS=57 SKIP=26 FAIL=1 where
  the single failure is the pre-existing `capability_stub.ark` W0005 span
  drift recorded above, not this change.
- Remaining #860 residuals: blanket/generic trait impls (`impl<I: Iterator>`
  `Iterator for X<I>` is still compiler-owned per ADR-062 review), adapter
  migration to `I: Iterator` bodies, and the final residual-zero audit
  once #691 closes. #860 is not closed.

## Progress receipt — 2026-09-23 (parametric trait impls, ADR-078)

- ADR-078 (`docs/adr/ADR-078-language-v2-parametric-trait-impl.md`,
  ACCEPTED) splits parametric impls (`impl Iterator for Ctor<T>`) from
  blanket impls (`impl<T> Trait for T`, still E0216). Coverage rule:
  every trait-reference variable must appear in the self constructor;
  overlap rule: at most one impl per (trait, constructor head) when
  either side carries variables.
- Commit `0c5af3dc5` implements the split. `trait_registry` admits
  var-carrying impls only when the self type is a nominal constructor
  application and trait-ref vars are covered by it; parametric-vs-any
  overlap on the same (trait, ctor head) reports E0215. Associated
  types (`type Item = T`) register for parametric impls and
  `lookup_associated_type` resolves them by pattern-matching the
  receiver against the impl pattern and instantiating the result.
- `for` desugaring now works over parametric impls: `check_for_source`
  records a mono instance, instantiated return type, and a span-keyed
  call-site marker for each desugared `into_iter`/`next` call
  (`record_for_desugar_mono`). MIR resolution
  (`mir_for_desugar_callee`) tries the per-instantiation owner, then the
  bare constructor owner, resolving through the span rewrite first and
  the registered name rewrite as fallback — so `for` over
  `WrapIter<i32>` emits `WrapIter__next__i32`, and a `WrapIter<String>`
  loop in the same program resolves the String specialization.
- Positive fixture `generics_v1/parametric_trait_impl.ark` covers
  `impl Iterator for WrapIter<T>` (direct `next` + `for`), a second
  `WrapIter<String>` instantiation, and `impl IntoIterator for Once<T>`
  chained into `WrapIter<T>`; it runs identically on wasm32 (`7 9 hi
  11`) and the wasm32-gc component. Negative probes verified: bare-var
  self (`impl Iterator for T`) stays E0216, parametric+concrete overlap
  reports E0215, and an unconstrained trait-ref var (`impl N3<T> for
  P3`) stays E0216.
- `verify lane` exits 0. Known pre-existing defect observed while
  testing: generic struct construction at `WrapIter<String>` produces
  invalid GC core Wasm on the Sep-19 compiler too (func type mismatch)
  — unrelated to this change; the fixture's `run:` lane is wasm32.
- Remaining #860 residuals: adapter migration to `I: Iterator` bodies
  (std adapters still use concrete lanes), generic `for` sources over
  bare type parameters rely on the name-rewrite fallback rather than a
  recorded marker, and the final residual-zero audit once #691 closes.
  #860 is not closed.

## Progress receipt — 2026-09-23 (nested mono propagation)

- Commit `088fc3204` closes the nested-monomorphization gap left by
  parametric impls: `self.iter.next()` inside `impl Iterator for
  Twice<I>` at `I=WrapIter<i32>` now resolves and runs identically on
  wasm32 and wasm32-gc (`param_adapter` probe prints `5` on both).
- Three layers were needed. (1) The typechecker cannot see inside impl
  bodies, so `record_mono_instance` now propagates: each concrete
  generic arg `Ctor<X..>` is matched against var-carrying
  `env.trait_impls` self patterns, and the impl's trait methods are
  recorded at the bound arguments — filtered by `fn_sig_index` so
  trait defaults without a body (`collect`, `count`) do not record
  phantom instances. (2) MIR callee resolution missed arg-less calls
  (`first_method_arg_idx=-1`), so `mir_parametric_impl_callee` maps
  the per-instantiation owner spelling `WrapIter_i32::next` back to
  the propagated instance `WrapIter__next__i32`. (3)
  `instantiate_type` never reduces `I::Item` projections, leaving
  `Option<I::Item>` as the recorded return type and breaking GC enum
  variant selection (invalid core Wasm); `normalize_projection_ty`
  reduces projections through the impl's associated types, recursing
  for chains (`I::Item -> WrapIter<i32>::Item -> i32`).
- Verified on the rebuilt s2: `param_adapter` -> `5` on wasm32 and
  the wasm32-gc component; `param_iter` -> `7 9`;
  `generics_v1/parametric_trait_impl.ark` -> `7 9 hi 11` on wasm32.
  `verify lane` exits 0 (W0010/W0003 style warnings only).
- Known limitation: propagation resolves through `ctx_has_fn_name`,
  so the propagated impl must be emitted before the consuming impl
  body lowers (source-order constraint; forward references still
  fail). The pre-existing `WrapIter<String>` GC construction defect
  is unchanged and tracked separately.
- Remaining #860 residuals: adapter migration to `I: Iterator`
  bodies, generic `for` over bare type parameters (name-rewrite
  fallback only), forward-ref emit ordering, and the residual-zero
  audit once #691 closes. #860 is not closed.

## Progress receipt — 2026-09-23 (var-pinned span concretization + qualified bounds)

- Commit `12d077031` lets generic adapter bodies iterate themselves.
  `for item in self` inside `impl Iterator for Take<I>` records span
  rewrites against the template spelling (`Take__next__?I`); MIR now
  concretizes `?Var` segments through the enclosing instance's mono
  type-param map before accepting the target, and an unresolvable
  variable falls through to the receiver-derived paths instead of
  pinning a callee that can only trap. Demand-driven propagation gains
  the sibling axis (`propagate_owner_impl_mono`): recording
  `Ctor::method`+[args] also records every method of `Ctor`'s
  var-carrying impls at the same args, since the typechecker saw the
  impl body at the bare type variable.
- The same commit makes the instantiation-blind pools safe.
  `mono_target_matches_source` is owner-aware, so
  `Take__into_iter__VecIter` can no longer steal a `Vec::into_iter`
  request; `?`-template targets stay out of the name-rewrite pool; the
  pool never overrides a real function (`ctx_has_fn_name` wins); and
  both `mir_resolve_method_callee` and `mir_for_desugar_callee` run
  span, has-fn, and parametric resolution before the pooled fallback.
- Commit `82a9580b8` fixes a bound-check escape: the `for`-source gate
  compared bound names with `eq`, so `T: iter::IntoIterator` was
  rejected as unbounded. `type_param_bounds_has_trait` now compares
  through `names::same_type_name`; `iterator_adapters.ark`
  (`run:`/`t3-compile:` fixture) compiles and runs again.
- Verified on the rebuilt s2: `gen_take_std` (generic `Take<I>` with
  `for` in `collect`/`count` plus a nested `Take<Take<VecIter>>` loop)
  prints `2`/`9` on wasm32; `gts2` (nested `outer.next()`) prints
  `4`/`5`/`none_c`; `bound_q`/`bound_u` print `3`;
  `parametric_trait_impl` -> `7 9 hi 11`, `generic_inst_trait_impl` ->
  `2 2 11 2 11`, `custom_iterator` -> `0 1 2 3 4`,
  `iterator_adapters` -> `OK`, `iterator_query_consumers` exit 0.
  `generic_inst_trait_impl` also passes the wasm32-gc component lane
  (`2 2 11 2 11`). Both `for_deny_*` diag fixtures still emit their
  committed goldens. `verify lane` exits 0.
- Known residuals unchanged: `gts3` (`let it2 = outer.iter;
  it2.next()`) still traps because field accesses are `TY_UNKNOWN` to
  the typechecker, so `Take::next`+[VecIter] is never recorded;
  generic-struct construction on wasm32-gc still emits invalid core
  Wasm (pre-existing defect, same signature as `WrapIter<String>`);
  forward references relying on emit order remain open.
- Remaining #860 residuals: adapter migration to `I: Iterator` bodies
  in `std/core/iter.ark` (e.g. `TakeIter` -> `TakeIter<I>`), the
  field-access method-call gap above, forward-ref emit ordering, and
  the final residual-zero audit once #691 closes. #860 is not closed.
+## Progress receipt — 2026-09-23 (TakeIter<I> migration + projection element resolution)

- `5b15a75a9` resolves associated-type projections inside mono bodies.
  Generic struct return names keep their argument list
  (`TakeIter<VecIter>`) for per-instantiation impl owners; propagated
  impl-method returns instantiate impl-head vars before projection
  normalization, so recorded returns are concrete. Method mono bodies
  register the declared->concrete return projection like top-level fns,
  aggregate projections decompose element-wise (`vec:I::Item` ->
  `vec:i32` also binds `I::Item` -> `i32`), and Vec constructor callees
  recorded with a projection element (`Vec__new__I__Item`) concretize
  through the same map. This fixes the wasm32-gc invalid-core-Wasm
  signature for projection-element Vec construction inside parametric
  impl bodies (structref array -> i32 array).
- `f91d1031a` migrates `TakeIter` to `TakeIter<I>`: the adapter wraps
  the source iterator instead of copying VecIter state, so any
  `I: Iterator` source is boundable (ADR-078). Capability impls stay
  per-instantiation on `TakeIter<VecIter>` per ADR-076's i32-lane
  reservation. Manifest signatures, generated docs, the formatter
  exception hash, and the fixture registry count are refreshed
  (pre-existing 2965->2966 drift from `0c5af3dc5`).
- Verified on rebuilt s2: `iterator_take_basic`, `iterator_take_zero`,
  `iterator_take_more_than_len`, `iterator_map_filter_take_pipeline`,
  `iterator_adapters` (`OK`), `iterator_query_consumers`,
  `iterator_string`, `iterator_string_callbacks`, `iterator_stress_100`,
  `generic_inst_trait_impl` (`2 2 11 2 11`), `custom_iterator`
  (`0 1 2 3 4`) all pass on wasm32; the same set passes the wasm32-gc
  component lane including the previously invalid `iterator_take_basic`
  and `iterator_adapters`. Probes `adp_b` (generic `sum_into_for` over
  `TakeIter<VecIter>`) `OK`, `takeiter_gen` `11 2 2 3`, `gen_take_std`
  `2 9`, `gts2` `4 5 none_c`, `bound_q`/`bound_u` `3`. Both `for_deny_*`
  diag fixtures emit their goldens. `verify lane` exits 0; `docs
  regenerate` is clean.
- Remaining #860 residuals: adapter migration for the other lazy
  adapters (`SkipIter` -> `SkipIter<I>` and collapsing the concrete
  `*TakeIter`/`MapFilterTakeIter` fusion types onto `TakeIter<Src>`
  instantiations; the same for map/filter chains where applicable),
  the field-access method-call gap (`let it2 = outer.iter; it2.next()`
  still traps — field accesses are `TY_UNKNOWN`), forward-ref emit
  ordering, and the final residual-zero audit once #691 closes. The
  generic-struct construction defect on wasm32-gc remains open for
  non-projection shapes (`WrapIter<String>`-class). #860 is not closed.

## Progress receipt — 2026-09-23 (field-access mono resolution)

- `53411d63c` types field accesses through registered struct
  declarations. `TypeEnv` gains a `struct_defs` table (name, declaration
  type parameters, annotated field types) populated from `all_decls`, and
  `infer_expr` resolves `base.field` by substituting declaration type
  parameters with the receiver's concrete type arguments. A `let` binding
  or method receiver built from a field access therefore keeps the
  concrete generic instantiation
  (`Take<Take<VecIter>>.iter` -> `Take<VecIter>`), so `it2.next()` and
  `outer.iter.next()` record and lower the per-instantiation impl mono
  instance `Take::next`+[VecIter] instead of trapping on an unrecorded
  body.
- Positive fixture `tests/fixtures/generics_v1/generic_struct_field_method.ark`
  covers both the let-bound and the direct receiver-chain forms; expected
  output `3 2 3 4 none_b` matches on wasm32 and on the wasm32-gc
  component lane. Rebuilt s2 passes the iterator regression set
  (`iterator_take_*`, `iterator_adapters` `OK`,
  `iterator_map_filter_take_pipeline`, `iterator_query_consumers`,
  `iterator_string`, `iterator_stress_100`, `iterator_skip_basic`,
  `custom_iterator` `0 1 2 3 4`, `generic_inst_trait_impl` `2 2 11 2 11`,
  `parametric_trait_impl` `7 9 hi 11`). `verify lane` exits 0; `docs
  regenerate` is clean at registry count 2967.
- This closes the field-access method-call residual from the previous
  receipt: field accesses are no longer `TY_UNKNOWN`, so the mono
  instance for a method reached through a struct field is recorded.
  Remaining #860 residuals: adapter migration for `SkipIter` ->
  `SkipIter<I>` (blocked on callback adapters whose fields hold
  `fn(I::Item)` values, beyond current monomorphization), `ChainIter` /
  `EnumerateIter` / `ZipIter` item representations, forward-ref emit
  ordering, the `WrapIter<String>`-class generic-struct construction
  defect on wasm32-gc for non-projection shapes, the `.fold(0, add)`
  fn-ptr-param emit bug, and the final residual-zero audit once #691
  closes. #860 is not closed.

## Progress receipt — 2026-09-23 (fn-item stack args + fold evidence)

- `ef7d132f4` fixes the pre-existing fn-ptr-param emit bug that blocked
  `.fold(0, add)`. A `MIR_REF_FUNC` sitting between a value producer and
  a call is an argument push just like `LOCAL_GET(other)`: the store
  policy now treats it as an independent producer, so earlier stack args
  (`const.i32` init, receiver) stay on the stack for the call instead of
  being forced into their temp. Previously `fold(0, add)` emitted
  `const; ref_func; call` with the init stored away, leaving
  `VecIter::fold` one stack arg short — `expected i32 but nothing on
  stack` on wasm32 and invalid core Wasm on wasm32-gc.
- `v.into_iter().fold(0, add)` now prints `6` on both wasm32 and the
  wasm32-gc component lane. The `.fold` assertions removed earlier are
  restored in `iterator_query_consumers.ark` (`v.iter().fold(0, add_acc)`
  = 18, `map(double).fold` = 36, `take(3).fold` = 8) and pass on both
  targets through the canonical `IteratorFold` path.
- Regression: iterator fixture set (`iterator_take_*`,
  `iterator_adapters` `OK`, `iterator_string_callbacks`,
  `iterator_map_filter_take_pipeline`, `iterator_skip_basic`,
  `iterator_stress_100`, `custom_iterator` `0 1 2 3 4`,
  `generic_inst_trait_impl` `2 2 11 2 11`,
  `generic_struct_field_method` `3 2 3 4 none_b`,
  `parametric_trait_impl` `7 9 hi 11`) passes on wasm32; the GC
  component lane passes for the same key fixtures. `verify lane` exits
  0 on rebuilt s2.
- Remaining #860 residuals: adapter migration for `SkipIter` ->
  `SkipIter<I>` (blocked on callback adapters whose fields hold
  `fn(I::Item)` values — the call side now works, but generic struct
  fields of fn type still exceed current monomorphization), `ChainIter` /
  `EnumerateIter` / `ZipIter` item representations, forward-ref emit
  ordering, the `WrapIter<String>`-class generic-struct construction
  defect on wasm32-gc for non-projection shapes, and the final
  residual-zero audit once #691 closes. #860 is not closed.

## Progress receipt — 2026-09-23 (generic-shell struct-literal instantiation on wasm32-gc)

- `673b803e2` closes the `WrapIter<String>`-class generic-struct
  construction defect on wasm32-gc. Root cause: inside a monomorphized
  body, `mir_emit_field_access` stores the resolved field type on the
  value local in the storage spelling `struct:Name:id`
  (`struct:VecIter:12`). `mir_struct_lit_instance_name` bound that raw
  name to the declared parameter, derived `Wrap2<struct:VecIter:12>`,
  failed layout derivation, and fell back to the unsubstituted `Wrap2`
  layout — so `struct.new_default` picked a placeholder ref field and
  the module failed validation (`func 19 type mismatch` on
  `Skip__rewrap__VecIter`). `ctx_derive_generic_struct_type_name` now
  strips the storage spelling via `ctx_struct_semantic_type_name` before
  mono resolution, so the binding sees `VecIter` and derives
  `Wrap2<VecIter>` (canonicalizes to the same GC type index as
  `Skip<VecIter>`). `ctx_type_var_names_from_local_type` also collects
  type variables from `Name<args>` local spellings so non-param locals
  in mono bodies contribute to the ordinal pairing.
- Verified on the `skip_gen` probe (parametric `Skip<I>` + `SkipMap<I>`
  with a `fn(i32) -> i32` field, inherent `Skip::map` constructing
  `SkipMap<I>`, `impl Iterator for Skip<I>` with `I::Item` projection,
  per-instantiation `impl Iterator for SkipMap<VecIter>`): prints `3` /
  `4` on both wasm32 and the wasm32-gc component lane; the GC core
  module now validates. New fixture
  `tests/fixtures/generics_v1/generic_inherent_struct_lit.ark` is
  registered as `run:` + `t3-run:` (registry 2969) and passes on both
  targets. `verify lane` exits 0; `docs regenerate` is clean.
- This unblocks the `SkipIter` -> `SkipIter<I>` migration: the probe
  proves the full target shape (parametric shell + fn-valued field +
  per-instantiation Iterator impl) works on both backends.
- Remaining #860 residuals: `SkipIter` -> `SkipIter<I>` std migration
  itself (now unblocked), `ChainIter` / `EnumerateIter` / `ZipIter`
  item representations, forward-ref emit ordering, and the final
  residual-zero audit once #691 closes. #860 is not closed.

## Progress receipt — 2026-09-23 (SkipIter<I> migration + from_iter owner-key resolution)

- `40b3c9652` migrates `SkipIter`, `SkipMapIter`, and `SkipFilterIter`
  to the source-wrapping shape (`iter: I, to_skip`) matching
  `TakeIter<I>`. `impl Iterator for SkipIter<I>` is parametric with
  `Item = I::Item`; `impl IntoIterator for SkipIter<I>` likewise. The
  callback adapters keep `fn(i32)` fields (the i32 lane) and register
  `Iterator`, `IntoIterator`, `IteratorSum`, `IteratorFold`,
  `IteratorContains`, `IteratorCountEq`, and `FromIterator` per
  instantiation (`SkipMapIter<VecIter>` / `SkipFilterIter<VecIter>` /
  `TakeIter<SkipIter<VecIter>>`). `collect`/`count` on `SkipIter<I>`
  inline the `for item in self` loops like `TakeIter<I>` — calling the
  generic `collect_iterator`/`count_iterator` helpers from a
  monomorphized body leaves a bare generic callee that prunes to a
  phantom fid and traps.
- The same commit fixes `Vec::from_iter` for generic-instantiation
  sources: `mir_bare_receiver_type_name` returned `SkipMapIter<VecIter>`
  (or `struct:SkipMapIter<VecIter>:id`) verbatim, so the associated-fn
  candidate `Vec::from_iter__SkipMapIter<VecIter>` never matched the
  emitted `Vec::from_iter__SkipMapIter_VecIter` and the call stayed a
  phantom `Vec::from_iter` (fid=-1, unreachable trap). It now routes
  generic spellings through `mir_inst_owner_name`, matching
  `impl_conversion_source_type_name`'s owner-key flattening.
- Verified: `iterator_adapters` `OK`, `iterator_skip_basic`,
  `iter_skip`, `iterator_skip_zero`, `iterator_skip_more_than_len`,
  `seq_take_skip_fold` (`3 1 3 3 3 5 15 120 5 0`),
  `iterator_query_consumers`, `iterator_string` on wasm32;
  `iterator_adapters` `OK` on the wasm32-gc component lane; probe
  chains `skip(2).take/map/filter.collect`, `sum_into_for`, and
  `Vec::from_iter` all print expected values on wasm32. `verify lane`
  exits 0; the formatter-exception hash for `std/core/iter.ark` is
  refreshed.
- Remaining #860 residuals: `ChainIter` / `EnumerateIter` / `ZipIter`
  item representations (still concrete Vec<i32> state), forward-ref
  emit ordering, and the final residual-zero audit once #691 closes.
  #860 is not closed.

## Progress receipt — 2026-09-23 (struct-payload binds + generic struct-literal instantiation)

- `fa8396c9b` fixes `match o { Option::Some(q) => q.field }` returning
  field 0 for every struct payload (`Option<Point>`,
  `Option<Pair<i32,i32>>`, user enum `Wrapper::Wrap(Point)`) and a
  wasm32-gc cast trap on generic struct payloads.
- Bind side: `core_infer_payload_set_type` /
  `set_gc_payload_bind_type_if_needed` (CoreHir + AST twins) and
  `mir_infer_payload_set_type` now derive generic instantiations through
  `ctx_derive_generic_struct_type_name` and keep the bare layout name
  whenever the layout registry knows the struct — the GC-slot-only check
  silently rewrote struct binds to `enum:Name`, so
  `ctx_field_index` missed and every field read offset 0.
- Let-annotation side: `apply_let_annotation_type` promotes locals whose
  annotation names a registered struct/enum to VT_GC_REF via the named
  predicate `let_annotation_is_gc_ref_type`; kept VT_I32 made
  `Option::Some(p)` register an `{i32,i32}` variant and emit
  `i32.wrap_i64` on a ref (invalid wasm).
- Construction side: `gc_payload_typed_variant_name` maps
  `struct:Name:size` storage spellings through
  `ctx_struct_semantic_type_name`, and
  `mir_struct_lit_instance_name` unwraps `Option<T>` expected types and
  recovers scalar type args from MIR value width — `Pair { first: 7,`
  `second: 0 }` under `Option<Pair<i32,i32>>` previously stayed a bare
  `Pair`, so construction registered a different `Some` variant than
  the bind's `Pair<i32,i32>` gcsref and the cast trapped.
  `ctx_derive_generic_struct_layout` also ensures the GC struct on its
  already-registered early return so the derive contract holds for
  layouts registered by non-derive paths.
- Verified: new fixture `enums/option_struct_payload_fields.ark`
  prints `7 0 7 0 7 0` on wasm32 and the wasm32-gc component lane;
  probes `opt_point`/`opt_pair`/`enum_wrap`/`pair_fields`/
  `pair_mono`/`skip_gen`/`skip_chain`/`skip_chain2`/
  `enum_zip_probe`/`gc_skip_only` unchanged on both targets;
  `verify lane` and `quality changed` exit 0.
- Remaining #860 residuals: `ChainIter` / `EnumerateIter` / `ZipIter`
  item representations (packed-i32 semantic debt to replace with
  source-wrapping generic items), forward-ref emit ordering, and the
  final residual-zero audit once #691 closes. #860 is not closed.

## Progress receipt — 2026-09-23 (payload-base projection for generic-struct Option items)

- `f5dfabae5` fixes the wasm32-gc failure that blocked the
  EnumerateIter/ZipIter/ChainIter item migration: a monomorphized impl
  method returning Option<Pair<i32,I::Item>> emitted an invalid module
  because Some(p) selected the enum-open _f1_ref14 variant and stored a
  (ref null Pair) into a (ref null open-enum) field.
- Root cause: the mono map only held option:Pair -> option:Pair<i32,i32>
  (the declared return keeps the payload base, the instantiated return
  carries args), so Pair<i32,I::Item> never bound — i32 is not a mono
  param and I::Item had no entry. ctx_resolve_angle_type_vars now
  resolves Name<args> through a payload-base projection
  (ctx_resolve_mono_base_projection): an exact Pair -> Pair<i32,i32>
  entry wins, else the payload of an option:/Option_/vec:-prefixed pair
  answers the base. Unresolved args keep their spelling instead of
  failing the whole substitution, args are trimmed, and nested generics
  resolve recursively. ctx_projection_canonical_wrapper canonicalizes
  option:payload to the Option_ segment path so declared/instantiated
  pairs decompose into Pair -> Pair<i32,i32> and I::Item -> i32.
- The bootstrap overlay stub (scripts/selfhost/checks.py) received the
  equivalent stub_base_payload_projection — the stub replaces the real
  module in s2 builds, so source-only fixes never reached the compiler
  under test. This also explains why earlier attempts appeared to no-op.
- build_return_type_value_sig (sections_types_sigs_detail.ark) no longer
  unwraps Option/Result/enum-spelled return names to the payload
  gc_struct slot; option:Pair<i32,i32> returns keep the enum-open
  signature the variant value actually needs.
- Verified: new fixture stdlib_trait/iterator_generic_struct_item prints
  3 0 7 7 108 209 on wasm32 and the wasm32-gc component lane; the GC
  core module passes wasm-tools validate (previously func 20 failed);
  probes opt_point/opt_pair/enum_wrap/pair_fields/pair_mono/skip_gen/
  skip_chain/skip_chain2/enum_zip_probe/gc_skip_only unchanged on both
  targets; verify lane and quality changed exit 0.
- Remaining #860 residuals: the EnumerateIter/ZipIter/ChainIter
  packed-i32 -> Pair migration itself (now unblocked), forward-ref emit
  ordering, and the final residual-zero audit once #691 closes.
  #860 is not closed.

## Progress receipt — 2026-09-23 (EnumerateIter/ZipIter/ChainIter item migration)

- `0daf5fc8d` replaces the packed-i32 item encoding with real generic
  items, the last concrete-Vec<i32> adapters called out in the previous
  receipt. New pub struct Pair<A,B> { first, second } in std::core::iter
  is the enumerate/zip item type (ADR-076 rejected packed-int items).
- EnumerateIter<I> wraps I: Iterator and yields Pair<i32,I::Item>;
  enumerate_iter(Vec<i32>) -> EnumerateIter<VecIter>. ZipIter<A,B> wraps
  two iterator sources and yields Pair<A::Item,B::Item>;
  zip_iter(Vec,Vec) -> ZipIter<VecIter,VecIter>. ChainIter<I> wraps two
  I: Iterator sources lazily via an on_first flag instead of indexing
  vectors; chain_iter(Vec,Vec) -> ChainIter<VecIter> and
  collect_chain_iter retargeted.
- Numeric capability impls (IteratorSum/Fold/Contains/CountEq) were
  dropped for EnumerateIter/ZipIter because Item is now a Pair — ADR-076
  forbids fake numeric impls; ChainIter impls retargeted to
  ChainIter<VecIter>. IntoIterator impls are generic per adapter.
- Fixtures assert .first/.second fields instead of packed arithmetic;
  manifest signatures/docs regenerated; formatter baseline refreshed.
- Verified: all 8 iterator fixtures (enumerate, zip, zip_unequal,
  chain x4, generic_struct_item) pass wasm32 and wasm32-gc component
  lanes (16/16 rc=0); verify lane and quality changed exit 0.
- Remaining #860 residuals: forward-ref emit ordering (uninvestigated),
  and the final residual-zero audit once #691 closes. #860 is not
  closed.

## Progress receipt — 2026-09-23 (generic-body callee-axis propagation)

- `c5cdea4a7` fixes the forward-ref/generic-receiver residual from the
  previous receipts. A call inside a generic body on a receiver that
  still mentions the enclosing type parameters (`inner.next()` on
  `Inner<I>` inside `Outer<I>::next`, or `self.inner.next()` through a
  generic field) recorded only the template `Inner::next`+[?I]; the
  concrete `Inner::next`+[VecIter] was never recorded because
  demand-driven propagation (ADR-078) covered only the owner and
  argument axes, so MIR lowering bound the call to the trapping
  template body (`I::next` fid=-1).
- The callee axis: `MonoTemplateSite` records (fn, resolved args,
  enclosing fn, enclosing vars) whenever a recording's resolved args
  mention the current function's type parameters
  (mono_instances.ark, contract_mono.ark, contract_env_record.ark).
  `rebind_enclosing_template_sites` re-binds those sites through the
  enclosing instantiation's concrete arguments when it is recorded or
  merged into the parent env (module_env_merge.ark); recursion handles
  deeper nesting. `env.current_fn_name` is set per body check
  (body.ark).
- `let` annotations now resolve against the enclosing signature's type
  parameters (stmt_let.ark): `Inner<I>` spells the type variable that
  produces the `Inner__next__?I` template spelling span-rewrite
  concretization keys on, instead of a nominal struct named "I".
- Verified: new fixture stdlib_trait/iterator_nested_generic_adapter
  covers field-access and local-binding adapter forms and prints 7/9
  on wasm32 and the wasm32-gc component lane; probes for impl-order
  and generic free-fn variants print expected values; generics_v1 and
  functions/generic_fn regression fixtures pass (17/17); verify lane
  and quality changed exit 0.
- Remaining #860 residuals: the final residual-zero audit once #691
  closes. The earlier "forward-ref emit ordering" and "field-access
  method-call gap" residuals are resolved by this unit. #860 is not
  closed.

## Progress receipt — 2026-09-23 (eager any/all dedup)

- `3bebe2622` removes the last eager traversal bodies in
  std/core/iter.ark: `iter_any_i32`/`iter_all_i32` indexed the vector
  directly with get_unchecked (and scanned the whole range even after
  the answer was known). They now delegate to private
  `any_iterator_i32`/`all_iterator_i32` that walk `it.next()` like
  `contains_iterator_i32`, short-circuiting on the first hit/miss.
  Every `get_unchecked` left in the file is inside a lazy adapter's
  own `next()` impl, which is the canonical place for index access.
- Verified: iterator_any_basic/empty, iterator_all_basic/empty/false
  compile+run pass on wasm32; iterator_any_basic passes the wasm32-gc
  component lane; verify lane and quality changed exit 0.
- Remaining #860 residuals: desugared-iteration vs hand-written
  Iterator equivalence evidence, and the final residual-zero audit
  once #691 closes. #860 is not closed.

## Progress receipt — 2026-09-23 (generic struct-literal composite fields + mono wrapper vars)

- `ca34b0aee` fixes generic struct construction on wasm32-gc for
  fields that embed the type parameter inside a container.
  `mir_struct_lit_instance_name` only bound a declared parameter when
  the field type was the bare parameter, so `GenVecIter<T> { vec:
  Vec<T> }` never bound T, derived the unsubstituted base layout, and
  emitted invalid GC core Wasm (bogus `Vec<i32>` `ref.cast` on a
  `Vec<String>` field). The binding now unifies the declared field
  type against the resolved value type segment by segment (`vec:T`,
  `option:T`, `result:T:E`, `Inner<T>`), and
  `mir_emit_struct_field_set` no longer stamps a still-generic field
  type onto a value local that already carries the concrete mono
  type.
- `b9fdbaa92` fixes the `fn first<T>(v: Vec<T>) -> Option<T>` GC
  failure (`generics/vec_generic`). A `vec:T` param local contributed
  no type variable to the ordinal pairing, so the only map entry was
  the return projection `option:T -> option:i32`, and the
  colon-segment name match returned the whole `option:i32` for a `T`
  query — producing `vec:option:i32` and a bogus `Vec<Option>`
  `ref.cast`. `ctx_type_var_names_from_local_type` now collects
  variables from canonical wrapper spellings (`vec:`/`option:`/
  `result:`/`hashmap:`, `Option:`/`Result:`) through a depth-aware
  colon scan that recurses into composite segments, and `I::Item`
  projections contribute their head variable.
  `ctx_resolve_mono_type_param` returns the segment at the matched
  position when stored name and type shapes align (`option:T` ->
  `option:i32` answers `T` -> `i32`); mismatched shapes keep the
  whole-type answer. The bootstrap overlay stub in
  `scripts/selfhost/checks.py` received the same treatment (including
  the angle-argument path the stub lacked) so the fix reaches the s2
  compiler.
- Verified on rebuilt s2: new fixture
  `generics_v1/generic_struct_vec_field` (parametric `impl Iterator
  for GenVecIter<T>` over a `Vec<T>` field, two instantiations)
  prints `60`/`4` on wasm32 and the wasm32-gc component; the GC core
  module passes `wasm-tools validate`. `generics/vec_generic` now
  compiles and prints `99` on wasm32-gc (previously GC_COMPILE_FAIL)
  and still prints `99` on wasm32. Fixture batch: 31/31 PASS on
  wasm32, 26/27 PASS on the wasm32-gc component lane — the single
  failure is the pre-existing `generics_v1/test_enum_return` resolve
  error (`cmp::cmp_i32` undefined), unrelated to this change.
  `verify lane`, `quality changed`, and `docs regenerate` exit 0
  (registry 2983; the pre-existing remainder drift was corrected to
  the manifest-observed invariant).
- Remaining #860 residuals: the final residual-zero audit once #691
  closes. #860 is not closed.

## Progress receipt — 2026-09-24 (concrete fn(T)->U signatures through mono + GC emit)

- `2021eb7b1` makes generic `fn(T) -> U` fields and parameters carry
  their concrete per-instance signature end-to-end. Previously every
  fn-type position collapsed to the legacy i32-ABI `fnref` bucket, so
  `GenMap<T,U> { f: fn(T) -> U }` stored and invoked its callback
  through the wrong func type and the wasm32-gc module failed
  validation. The new `fnsig:<param>;<return>` spelling is produced by
  corehir for unary fn types outside the i32-family bucket, bound by
  the typechecker from a named callee's declared signature
  (`fn_value_type_from_sig` recovers `strlen` -> `fn(String) -> i32`
  where bare-ident inference yielded TY_UNKNOWN), specialized per
  segment by `mono_subst_fnsig_param`, and resolved to the exact
  callback func type by wasm emit (`fnsig:` params/locals/struct
  fields on wasm32-gc; the i32 ABI on linear wasm).
- Two non-obvious fixes were required beyond the spelling pipeline.
  First, the bootstrap overlay stub for `ctx_mono_type_params` in
  `scripts/selfhost/checks.py` replaces the whole module in s2 builds,
  and its `ctx_type_var_names_from_local_type` lacked the `fnsig:`
  branch — `f` contributed no type variables, the ordinal map bound
  only T, and the `mono_type_var_to_index("U")=2` fallback picked
  keys[0], producing `fnsig:String;String` on `gen_map__String__i32`.
  Second, `struct_lit_unify_segments` split field spellings on `:`
  only, so `fnsig:T;U` never aligned with `fnsig:String;i32` and the
  literal derived the unsubstituted base `GenMap` layout (struct.new
  on the wrong type id); `;` is now a top-level segment separator.
- Verified on rebuilt s2: new fixture
  `stdlib_trait/iterator_generic_fn_field` (parametric `impl Iterator
  for GenMap<T,U>` over a `f: fn(T) -> U` field; `GenMap<i32,i32>`
  doubles sum to 14, `GenMap<String,i32>` strlen yields 3) prints
  `14`/`3` on wasm32 and on the wasm32-gc component, and the GC core
  module passes `wasm-tools validate`. `verify lane` and
  `quality changed` exit 0 (lint 22/22, ratchet clean).
- Remaining #860 residuals: #691 adapter migration to source-generic
  or item-generic form can now use `fn(T) -> U` fields; the final
  residual-zero audit once #691 closes. #860 is not closed.

## Progress receipt — 2026-09-24 (Owner::Item projections; item-generic Skip adapters)

- `7906a462d` resolves associated-type projections of the form
  `Owner::Item` inside field and fnsig type spellings. Parameter
  substitution turned `fn(I::Item) -> I::Item` into
  `fnsig:VecIter::Item;VecIter::Item`, whose segments never resolved —
  on wasm32-gc the callback func type fell back to an unrelated
  signature and the core module failed validation
  (`call_ref`/`local` type mismatch in the monomorphized
  `SkipMap3__next__VecIter`). The resolver reads the item type from
  the owner's `Iterator::next` signature in the function index
  (`Option<Item>` payload), covering concrete impls
  (`VecIter::next`) and monomorphized ones
  (`SkipIter__next__StringIter`). `ctx_resolve_fnsig_segment` (plus
  the `mono_param_subst` mirror) falls back to the projection
  resolver, `ctx_derive_generic_struct_layout` resolves projections
  in substituted field spellings, `gc_struct_field_sig_for_type`
  resolves before the enum/container fallthroughs, and
  `struct_lit` treats `::` as a name qualifier rather than a
  segment separator. The bootstrap overlay stub in
  `scripts/selfhost/checks.py` received the same helpers in
  flattened form so the fix reaches the s2 compiler.
- Stdlib: `SkipMapIter<I>` and `SkipFilterIter<I>` are now item-generic
  (`map_f: fn(I::Item) -> I::Item`, `filter_f: fn(I::Item) -> bool`)
  with generic `Iterator`/`IntoIterator` impls;
  `StringIter::skip(n)` returns `SkipIter<StringIter>` and the
  concrete `StringSkipIter`/`StringSkipMapIter`/
  `StringSkipFilterIter` types and their impls are removed.
  `Vec<String>` gains `FromIterator<SkipMapIter<StringIter>>` and
  `FromIterator<SkipFilterIter<StringIter>>`. The filter predicate
  receives a cloned handle so the accepted original is still yielded,
  matching the String-lane ownership contract.
- Verified on rebuilt s2: a probe struct
  `SkipMap3<I> { iter: I, to_skip: i32, map_f: fn(I::Item) -> I::Item }`
  now emits a valid wasm32-gc component and prints `2`/`bbb` for the
  `VecIter` and `StringIter` instantiations (previously invalid
  core Wasm); the linear-memory build prints the same values.
  Fixture batch on wasm32-gc components: `iterator_string_callbacks`,
  `iterator_adapters`, `iterator_string`, `iterator_skip_basic`,
  `iterator_skip_zero`, `iterator_skip_more_than_len`,
  `iterator_query_consumers` all PASS; the first three also PASS on
  linear wasm32. `verify lane` exits 0 (comment policy, editorconfig,
  quality contract, touched-code ratchet, lint all PASS).
  `docs/language/spec.md` and `docs/stdlib/migration-guidance.md`
  now name the item-generic adapters.
- Remaining #860 residuals: `FilterIter<T>::map` stays `T -> T`
  (needs method-level generics for `fn(I::Item) -> U`),
  `impl<K,V> IntoIterator for HashMap<K,V>` (needs unified
  `VecIter<T>`), `MirInst_call` arg0=-1 design, the RFC-011
  residual-zero audit once #691 closes, and `verify quick` remains
  blocked by unrelated pre-existing gates (#652, #765, MIR
  reachability baseline, T3 memory64). #860 is not closed.

## Progress receipt — 2026-09-24 (unified VecIter<T>; StringIter retired)

- Commit `78809ce46` unifies the producer adapters: `VecIter` is now
  item-generic (`VecIter<T> { vec: Vec<T>, index, length }`), serving
  the i32 lane at `VecIter<i32>` and the String lane at
  `VecIter<String>`. `StringIter` and its dedicated impls are
  removed; `Vec<String>::iter`/`into_iter` return the shared type.
  All adapter composition (`TakeIter<VecIter<T>>`,
  `SkipIter<VecIter<T>>`, `SkipMapIter<VecIter<String>>`), the i32
  capability impls, drain helpers, `FromIterator` targets, and the
  HashMap `IntoIterator` impls moved to the instantiated spellings.
  `unique`/`rev` stay on a concrete `impl VecIter<i32>` block
  because `UniqueIter`/`RevIter` remain i32-fixed.
- Compiler: `mark_call_result_from_return_type_name` now derives
  `Name<args>` generic-struct layouts on demand. A call declared to
  return `VecIter<i32>` previously reached the lookup-only registry
  before lazy derivation ran; the result local fell back to i64 and
  the wasm32-gc module failed validation. Any declared
  generic-instantiation return type now resolves to its struct
  reference regardless of lowering order.
- Verified on rebuilt s2: 21 fixtures compile and run on wasm32-gc
  components (full `stdlib_trait/iterator_*` batch, `generics_v1`
  iterator fixtures, `stdlib_seq`, `stdlib_hashmap`); 10 also pass
  on linear wasm32 with identical assertions. The numeric-consumer
  diagnostic reads `VecIter does not implement IteratorSum`.
  `verify lane` and `docs check` pass (docs regeneration also
  cleared pre-existing registry drift).
- Remaining #860 residuals: `FilterIter<T>::map` stays `T -> T`
  (needs method-level generics for `fn(I::Item) -> U`),
  `impl<K,V> IntoIterator for HashMap<K,V>` (now unblocked — a
  parametric impl can return `VecIter<K>`), `MirInst_call` arg0=-1
  design, the RFC-011 residual-zero audit once #691 closes, and
  `verify quick` remains blocked by unrelated pre-existing gates
  (#652, #765, MIR reachability baseline, T3 memory64). #860 is not
  closed.
