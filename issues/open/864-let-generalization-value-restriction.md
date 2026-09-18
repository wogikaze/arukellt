---
Status: open
Created: 2026-09-19
Updated: 2026-09-19
ID: 864
Parent: 852
Track: language-v2
Depends on: "856"
Related: "ADR-058, RFC-011, #852"
Orchestration class: implementation-ready
Orchestration upstream: 852
Blocks v{N}: language-v2
Priority: 1
Source: "RFC-011 27"
---

# 864 — let-generalization / value restriction を形式化する

## Scope

RFC-011: **27**。

この issue の独立範囲だけを実装する。別の設計判断が必要なら child issue を追加し、ここへ混ぜない。

## Acceptance

- [ ] generalize 可能な non-expansive expression を定義する
- [ ] allocation/call/mutable state の扱いを固定する
- [ ] polymorphic let の positive/negative fixture を追加する
- [ ] 型推論を backend 非依存にする
- [ ] 将来 effect system との置換境界を文書化する

## Required evidence

- implementation commit SHA
- normative spec / ADR/RFC update reference
- positive fixture + negative/diagnostic fixture
- applicable compile/validate/runtime gate output

## False-done guard

- [ ] Acceptance の一部だけで done にしない
- [ ] TODO/skip/hack を残す場合は tracking issue + removal condition を必須にする
- [ ] 親 #852 の担当項目を evidence 付きで更新する
