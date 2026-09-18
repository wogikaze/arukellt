---
Status: open
Created: 2026-09-19
Updated: 2026-09-19
ID: 880
Parent: 852
Track: language-v2
Depends on: "877, 878, 879"
Related: "ADR-058, RFC-011, #852"
Orchestration class: implementation-ready
Orchestration upstream: 852
Blocks v{N}: language-v2
Priority: 1
Source: "RFC-011 final"
---

# 880 — Language v2 最終 close audit を実施する

## Scope

RFC-011: **final**。

この issue の独立範囲だけを実装する。別の設計判断が必要なら child issue を追加し、ここへ混ぜない。

## Acceptance

- [ ] RFC item 1–45 の担当 issue がすべて done/evidence 有り
- [ ] RFC-011 未決事項が 0 または accepted decision へ移管済み
- [ ] wasm32-gc/fallback validate/run/parity が PASS
- [ ] selfhost fixpoint/fixture parity/diag parity/docs consistency が PASS
- [ ] benchmark receipt が再現可能
- [ ] generated issue index/dependency graph が最新
- [ ] clean commit から final verification receipt を保存する

## Required evidence

- implementation commit SHA
- normative spec / ADR/RFC update reference
- positive fixture + negative/diagnostic fixture
- applicable compile/validate/runtime gate output

## False-done guard

- [ ] Acceptance の一部だけで done にしない
- [ ] TODO/skip/hack を残す場合は tracking issue + removal condition を必須にする
- [ ] 親 #852 の担当項目を evidence 付きで更新する
