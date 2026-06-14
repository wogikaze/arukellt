---
Status: done
Created: 2026-03-29
Updated: 2026-06-14
ID: 183
Track: parallel
Depends on: 184, 185, 186, 187, 188, 205, 206, 207
Orchestration class: implementation-ready
Blocks v1 exit: False
Status note: Parent issue for the all-in-one VS Code experience. Core editor, test, debug, project metadata, and cross-cutting DX surfaces are tracked separately. Closed after child rollup audit.
---

## Reopened by audit — 2026-06-12

**Reopen reason:** Epic rollup closed while child issues (#191, #479, legacy LSP nav) remain false-done or unmet.

**Violated acceptance:** All decomposed extension/LSP children truly done with repo proof

**Evidence files:**
- `issues/done/191-*.md`
- `issues/done/479-*.md`
- `selfhost LSP feature gaps`

**Follow-up split issue:** see #634 for stdio LSP/DAP transport where applicable

---

# VS Code: `arukellt-all-in-one` 拡張機能 epic

## Summary

`arukellt-all-in-one` は Arukellt 向け VS Code 体験を 1 つの拡張に集約する epic である。
基礎 editor support に加え、test / debug / project scripts / docs / pipeline / security まで含む DX 面を child issue に分けて追跡する。

## Acceptance

- [x] #184, #185, #186, #187, #188, #205, #206, #207 が完了している
- [x] VS Code 体験の責務が foundation / authoring / test / debug / project / cross-cutting DX に分離されている
- [x] all-in-one 拡張の残課題が issue queue 上で追跡できる
- [x] `docs/debug-support.md` Limitations section synced with DAP reality (simulated breakpoints, static variables, #638 scope) — audited 2026-06-14; extension README mirrors limitations; runtime gaps tracked under #638

## Child issues (rollup)

| ID | Status | Scope |
|----|--------|-------|
| #184 | done | extension foundation (package, commands/tasks, setup doctor) |
| #185 | done | LSP authoring workflows (rename, code actions, formatting) |
| #186 | done | test runner + VS Code Test Explorer surface |
| #187 | done | debug surface (DAP + source-level stepping) |
| #188 | done | `ark.toml` project / workspace / scripts |
| #205 | done | docs ↔ code intelligence surfaces |
| #206 | done | interactive compiler pipeline + inline profiling |
| #207 | done | extension security surface analysis |

**Downstream (not in epic scope):** #638 runtime-level Wasm debugging (live variables, Wasm breakpoint injection); #634 stdio LSP/DAP transport where applicable; #480 extension README/settings doc sync.

## References

- `issues/done/184-vscode-extension-foundation.md`
- `issues/done/185-lsp-ide-workflows-rename-code-actions-formatting.md`
- `issues/done/186-test-runner-and-vscode-test-explorer-surface.md`
- `issues/done/187-debug-surface-dap-and-source-level-debugging.md`
- `issues/done/188-ark-toml-project-workspace-and-scripts.md`
- `issues/done/205-docs-and-codebase-intelligence-surfaces.md`
- `issues/done/206-interactive-compiler-pipeline-and-inline-profiling.md`
- `issues/done/207-extension-security-surface-analysis.md`
- `extensions/arukellt-all-in-one/`
- `docs/debug-support.md`
- `crates/ark-lsp/src/lib.rs`
- `crates/arukellt/src/main.rs`

## Verification

- `extensions/arukellt-all-in-one/src/test/extension.test.js` — foundation (#184/#191), DAP registration (#280), breakpoint-stop E2E (#255), marketplace metadata
- `python3 scripts/manager.py verify quick`

## Audit resolution — 2026-06-14

**Classification:** `truly-done`

**Repo proof:** All eight epic children are in `issues/done/` with user-visible surfaces in `extensions/arukellt-all-in-one/`:

- **Foundation (#184):** `extension.js` language client bootstrap, `registerCommands`, `registerTaskProvider`, `ops-surfaces.js` setup doctor / command graph / environment diff
- **Authoring (#185):** LSP-driven rename, code actions, formatting via `arukellt lsp` + extension settings (`initializationOptions`)
- **Test (#186):** `setupTestController` — VS Code Test Controller with discovery (`arukellt test --list --json`) and run profile
- **Debug (#187):** `registerDebugAdapter`, `package.json` `debuggers` contribution, breakpoint-stop E2E in `extension.test.js`
- **Project (#188):** `ark.toml` target detection, script/task surfaces, project tree view
- **Docs intelligence (#205):** `arukellt.openDocs` opens repo docs URL; `arukellt.explainCode` placeholder toast
- **Pipeline (#206):** `arukellt.showPipeline` command with phase output channel
- **Security (#207):** `arukellt.securityReview` command

**DAP limitations sync:** `docs/debug-support.md` Limitations (simulated breakpoints, static variables, no watch/evaluate) matches extension DAP wiring (`debug-adapter` executable, no runtime hooks). Extension README now mirrors these limits and points runtime-level gaps to #638.

**Stale reopen evidence:** #191 and #479 are done with repo proof; 2026-06-12 reopen reason no longer applies.

**Action:** Moved to `issues/done/`. Runtime Wasm debugging remains under #638 (Lane 5 step D).
