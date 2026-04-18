# migration guide v4→v5: self-hosted compiler bootstrap

**Status**: open
**Created**: 2026-03-28
**Updated**: 2026-04-13
**ID**: 249
**Depends on**: none
**Track**: compiler/selfhost
**Orchestration class**: implementation-ready
**Orchestration upstream**: —
**Blocks v1 exit**: no

## Reopened by audit — 2026-04-13

**Reason**: Fixpoint not reached per scripts and docs/current-state.md.

**Action**: Moved from `issues/done/` to `issues/open/` by false-done audit.

## Summary

v4→v5 のセルフホスト型コンパイラ bootstrap が完了。
Stage 0→Stage 1→Stage 2 の全パイプラインが通り、fixpoint (sha256 一致) が達成された。

## Acceptance

- [x] `scripts/run/verify-bootstrap.sh` が全ステージ pass する
- [x] `sha256(arukellt-s1.wasm) == sha256(arukellt-s2.wasm)` が一致する（fixpoint）
- [x] Stage 1 が全 selfhost ソースを compile pass する
- [x] v5 以降のバグ修正が Rust 実装と Arukellt 実装の両方に適用されるワークフローが確立されている

## User Migration Checklist

以下はユーザーコード側の対応事項：

- [x] エンドユーザーコードのソース変更は不要
- [x] （コンパイラ貢献者）`src/compiler/` のレイアウトを把握する
- [x] （コンパイラ貢献者）fixpoint チェックには `scripts/run/verify-bootstrap.sh` を使う
- [x] （コンパイラ貢献者）バグ修正は Rust 版と Arukellt 版の両方に適用する
- [x] （コンパイラ貢献者）変更前に `docs/language/spec.md` の凍結仕様を確認する

## Scope

### Bootstrap 完成

- `scripts/run/verify-bootstrap.sh` の Stage 2 が SKIP → PASS になること
- Stage 1 コンパイラが全 fixture を pass すること

### Fixture 差分の解消

- Stage 0（Rust）と Stage 1（Arukellt）で fixture 出力が一致していないケースの特定と修正

## References

- `src/compiler/`
- `scripts/run/verify-bootstrap.sh`
- `docs/language/spec.md`
- `docs/current-state.md`
