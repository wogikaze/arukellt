# MIR: CSE (Common Subexpression Elimination) パス

**Status**: open
**Created**: 2026-03-28
**Updated**: 2026-03-28
**ID**: 085
**Depends on**: —
**Track**: mir-opt
**Blocks v4 exit**: no

## Summary

同一基本ブロック内で同じ計算を複数回行う式を一度だけ計算してキャッシュするパスを追加する。
`struct.get` の繰り返しアクセスや、同じ `BinOp` が複数回出現するケースで効果大。
`const_fold` + `copy_prop` との組み合わせでさらに削減できる。

## 受け入れ条件

1. `passes/cse.rs`: 同一ブロック内の純粋な計算 (副作用なし) を重複排除
2. `struct.get` / `array.get` (境界チェック済みの場合) の CSE を対象
3. CSE で排除された計算数を `OptimizationSummary.cse_eliminated` に記録
4. `--opt-level 1` 以上で有効

## 参照

- roadmap-v4.md §5.2
