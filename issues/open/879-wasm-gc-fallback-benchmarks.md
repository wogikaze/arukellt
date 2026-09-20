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

## Progress receipt — 2026-09-20 (fail-closed current-s2 benchmark run)

- Commit `f499fccad` makes the benchmark runner fail closed for compile errors,
  runtime errors, incorrect output, and missing comparison metrics. It also
  selects the current selfhost s2 artifact and keeps compile/runtime paths
  repository-relative so the same artifact can be executed by Wasmtime.
- `python3 scripts/manager.py perf gate` was run with the current s2 wrapper
  on the fixed 20-workload suite. Raw output is in
  `tests/baselines/perf/current.json`, with the generated report in
  `docs/history/benchmarks/benchmark-results.md`; the envelope records the
  commit-independent tool/environment metadata and `wasm32` target.
- The run records 7 compile errors, 6 runtime errors, and 1 correctness
  failure (`parse_tree_distance`). Six workloads compile, run, and produce
  the expected result. The gate exits non-zero, as required; the 15-entry
  historic baseline was not rewritten to conceal the current compiler/runtime
  regressions. Wasmtime CLI still reports GC pause/allocation telemetry as
  unavailable.
- This is an honest fallback-target measurement receipt only. A complete
  `wasm32-gc` versus linear-memory comparison, allocation/GC instrumentation,
  and the dependent lowering/parity work remain open; #879 is not closed.
