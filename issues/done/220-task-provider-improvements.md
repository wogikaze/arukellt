# Task provider: background task support and pre-execution validation

**Status**: done
**Created**: 2026-03-30
**Updated**: 2026-03-30
**ID**: 220
**Depends on**: none
**Track**: parallel
**Blocks v1 exit**: no

## Summary

background task 対応と task 実行前 validation を task provider に追加する。
基本 task provider（#190）を土台に、ウォッチモードと事前環境チェックを加える。

## Acceptance

- [ ] background task（`isBackground: true`）が動作し、ウォッチモードなどで継続実行できる
- [ ] task 実行前に環境・設定を validation し、問題があれば actionable なメッセージを出す

## Scope

### Background task

- `isBackground: true` task の定義
- begin / end pattern による background 判定
- ウォッチモード（`arukellt check --watch` 相当）対応
- task 終了検知と再起動

### Task 実行前 validation

- binary が存在するか確認してから実行
- `ark.toml` の読み取りエラーを事前検出
- target / emit の矛盾を検出
- 問題時に Doctor を開く導線

## References

- `issues/done/190-vscode-commands-tasks-and-status-surfaces.md`
- `issues/open/184-vscode-extension-foundation.md`
- `issues/open/191-vscode-setup-doctor-and-environment-inspection.md` (done)
- `extensions/arukellt-all-in-one/src/`
