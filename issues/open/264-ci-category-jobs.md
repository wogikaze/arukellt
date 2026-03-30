# CI 上でテストカテゴリ別ジョブを構成する

**Status**: open
**Created**: 2026-03-30
**Updated**: 2026-03-30
**ID**: 264
**Depends on**: 261, 262, 263
**Track**: main
**Blocks v1 exit**: yes

## Summary

現行 CI は unit テスト・fixture harness が中心で、カテゴリ別に独立したジョブが存在しない。261-263 で定義・整備されたカテゴリを CI ジョブとして配線し、各層の責務を CI 上で明示する。

## Acceptance

- [ ] CI に `unit / fixture / integration / target-contract / component-interop / package-workspace / bootstrap / editor-tooling / determinism` の各ジョブが存在する
- [ ] 各ジョブが独立して失敗・成功を報告する
- [ ] `perf` ジョブは必須ではなく、schedule または手動 dispatch で実行される
- [ ] PR マージ前に通過必須なジョブが明示されている

## Scope

- `.github/workflows/ci.yml` にカテゴリ別ジョブを追加
- 各ジョブの `needs` 依存関係を整理
- マージ前必須ジョブを branch protection rule で設定

## References

- `.github/workflows/ci.yml`
- `issues/open/242-ci-layer-structure.md`
- `issues/open/261-test-category-classification-scheme.md`
- `issues/open/252-test-strategy-overhaul.md`
