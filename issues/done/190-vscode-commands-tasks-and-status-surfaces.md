---
Status: done
Created: 2026-03-29
Updated: 2026-04-03
ID: 190
Track: parallel
Depends on: 189
Orchestration class: implementation-ready
Blocks v1 exit: no
Reason: "This issue has `Status: open` in its frontmatter but was filed under `issues/done/`. The issue was never marked done; it was misplaced. All acceptance criteria remain unverified by repo evidence."
Evidence: check/compile/run commands registered in extension.js, status bar and output channel present
Action: "Moved from `issues/done/` → `issues/open/` by false-done audit (2026-04-03)."
---
# VS Code commands / tasks / status surfaces

---

## Closed by audit — 2026-04-03




## Reopened by audit — 2026-04-03


**Audit evidence**:
- `**Status**: open` in this file's own frontmatter confirms it was never closed.
- File was located at `issues/done/190-vscode-commands-tasks-and-status-surfaces.md` — incorrect directory for an open issue.


## Summary

extension package の上で、Command Palette、task provider、status bar、output channel、restart-LSP、target / emit 設定 handoff を整備する。language client bootstrap とは別の command surface として追う。

## Acceptance

- [x] `check` / `compile` / `run` / restart-LSP の command surface が追跡できる
- [x] task provider / status bar / output channel の責務が整理されている
- [x] target / emit / adapter などの設定 handoff を issue queue 上で追跡できる

## References

- `issues/open/184-vscode-extension-foundation.md`
- `issues/open/189-vscode-extension-package-and-language-client-bootstrap.md`
- `crates/arukellt/src/commands.rs`