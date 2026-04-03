# テストカテゴリ分類スキームを定義する

**Status**: done
**Created**: 2026-03-30
**Updated**: 2026-04-03
**ID**: 261
**Depends on**: 252
**Track**: main
**Blocks v1 exit**: yes


---

## Closed by audit — 2026-04-03

**Reason**: All acceptance criteria verified by repo evidence.

**Evidence**: docs/test-strategy.md exists with all 10 test categories

**Action**: Moved from `issues/open/` → `issues/done/` by false-done audit (confirmed truly-done).


## Reopened by audit — 2026-04-03

**Reason**: This issue has `Status: open` in its frontmatter but was filed under `issues/done/`. The issue was never marked done; it was misplaced. All acceptance criteria remain unverified by repo evidence.

**Audit evidence**:
- `**Status**: open` in this file's own frontmatter confirms it was never closed.
- File was located at `issues/done/261-test-category-classification-scheme.md` — incorrect directory for an open issue.

**Action**: Moved from `issues/done/` → `issues/open/` by false-done audit (2026-04-03).

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
- `scripts/run/verify-harness.sh`
- `issues/open/252-test-strategy-overhaul.md`
