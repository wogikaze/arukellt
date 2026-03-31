# Repo Hygiene: archive / retention policy を定め、情報の鮮度を維持する

**Status**: open
**Created**: 2026-03-31
**Updated**: 2026-03-31
**ID**: 375
**Depends on**: —
**Track**: repo-hygiene
**Blocks v1 exit**: no
**Priority**: 18

## Summary

`issues/done/`、`docs/spec/`、`docs/migration/`、historical docs、ADR archive の retention policy を定め、「何を残す」「何を archive に寄せる」「何を削除する」のルールを明文化する。情報量が増え続ける repo で、閲覧者が current と historical を区別できる状態を維持する。

## Current state

- `issues/done/` に完了 issue が蓄積 (数十件)
- `docs/spec/` は archive と明示済み
- `docs/migration/` に移行資料が蓄積
- ADR は `docs/adr/` に蓄積 (decided / superseded の管理あり)
- retention policy (いつ archive に寄せるか、いつ削除するか) が明文化されていない

## Acceptance

- [ ] retention policy 文書が `docs/` に存在する
- [ ] policy が archive 移動条件、削除条件、保持期間を定める
- [ ] `issues/done/` の扱い (永久保持 or N 版後にアーカイブ) が明記される
- [ ] historical docs に archive 移動日と理由が記録される

## References

- `issues/done/` — 完了 issue
- `docs/spec/` — archive
- `docs/migration/` — 移行資料
- `docs/adr/` — ADR
