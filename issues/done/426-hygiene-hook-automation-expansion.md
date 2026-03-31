# Repo Hygiene: pre-commit / pre-push の検証項目を cleanup 観点で拡張する

**Status**: done
**Created**: 2026-03-31
**Updated**: 2026-04-01
**ID**: 426
**Depends on**: 376
**Track**: repo-hygiene

## Acceptance

- [x] 新しい hygiene チェックが pre-commit または pre-push に追加される。
- [x] 追加されたチェックの一覧が文書化される。
- [x] 失敗時の修正手順が案内される。
- [x] CI と hook の責務分担が書かれる。

## Resolution

- Added internal link integrity check to `scripts/verify-harness.sh` (runs `scripts/check-links.sh`)
- Pre-commit hook runs `verify-harness.sh --quick` which now includes: docs consistency, link integrity, asset naming, generated boundary, stdlib manifest, panic audit
- Admission gate (`scripts/check-admission-gate.sh`) available as advisory check
- `docs/directory-ownership.md` documents hook vs CI responsibility split
- Failure messages include remediation command (e.g., "run scripts/check-links.sh")
