---
Status: done
Created: 2026-09-28
Updated: 2026-09-28
ID: 902
Parent: 875
Track: language-v2
Depends on: none
Related: "ADR-058, ADR-082, RFC-011, #893, #899, #901, #852"
Orchestration class: design-ready
Orchestration upstream: 875
Blocks v{N}: language-v2
Priority: 2
Source: "#901 verification residual — variant payload literal sub-patterns are not compared at match time"
---

# 902 — variant payload の literal/複合 sub-pattern が照合されない

## Scope

variant pattern の payload sub-pattern は照合条件に反映されない。
Option::Some(5) => は任意の Some にマッチし、Some(7) を渡しても
Some(5) arm が選択される。tag のみが比較され、sub-pattern の
literal 値・ネスト構造は評価されない。

## Evidence

fn mk() -> Option<i32> {
    Option::Some(7)
}

fn main() {
    let r = match mk() {
        Option::Some(5) => "five",
        _ => "other",
    }
    stdio::println(r)
}

- wasm32 linear core: five を出力(Some(7) が Some(5) arm に
  マッチ)。wasm32-gc も同じ tag-only 照合で five。
- 両 backend で同一の誤挙動のため parity 監査では検出されない。
- ADR-082 の coverage 側は Some(0) を refutable として網羅扱い
  しないよう修正済みだが、照合 emit 側は tag 比較のみで
  payload 条件を読まない(表裏の別問題)。

## Design notes

- literal sub-pattern(Some(5))、ネスト sub-pattern(Some(Some(x)))、
  range/比較 sub-pattern の照合意味論を確定する。payload 値の
  比較は variant tag 成立後に payload field を load して行う必要が
  ある(GC では ref.cast + struct.get、linear では tag 分岐後の
  payload slot 読み出し)。
- or-pattern alternative 内の literal payload(Circle(5) | Square)
  も同じ穴を共有する。
- binding sub-pattern(Some(v))は payload bind として実装済み;
  本 issue は非 binding sub-pattern の照合欠落のみを扱う。

## Acceptance

- [x] variant payload の literal sub-pattern が照合条件に含まれ、
      Some(5) が Some(7) にマッチしない。
- [x] wasm32-gc / wasm32 linear で同一の観測結果になる。
- [x] 対応しない場合は compile diagnostic で拒否する
      (誤った照合を静かに生成しない)。

## Required evidence

- implementation commit SHA または採択 ADR
- positive/negative fixture の両 backend 実行結果

## Done (2026-09-28)

- root cause: match emit が variant pattern を tag 比較だけで受理し、
  payload child の literal/composite sub-pattern を条件に含めなかっ
  た。coverage 側(ADR-082)は refutable 扱い済みだが照合 emit は
  tag のみ — Some(5) が Some(7) にマッチしていた。
- implementation: 58faf3eca fix(#902)。新規 mir/lower/match_subpat.ark
  と core_match_subpat.ark が guarded flag 条件を emit
  (flag=0; if tag==v { flag=AND-folded child conds })。payload field
  の load は tag-true branch 内に限定され、ref.cast/record 読みが
  layout 有効な位置に置かれる。literal/unit-variant-ident/tuple/
  nested-variant/or children を照合し、or-fold と tuple element
  compare からも同じ flag を使う。
- named-bind 経路は literal と登録済み unit-variant ident child を
  binding から除外し、composite child(tuple/nested variant)を匿名
  temp 経由で再帰 destructure — Some((k,_)) が k を、Some(Some(v))
  が v を AST/CoreHIR 両経路で束縛する。
- typechecker は nested payload ident を再帰 bind し、unit-variant
  ident child を tag 条件として扱い、payload 内 struct sub-pattern
  を error[E0200] で拒否(静かな tag-only 照合を防ぐ)。
- positive fixture: tests/fixtures/match_extensions/
  variant_payload_subpat.ark → seven other hi inner7 inner-none seven
  or12 10 7 s5c9 big(wasm32-gc component / wasm32 linear core で
  expected と同一出力)。11 probes(literal/nested/tuple/or-in-elem/
  composite bind/or-alternative/guard/string/unit-ident/tuple-or)
  も両 backend で同一結果。
- negative/diagnostic: diagnostics/variant_payload_struct_subpat.ark
  が error[E0200|typecheck]: unsupported match pattern: struct
  sub-patterns in variant payloads are not implemented を
  GC/linear 両 compile 経路で発行。
- 継続 PASS: fixture-parity --filter-dir が match_extensions(8),
  enums(12), closure_capture(13), stdlib_option_result(133),
  stdlib_collections_ordered(6), diagnostics(5), regression(4)
  で全件 PASS。verify lane PASS、docs check 6/6 PASS。
- 残余(別 issue): untyped generic Option scrutinee(inline ctor、
  推論 let、Option<tuple> 注釈)は payload を GC-ref slot で読み
  wasm32-gc が invalid core Wasm — #902 適用前 s2 でも再現する
  named-bind 系の既存バグとして #904 へ分離。
