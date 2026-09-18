---
Status: open
Created: 2026-09-19
Updated: 2026-09-19
ID: 867
Parent: 852
Track: language-v2
Depends on: "866"
Related: "ADR-058, RFC-011, #852"
Orchestration class: implementation-ready
Orchestration upstream: 852
Blocks v{N}: language-v2
Priority: 1
Source: "RFC-011 11, 31, 38"
---

# 867 — `pub` / Component export / WIT import 境界を分離する

## Scope

RFC-011: **11, 31, 38**。

この issue の独立範囲だけを実装する。別の設計判断が必要なら child issue を追加し、ここへ混ぜない。

## Acceptance

- [ ] `pub` を source visibility に限定する
- [ ] Component export/import を別契約にする
- [ ] WIT package identifier の canonical path を統一する
- [ ] name mangling 具体形式を language spec から外す
- [ ] source namespace と WIT namespace の分離 fixture を追加する

## Required evidence

- implementation commit SHA
- normative spec / ADR/RFC update reference
- positive fixture + negative/diagnostic fixture
- applicable compile/validate/runtime gate output

## False-done guard

- [ ] Acceptance の一部だけで done にしない
- [ ] TODO/skip/hack を残す場合は tracking issue + removal condition を必須にする
- [ ] 親 #852 の担当項目を evidence 付きで更新する
