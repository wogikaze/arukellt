# 実行時性能: GC write barrier 削減 (immutable フィールド検出)

**Status**: open
**Created**: 2026-03-28
**Updated**: 2026-03-28
**ID**: 104
**Depends on**: —
**Track**: runtime-perf
**Blocks v4 exit**: no

## Summary

Wasm GC の `struct.set` は wasmtime の write barrier を発生させる可能性がある。
一度だけ書き込まれるフィールド (実質 immutable) を `(field (mut ...)`) から
`(field ...)` (immutable) に変更することで write barrier を排除し、
GC スキャン時のオーバーヘッドを削減する。

## 受け入れ条件

1. MIR 解析で「構築時1回のみ書き込まれるフィールド」を検出
2. T3 emitter でそのフィールドを immutable (`const`) として宣言
3. immutable フィールドへの二重書き込みはコンパイルエラー
4. `binary_tree.ark` ベンチマークで GC pause 時間が削減されることを確認

## 参照

- `docs/spec/spec-3.0.0/proposals/gc/MVP.md` §struct mutability
