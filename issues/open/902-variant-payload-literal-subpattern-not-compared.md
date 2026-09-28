---
Status: open
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

- [ ] variant payload の literal sub-pattern が照合条件に含まれ、
      Some(5) が Some(7) にマッチしない。
- [ ] wasm32-gc / wasm32 linear で同一の観測結果になる。
- [ ] 対応しない場合は compile diagnostic で拒否する
      (誤った照合を静かに生成しない)。

## Required evidence

- implementation commit SHA または採択 ADR
- positive/negative fixture の両 backend 実行結果
