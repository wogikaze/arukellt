---
Status: done
Created: 2026-09-19
Updated: 2026-09-20
ID: 870
Parent: 852
Track: language-v2
Depends on: none
Related: "ADR-058, RFC-011, #852"
Orchestration class: implementation-ready
Orchestration upstream: 852
Blocks v{N}: language-v2
Priority: 1
Source: "RFC-011 9, 39, 40"
---

# 870 — target capability / Memory64 / host profile を分解する

## Scope

RFC-011: **9, 39, 40**。

この issue の独立範囲だけを実装する。別の設計判断が必要なら child issue を追加し、ここへ混ぜない。

## Acceptance

- [x] GC availability を source legality と分離する
- [x] memory width/WASI profile/Component emit を独立設定にする
- [x] `wasm32-gc` と Memory64 の関係を明示する
- [x] CLI/manifest/internal target representation を同期する
- [x] target-specific capability error fixture を追加する

## Required evidence

- implementation commit SHA
- normative spec / ADR/RFC update reference
- positive fixture + negative/diagnostic fixture
- applicable compile/validate/runtime gate output

## False-done guard

- [x] Acceptance の一部だけで done にしない
- [x] TODO/skip/hack を残す場合は tracking issue + removal condition を必須にする
- [x] 親 #852 の担当項目を evidence 付きで更新する

## Close receipt (2026-09-20)

- Implementation: `b6b7b2e0f` (`language-v2: separate target capability axes`)
  and `10fdbe597` (`fix: keep GC hashmap indices memory-width aware`).
- Normative updates: [ADR-074](../../docs/adr/ADR-074-language-v2-target-capability-configuration.md), ADR-007, ADR-013, ADR-035, and RFC-011 sections 9, 39, and 40.
- Contract: `docs/data/project-state.toml` now owns GC availability, default/allowed memory widths, host-profile support, and artifact-kind support. CLI parsing, generated target helpers, driver config, and Wasm writer propagation consume the same axes. `wasm32-gc` keeps GC and Memory64 independent; `wasm32` remains the Memory32 linear-memory fallback. Component and WAT scaffold paths resolve `auto` to Memory32 and reject explicit Memory64.
- Fixtures: `target_capability/gc_linear_same_source.ark` and `target_capability/wasm32_same_source.ark` cover the same source on the GC and linear-memory paths; `diagnostics/target_memory_width_unavailable.ark` covers the target-specific Memory64 rejection.
- Verification: `python3 scripts/manager.py verify lane --gate component-interop` passed, including quality changed and the full component-interop domain. `python3 scripts/tests/test_memory64_p2_realloc.py` passed (`2 tests`). The acceptance-focused target contract tests for memory-width axes and positive/diagnostic fixtures passed. The current s2 build passed through the guarded source-only Box/Slice overlay; the generic HashMap fixture now compiles and validates as the default GC P2 component, `wasm-tools validate` passes, and wasmtime execution returns rc=0. The same source remains covered by the GC Memory32 and linear-memory paths, and `wasm32` artifacts contain no Wasm GC instructions. Final docs freshness, consistency, deprecated-API, and doc-example checks passed (`4/4`).
- Pin/fixpoint refresh remains a separate bootstrap follow-up owned by #854/#878/#851; it does not leave an unchecked acceptance item in this target-capability issue.
