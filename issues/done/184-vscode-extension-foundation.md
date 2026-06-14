---
Status: done
Created: 2026-03-29
Updated: 2026-06-14
ID: 184
Track: parallel
Depends on: 189, 190, 191
Orchestration class: implementation-ready
Blocks v1 exit: False
Status note: Parent issue for extension bootstrap, command/task wiring, and setup/inspection surfaces. Closed after child rollup audit.
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

## Child issues (rollup)

| ID | Status | Scope |
|----|--------|-------|
| #189 | done | extension package + language client bootstrap |
| #190 | done | commands / tasks / status surfaces |
| #191 | done | setup doctor / command graph / environment diff |

**Downstream (not in foundation scope):** #183 epic, #634 stdio LSP/DAP transport.

## References

- `issues/done/189-vscode-extension-package-and-language-client-bootstrap.md`
- `issues/done/190-vscode-commands-tasks-and-status-surfaces.md`
- `issues/done/191-vscode-setup-doctor-command-graph-and-environment-inspection.md`
- `extensions/arukellt-all-in-one/src/extension.js`
- `extensions/arukellt-all-in-one/src/ops-surfaces.js`
- `extensions/arukellt-all-in-one/src/test/extension.test.js`
- `src/compiler/lsp.ark`

## Verification

- `extensions/arukellt-all-in-one/src/test/extension.test.js` — Ops Surfaces (#191), Command Registration (#273), LSP Handshake (#273)
- `python3 scripts/manager.py verify quick`

## Audit resolution — 2026-06-14

**Classification:** `truly-done`

**Repo proof:** All three foundation children are in `issues/done/` with user-visible entrypoints in `extensions/arukellt-all-in-one/`:

- **Package / LSP bootstrap (#189):** `package.json` language registration; `extension.js` `startLanguageServer`, `discoverBinary`, `verifyBootstrap`
- **Commands / tasks / status (#190):** `registerCommands`, `registerTaskProvider`, status bar, output channels, `arukellt:*` task definitions
- **Setup / inspection (#191):** `ops-surfaces.js` — `presentSetupDoctor`, `revealCommandGraph`, `presentEnvironmentDiff`, `arukellt-command-graph` tree view

**Action:** Moved to `issues/done/`. The 2026-06-12 reopen reason was stale (#191 closed with executable command graph UI). Epic-level gaps remain under #183; transport gaps under #634.
