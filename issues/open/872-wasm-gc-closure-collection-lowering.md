---
Status: open
Created: 2026-09-19
Updated: 2026-09-19
ID: 872
Parent: 852
Track: language-v2
Depends on: "855, 859, 860, 871"
Related: "ADR-058, RFC-011, #852"
Orchestration class: implementation-ready
Orchestration upstream: 852
Blocks v{N}: language-v2
Priority: 1
Source: "RFC-011 8 (closure/collection)"
---

# 872 — Wasm GC closure / collection lowering を完成する

## Scope

RFC-011: **8 (closure/collection)**。

この issue の独立範囲だけを実装する。別の設計判断が必要なら child issue を追加し、ここへ混ぜない。

## Acceptance

- [ ] closure env を Wasm GC managed representation にする
- [ ] Vec/String/HashMap の managed parts を GC representation にする
- [ ] generic element boxing policy を固定する
- [ ] runtime helper 依存を最小化する
- [ ] representative compile/run/validate fixture を追加する

## Required evidence

- implementation commit SHA
- normative spec / ADR/RFC update reference
- positive fixture + negative/diagnostic fixture
- applicable compile/validate/runtime gate output

## False-done guard

- [ ] Acceptance の一部だけで done にしない
- [ ] TODO/skip/hack を残す場合は tracking issue + removal condition を必須にする
- [ ] 親 #852 の担当項目を evidence 付きで更新する
