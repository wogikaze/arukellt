---
Status: open
Created: 2026-09-19
Updated: 2026-09-19
ID: 855
Parent: 852
Track: language-v2
Depends on: "853"
Related: "ADR-058, RFC-011, #852"
Orchestration class: implementation-ready
Orchestration upstream: 852
Blocks v{N}: language-v2
Priority: 1
Source: "RFC-011 10"
---

# 855 — Closure capture / escape / lifetime を実装する

## Scope

RFC-011: **10**。

この issue の独立範囲だけを実装する。別の設計判断が必要なら child issue を追加し、ここへ混ぜない。

## Acceptance

- [ ] shared/mutable borrow と move capture の規則を固定する
- [ ] 短命 borrow を保持する escaping closure を reject する
- [ ] return/store/callback の fixture を追加する
- [ ] capture/lifetime diagnostic に正確な span を付ける
- [ ] backend 表現で source semantics を変えない

## Required evidence

- implementation commit SHA
- normative spec / ADR/RFC update reference
- positive fixture + negative/diagnostic fixture
- applicable compile/validate/runtime gate output

## False-done guard

- [ ] Acceptance の一部だけで done にしない
- [ ] TODO/skip/hack を残す場合は tracking issue + removal condition を必須にする
- [ ] 親 #852 の担当項目を evidence 付きで更新する
