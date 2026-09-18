---
Status: open
Created: 2026-09-19
Updated: 2026-09-19
ID: 861
Parent: 852
Track: language-v2
Depends on: none
Related: "ADR-058, RFC-011, #852"
Orchestration class: implementation-ready
Orchestration upstream: 852
Blocks v{N}: language-v2
Priority: 1
Source: "RFC-011 20, 21"
---

# 861 — 数値の暗黙変換を廃止し明示変換を定義する

## Scope

RFC-011: **20, 21**。

この issue の独立範囲だけを実装する。別の設計判断が必要なら child issue を追加し、ここへ混ぜない。

## Acceptance

- [ ] 異種 concrete numeric arithmetic を暗黙 promotion なしでは reject する
- [ ] integer/float の明示 cast 規則を定義する
- [ ] lossless/checked/wrapping/saturating を区別する
- [ ] literal inference と concrete conversion を分離する
- [ ] backend 間で型結果が一致する fixture を追加する

## Required evidence

- implementation commit SHA
- normative spec / ADR/RFC update reference
- positive fixture + negative/diagnostic fixture
- applicable compile/validate/runtime gate output

## False-done guard

- [ ] Acceptance の一部だけで done にしない
- [ ] TODO/skip/hack を残す場合は tracking issue + removal condition を必須にする
- [ ] 親 #852 の担当項目を evidence 付きで更新する
