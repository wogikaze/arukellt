---
Status: open
Created: 2026-09-27
Updated: 2026-09-27
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

- [ ] `(pat, pat, ...)` arm が各要素の照合として動作し、
      payload bind / wildcard / literal / variant を要素位置に許す。
- [ ] wasm32-gc と wasm32 linear で同じ意味論・同じ観測結果になる。
- [ ] 非対応形(実装外の nested 形)は invalid wasm emit ではなく
      明確な診断で拒否される。

## Required evidence

- implementation commit SHA
- positive fixture(tuple arm match)の両 backend 実行結果
- `scripts/check/check-cross-backend-parity.py` の diverge 一覧から
      該当ケースが消えていること
