---
Status: open
Created: 2026-09-19
Updated: 2026-09-19
ID: 877
Parent: 852
Track: language-v2
Depends on: "857, 858, 860, 875, 876"
Related: "ADR-058, RFC-011, #852"
Orchestration class: implementation-ready
Orchestration upstream: 852
Blocks v{N}: language-v2
Priority: 1
Source: "RFC-011 44, 45"
---

# 877 — normative examples と操作的意味論を実行可能にする

## Scope

RFC-011: **44, 45**。

この issue の独立範囲だけを実装する。別の設計判断が必要なら child issue を追加し、ここへ混ぜない。

## Acceptance

- [ ] normative example を compile/run/check fixture に接続する
- [ ] skip に reason/owner/exit condition を必須化する
- [ ] observable semantics を集約する
- [ ] zero-from-scratch implementation に必要な意味論を散在させない
- [ ] CI で normative docs consistency を検証する

## Required evidence

- implementation commit SHA
- normative spec / ADR/RFC update reference
- positive fixture + negative/diagnostic fixture
- applicable compile/validate/runtime gate output

## False-done guard

- [ ] Acceptance の一部だけで done にしない
- [ ] TODO/skip/hack を残す場合は tracking issue + removal condition を必須にする
- [ ] 親 #852 の担当項目を evidence 付きで更新する
