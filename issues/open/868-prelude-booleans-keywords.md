---
Status: open
Created: 2026-09-19
Updated: 2026-09-19
ID: 868
Parent: 852
Track: language-v2
Depends on: none
Related: "ADR-058, RFC-011, #852"
Orchestration class: implementation-ready
Orchestration upstream: 852
Blocks v{N}: language-v2
Priority: 1
Source: "RFC-011 32, 33, 41"
---

# 868 — Prelude / boolean literal / keyword policy を固定する

## Scope

RFC-011: **32, 33, 41**。

この issue の独立範囲だけを実装する。別の設計判断が必要なら child issue を追加し、ここへ混ぜない。

## Acceptance

- [ ] Prelude を小さく stable な固定集合として列挙する
- [ ] `true`/`false` を lexical literal に一意化する
- [ ] hard/contextual/future-reserved を分類する
- [ ] 未実装を理由に future-reserved を解放しない
- [ ] parser/formatter/highlighting fixture を同期する

## Required evidence

- implementation commit SHA
- normative spec / ADR/RFC update reference
- positive fixture + negative/diagnostic fixture
- applicable compile/validate/runtime gate output

## False-done guard

- [ ] Acceptance の一部だけで done にしない
- [ ] TODO/skip/hack を残す場合は tracking issue + removal condition を必須にする
- [ ] 親 #852 の担当項目を evidence 付きで更新する
