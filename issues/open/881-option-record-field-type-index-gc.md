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

## Second repro — 2026-09-26 (generic Vec<T> field, same family)

#883-A の調査中に同族の type index ずれを \`Vec<T>\` field で確認した。

```ark
pub struct ScanBox<T> {
    vec: Vec<T>,
    index: i32,
    length: i32,
    state: T,
    f: fn(T, T) -> Option<T>,
}

let v: Vec<i32> = Vec::new<i32>()
let it = ScanBox { vec: v, index: 0, length: 3, state: 0, f: sum2 }
```

mono 化された \`ScanBox_T\` (T=i32) の field 0 が erased-generic
\`{ (ref null (array structref)), i32 }\` layout を引く一方、main 側の
\`v: Vec<i32>\` は \`{ (ref null (array i32)), i32 }\`。
\`struct.set ScanBox_T 0\` が
\`type mismatch: expected (ref null $type), found (ref null $type)\`
で func 20 (main) を拒否する。linear では正しく動く (1,3 と出力)。
scalar field (index/length/state) と fnsig field は mono 解決済みで、
nested container field だけが未解決 — #882 で直した payload bind 系と
同じく、type-arg map が nested \`Vec<T>\` の field layout 構築に
適用されていない。
