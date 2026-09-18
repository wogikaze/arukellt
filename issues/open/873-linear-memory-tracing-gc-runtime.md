---
Status: open
Created: 2026-09-19
Updated: 2026-09-19
ID: 873
Parent: 852
Track: language-v2
Depends on: "870"
Related: "ADR-058, RFC-011, #852"
Orchestration class: implementation-ready
Orchestration upstream: 852
Blocks v{N}: language-v2
Priority: 1
Source: "RFC-011 8 (fallback runtime)"
---

# 873 — linear-memory tracing GC fallback runtime を実装する

## Scope

RFC-011: **8 (fallback runtime)**。

この issue の独立範囲だけを実装する。別の設計判断が必要なら child issue を追加し、ここへ混ぜない。

## Acceptance

- [ ] linear-memory allocator を実装する
- [ ] root registration/update を実装する
- [ ] tracing collector を実装する
- [ ] cycle collection を可能にする
- [ ] fallback artifact が Wasm GC instructions を要求しない
- [ ] runtime contract を文書化する

## Required evidence

- implementation commit SHA
- normative spec / ADR/RFC update reference
- positive fixture + negative/diagnostic fixture
- applicable compile/validate/runtime gate output

## False-done guard

- [ ] Acceptance の一部だけで done にしない
- [ ] TODO/skip/hack を残す場合は tracking issue + removal condition を必須にする
- [ ] 親 #852 の担当項目を evidence 付きで更新する
