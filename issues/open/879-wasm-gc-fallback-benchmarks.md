---
Status: open
Created: 2026-09-19
Updated: 2026-09-19
ID: 879
Parent: 852
Track: language-v2
Depends on: "871, 872, 874, 875"
Related: "ADR-058, RFC-011, #852"
Orchestration class: implementation-ready
Orchestration upstream: 852
Blocks v{N}: language-v2
Priority: 1
Source: "RFC-011 performance"
---

# 879 — Wasm GC / fallback の性能・サイズ receipt を取得する

## Scope

RFC-011: **performance**。

この issue の独立範囲だけを実装する。別の設計判断が必要なら child issue を追加し、ここへ混ぜない。

## Acceptance

- [ ] workload を固定する
- [ ] `.wasm` size/cold startup/steady runtime を両 backend で測る
- [ ] allocation/GC time/peak memory を測る
- [ ] compiler compile time/RSS も必要に応じて測る
- [ ] commit/runtime/wasm-tools/host/commands/raw results を保存する
- [ ] 不利な結果もそのまま記録する

## Required evidence

- implementation commit SHA
- normative spec / ADR/RFC update reference
- positive fixture + negative/diagnostic fixture
- applicable compile/validate/runtime gate output

## False-done guard

- [ ] Acceptance の一部だけで done にしない
- [ ] TODO/skip/hack を残す場合は tracking issue + removal condition を必須にする
- [ ] 親 #852 の担当項目を evidence 付きで更新する
