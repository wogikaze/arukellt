# ADR-010: Extended Const Expressions (Wasm)

ステータス: **DECIDED** — T3 emitterにextended constのインフラを追加

決定日: 2026-03-28

---

## 文脈

WebAssembly Extended Const 提案により、定数式の中で `i32.add`, `i32.sub`,
`i32.mul` (および `i64` 版) が使用可能になった。これにより、グローバル変数の
初期値・データセグメントのオフセット・要素セグメントのオフセットで算術演算を
記述できる。

wasmtime 29+ および主要ランタイムは Extended Const をデフォルトで有効化している。
`wasm-encoder` 0.225 は `ConstExpr::with_i32_add()` 等のビルダーメソッドを提供する。

---

## 決定

**T3 emitter に extended const のインフラを追加する。**

- `crates/ark-wasm/src/emit/t3/const_expr.rs` に型安全なビルダーヘルパーを実装
- `opt_level >= 2` のとき、heap pointer グローバルの初期値を extended const で出力:
  `(i32.add (i32.const DATA_START) (i32.const data_size))`
- `opt_level < 2` または `data_size == 0` の場合は従来の `(i32.const offset)` を維持
- ユーザー定義グローバル変数が v5+ で導入された際、`global.get` を含む
  extended const 式にも対応可能な基盤を提供

---

## 現在のスコープ

- ユーザー定義グローバル変数は未実装のため、`global.get` を含む extended const
  式は使用されない
- 現在の適用箇所は heap pointer グローバルの初期化のみ
- MIR の `Const + BinOp` から extended const への変換は v5+ で検討

---

## 参照

- `docs/spec/spec-3.0.0/proposals/extended-const/Overview.md`
- `issues/open/065-wasm-extended-const.md`
- [WebAssembly Extended Const Proposal](https://github.com/WebAssembly/extended-const)
