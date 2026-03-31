# Repo Hygiene: archive / retention policy を定め、情報の鮮度を維持する

**Status**: done
**Created**: 2026-03-31
**Updated**: 2026-04-01
**ID**: 375
**Depends on**: —
**Track**: repo-hygiene

## Acceptance

- [x] retention policy 文書が `docs/` に存在する
- [x] policy が archive 移動条件、削除条件、保持期間を定める
- [x] `issues/done/` の扱い (永久保持 or N 版後にアーカイブ) が明記される
- [x] historical docs に archive 移動日と理由が記録される

## Resolution

- Created `docs/retention-policy.md` covering completed issues, ADRs, historical docs, migration docs, generated files, build artifacts, and large files
- issues/done/ permanently retained as audit trail
- Migration docs retained for 2 major versions then archived
- ADRs append-only, never deleted
- Archive workflow documented with banner template and review schedule
