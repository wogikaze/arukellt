---
Status: open
Created: 2026-09-19
Updated: 2026-09-19
ID: 863
Parent: 852
Track: language-v2
Depends on: "854"
Related: "ADR-058, RFC-011, #852"
Orchestration class: implementation-ready
Orchestration upstream: 852
Blocks v{N}: language-v2
Priority: 1
Source: "RFC-011 24"
---

# 863 — String / Unicode semantics を確定する

## Scope

RFC-011: **24**。

この issue の独立範囲だけを実装する。別の設計判断が必要なら child issue を追加し、ここへ混ぜない。

## Acceptance

- [ ] String encoding を固定する
- [ ] byte index と文字単位 API を分離する
- [ ] `char` API の整数 sentinel を廃止または移行する
- [ ] invalid UTF/boundary/slicing を規定する
- [ ] ASCII/BMP/supplementary/combining fixture を追加する

## Required evidence

- implementation commit SHA
- normative spec / ADR/RFC update reference
- positive fixture + negative/diagnostic fixture
- applicable compile/validate/runtime gate output

## False-done guard

- [ ] Acceptance の一部だけで done にしない
- [ ] TODO/skip/hack を残す場合は tracking issue + removal condition を必須にする
- [ ] 親 #852 の担当項目を evidence 付きで更新する
