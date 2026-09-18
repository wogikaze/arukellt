---
Status: open
Created: 2026-09-19
Updated: 2026-09-19
ID: 866
Parent: 852
Track: language-v2
Depends on: none
Related: "ADR-058, RFC-011, #852"
Orchestration class: implementation-ready
Orchestration upstream: 852
Blocks v{N}: language-v2
Priority: 1
Source: "RFC-011 30"
---

# 866 — source module の import / use 構文を整理する

## Scope

RFC-011: **30**。

この issue の独立範囲だけを実装する。別の設計判断が必要なら child issue を追加し、ここへ混ぜない。

## Acceptance

- [ ] `import` と `use` の役割を重複させない
- [ ] alias/selective import/re-export を仕様化する
- [ ] name resolution/formatter/LSP を同期する
- [ ] 旧構文 migration diagnostic を追加する
- [ ] WIT 境界は #867 に分離する

## Required evidence

- implementation commit SHA
- normative spec / ADR/RFC update reference
- positive fixture + negative/diagnostic fixture
- applicable compile/validate/runtime gate output

## False-done guard

- [ ] Acceptance の一部だけで done にしない
- [ ] TODO/skip/hack を残す場合は tracking issue + removal condition を必須にする
- [ ] 親 #852 の担当項目を evidence 付きで更新する
