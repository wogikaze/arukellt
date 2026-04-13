# CoreHIR lowering: IfExpr を制御フローに desugar する

**Status**: open
**Created**: 2026-03-31
**Updated**: 2026-04-13
**ID**: 281
**Depends on**: —
**Track**: corehir
**Blocks v1 exit**: no
**Priority**: 1


## Reopened by audit — 2026-04-13

**Reason**: CoreHIR IfExpr still backend-illegal.

**Action**: Moved from issues/done/ to issues/open/ by false-done audit.

## Summary

`lower_hir_to_mir()` は現在スタブで空の MirModule を返す。`Operand::IfExpr` が backend-illegal のまま残り、`validate_backend_legal_module` が reject する。CoreHIR path を default にするための最初のブロッカー。

## Current state

- `crates/ark-mir/src/lower/mod.rs:209-281`: `lower_hir_to_mir()` は統計だけ取って空 MirModule を返す
- `crates/ark-mir/src/mir.rs:419-424`: `is_backend_legal_operand` が `IfExpr` を reject
- fallback で legacy path に転送されるため実害はないが、一本化できない

## Acceptance

- [x] `Operand::IfExpr` が branch + basic-block 形式の `MirStmt` 列に変換される
- [x] 変換後の MIR が `validate_backend_legal_module` を pass する
- [x] `if` を含む fixture（少なくとも 50 個）が CoreHIR path 単独で compile & run 成功

## References

- `crates/ark-mir/src/lower/mod.rs`
- `crates/ark-mir/src/mir.rs:419-424`
- `crates/ark-mir/src/validate.rs:34-48`
