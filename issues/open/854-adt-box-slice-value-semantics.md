---
Status: open
Created: 2026-09-19
Updated: 2026-09-19
ID: 854
Parent: 852
Track: language-v2
Depends on: "853"
Related: "ADR-058, RFC-011, #852"
Orchestration class: implementation-ready
Orchestration upstream: 852
Blocks v{N}: language-v2
Priority: 1
Source: "RFC-011 5–7, 25"
---

# 854 — ADT / Box / slice の値意味論を統一する

## Scope

RFC-011: **5–7, 25**。

この issue の独立範囲だけを実装する。別の設計判断が必要なら child issue を追加し、ここへ混ぜない。

## Acceptance

- [ ] Option/Result/enum を owned ADT として定義する
- [ ] `Box<T>` を heap 上の `T` の unique owner として実装する
- [ ] slice を borrow/view として定義する
- [ ] GC object/tagged/unboxed は backend detail に限定する
- [ ] move/Copy/borrow fixture を追加する

## Required evidence

- implementation commit SHA
- normative spec / ADR/RFC update reference
- positive fixture + negative/diagnostic fixture
- applicable compile/validate/runtime gate output

## False-done guard

- [ ] Acceptance の一部だけで done にしない
- [ ] TODO/skip/hack を残す場合は tracking issue + removal condition を必須にする
- [ ] 親 #852 の担当項目を evidence 付きで更新する
