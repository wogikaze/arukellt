---
Status: open
Created: 2026-03-28
Updated: 2026-03-28
ID: 50
Track: wasm-feature
Depends on: —
Orchestration class: implementation-ready
---
# Wasm Memory64: i64 アドレス空間対応
**Blocks v4 exit**: no

**Status note**: Wasm proposal — deferred to v5+. Not implemented.

## Summary

WebAssembly Memory64 提案 (`docs/spec/spec-3.0.0/proposals/memory64/Overview.md`) を T3 emitter に実装する。
現在 `memory64: false` でハードコードされており、4GB 超のリニアメモリを使う大規模データ処理に対応できない。
T3 (GC-native) では線形メモリは WASI I/O 専用 (1〜4 ページ) だが、将来の大規模 stdlib や
数値計算ライブラリ向けに memory64 オプションフラグを実装しておく。

## 受け入れ条件

1. `--memory64` フラグでコンパイル時に memory64 モードを選択可能
2. `memory64: true` 時、ロード/ストア命令のアドレスを `i64` で渡す
3. WASI P1 の iovec ポインタも `i64` として渡す (WASI P1 memory64 variant)
4. wasmtime の memory64 サポートで動作確認
5. デフォルトは `false` (後方互換性)

## 実装タスク

1. `ark-wasm/src/emit/t3_wasm_gc.rs`: `memory64` フラグを `EmitOptions` に追加
2. メモリ型生成時の `memory64` フィールドを `opts.memory64` で制御
3. ロード/ストアのアドレス引数型を `i64` に変更するモード分岐
4. `crates/arukellt/src/main.rs`: `--memory64` CLI フラグ追加

## 参照

- `docs/spec/spec-3.0.0/proposals/memory64/Overview.md`