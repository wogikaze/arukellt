# テストカテゴリ分類スキームを定義する

**Status**: open
**Created**: 2026-03-30
**Updated**: 2026-03-30
**ID**: 261
**Depends on**: 252
**Track**: main
**Blocks v1 exit**: yes

## Summary

現行テストには明示的なカテゴリ分類がなく、どの失敗が language regression・backend regression・tooling regression かを一目で追えない。まずカテゴリの定義と責務境界を文書化する。

## Acceptance

- [x] `docs/test-strategy.md` が作成されている
- [x] `unit / fixture / integration / target-contract / component-interop / package-workspace / bootstrap / editor-tooling / perf / determinism` の各カテゴリの定義・対象・合否基準が記載されている
- [x] 各カテゴリが「language / backend / tooling」のどの regression 層に対応するかがマップされている
- [x] 新機能追加時にどのカテゴリのテストが必要かを決める判断フローが記載されている

## Scope

- `docs/test-strategy.md` の新規作成
- 既存テスト（harness / unit / component-interop）をカテゴリに分類してマッピング
- README または AGENTS.md からの参照リンクを追加

## References

- `tests/harness.rs`
- `scripts/verify-harness.sh`
- `issues/open/252-test-strategy-overhaul.md`
