# Canonical compiler path の一本化

**Status**: open
**Created**: 2026-03-31
**ID**: 281
**Depends on**: —
**Track**: main
**Blocks v1 exit**: no
**Priority**: 1 (最高)

## Summary

CoreHIR がまだ既定の compile 経路ではなく、legacy path が残っている。`check` は CoreHIR 経由だが `compile` は legacy のまま。実装は進んでいても、コンパイラの正規ルートがまだ一本化されていない。ここを終えない限り、以後の最適化・セルフホスト・デバッグは全部「片方だけ直る」状態になる。

## Current state

- `INTERFACE-COREHIR.md`: CoreHIR is **not yet the default** compile path
- `--mir-select legacy` がデフォルト（compile 時）
- `--mir-select corehir` がデフォルト（check 時）
- CoreHIR MIR lowering が `IfExpr`, `LoopExpr`, `TryExpr` をまだ desugar していない
- `validate_backend_legal_module` がこれらを reject する

## Acceptance

- [ ] CoreHIR lowering が `IfExpr` を制御フローグラフ形式（分岐 + 基本ブロック）に desugar する
- [ ] CoreHIR lowering が `LoopExpr` を制御フローグラフ形式に desugar する
- [ ] CoreHIR lowering が `TryExpr` を制御フローグラフ形式に desugar する
- [ ] `validate_backend_legal_module` が CoreHIR 経由の全 fixture で pass する
- [ ] `--mir-select` のデフォルトが `corehir` に切り替わる（compile 時も）
- [ ] legacy path を `#[deprecated]` または feature gate で隔離する
- [ ] 全 588+ harness fixture が CoreHIR path で pass する
- [ ] `INTERFACE-COREHIR.md` の compile path status が「default」に更新される
- [ ] `docs/current-state.md` の Pipeline 節が一本化を反映する

## Approach

1. `crates/ark-hir/src/lower.rs` の `lower_check_output_to_mir` に `IfExpr` → branch + basic-block 変換を実装
2. 同様に `LoopExpr` → loop header + back-edge + break 変換を実装
3. 同様に `TryExpr` → match-on-result + early-return 変換を実装
4. `validate_backend_legal_module` を全 fixture で通す
5. `MirSelection::default()` を `CoreHir` に切り替え
6. Legacy path を deprecated にマーク
7. 全 harness を CoreHIR path で再検証

## References

- `INTERFACE-COREHIR.md`
- `crates/ark-hir/src/hir.rs`
- `crates/ark-hir/src/lower.rs`
- `crates/ark-hir/src/validate.rs`
- `crates/ark-mir/src/mir.rs`
- `docs/current-state.md` §Pipeline
