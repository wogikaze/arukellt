# 失敗時の層別特定（language/backend/tooling regression）を可能にする

**Status**: open
**Created**: 2026-03-30
**Updated**: 2026-03-30
**ID**: 265
**Depends on**: 264
**Track**: main
**Blocks v1 exit**: no

## Summary

テスト失敗時に、それが language regression・backend regression・tooling regression のどれかを一目で判別できる仕組みが不足している。開発者が問題を素早くトリアージできるよう、テスト命名・レポート・CI 表示を整備する。

## Acceptance

- [ ] テスト名に `[lang]` / `[backend]` / `[tooling]` 等のプレフィックスまたはラベルが付いている
- [ ] CI の失敗サマリーで「どの層で落ちたか」が一目で分かる
- [ ] 複数カテゴリにまたがる失敗の場合、その旨が明示される
- [ ] `docs/test-strategy.md` に層別特定の方針が記載されている

## Scope

- 既存 fixture テスト名のレビューと必要に応じたリネーム
- CI の失敗サマリー表示の改善（GitHub Actions の `::group::` 等を活用）
- `docs/test-strategy.md` に層別特定フローを追記

## References

- `tests/harness.rs`
- `issues/open/261-test-category-classification-scheme.md`
- `issues/open/264-ci-category-jobs.md`
- `issues/open/252-test-strategy-overhaul.md`
