# Backend-illegal operand の棚卸し

**Status**: open
**Created**: 2026-03-31
**ID**: 306
**Depends on**: 281, 282, 283
**Track**: corehir
**Priority**: 4

## Summary

IfExpr / LoopExpr / TryExpr の 3 ノードは既知だが、CoreHIR path をデフォルトに昇格する前に、他に backend-illegal として残るノードがないか網羅的に監査する必要がある。昇格後に未発見ノードで壊れるリスクを潰す。

## Current state

- `crates/ark-mir/src/mir.rs:419-424`: `is_backend_legal_operand` は IfExpr / LoopExpr / TryExpr のみチェック
- `lower_hir_to_mir()` が空 MirModule を返すため、実際に CoreHIR → MIR で生成される全 Operand の種類が未確認
- legacy path は `func.rs` で全 AST ノードを lowering しているが、CoreHIR path と 1:1 対応が取れているか不明

## Acceptance

- [ ] CoreHIR の全 HIR ノード種 (`crates/ark-hir/src/hir.rs`) を列挙し、MIR Operand への変換状態を表にする
- [ ] `is_backend_legal_operand` が reject すべきノードが 3 種のみであることを確認、または追加すべきノードを特定
- [ ] 全 fixture を `--mir-select corehir` で compile し、backend-illegal エラーの種類を収集
- [ ] 結果を #284 の前提として記録

## References

- `crates/ark-hir/src/hir.rs`
- `crates/ark-mir/src/mir.rs:419-424`
- `crates/ark-mir/src/lower/mod.rs`
