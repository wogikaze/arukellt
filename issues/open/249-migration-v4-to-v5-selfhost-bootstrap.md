# migration guide v4→v5: self-hosted compiler bootstrap

**Status**: open
**Created**: 2026-03-28
**Updated**: 2026-03-30
**ID**: 249
**Depends on**: none
**Track**: compiler/selfhost
**Blocks v1 exit**: no

## Summary

v4→v5 のセルフホスト型コンパイラ bootstrap が未完了。
Stage 1 (`arukellt-s1.wasm`) の生成と Stage 2 との fixpoint 一致が達成されていない。
`src/compiler/*.ark` のセルフホスト実装は存在するが、`scripts/verify-bootstrap.sh` が FAILED を返す。

元ドキュメント: `docs/migration/v4-to-v5.md`（issue 化により移動済み）

## Acceptance

- [ ] `scripts/verify-bootstrap.sh` が全ステージ pass する
- [ ] `sha256(arukellt-s1.wasm) == sha256(arukellt-s2.wasm)` が一致する（fixpoint）
- [ ] Stage 1 が全 fixture test を pass する
- [ ] v5 以降のバグ修正が Rust 実装と Arukellt 実装の両方に適用されるワークフローが確立されている

## User Migration Checklist

以下はユーザーコード側の対応事項：

- [ ] エンドユーザーコードのソース変更は不要
- [ ] （コンパイラ貢献者）`src/compiler/` のレイアウトを把握する
- [ ] （コンパイラ貢献者）fixpoint チェックには `scripts/verify-bootstrap.sh` を使う
- [ ] （コンパイラ貢献者）バグ修正は Rust 版と Arukellt 版の両方に適用する
- [ ] （コンパイラ貢献者）変更前に `docs/language/spec.md` の凍結仕様を確認する

## Scope

### Bootstrap 完成

- `scripts/verify-bootstrap.sh` の Stage 2 が SKIP → PASS になること
- Stage 1 コンパイラが全 fixture を pass すること

### Fixture 差分の解消

- Stage 0（Rust）と Stage 1（Arukellt）で fixture 出力が一致していないケースの特定と修正

## References

- `src/compiler/`
- `scripts/verify-bootstrap.sh`
- `docs/language/spec.md`
- `docs/current-state.md`
