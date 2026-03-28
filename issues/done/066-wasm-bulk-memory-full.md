# Wasm Bulk Memory: memory.copy / memory.fill / table.copy フル対応

**Status**: open
**Created**: 2026-03-28
**Updated**: 2026-03-28
**ID**: 066
**Depends on**: —
**Track**: wasm-feature
**Blocks v4 exit**: no

**Status note**: Wasm proposal — deferred to v5+. Not implemented.

## Summary

WebAssembly Bulk Memory 提案 (`docs/spec/spec-1.0.0/proposals/bulk-memory-operations/Overview.md`) の
`memory.copy`・`memory.fill`・`memory.init`・`table.copy`・`table.init`・`elem.drop`・`data.drop` を
T3 emitter で活用する。現在は `array.new_data` で passive data segment を消費しているが、
メモリ間コピーやゼロ初期化には bulk memory 命令が高速 (SIMD 等でランタイムが最適化可能)。

## 受け入れ条件

1. `std/bytes` の `memcpy` 相当関数が `memory.copy` を emit する
2. ゼロ初期化バッファが `memory.fill 0` を emit する
3. `table.copy` を使った関数テーブルのコピーサポート
4. 対応する MIR intrinsic を `std/wasm` に追加 (`wasm_memory_copy`, `wasm_memory_fill`)

## 実装タスク

1. `ark-wasm/src/emit/t3_wasm_gc.rs`: `memory.copy` / `memory.fill` emit ヘルパー追加
2. `std/wasm/mod.ark`: `memory_copy(dst, src, len)` / `memory_fill(ptr, val, len)` 追加
3. `std/bytes/mod.ark`: 内部実装で `wasm::memory_copy` を呼ぶように変更

## 参照

- `docs/spec/spec-1.0.0/proposals/bulk-memory-operations/Overview.md`
