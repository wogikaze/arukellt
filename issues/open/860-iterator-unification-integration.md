---
Status: open
Created: 2026-09-19
Updated: 2026-09-20
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
