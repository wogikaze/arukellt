---
Status: open
Created: 2026-09-25
Updated: 2026-09-25
ID: 881
Parent: 691
Track: language-v2
Depends on: none
Related: "ADR-058, RFC-011, #691, #874"
Orchestration class: implementation-ready
Orchestration upstream: 691
Blocks v{N}: none
Priority: 2
Source: "#691 PeekIter implementation — 2026-09-25"
---

# 881 — Option<T> record field と literal の GC type index を一致させる

## Scope

wasm32-gc で `Option<T>` を record field に持つ struct を struct literal で
構築すると、field 宣言側の `Option<T>` type index と `None`/`Some`
literal が生成する type index がずれ、invalid な core module になる。

## Reproduction

2026-09-25 に `PeekIter<T>` で再現:

```ark
pub struct PeekIter<T> {
    vec: Vec<T>,
    index: i32,
    length: i32,
    peeked: Option<T>,
    has_peeked: bool,
}

PeekIter { vec: v, index: 0, length: n, peeked: None, has_peeked: false }
```

同一コンパイル内で `Option<i32>` が 3 つの異なる type index
(field 宣言=78 field 3 の `(ref null 28)`、`peekable` 内 `None`
literal の `struct.new_default 79`、`peek` 内 `Some` 構築の
`struct.new_default 80`) として生成され、`wasm-tools validate` が
`type mismatch: expected (ref null \$type), found (ref null \$type)`
で func を拒否する。linear target では同じコードが通る。

## Workaround in tree

`PeekIter<T>` は `peeked` を single-slot `Vec<T>` で表現している
(`519f5c4c6`)。この issue は record field 型としての `Option<T>`
一般対応が必要になったときに解消する。`Option<T>` field を必要とする
他の実装が現れなければ YAGNI で閉じてよい。

## Acceptance

- [ ] `Option<T>` を field に持つ record を `None`/`Some` で構築した
      module が wasm32-gc で validate する
- [ ] 同一コンパイル内の `Option<T>` が単一の type index に正規化される
- [ ] linear backend での回帰がない

## Required evidence

- implementation commit SHA
- positive fixture (Option<T> record field build on wasm32-gc)
- validate output
