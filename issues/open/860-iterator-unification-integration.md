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
