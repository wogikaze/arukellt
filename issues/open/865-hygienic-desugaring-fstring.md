---
Status: open
Created: 2026-09-19
Updated: 2026-09-19
ID: 865
Parent: 852
Track: language-v2
Depends on: "729"
Related: "ADR-058, RFC-011, #852"
Orchestration class: implementation-ready
Orchestration upstream: 852
Blocks v{N}: language-v2
Priority: 1
Source: "RFC-011 28, 29"
---

# 865 — Hygienic desugaring と f-string lowering を実装する

## Scope

RFC-011: **28, 29**。

この issue の独立範囲だけを実装する。別の設計判断が必要なら child issue を追加し、ここへ混ぜない。

## Acceptance

- [ ] lang item/pre-resolved symbol/intrinsic の canonical mechanism を選ぶ
- [ ] comprehension/iteration/f-string を hygienic に lower する
- [ ] user shadowing で sugar の意味を変えない
- [ ] 埋め込み式の評価順を保持する
- [ ] core form との equivalence fixture を追加する

## Required evidence

- implementation commit SHA
- normative spec / ADR/RFC update reference
- positive fixture + negative/diagnostic fixture
- applicable compile/validate/runtime gate output

## False-done guard

- [ ] Acceptance の一部だけで done にしない
- [ ] TODO/skip/hack を残す場合は tracking issue + removal condition を必須にする
- [ ] 親 #852 の担当項目を evidence 付きで更新する
