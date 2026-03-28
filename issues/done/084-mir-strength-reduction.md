# MIR: Strength Reduction — 乗算→シフト、除算→逆数乗算

**Status**: open
**Created**: 2026-03-28
**Updated**: 2026-03-28
**ID**: 084
**Depends on**: —
**Track**: mir-opt
**Blocks v4 exit**: no

## Summary

定数との乗算・除算・剰余演算をより安価な命令に置き換えるパスを追加する。
Wasm の乗算は i32.mul だが、2のべき乗の場合は i32.shl の方が実行速度が速いケースがある。
また、定数除算を乗算 + シフトに変換する「magic number」最適化を実装する。

## 受け入れ条件

1. `passes/strength_reduction.rs`: `x * 2^n` → `x << n`、`x / 2^n` → `x >> n`
2. 符号付き除算の magic number 変換 (Hacker's Delight アルゴリズム)
3. Wasm バイナリの mul/div 命令数の削減を確認
4. `--opt-level 2` でのみ有効

## 参照

- Hacker's Delight, Chapter 10
