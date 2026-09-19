---
Status: done
Created: 2026-09-19
Updated: 2026-09-19
ID: 861
Parent: 852
Track: language-v2
Depends on: none
Related: "ADR-058, RFC-011, #852"
Orchestration class: implementation-ready
Orchestration upstream: 852
Blocks v{N}: language-v2
Priority: 1
Source: "RFC-011 20, 21"
---

# 861 — 数値の暗黙変換を廃止し明示変換を定義する

## Scope

RFC-011: **20, 21**。

この issue の独立範囲だけを実装する。別の設計判断が必要なら child issue を追加し、ここへ混ぜない。

## Acceptance

- [x] 異種 concrete numeric arithmetic を暗黙 promotion なしでは reject する
- [x] integer/float の明示 cast 規則を定義する
- [x] lossless/checked/wrapping/saturating を区別する
- [x] literal inference と concrete conversion を分離する
- [x] backend 間で型結果が一致する fixture を追加する

## Required evidence

- implementation commit SHA
- normative spec / ADR/RFC update reference
- positive fixture + negative/diagnostic fixture
- applicable compile/validate/runtime gate output

## Evidence

- Implementation commits: `549fc4e1a` (`Implement issue 861 numeric conversion
  rules`) and `72c188934` (`Fix numeric conversion fixture trait import`).
- Normative references: [ADR-067](../../docs/adr/ADR-067-language-v2-numeric-conversion.md),
  [RFC-011](../../docs/rfcs/011-language-v2-semantics-redesign.md), and
  [language spec](../../docs/language/spec.md). The common typechecker rejects
  mixed concrete integer arithmetic; CoreHIR/MIR preserve `f32` literal suffixes;
  `std/core/convert.ark` exposes lossless `From`/`Into` only and separates
  checked, wrapping, and saturating `i64` to `i32` conversions.
- Positive fixture:
  `tests/fixtures/scalar/numeric_explicit_conversions.ark` covers lossless
  `i32 -> i64`, checked boundary results, two's-complement wrapping, saturating
  bounds, and `f32`/`f64` conversion round trips. The required
  `std::core::convert` import is retained because the current resolver/lowerer
  needs the trait import to emit the conversion calls, even though the lint
  pass reports it as unused.
- Negative fixture:
  `tests/fixtures/diagnostics/numeric_implicit_promotion.ark` returns `rc=1`
  with `E0200: type mismatch: i64 vs i32` for mixed arithmetic.
- Canonical current-source receipt: `python3 scripts/manager.py selfhost
  build-compiler` passed (`6.2s`). Both targets compiled from the resulting
  `.build/selfhost/arukellt-s2-runtime.wasm`: GC emitted 3512 bytes with
  `reg-vt-audit candidates=11 matched=11 mismatched=0`,
  `core-op-shadow candidates=13 matched=13 mismatched=0 unresolved=0`, and
  `gc-layout-audit typed=18 name=0 fallback=0 conflict=0`; linear emitted
  2528 bytes with the same registry/core-op audit result.
- Backend execution: `wasm-tools validate --features
  gc,function-references,memory64` plus wasmtime execution passed for
  `.build/selfhost/numeric-gc-861.wasm`; `wasm-tools validate --features
  memory64` plus wasmtime execution passed for
  `.build/selfhost/numeric-linear-861.wasm`.
- Targeted quality/docs gates passed: fixture `fmt --check` (`checked=1`),
  `check-adrs.py`, `generate-docs.py --check`, and
  `check-false-done-hygiene.py`. The targeted quality lane had passed before
  close preparation. The repository-wide `verify quick` was run and returned
  `146/151`; its five failures are unrelated existing manifest/core-op/T3 and
  thin-wrapper baseline failures, so this issue does not claim that gate as
  green.

## False-done guard

- [x] Acceptance の一部だけで done にしない
- [x] TODO/skip/hack を残す場合は tracking issue + removal condition を必須にする
- [x] 親 #852 の担当項目を evidence 付きで更新する
