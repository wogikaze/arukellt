# WASI P2 ネイティブ: P1 アダプタ不要のコンポーネント直接生成

**Status**: open
**Created**: 2026-03-28
**Updated**: 2026-03-28
**ID**: 074
**Depends on**: —
**Track**: wasi-feature
**Blocks v4 exit**: no

**Status note**: WASI feature — deferred to v5+. Requires WASI P2 runtime maturity.

## Summary

現在の Component Model 出力は「Core Wasm → WASI P1 adapter → Component」という
2段変換パイプラインを使っている (`wasm-tools component new` + P1 アダプタ)。
WASI P2 ネイティブ対応では、Core Wasm が直接 WIT インターフェースをインポート/エクスポートする
コンポーネントを生成し、P1 アダプタオーバーヘッドをなくす。

## 背景

`wasm-tools component new` + `wasi_snapshot_preview1.reactor.wasm` は
アダプタモジュールのサイズ (~100KB) と変換オーバーヘッドを伴う。
P2 ネイティブでは Core Wasm が直接 `wasi:io/streams` 等をインポートするため、
バイナリサイズと起動時間が改善する。

## 受け入れ条件

1. `--wasi-version p2` フラグで P2 ネイティブコンポーネントをコンパイル
2. Core Wasm に `wasi:cli/environment@0.2.x` 等を直接 import するセクション生成
3. P1 アダプタなしで wasmtime 17+ で実行可能
4. バイナリサイズが P1 アダプタ版より 80KB 以上削減されることを確認

## 実装タスク

1. `ark-wasm/src/emit/t3_wasm_gc.rs`: WASI P2 モード分岐 (import 名を P2 形式に変更)
2. `ark-wasm/src/component/wrap.rs`: P2 ネイティブの場合 `component new` を迂回
3. WIT world 出力を `wasi:cli/command` ベースに変更

## 参照

- `docs/spec/spec-WASI-0.2.10/OVERVIEW.md`
- `docs/spec/spec-WASI-0.2.10/specifications/wasi-0.2.10/`
