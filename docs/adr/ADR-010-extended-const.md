# ADR-010: Extended Const Expressions (Wasm)

ステータス: **ACCEPTED** — 実装見送り。heap pointer 初期化は単純定数で十分

決定日: 2026-03-28（2026-07-10 改訂: 実装見送りを反映）

---

## 文脈

WebAssembly Extended Const 提案により、定数式の中で `i32.add`, `i32.sub`,
`i32.mul` (および `i64` 版) が使用可能になった。これにより、グローバル変数の
初期値・データセグメントのオフセット・要素セグメントのオフセットで算術演算を
記述できる。

wasmtime 29+ および主要ランタイムは Extended Const をデフォルトで有効化している。
`wasm-encoder` 0.225 は `ConstExpr::with_i32_add()` 等のビルダーメソッドを提供する。

---

## 決定（当初）

**`wasm32-gc` / selfhost emitter に extended const のインフラを追加する。**（当初決定・後に撤回）

- selfhost emitter (`src/compiler/emitter.ark`) に extended const のための型安全なビルダーヘルパーを実装
- `opt_level >= 2` のとき、heap pointer グローバルの初期値を extended const で出力:
  `(i32.add (i32.const DATA_START) (i32.const data_size))`
- `opt_level < 2` または `data_size == 0` の場合は従来の `(i32.const offset)` を維持
- ユーザー定義グローバル変数が将来導入された際、`global.get` を含む
  extended const 式にも対応可能な基盤を提供

---

## 改訂（2026-07-10）: 実装見送り

当初の「extended const インフラを追加する」決定は撤回する。issue #065 は「done」とマークされていたが、実際には extended const の実装は
行われなかった。調査の結果、以下の理由から実装を見送る:

1. **heap pointer 初期化は単純定数で十分**: `sections_memory.ark:23-31` の
   `emit_heap_global_entry` は `OP_I32_CONST` で `heap_start_from_data_offset(data_offset)`
   の計算結果を直接出力している。コンパイル時に値が確定するため、extended const で
   実行時に計算する必要がない
2. **恩恵が限定的**: ユーザー定義グローバル変数が未実装のため、`global.get` を
   含む extended const 式の使用ケースが存在しない
3. **Wasm GC への移行**: ADR-035 の Wasm GC 実装が進行中であり、GC target では
   heap pointer の線形メモリ依存自体が将来的に縮小する可能性がある

**現在の状態**: `sections_memory.ark` は `OP_I32_CONST` のみを使用。
extended const のビルダーヘルパーは未実装。

将来的にユーザー定義グローバル変数が導入される際、extended const の再評価を
検討する。その際は本 ADR を supersede する新規 ADR を起こすこと。

---

## 参照

- `docs/spec/spec-3.0.0/proposals/extended-const/Overview.md`
- `issues/done/065-wasm-extended-const.md` — done マークだが実装は未完了
- [WebAssembly Extended Const Proposal](https://github.com/WebAssembly/extended-const)
