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

## Progress receipt — 2026-09-20 (closure annotation typing prerequisite)

- Commit `104b108d2` preserves closure parameter annotations and the optional
  return annotation in the selfhost AST. Closure inference now uses annotated
  parameter types before operator-trait checks and unifies an annotated return
  type with the body type; unannotated closures retain fresh-variable
  inference.
- `benchmarks/bench_cpu_closure_map.ark` is a positive regression workload with
  two typed capturing closures. The current s2 compiler produces a valid GC
  component (3447 bytes) and a valid linear-memory core module (3099 bytes).
  Both validate and run under Wasmtime with the expected output `6017000`.
- `python3 scripts/manager.py verify lane` passed, including formatter,
  quality contract, lint smoke, and lint ratchet. This receipt covers the
  typing/parser prerequisite only; managed closure environments, collection
  lowering, and the representative negative fixture remain open, so #872 is
  not closed.
