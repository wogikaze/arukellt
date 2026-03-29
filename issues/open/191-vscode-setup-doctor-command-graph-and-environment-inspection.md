# VS Code setup doctor + command graph + environment inspection

**Status**: open
**Created**: 2026-03-29
**Updated**: 2026-03-29
**ID**: 191
**Depends on**: 190
**Track**: parallel
**Blocks v1 exit**: no

## Summary

dependency/setup の自己診断、`check → compile → run → test` の command graph、local / CI / profile 間の environment diff を extension から扱えるようにする。通常コマンド実行とは別の運用支援 UX として分離する。

## Acceptance

- [ ] setup doctor と dependency diagnosis の責務が追跡できる
- [ ] command graph UI と実行導線が定義されている
- [ ] environment / profile diff の責務が issue queue 上で追跡できる

## References

- `issues/open/184-vscode-extension-foundation.md`
- `issues/open/190-vscode-commands-tasks-and-status-surfaces.md`
- `docs/current-state.md`
