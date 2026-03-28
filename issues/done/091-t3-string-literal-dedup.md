# T3: 同一文字列リテラルのデータセグメント共有

**Status**: open
**Created**: 2026-03-28
**Updated**: 2026-03-28
**ID**: 091
**Depends on**: —
**Track**: backend-opt
**Blocks v4 exit**: no

## Summary

同じ文字列リテラル (例: `"hello"` が複数箇所に出現) を
同一の passive data segment にまとめ、`array.new_data` でそれを参照する。
roadmap-v4.md §5.3 で明示的に要求されている最適化。

## 受け入れ条件

1. `EmitContext.string_segments: HashMap<String, u32>` (文字列→セグメントインデックス) を持つ
2. 同一文字列の2回目以降は既存セグメントを再利用
3. データセクションの総サイズ削減を確認 (同一文字列が多いプログラム)
4. `--opt-level 1` 以上で有効

## 参照

- roadmap-v4.md §5.3
