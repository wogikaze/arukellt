---
Status: open
Created: 2026-09-27
Updated: 2026-09-27
ID: 892
Parent: 875
Track: language-v2
Depends on: none
Related: "ADR-058, RFC-011, #875, #852"
Orchestration class: implementation-ready
Orchestration upstream: 875
Blocks v{N}: language-v2
Priority: 1
Source: "#875 cross-backend parity audit — first observed divergence"
---

# 892 — wasm32 linear の enum pattern dispatch が wasm32-gc と diverge する

## Scope

#875 の cross-backend parity audit で観測された実 divergence。
multi-clause 関数の引数パターンおよび tuple 内 enum パターンが
wasm32 (linear memory) バックエンドで壊れる。wasm32-gc では正しく
動作するため、意味論 parity の acceptance を阻害する。

この issue の独立範囲だけを実装する。別の設計判断が必要なら
child issue を追加し、ここへ混ぜない。

## Evidence

最小プローブ(両 backend で同一ソース):

```ark
enum Opt { Some(i32), None }
fn describe(Opt::Some(v): Opt) -> String | v > 10 = "big"
fn describe(Opt::Some(_): Opt) -> String = "small"
fn describe(Opt::None: Opt) -> String = "none"
```

- wasm32-gc: `describe(Opt::None)` → `none`(正しい)
- wasm32: `describe(Opt::Some(42))`→`big`, `Some(5)`→`small`
  までは出力するが `describe(Opt::None)` で
  `memory fault at wasm address 0xfffffffc` → rc=134。
  仮説: payload 無しバリアントは linear で unboxed tag なのに、
  clause 分岐が record pointer として tag field を読んでいる。

第二プローブ(tuple 内 enum パターン):

```ark
let t = (o, 7)
match t {
    (Opt::Some(v), _) => "some",
    (Opt::None, _) => "none",
}
```

- wasm32: **compile 時点**で invalid core wasm
  (`type mismatch: expected i32 but nothing on stack`)。
- wasm32-gc: 未検証(要確認 — component emit で同様に壊れる場合は
  共通の match lowering 問題)。

同一意味を `match o { Opt::None => ... }`(非 tuple)で書くと
wasm32 でも正しく `none` を返す。つまり壊れるのは multi-clause
desugar が生成する tuple scrutinee 経路と、明示的な tuple 内 enum
パターンの両方である。

## Acceptance

- [ ] 上記両プローブが wasm32 で compile/validate/run し、
      wasm32-gc と stdout が一致する。
- [ ] 既存の `match_extensions` / multi-clause fixture が両
      backend で回帰なし。
- [ ] diverge の root cause(pattern 分岐が payload-less variant の
      storage 形を区別していない、等)がコードコメントまたは
      issue 内に記録される。

## Required evidence

- implementation commit SHA
- positive fixture(両プローブ相当、run: 登録)
- 両 backend の compile/validate/runtime 出力

## False-done guard

- [ ] Acceptance の一部だけで done にしない
- [ ] TODO/skip/hack を残す場合は tracking issue + removal condition を必須にする
- [ ] 親 #875 の parity 表を evidence 付きで更新する
