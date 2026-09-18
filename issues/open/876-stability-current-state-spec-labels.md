---
Status: open
Created: 2026-09-19
Updated: 2026-09-19
ID: 876
Parent: 852
Track: language-v2
Depends on: none
Related: "ADR-058, RFC-011, #852"
Orchestration class: implementation-ready
Orchestration upstream: 852
Blocks v{N}: language-v2
Priority: 1
Source: "RFC-011 42, 43"
---

# 876 — stability / current-state / redesign の分類を整合する

## Scope

RFC-011: **42, 43**。

この issue の独立範囲だけを実装する。別の設計判断が必要なら child issue を追加し、ここへ混ぜない。

## Acceptance

- [ ] stable/provisional/experimental/unimplemented を一意に定義する
- [ ] current-state と future proposal を分離する
- [ ] ADR-014/ADR-018 と spec labels を同期する
- [ ] 未実装 proposal を stable 表示しない
- [ ] docs consistency で drift を検出する

## Required evidence

- implementation commit SHA
- normative spec / ADR/RFC update reference
- positive fixture + negative/diagnostic fixture
- applicable compile/validate/runtime gate output

## False-done guard

- [ ] Acceptance の一部だけで done にしない
- [ ] TODO/skip/hack を残す場合は tracking issue + removal condition を必須にする
- [ ] 親 #852 の担当項目を evidence 付きで更新する
