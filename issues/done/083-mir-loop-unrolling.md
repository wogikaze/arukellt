# MIR: ループ展開 (Loop Unrolling) パス

**Status**: open
**Created**: 2026-03-28
**Updated**: 2026-03-28
**ID**: 083
**Depends on**: 080
**Track**: mir-opt
**Blocks v4 exit**: no

## Summary

固定長のループ (コンパイル時定数回数) を本体のコピーに展開するパスを追加する。
展開後に DCE・const_fold が追加削減できるケースが多く、
特に小さい配列の処理 (4〜16要素) で効果大。

## 受け入れ条件

1. `passes/loop_unroll.rs`: ループ上限が定数でかつ ≤ 16 の場合に展開
2. 展開後に `const_fold` → `dce` を自動実行
3. 展開後のコードサイズが元の 8x を超える場合は展開しない (コードサイズ上限)
4. `--opt-level 2` でのみ有効

## 参照

- roadmap-v4.md §5.2
