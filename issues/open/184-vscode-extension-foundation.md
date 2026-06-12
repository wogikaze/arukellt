---
Status: open
Created: 2026-03-29
Updated: 2026-06-12
ID: 184
Track: parallel
Depends on: 189, 190, 191
Orchestration class: implementation-ready
Blocks v1 exit: False
Status note: Parent issue for extension bootstrap, command/task wiring, and setup/inspection surfaces.
---

## Reopened by audit — 2026-06-12

**Reopen reason:** Foundation rollup depends on #189-191; #191 command-graph UI and audit-reopen debt remain.

**Violated acceptance:** Extension foundation children complete with user-visible entrypoints

**Evidence files:**
- `issues/done/191-vscode-setup-doctor-command-graph-and-environment-inspection.md`

**Follow-up split issue:** see #634 for stdio LSP/DAP transport where applicable

---

# VS Code: `arukellt-all-in-one` 拡張の基盤整備

## Summary

`arukellt-all-in-one` の foundation は、language client の起動、VS Code command / task surface、環境診断や command graph などの土台を分けて追う必要がある。
LSP 機能そのものや test / debug / manifest 固有の責務は別 issue に分離する。

## Acceptance

- [x] #189, #190, #191 が完了している
- [x] extension package / command-task surface / setup-inspection surface の責務が child issue に分解されている
- [x] foundation 系の残課題が issue queue 上で追跡できる

## References

- `issues/open/189-vscode-extension-package-and-language-client-bootstrap.md`
- `issues/open/190-vscode-commands-tasks-and-status-surfaces.md`
- `issues/open/191-vscode-setup-doctor-command-graph-and-environment-inspection.md`
- `crates/ark-lsp/src/lib.rs`
- `crates/arukellt/src/main.rs`
- `crates/arukellt/src/commands.rs`
