---
Status: open
Created: 2026-09-19
Updated: 2026-09-19
ID: 874
Parent: 852
Track: language-v2
Depends on: "854, 855, 859, 860, 873"
Related: "ADR-058, RFC-011, #852"
Orchestration class: implementation-ready
Orchestration upstream: 852
Blocks v{N}: language-v2
Priority: 1
Source: "RFC-011 8 (fallback lowering)"
---

# 874 — fallback managed-value lowering を実装する

## Scope

RFC-011: **8 (fallback lowering)**。

この issue の独立範囲だけを実装する。別の設計判断が必要なら child issue を追加し、ここへ混ぜない。

## Acceptance

- [ ] struct/enum/Option/Result の fallback layout を実装する
- [ ] closure env を fallback heap へ lower する
- [ ] Vec/String/HashMap を fallback runtime に接続する
- [ ] 共有・循環参照を reject せず実行する
- [ ] Wasm GC backend と同じ move/borrow semantics を保つ

## Required evidence

- implementation commit SHA
- normative spec / ADR/RFC update reference
- positive fixture + negative/diagnostic fixture
- applicable compile/validate/runtime gate output

## False-done guard

- [ ] Acceptance の一部だけで done にしない
- [ ] TODO/skip/hack を残す場合は tracking issue + removal condition を必須にする
- [ ] 親 #852 の担当項目を evidence 付きで更新する
