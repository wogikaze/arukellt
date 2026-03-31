# CoreHIR lowering: LoopExpr を制御フローに desugar する

**Status**: open
**Created**: 2026-03-31
**ID**: 282
**Depends on**: 281
**Track**: main
**Priority**: 2

## Summary

`Operand::LoopExpr` が backend-illegal のまま残っている。loop header + back-edge + break への変換が必要。

## Current state

- `crates/ark-mir/src/mir.rs`: `LoopExpr { init, body, result }` が Operand enum に存在
- legacy path の `func.rs` では loop を正しく lowering している
- CoreHIR path では未処理

## Acceptance

- [ ] `Operand::LoopExpr` が loop header / back-edge / break 形式の MirStmt 列に変換される
- [ ] `while`, `loop`, `for` を含む fixture が CoreHIR path 単独で compile & run 成功
- [ ] `validate_backend_legal_module` が loop 含む MIR で pass する

## References

- `crates/ark-mir/src/lower/mod.rs`
- `crates/ark-mir/src/lower/func.rs` (legacy の loop lowering)
- `crates/ark-mir/src/mir.rs`
