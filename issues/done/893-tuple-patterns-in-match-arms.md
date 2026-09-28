---
Status: done
Created: 2026-09-27
Updated: 2026-09-28
ID: 893
Parent: 875
Track: language-v2
Depends on: none
Related: "ADR-058, RFC-011, #875, #892, #852"
Orchestration class: implementation-ready
Orchestration upstream: 875
Blocks v{N}: language-v2
Priority: 2
Source: "#892 residual — PatTuple arm patterns unimplemented in both match lowerings"
---

# 893 — match アームの tuple パターンが両経路で未実装

## Scope

`match (a, b) { (pat1, pat2) => ... }` のような arm 頭部の
`PatTuple` が、CoreHIR 経路 (`lower_core_match`) と legacy body 経路
(`lower_match_expr`) のどちらでも destructuring として下ろせない。
`let (a, b) = t` の destructuring は `stmt_let_tuple` で動くが、
match arm 位置では compile 時に invalid core wasm を emit する。

この issue の独立範囲だけを実装する。nested pattern の再帰設計が
大きくなる場合は child issue に分け、ここへ混ぜない。

## Evidence

最小プローブ(両 backend で同一ソース):

```ark
enum Opt { Some(i32), None }
fn main() {
    let o = Opt::None
    let t = (o, 7)
    match t {
        (Opt::Some(v), _) => stdio::println("some"),
        (Opt::None, _) => stdio::println("none"),
    }
}
```

- wasm32: emit された core wasm が invalid
  (`type mismatch: expected i32 but nothing on stack`)。
  `lower_core_pattern_value` が `PAT_TUPLE` を値に下ろせず -1 を返し、
  `eq` の operand が 1 つ欠ける。
- wasm32-gc: 同じく invalid core wasm(wrapper が
  `compiler produced invalid core Wasm` で拒否)。divergence ではなく
  両 backend 共通の未実装領域。
- legacy body 経路も `lower_expr(PatTuple)` を match compare に
  結線しておらず、同じ構造的问题を持つ。

## Design notes

- arm 頭部の tuple は「scrutinee を n 要素に分解し、各要素を
  sub-pattern に再帰的に照合する」必要がある。現行の compare 機構は
  単一 i32 の `compare_idx` 前提なので、sub-pattern ごとの
  compare/bind 列への展開が要る。
- `(Opt::Some(v), _)` の中の `Opt::Some(v)` は payload bind を伴う
  nested PatVariant なので、tag 比較 + field bind の再帰が必須。
  flat な wildcard/ident bind だけなら前段の destructuring で済む。
- guard 連鎖(#892 で修正した matched-local 構造)と合成できる形に
  する。

## Acceptance

- [x] `(pat, pat, ...)` arm が各要素の照合として動作し、
      payload bind / wildcard / literal / variant を要素位置に許す。
- [x] wasm32-gc と wasm32 linear で同じ意味論・同じ観測結果になる。
- [x] 非対応形(実装外の nested 形)は invalid wasm emit ではなく
      明確な診断で拒否される。

## Required evidence

- implementation commit SHA
- positive fixture(tuple arm match)の両 backend 実行結果
- `scripts/check/check-cross-backend-parity.py` の diverge 一覧から
      該当ケースが消えていること

## Done (2026-09-28)

- implementation: `fix(#893)` commit. 新規
  `mir/lower/body_match_tuple.ark`（legacy AstNode 経路）と
  `mir/lower/core_match_tuple.ark`（CoreHIR 経路）が arm 頭部
  tuple を要素ごとの compare AND-fold + arm-if 内 bind へ下ろす。
  parser は要素位置の `|` alternatives を NK_PAT_OR として受理、
  typechecker は要素 bind と非対応形の診断を担当する。
- positive fixture: `tests/fixtures/match_extensions/tuple_arm_match.ark`
  （variant payload bind / ident / literal / string literal / or /
  guard / last-arm）— gc component と linear core の両方で
  `some:42 / none / 12 / 104 / -1 / 204 / k5 / 3` を出力。
- parity receipt: `.build/parity/report-full.json` —
  223 fixtures, parity=208, compile-fail:gc=15, diverge=0。
  `match_extensions/tuple_arm_match.ark` は parity。diverge 一覧に
  該当ケースは残っていない（compile-fail:gc 15 件はすべて
  resolve/typecheck 系の既存未実装領域で match lowering 由来なし）。
- 非対応形の診断: nested tuple/struct 要素、variant 子の composite
  要素、literal+variant 混合 or-alt は
  `error[E0200|typecheck]: unsupported match pattern: ...` で拒否。
  lowering 側にも防御用 MIR_UNREACHABLE を残す。
- 残課題(別 issue 候補): guarded/last or-arm が参照する
  `lower_or_match_compare_condition` が body_match_compare_or に
  未定義（既存 bug）、variant payload 内 literal（`Some(0)`）が
  top-level/tuple 要素の双方で tag 一致のみとして扱われる既存意味論、
  multi-param multiclause の parser ゲート（ADR-073 の別判断）。
