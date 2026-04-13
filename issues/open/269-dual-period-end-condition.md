# Rust 実装と selfhost 実装の dual period 終了条件を定義する

**Status**: open
**Created**: 2026-03-30
**Updated**: 2026-04-13
**ID**: 269
**Depends on**: 266, 268
**Track**: main
**Blocks v1 exit**: no


## Reopened by audit — 2026-04-13

**Reason**: Dual-period not reached.

**Action**: Moved from issues/done/ to issues/open/ by false-done audit.

## Summary

selfhost が進んでも、Rust 実装をいつ削除するかの条件が定義されていないため、二重管理が続く可能性がある。dual period を終わらせるトリガー条件と移行手順を明文化する。

## Acceptance

- [x] `docs/compiler/bootstrap.md` に dual period 終了条件が記載されている
- [x] 終了条件が「268 の全 parity 条件を N 週間連続で達成」等の客観的な基準で定義されている
- [x] Rust 実装削除の移行手順（削除対象・削除手順・検証方法）が記載されている
- [x] 終了条件達成時に何をするかが issue template 等で追跡できる形になっている

## Scope

- `docs/compiler/bootstrap.md` に dual period セクションを追加
- 終了条件の定義（parity 達成の継続期間・除外条件等）
- 移行チェックリストの作成

## References

- `docs/compiler/bootstrap.md`
- `issues/open/266-selfhost-completion-definition.md`
- `issues/open/268-selfhost-parity-ci-verification.md`
- `issues/open/253-selfhost-completion-criteria.md`
