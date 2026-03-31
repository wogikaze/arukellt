# Repo Hygiene: benchmark / test asset の naming linter を入れる

**Status**: done
**Created**: 2026-03-31
**Updated**: 2026-03-31
**ID**: 421
**Depends on**: 374
**Track**: repo-hygiene
**Blocks v1 exit**: no
**Priority**: 5

## Summary

命名規約を文書化するだけでなく、違反を機械的に検出する linter を追加する。

## Acceptance

- [x] asset naming checker が追加される
- [x] 許可される命名規則が 1 つに定義される (snake_case)
- [x] 違反時に修正候補またはルール説明が出る
- [x] pre-commit / CI で走る (verify-harness check #15)

## Implementation

- `scripts/check-asset-naming.sh`: checks benchmarks/*.ark and tests/fixtures/**/*.ark
- snake_case enforced: lowercase letters, digits, underscores, must start with letter
- Provides sed-based fix suggestions for violations
- Integrated into verify-harness.sh as background check
