# selfhost 完了条件を定義し文書に固定する

**Status**: open
**Created**: 2026-03-30
**Updated**: 2026-03-30
**ID**: 266
**Depends on**: 253
**Track**: main
**Blocks v1 exit**: yes

## Summary

「selfhost できたかどうか」に対して、資料ごとに異なる答えが返る状態が続いている。完了条件を 1 行で言える形で文書に固定し、今後の判断基準とする。

## Acceptance

- [x] `docs/compiler/bootstrap.md` に selfhost 完了条件が「checklist」として明記されている
- [x] 完了条件が `Stage0→Stage1→Stage2 fixpoint` / `Stage1 fixture parity` / `CLI parity` / `diagnostic parity` / `determinism` の 5 項目で構成されている
- [x] 各項目の「達成 / 未達」を判定するコマンドまたは CI ジョブが対応付けられている
- [x] `docs/current-state.md` の selfhost セクションがこの checklist を参照している

## Scope

- `docs/compiler/bootstrap.md` の完了条件セクションを新規作成または書き直し
- 各条件に対応する検証コマンドを記載
- `docs/current-state.md` から `docs/compiler/bootstrap.md` への参照に変換

## References

- `docs/compiler/bootstrap.md`
- `docs/migration/v4-to-v5.md`
- `scripts/run/verify-bootstrap.sh`
- `issues/open/253-selfhost-completion-criteria.md`
