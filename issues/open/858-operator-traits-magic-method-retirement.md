---
Status: open
Created: 2026-09-19
Updated: 2026-09-19
ID: 858
Parent: 852
Track: language-v2
Depends on: "856"
Related: "ADR-058, RFC-011, #852"
Orchestration class: implementation-ready
Orchestration upstream: 852
Blocks v{N}: language-v2
Priority: 1
Source: "RFC-011 14"
---

# 858 — 演算子 Trait へ統一し magic method を退役する

## Scope

RFC-011: **14**。

この issue の独立範囲だけを実装する。別の設計判断が必要なら child issue を追加し、ここへ混ぜない。

## Acceptance

- [ ] 各演算子を canonical Trait method に対応づける
- [ ] 演算子解決を Trait coherence に従わせる
- [ ] 旧 magic method の移行方針を定める
- [ ] builtin/user-defined/generic fixture を追加する
- [ ] stdlib/compiler の二重実装を残さない

## Required evidence

- implementation commit SHA
- normative spec / ADR/RFC update reference
- positive fixture + negative/diagnostic fixture
- applicable compile/validate/runtime gate output

## False-done guard

- [ ] Acceptance の一部だけで done にしない
- [ ] TODO/skip/hack を残す場合は tracking issue + removal condition を必須にする
- [ ] 親 #852 の担当項目を evidence 付きで更新する
