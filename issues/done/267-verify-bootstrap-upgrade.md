# verify-bootstrap.sh を達成判定本体へ昇格させる

**Status**: open
**Created**: 2026-03-30
**Updated**: 2026-03-30
**ID**: 267
**Depends on**: 266
**Track**: main
**Blocks v1 exit**: yes

## Summary

`scripts/run/verify-bootstrap.sh` は Stage 0 で `main.wasm` が生成されない場合に Stage 1/2 を skip する構造になっており、fixpoint の継続検証が担保されていない。このスクリプトを skip 前提の scaffold から達成判定の本体へ昇格させる。

## Acceptance

- [x] `scripts/run/verify-bootstrap.sh` が Stage 0/1/2 を条件なしで逐次実行する
- [x] Stage 0 の失敗時に具体的なエラーメッセージとデバッグヒントを出力して終了する（skip しない）
- [x] Stage 2 で byte-exact fixpoint を確認するステップが有効になっている
- [x] スクリプトが `--check` モードで「達成済みか否か」を exit code で返せる

## Scope

- `scripts/run/verify-bootstrap.sh` の skip 条件を除去
- 各 stage の失敗ログを詳細化
- byte-exact fixpoint 確認ステップの実装（`diff` または `sha256sum` 比較）
- `--check` フラグの追加

## References

- `scripts/run/verify-bootstrap.sh`
- `docs/compiler/bootstrap.md`
- `issues/open/266-selfhost-completion-definition.md`
- `issues/open/253-selfhost-completion-criteria.md`
