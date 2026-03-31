# Repo Hygiene: benchmark / test asset の naming linter を入れる

**Status**: open
**Created**: 2026-03-31
**Updated**: 2026-03-31
**ID**: 421
**Depends on**: 374
**Track**: repo-hygiene
**Blocks v1 exit**: no
**Priority**: 5

## Summary

命名規約を文書化するだけでなく、違反を機械的に検出する linter を追加する。新規 asset が増えるたびに kebab-case / snake_case が混ざるのを防ぐ。

## Current state

- asset naming は人間の注意に依存している。
- 既存 asset でも揺れが潜んでいる可能性がある。
- playground や docs examples が増える前に縛りが必要。

## Acceptance

- [ ] asset naming checker が追加される。
- [ ] 許可される命名規則が 1 つに定義される。
- [ ] 違反時に修正候補またはルール説明が出る。
- [ ] pre-commit か CI で走る。

## References

- ``benchmarks/**``
- ``tests/fixtures/**``
- ``scripts/pre-commit-verify.sh``
