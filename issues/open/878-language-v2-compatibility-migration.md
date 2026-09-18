---
Status: open
Created: 2026-09-19
Updated: 2026-09-19
ID: 878
Parent: 852
Track: language-v2
Depends on: "875, 876"
Related: "ADR-058, RFC-011, #852"
Orchestration class: implementation-ready
Orchestration upstream: 852
Blocks v{N}: language-v2
Priority: 1
Source: "RFC-011 migration"
---

# 878 — Language v2 compatibility migration を完了する

## Scope

RFC-011: **migration**。

この issue の独立範囲だけを実装する。別の設計判断が必要なら child issue を追加し、ここへ混ぜない。

## Acceptance

- [ ] stable API ごとに deprecation window/removal condition を持つ
- [ ] alias→move/borrow・numeric・import/operator/free-function の migration diagnostic を提供する
- [ ] stdlib/examples/selfhost source を v2 へ移行する
- [ ] compatibility layer の orphan entry をなくす
- [ ] canonical gates で FAIL/SKIP の不正増加を出さない

## Required evidence

- implementation commit SHA
- normative spec / ADR/RFC update reference
- positive fixture + negative/diagnostic fixture
- applicable compile/validate/runtime gate output

## False-done guard

- [ ] Acceptance の一部だけで done にしない
- [ ] TODO/skip/hack を残す場合は tracking issue + removal condition を必須にする
- [ ] 親 #852 の担当項目を evidence 付きで更新する
