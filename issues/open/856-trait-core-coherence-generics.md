---
Status: open
Created: 2026-09-19
Updated: 2026-09-19
ID: 856
Parent: 852
Track: language-v2
Depends on: none
Related: "ADR-058, RFC-011, #852"
Orchestration class: implementation-ready
Orchestration upstream: 852
Blocks v{N}: language-v2
Priority: 1
Source: "RFC-011 12, 17"
---

# 856 — Trait core / coherence / generic arity を確定する

## Scope

RFC-011: **12, 17**。

この issue の独立範囲だけを実装する。別の設計判断が必要なら child issue を追加し、ここへ混ぜない。

## Acceptance

- [ ] `Self`、型引数、associated type/function を実装する
- [ ] coherence/conflicting impl を診断する
- [ ] blanket impl の適用条件と termination を固定する
- [ ] static dispatch と `dyn Trait` の境界を固定する
- [ ] generic parameter count に恣意的な小さい仕様上限を残さない

## Required evidence

- implementation commit SHA
- normative spec / ADR/RFC update reference
- positive fixture + negative/diagnostic fixture
- applicable compile/validate/runtime gate output

## False-done guard

- [ ] Acceptance の一部だけで done にしない
- [ ] TODO/skip/hack を残す場合は tracking issue + removal condition を必須にする
- [ ] 親 #852 の担当項目を evidence 付きで更新する
