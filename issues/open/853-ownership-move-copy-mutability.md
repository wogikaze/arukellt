---
Status: open
Created: 2026-09-19
Updated: 2026-09-19
ID: 853
Parent: 852
Track: language-v2
Depends on: none
Related: "ADR-058, RFC-011, #852"
Orchestration class: implementation-ready
Orchestration upstream: 852
Blocks v{N}: language-v2
Priority: 1
Source: "RFC-011 1–4"
---

# 853 — Ownership / move / Copy / mutability を確定・実装する

## Scope

RFC-011: **1–4**。

この issue の独立範囲だけを実装する。別の設計判断が必要なら child issue を追加し、ここへ混ぜない。

## Acceptance

- [ ] move-after-use を型検査し positive/negative fixture を追加する
- [ ] `let` / `let mut` の再代入と mutation 規則を仕様・実装で一致させる
- [ ] mutability と `Copy` を独立に扱う
- [ ] user-defined struct/enum の `Copy` は明示 opt-in にする
- [ ] tuple / fixed array / struct の move/Copy を一貫させる

## Required evidence

- implementation commit SHA
- normative spec / ADR/RFC update reference
- positive fixture + negative/diagnostic fixture
- applicable compile/validate/runtime gate output

## False-done guard

- [ ] Acceptance の一部だけで done にしない
- [ ] TODO/skip/hack を残す場合は tracking issue + removal condition を必須にする
- [ ] 親 #852 の担当項目を evidence 付きで更新する
