---
Status: open
Created: 2026-09-19
Updated: 2026-09-19
ID: 871
Parent: 852
Track: language-v2
Depends on: "854, 870"
Related: "ADR-058, RFC-011, #852"
Orchestration class: implementation-ready
Orchestration upstream: 852
Blocks v{N}: language-v2
Priority: 1
Source: "RFC-011 8 (aggregate)"
---

# 871 — Wasm GC aggregate lowering を完成する

## Scope

RFC-011: **8 (aggregate)**。

この issue の独立範囲だけを実装する。別の設計判断が必要なら child issue を追加し、ここへ混ぜない。

## Acceptance

- [ ] struct/tuple/enum/Option/Result を Wasm GC types/instructions へ lower する
- [ ] tag/payload representation を実装する
- [ ] primary target の silent fallback を binary inspection で防ぐ
- [ ] `wasm-tools validate` を通す
- [ ] value semantics と representation optimization を分離する

## Required evidence

- implementation commit SHA
- normative spec / ADR/RFC update reference
- positive fixture + negative/diagnostic fixture
- applicable compile/validate/runtime gate output

## False-done guard

- [ ] Acceptance の一部だけで done にしない
- [ ] TODO/skip/hack を残す場合は tracking issue + removal condition を必須にする
- [ ] 親 #852 の担当項目を evidence 付きで更新する
