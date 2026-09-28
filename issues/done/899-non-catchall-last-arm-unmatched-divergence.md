---
Status: done
Created: 2026-09-28
Updated: 2026-09-28
ID: 899
Parent: 875
Track: language-v2
Depends on: none
Related: "ADR-058, RFC-011, #875, #892, #898, #852"
Orchestration class: design-ready
Orchestration upstream: 875
Blocks v{N}: language-v2
Priority: 2
Source: "#898 verification residual — unmatched non-catchall last arm diverges GC vs linear"
---

# 899 — 非 catchall last arm が非マッチ時に GC/linear で発散

## Scope

i32 など builtin 以外の scrutinee では typechecker の exhaustive
チェックが無い(`check_builtin_match_exhaustive` は
bool/Option/Result のみ、enum は別経路)。そのため
`match x { 1 => "one", 2 => "two" }` のような
catchall を持たない match が受理されるが、**全 arm が非マッチ**
のときの観測結果が backend で発散する。

## Evidence

最小プローブ(`x = 9`):

```ark
let r = match x {
    1 => "one",
    2 => "two",
}
stdio::println(r)
```

- wasm32 linear core: `two` を出力(last arm が無条件で実行される)。
- wasm32-gc component: `wasm trap: null reference`
  (条件付き arm が skip され result が null ref のまま println へ)。
- or-pattern last arm(`2 | 3 => "two-three"`)も同一の発散を示す。
  `check-cross-backend-parity.py` の 223 fixture ではこの形を
  カバーする fixture が無く diverge=0 として観測されない。

## Design notes

- 言語意味論の確定が先: 非 exhaustive match は compile 時診断か、
  runtime trap か、defined default か。match lowering の所産ではなく
  意味論決定の問題なので ADR 判断に従う。
- last arm の lowering 形状が backend 間で異なる可能性
  (CoreHIR 経路の last arm は non-catchall でも condition を
  emit しない構造)を調査対象に含める。
- exhaustive 診断を i32 等の literal scrutinee へ拡張する場合は
  `check_builtin_match_exhaustive` と coverage 記録の拡張が要る。

## Acceptance

- [x] 非 catchall match の非マッチ時の意味論が確定し、
      wasm32-gc / wasm32 linear で同一の観測結果になる。
- [x] 採用した仕様(診断拒否 or runtime trap or default)に対する
      positive/negative fixture が両 backend で等しい出力を持つ。

## Required evidence

- implementation commit SHA または採択 ADR
- 両 backend の非マッチ時挙動の実行結果
- parity receipt での当該ケース非発散

## Done (2026-09-28)

- 意味論: ADR-082 ACCEPTED — match は証明可能な網羅性を要求する
  compile 時診断(E0200)。unguarded catchall、有限 domain
  (bool/Option/Result/既知 enum) の全 variant 網羅、または全
  sub-pattern が irrefutable な tuple/struct arm のいずれかで受理。
  非マッチ match は codegen 前に両 backend で同一に reject される
  ため、runtime 発散(linear の無条件 last-arm 実行 / GC の null ref
  trap)は観測不能になった。
- implementation: 70a544811 feat(#899)。MatchCoverage に
  has_irrefutable を追加し、guard 付き arm を coverage から除外、
  or-pattern alternative を独立記録、refutable payload sub-pattern
  を持つ variant arm を網羅扱いから除外、全 irrefutable tuple/struct
  arm を catchall 等価として受理。TY_UNKNOWN scrutinee
  (let-generalized binding) は従来の permissive 経路を維持
  (ADR-082 記録の残余)。docs/ADR-082 は 89711cba2。
- positive fixture(両 backend 同一出力、s2 89711cba2 時点で検証):
  match_extensions/or_arm_edge.ark → two-three tf other hit、
  match_extensions/tuple_arm_match.ark、
  match_extensions/catchall_unmatched.ark → low other low、
  match_extensions/struct_pattern.ark → 3 4(従来 fail を解消)、
  question_mark/user_error_type.ark → OK、
  enums/option_tuple_payload.ark → 15 hi42 k17 k17 12。
- negative/diagnostic: diagnostics/non_exhaustive_literal.ark
  (i32 literal scrutinee)、non_exhaustive_payload.ark
  (Some(0) refutable payload)、non_exhaustive_guard.ark
  (guard 付き Some arm) がいずれも
  error[E0200|typecheck]: non-exhaustive match を wasm32-gc /
  wasm32 linear の両 compile 経路で発行。
- parity receipt: 48-dir selfhost fixture-parity FAIL=233。
  pre-899 baseline(FAIL=237)との差分で新規 regression ゼロ、
  match 関連 4 fixture(option_tuple_payload / or_arm_edge /
  tuple_arm_match / user_error_type)が fix。残余 FAIL は進行中
  epic 既存失敗(stdlib_vec i64 系は match を含まず ADR-067 の
  numeric literal 変換で fail — 別 issue 管轄)。
- 残余(別 issue): or-pattern arm の enum-variant alternative が
  invalid core Wasm を生成する既存 emit バグ(pre-899 で同一症状)は
  #901 へ分離。
