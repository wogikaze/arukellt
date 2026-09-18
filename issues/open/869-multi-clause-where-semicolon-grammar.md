---
Status: open
Created: 2026-09-19
Updated: 2026-09-19
ID: 869
Parent: 852
Track: language-v2
Depends on: none
Related: "ADR-058, RFC-011, #852"
Orchestration class: implementation-ready
Orchestration upstream: 852
Blocks v{N}: language-v2
Priority: 1
Source: "RFC-011 34–36"
---

# 869 — multi-clause / where / semicolon grammar を確定する

## Scope

RFC-011: **34–36**。

この issue の独立範囲だけを実装する。別の設計判断が必要なら child issue を追加し、ここへ混ぜない。

## Acceptance

- [ ] multi-clause grouping を adjacency に依存させない
- [ ] pattern-bound variable と `where` scope を固定する
- [ ] newline/semicolon/continuation grammar を固定する
- [ ] ambiguous parse の negative fixture を追加する
- [ ] parser/formatter round-trip を一致させる

## Required evidence

- implementation commit SHA
- normative spec / ADR/RFC update reference
- positive fixture + negative/diagnostic fixture
- applicable compile/validate/runtime gate output

## False-done guard

- [ ] Acceptance の一部だけで done にしない
- [ ] TODO/skip/hack を残す場合は tracking issue + removal condition を必須にする
- [ ] 親 #852 の担当項目を evidence 付きで更新する
