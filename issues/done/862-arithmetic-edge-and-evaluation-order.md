---
Status: done
Created: 2026-09-19
Updated: 2026-09-19
ID: 862
Parent: 852
Track: language-v2
Depends on: none
Related: "ADR-058, ADR-068, RFC-011, #852"
Orchestration class: implementation-ready
Orchestration upstream: 852
Blocks v{N}: language-v2
Priority: 1
Source: "RFC-011 22, 23"
---

# 862 — 算術 edge case と評価順序を仕様化する

## Scope

RFC-011: **22, 23**。

この issue の独立範囲だけを実装する。別の設計判断が必要なら child issue を追加し、ここへ混ぜない。

## Acceptance

- [x] integer overflow/division/shift edge を固定する
- [x] NaN/inf/comparison/conversion を fixture 化する
- [x] args/binary/aggregate/index/assignment の評価順を固定する
- [x] 副作用付き fixture を追加する
- [x] backend 間の観測可能順序差をなくす

## Required evidence

- implementation commit SHA
- normative spec / ADR/RFC update reference
- positive fixture + negative/diagnostic fixture
- applicable compile/validate/runtime gate output

## Evidence

- Implementation commit: `a720c2cc7` (`Implement issue 862 arithmetic evaluation semantics`).
  `src/compiler/mir/lower/body_aggregate_place.ark`、`core_assign.ark`、および
  field-place helper は place を先に lower し、RHS を次に lower してから store する。
- Normative contract: [ADR-068](../../docs/adr/ADR-068-language-v2-arithmetic-evaluation-semantics.md)
  is ACCEPTED. `docs/language/spec.md` §3.3.1–3.3.2 と
  [RFC-011](../../docs/rfcs/011-language-v2-semantics-redesign.md) items 22–23 を同期した。
  整数 wrap、signed division/remainder trap、masked shift、IEEE/Wasm float edge、NaN 比較、
  explicit conversion、および receiver/argument/operand/aggregate/index/place/RHS/store の
  順序を GC と linear の共通契約として固定した。
- Positive fixtures:
  `tests/fixtures/operators/arithmetic_edges.ark` は i32 wrap、i64 signed remainder edge、
  shift count masking、infinity/NaN comparison、`f64`→`f32` rounding と widening を検査する。
  `tests/fixtures/operators/evaluation_order.ark` は call arguments、binary、aggregate、
  index base/index、field assignment の副作用 marker を source order で検査する。
- Negative/diagnostic fixture:
  `tests/fixtures/diagnostics/arithmetic_mixed_comparison.ark` は GC/linear とも rc=1、
  `E0200: type mismatch: f64 vs i32` を返す。
- Target parity receipt from current-source artifact
  `.build/selfhost/arukellt-s2-862.wasm`: both GC component and linear core artifacts
  compiled and validated; `arithmetic_edges` and `evaluation_order` each executed with
  exit 0 on both targets. The marker assertions make the evaluation-order result observable.
  The existing `native_cpp_public/trap_div_zero.ark` also compiled and validated on both
  targets and terminated with exit 134 and `wasm trap: integer divide by zero` on both.
- Verification: full Ark formatter `checked=1967 failed=0 baseline-skipped=10`; targeted
  `verify lane --gate quality-quick` passed (fmt/lint, comment policy, semantic-debt,
  quality ratchet); `quality structure`, `check-adrs.py`, `check-false-done-hygiene.py`,
  and `docs regenerate` passed. The operator fixture slice was `PASS=11 FAIL=1` with the
  one unrelated pre-existing `operators/mixed_types.ark` #861 type-boundary failure; the
  two new fixtures passed.
- `selfhost build-compiler` was attempted after batching the source edits and still fails
  at the pre-existing #854 generic-enum bootstrap mismatch (`unbox` / `slice_get`). The
  receipts above use a generated-only targeted bootstrap shim artifact and do not claim a
  canonical bootstrap rebuild. This limitation is recorded consistently in prior v2 child
  issues and remains a #854/#878 migration boundary; no SKIP or semantic-debt allowlist was
  added for #862.

## False-done guard

- [x] Acceptance の一部だけで done にしない
- [x] TODO/skip/hack を残す場合は tracking issue + removal condition を必須にする
- [x] 親 #852 の担当項目を evidence 付きで更新する
