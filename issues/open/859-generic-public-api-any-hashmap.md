---
Status: open
Created: 2026-09-19
Updated: 2026-09-19
ID: 859
Parent: 852
Track: language-v2
Depends on: "856"
Related: "ADR-058, RFC-011, #852"
Orchestration class: implementation-ready
Orchestration upstream: 852
Blocks v{N}: language-v2
Priority: 1
Source: "RFC-011 15, 16, 18, 26"
---

# 859 — 公開 API を generic Trait 面へ統一する

## Scope

RFC-011: **15, 16, 18, 26**。

この issue の独立範囲だけを実装する。別の設計判断が必要なら child issue を追加し、ここへ混ぜない。

## Acceptance

- [ ] public free function を method/associated/Trait API へ移行する
- [ ] monomorphic helper を generic API に統合する
- [ ] public `Any` を必要最小限にする
- [ ] `HashMap<K,V>` を generic public API として完結させる
- [ ] 旧 API の removal condition を記録する

## Required evidence

- implementation commit SHA
- normative spec / ADR/RFC update reference
- positive fixture + negative/diagnostic fixture
- applicable compile/validate/runtime gate output

## False-done guard

- [ ] Acceptance の一部だけで done にしない
- [ ] TODO/skip/hack を残す場合は tracking issue + removal condition を必須にする
- [ ] 親 #852 の担当項目を evidence 付きで更新する
