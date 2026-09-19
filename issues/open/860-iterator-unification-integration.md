---
Status: open
Created: 2026-09-19
Updated: 2026-09-19
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
- This receipt does not mark any acceptance checkbox complete: eager/lazy
  duplication, canonical `collect`, desugared-iteration equivalence, and the
  RFC-011 residual audit still require implementation and review.
