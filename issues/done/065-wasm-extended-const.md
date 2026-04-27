---
Status: done
Created: 2026-03-28
Updated: 2026-03-28
ID: 53
Track: wasm-feature
Depends on: —
Orchestration class: implementation-ready
Blocks v4 exit: False
Status note: Infrastructure added. Extended const used for heap pointer init at opt_level >= 2.
# Wasm Extended Const: 定数式での算術演算
---
# Wasm Extended Const: 定数式での算術演算


## Summary

WebAssembly Extended Const 提案 (`docs/spec/spec-3.0.0/proposals/extended-const/Overview.md`) により、
グローバル変数の初期値・データセグメントのオフセット・要素セグメントのオフセットで
`i32.add`/`i32.sub`/`i32.mul` (および `i64` 版) が使用可能になる。
静的テーブルの計算済みオフセットや、大きな定数配列の初期化で有効。

## 受け入れ条件

1. T3 emitter が `GlobalSection` で extended const 式を生成できる
2. MIR の `Const` + `BinOp` を定数式として fold して global initializer に使用
3. `--opt-level 2` での定数畳み込み後に extended const 式として emit
4. wasmtime が extended const を有効化している確認

## 参照

- `docs/spec/spec-3.0.0/proposals/extended-const/Overview.md`