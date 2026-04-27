---
Status: open
Created: 2026-03-28
Updated: 2026-03-28
ID: 51
Track: wasm-feature
Depends on: —
Orchestration class: implementation-ready
---
# Wasm Multi-Memory: 複数メモリモジュール対応
**Blocks v4 exit**: no

**Status note**: Wasm proposal — deferred to v5+. Not implemented.

## Summary

WebAssembly Multi-Memory 提案 (`docs/spec/spec-3.0.0/proposals/multi-memory/Overview.md`) を実装し、
「WASI I/O 専用リニアメモリ」と「大容量データ用リニアメモリ」を分離できるようにする。
T3 では現在 1 つのリニアメモリを WASI I/O バッファに使用しているが、
数値計算や画像処理で別途大きなメモリが必要な場合に複数メモリが有効。

## 受け入れ条件

1. `MemorySection` に複数メモリを追加できる API を `ark-wasm` に実装
2. ロード/ストア命令にメモリインデックス即値を付与 (multi-memory 構文)
3. `std/wasm` モジュールに `memory_grow(mem_idx, pages)` / `memory_size(mem_idx)` を追加
4. 既存の単一メモリコードへの regression なし

## 参照

- `docs/spec/spec-3.0.0/proposals/multi-memory/Overview.md`