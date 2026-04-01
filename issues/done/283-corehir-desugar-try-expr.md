# CoreHIR lowering: TryExpr を制御フローに desugar する

**Status**: done
**Created**: 2026-03-31
**Updated**: 2025-07-15
**ID**: 283
**Depends on**: 281
**Track**: corehir
**Blocks v1 exit**: no
**Priority**: 3

## Summary

`Operand::TryExpr` (`expr?` 構文) が backend-illegal のまま。match-on-result + early-return への変換が必要。

## Current state

- `crates/ark-mir/src/mir.rs`: `TryExpr { expr, from_fn }` が Operand enum に存在
- legacy path では `?` を result 分岐に展開している
- CoreHIR path では未処理

## Acceptance

- [x] `Operand::TryExpr` が match-on-Result + early-return 形式の MirStmt 列に変換される
- [x] `?` 演算子を含む fixture が CoreHIR path 単独で compile & run 成功
- [x] `validate_backend_legal_module` が try 含む MIR で pass する

## References

- `crates/ark-mir/src/lower/mod.rs`
- `crates/ark-mir/src/lower/func.rs`
- `crates/ark-mir/src/mir.rs`
