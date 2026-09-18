---
Status: open
Created: 2026-09-19
Updated: 2026-09-19
ID: 862
Parent: 852
Track: language-v2
Depends on: none
Related: "ADR-058, RFC-011, #852"
Orchestration class: implementation-ready
Orchestration upstream: 852
Blocks v{N}: language-v2
Priority: 1
Source: "RFC-011 22, 23"
---

# 862 — 算術 edge case と評価順序を仕様化する

## Scope

RFC-011: **22, 23**。

この issue の独立範囲だけを実装する。別の設計判断が必要なら child issue を追加し、ここへ混ぜない。

## Acceptance

- [ ] integer overflow/division/shift edge を固定する
- [ ] NaN/inf/comparison/conversion を fixture 化する
- [ ] args/binary/aggregate/index/assignment の評価順を固定する
- [ ] 副作用付き fixture を追加する
- [ ] backend 間の観測可能順序差をなくす

## Required evidence

- implementation commit SHA
- normative spec / ADR/RFC update reference
- positive fixture + negative/diagnostic fixture
- applicable compile/validate/runtime gate output

## False-done guard

- [ ] Acceptance の一部だけで done にしない
- [ ] TODO/skip/hack を残す場合は tracking issue + removal condition を必須にする
- [ ] 親 #852 の担当項目を evidence 付きで更新する
