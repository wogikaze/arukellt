# Selfhost MIR lowering: 集合体操作を実装する

**Status**: open
**Created**: 2026-03-31
**Updated**: 2026-03-31
**ID**: 315
**Depends on**: 313
**Track**: selfhost-backend
**Blocks v1 exit**: no
**Priority**: 6

## Summary

struct construction / field access、enum variant construction / destructuring、array / tuple 操作を MIR に lowering する。

## Current state

- `src/compiler/mir.ark`: MIR_STRUCT_NEW, MIR_FIELD_GET 等の opcode は定義済みだが生成コードなし
- `src/compiler/parser.ark`: struct literal / field access / enum variant は parse 可能
- `src/compiler/hir.ark`: StructType / EnumType の定義構造あり
- Rust 版 `crates/ark-mir/src/lower/expr.rs` で struct/enum の lowering を行う

## Acceptance

- [ ] struct literal が `MIR_STRUCT_NEW` + field 初期化命令に変換される
- [ ] field access (`s.x`) が `MIR_FIELD_GET` に変換される
- [ ] enum variant construction が tag + payload の命令列に変換される
- [ ] pattern match での enum destructuring が tag check + field extract に変換される
- [ ] array literal が `MIR_ARRAY_NEW` + 要素初期化に変換される

## References

- `src/compiler/mir.ark` — MIR_STRUCT_NEW, MIR_FIELD_GET 等
- `src/compiler/hir.ark` — StructType, EnumType
- `crates/ark-mir/src/lower/expr.rs` — Rust struct/enum lowering
