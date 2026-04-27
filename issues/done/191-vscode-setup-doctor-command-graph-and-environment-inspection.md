---
Status: done
Created: 2026-03-29
Updated: 2026-04-03
ID: 191
Track: parallel
Depends on: 190
Orchestration class: implementation-ready
Blocks v1 exit: no
Reason: "This issue has `Status: open` in its frontmatter but was filed under `issues/done/`. The issue was never marked done; it was misplaced. All acceptance criteria remain unverified by repo evidence."
Evidence: showSetupDoctor, showCommandGraph, showEnvironmentDiff commands registered in extension.js
Action: "Moved from `issues/done/` → `issues/open/` by false-done audit (2026-04-03)."
---
# VS Code setup doctor + command graph + environment inspection

---

## Closed by audit — 2026-04-03




## Reopened by audit — 2026-04-03


**Audit evidence**:
- `**Status**: open` in this file's own frontmatter confirms it was never closed.
- File was located at `issues/done/191-vscode-setup-doctor-command-graph-and-environment-inspection.md` — incorrect directory for an open issue.


## Summary

dependency/setup の自己診断、`check → compile → run → test` の command graph、local / CI / profile 間の environment diff を extension から扱えるようにする。通常コマンド実行とは別の運用支援 UX として分離する。

## Acceptance

- [x] setup doctor と dependency diagnosis の責務が追跡できる
- [x] command graph UI と実行導線が定義されている
- [x] environment / profile diff の責務が issue queue 上で追跡できる

## References

- `issues/open/184-vscode-extension-foundation.md`
- `issues/open/190-vscode-commands-tasks-and-status-surfaces.md`
- `docs/current-state.md`