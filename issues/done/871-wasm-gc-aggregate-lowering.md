---
Status: done
Created: 2026-09-19
Updated: 2026-09-20
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

- [x] struct/tuple/enum/Option/Result を Wasm GC types/instructions へ lower する
- [x] tag/payload representation を実装する
- [x] primary target の silent fallback を binary inspection で防ぐ
- [x] `wasm-tools validate` を通す
- [x] value semantics と representation optimization を分離する

## Required evidence

- implementation commit SHA
- normative spec / ADR/RFC update reference
- positive fixture + negative/diagnostic fixture
- applicable compile/validate/runtime gate output

## Close receipt (2026-09-20)

- Implementation: `402326820` builds the explicit `TypeId`-owned MIR GC
  layout plan; `c107e7b42` makes GC layout lookup explicit and removes the
  emitter's name-to-offset fallback; `38cc7cc71` preserves scalar width at the
  `Range` GC aggregate boundary. These changes use Wasm GC `struct` / `array`
  instructions for the primary aggregate path while retaining the same source
  value semantics for linear lowering.
- Verification guard: `b57a3f670` and `c11c782fc` add binary and runtime
  evidence in `scripts/tests/test_wasm_gc_aggregate_lowering.py`; `f38ffe5b3`
  extends it to all nine enum fixtures, covering both unit variants, tuple,
  record, nested enum, `Option::None`, `Option::Some`, `Result::Err`, and
  `Result::Ok`, alongside struct/Vec, Option/Result, and tuple aggregate
  fixtures.
- Primary representation: for every enum fixture, the printed GC component
  contains `struct.new`, `struct.get`, and `ref.cast`, proving tag/payload
  construction/access uses the GC type path. The linear artifacts contain no
  `struct.*`, `array.*`, or `ref.*` instruction and therefore cannot silently
  claim GC lowering.
- Runtime parity: the GC artifact is packaged as a WASI P2 component and the
  fallback artifact as a WASI P1 core module. Both targets validate with
  `wasm-tools validate`, run under Wasmtime, and match each fixture's expected
  stdout for all 13 aggregate fixtures.
- Negative/diagnostic evidence: the non-exhaustive enum fixture reaches the
  typechecker and is rejected with `E0200` / `missing Direction::West` before
  backend emission on the GC target.
- Verification: `python3 -m unittest
  scripts.tests.test_wasm_gc_aggregate_lowering` passes (`Ran 2 tests in
  118.416s`); `python3 -m py_compile
  scripts/tests/test_wasm_gc_aggregate_lowering.py` and `git diff --check`
  pass. The normative meaning/representation boundary is [ADR-035](../../docs/adr/ADR-035-wasm-gc-implementation.md), [ADR-060](../../docs/adr/ADR-060-language-v2-owned-adts-box-slice.md), and RFC-011 item 8.
- Remaining GC work is outside this aggregate issue: closure/collection
  lowering is #872, linear managed-value lowering is #874, and the final
  cross-backend audit is #875. The current-state document continues to mark
  the overall GC backend as partial; this close claims only the aggregate
  scope above.

## False-done guard

- [x] Acceptance の一部だけで done にしない
- [x] TODO/skip/hack を残す場合は tracking issue + removal condition を必須にする
- [x] 親 #852 の担当項目を evidence 付きで更新する
