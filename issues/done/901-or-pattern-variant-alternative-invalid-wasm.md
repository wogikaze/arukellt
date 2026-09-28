---
Status: done
Created: 2026-09-28
Updated: 2026-09-28
ID: 901
Parent: 875
Track: language-v2
Depends on: none
Related: "ADR-058, ADR-082, RFC-011, #898, #899, #852"
Orchestration class: design-ready
Orchestration upstream: 875
Blocks v{N}: language-v2
Priority: 2
Source: "#899 implementation residual — or-pattern arm with enum-variant alternatives emits invalid core Wasm"
---

# 901 — variant alternative を含む or-pattern arm が invalid Wasm を生成

## Scope

literal alternative の or-pattern(`1 | 2 =>`)は emit できるが、
enum variant alternative を含む or-pattern arm
(`Shape::Circle(r) | Shape::Square =>`)を持つ match が
compile には成功するものの **invalid core Wasm** を生成する
（GC emit 経路）。pattern 照合と payload bind の OR-fold が
variant 形状に対応していないと考えられる。

## Evidence

最小プローブ（catchall 併用でも再現）:

```ark
enum Shape { Circle(i32), Square }

fn mk() -> Shape {
    Shape::Square
}

fn main() {
    let r = match mk() {
        Shape::Circle(r) | Shape::Square => "hit",
        _ => "other",
    }
    stdio::println(r)
}
```

- wasm32-gc emit component で `compile` 成功 →
  `compiler produced invalid core Wasm`。
- catchall 併用で pre-existing に到達可能だったが、ADR-082 以降は
  variant or-alternative が finite 型の coverage を満たす経路でも
  到達する。
- literal or-pattern(`1 | 2` + catchall)は両 backend で正常動作
  （match_extensions/or_arm_edge, or_pattern fixture）。

## Acceptance

- [x] variant alternative を含む or-pattern arm が compile・validate・
      実行でき、or の各 alternative の照合と payload bind が正しい。
- [x] wasm32-gc と wasm32 linear で同じ意味論・同じ観測結果になる。
- [x] 対応しない場合は compile diagnostic で拒否する
      （invalid Wasm を生成しない）。

## Required evidence

- implementation commit SHA
- positive fixture（variant or-alternative arm）の両 backend 実行結果
- 既存 or-pattern fixture の継続 PASS

## Done (2026-09-28)

- root cause: AST match lowering(fn body 経路、両 backend 共通)の
  mir_match_has_variant_pattern が or-pattern alternative を走査
  しなかったため has_variant_arm=false となり、compare_idx が
  tag local ではなく scrutinee ref のまま emit された。linear は
  record-variant oracle(mir_pat_is_record_variant)が or を降りて
  いたため動作していた。GC emit では i32.eq が (ref null $type) を
  i32 と比較して validate 失敗(func $main offset 0x6c6)。
- implementation: d966782b7 fix(#901)。AST 版 variant oracle に
  or-alternative 降下(variant または unit-variant ident alternative)
  を追加し、CoreHIR 版 core_match_has_variant_pattern にも同じ
  ident 判定を shared predicate で揃えた。照合は抽出済み tag
  local に対する i32.eq の OR-fold になり validate/実行共に正常。
- or-alternative 内の束縛は typechecker で診断拒否(bind_match_pattern
  の or 分岐): binding ident alternative(Some(v) の v を含む)と
  nested composite alternative は error[E0200] を両 backend で発行。
  束縛は matching alternative しか payload を書けず、resolver が
  symbol を割り当て済みのため linear では未初期化 local として
  0 を読んでいた — 静かな誤値を診断へ置き換え。
- positive fixture: tests/fixtures/match_extensions/
  or_variant_alternatives.ark → flat flat pointy handled handled
  (wasm32-gc component / wasm32 linear core で同一出力)。
  unqualified Some(_) | None も GC で either を出力確認。
- negative/diagnostic: diagnostics/or_pattern_binding.ark が
  error[E0200|typecheck]: unsupported match pattern: or-pattern
  alternatives cannot introduce bindings を GC/linear 両 compile
  経路で発行。issue 原本の probe(Shape::Circle(r) | Shape::Square)
  は invalid Wasm ではなく同一診断になる。
- 継続 PASS: or_arm_edge / or_pattern / tuple_arm_match は両 backend
  で同一出力のまま。6-dir fixture-parity(match_extensions, enums,
  question_mark, diagnostics, option_result, pattern_matching)は
  FAIL=1 のみで pre-existing の error_ecosystem(Error supertrait、
  #694 管轄) — baseline と同一、regression なし。
- 残余(別 issue): variant payload literal sub-pattern が照合されない
  (Some(5) が任意の Some にマッチ — non-or arm と共有の既存
  tag-only 比較)は #902 へ分離。全 alternative 同一束縛の
  uniform-binding 対応は診断の置き換え候補として将来の設計判断。
