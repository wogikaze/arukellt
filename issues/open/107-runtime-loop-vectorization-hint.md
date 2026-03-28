# 実行時性能: ループのベクトル化可能性アノテーション

**Status**: open
**Created**: 2026-03-28
**Updated**: 2026-03-28
**ID**: 107
**Depends on**: 064
**Track**: runtime-perf
**Blocks v4 exit**: no

**Status note**: Depends on #064 (branch hinting MIR infrastructure). WASM SIMD autovectorization hints are not yet standardized. Deferred to v5+.

## Summary

Branch Hinting 提案 (#064) の発展として、
独立したループイテレーション (SIMD 化可能) に対して
wasmtime の JIT が SIMD 最適化を適用できるようアノテーションを付与する。
WASM SIMD (v128) は roadmap-v4.md の非対象だが、
ランタイムのオートベクタライザへのヒントは有効。

## 受け入れ条件

1. MIR の `LoopInfo` に `vectorizable: bool` フラグを追加
2. ループ体が独立なイテレーション (データ依存なし) の場合に `vectorizable = true`
3. Branch Hinting カスタムセクションに loop vectorization ヒントを記録
4. wasmtime の autovectorizer がヒントを認識した場合の性能改善を確認

## 参照

- `docs/spec/spec-3.0.0/proposals/branch-hinting/Overview.md`
