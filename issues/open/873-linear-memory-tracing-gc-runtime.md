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

## Progress receipt (not done; 2026-09-19)

- Implementation: `6cb9bb01f` (`language-v2: add linear-memory tracing runtime foundation`). The `wasm32` emitter now reserves linear metadata, emits allocation records, provides explicit root-frame entry/push/update/leave functions, and emits fixed-point mark/sweep and free-list allocation functions without Wasm GC instructions in the runtime source.
- Normative updates: [ADR-075](../../docs/adr/ADR-075-language-v2-linear-memory-tracing-runtime.md), the [linear-memory lowering plan](../../docs/plans/wasm32-linear-memory-lowering.md), and RFC-011 section 2.2. The plan records that automatic collection remains disabled until root lowering is complete, so live values cannot be reclaimed during this migration step.
- Positive/negative contract: the existing same-source `wasm32-gc` / `wasm32` fixture pair remains covered by `scripts/tests/test_linear_gc_runtime_contract.py`; the target-capability diagnostic fixture remains the applicable negative path. The new contract test passes (`Ran 3 tests ... OK`).
- Verification: `verify lane` passed (fmt 14/14, lint 14/14, quality ratchet 13 files); `check-adrs.py` passed; docs regeneration and freshness/consistency passed; `git diff --cached --check` passed. `docs check` doc examples and canonical `selfhost build-compiler` remain blocked by the pre-existing #854 `unbox` / `slice_get` return-type mismatch in `src/compiler/main.ark`.
- Remaining acceptance evidence: canonical `wasm32` artifact validation, binary proof that the emitted artifact contains no Wasm GC instructions, and root/cycle runtime fixtures are still pending. The collector is intentionally not auto-invoked until #874 connects managed-value lowering and root updates.
