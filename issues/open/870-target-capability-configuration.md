---
Status: open
Created: 2026-09-19
Updated: 2026-09-19
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

- [ ] GC availability を source legality と分離する
- [ ] memory width/WASI profile/Component emit を独立設定にする
- [ ] `wasm32-gc` と Memory64 の関係を明示する
- [ ] CLI/manifest/internal target representation を同期する
- [ ] target-specific capability error fixture を追加する

## Required evidence

- implementation commit SHA
- normative spec / ADR/RFC update reference
- positive fixture + negative/diagnostic fixture
- applicable compile/validate/runtime gate output

## False-done guard

- [ ] Acceptance の一部だけで done にしない
- [ ] TODO/skip/hack を残す場合は tracking issue + removal condition を必須にする
- [ ] 親 #852 の担当項目を evidence 付きで更新する

## Progress receipt (not done; 2026-09-19)

- Implementation: `b6b7b2e0f` (`language-v2: separate target capability axes`).
- Normative updates: [ADR-074](../../docs/adr/ADR-074-language-v2-target-capability-configuration.md), ADR-007, ADR-013, ADR-035, and RFC-011 sections 9, 39, and 40.
- Contract: `docs/data/project-state.toml` now owns GC availability, default/allowed memory widths, host-profile support, and artifact-kind support. CLI parsing, generated target helpers, driver config, and Wasm writer propagation consume the same axes. `wasm32-gc` keeps GC and Memory64 independent; `wasm32` remains the Memory32 linear-memory fallback. Component and WAT scaffold paths resolve `auto` to Memory32 and reject explicit Memory64.
- Fixtures: `target_capability/gc_linear_same_source.ark` and `target_capability/wasm32_same_source.ark` cover the same source on the GC and linear-memory paths; `diagnostics/target_memory_width_unavailable.ark` covers the target-specific Memory64 rejection.
- Verification: `verify lane` passed (format, lint, quality ratchet); the four targeted target-contract tests passed; ADR checks, Python compilation, docs regeneration/freshness/consistency, and `git diff --check` passed. The docs check doc-example subgate remains 0 pass / 7 fail for the same #854 bootstrap mismatch.
- Blocker: `python3 scripts/manager.py selfhost build-compiler` remains blocked by the pre-existing #854 `unbox`/`slice_get` generic return mismatch. Therefore canonical selfhost compile/validate/runtime evidence for this issue is not yet available, and this issue remains open.
