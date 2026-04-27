---
Status: done
Created: 2026-03-29
Updated: 2026-03-29
ID: 183
Track: parallel
Depends on: 184, 185, 186, 187, 188, 205, 206, 207
Orchestration class: implementation-ready
Blocks v1 exit: False
Status note: Parent issue for the all-in-one VS Code experience. Core editor, test, debug, project metadata, and cross-cutting DX surfaces are tracked separately.
# VS Code: `arukellt-all-in-one` 拡張機能 epic
---
# VS Code: `arukellt-all-in-one` 拡張機能 epic


## Summary

`arukellt-all-in-one` は Arukellt 向け VS Code 体験を 1 つの拡張に集約する epic である。
基礎 editor support に加え、test / debug / project scripts / docs / pipeline / security まで含む DX 面を child issue に分けて追跡する。

## Acceptance

- [x] #184, #185, #186, #187, #188, #205, #206, #207 が完了している
- [x] VS Code 体験の責務が foundation / authoring / test / debug / project / cross-cutting DX に分離されている
- [x] all-in-one 拡張の残課題が issue queue 上で追跡できる

## References

- `issues/open/184-vscode-extension-foundation.md`
- `issues/open/185-lsp-ide-workflows-rename-code-actions-formatting.md`
- `issues/open/186-test-runner-and-vscode-test-explorer-surface.md`
- `issues/open/187-debug-surface-dap-and-source-level-debugging.md`
- `issues/open/188-ark-toml-project-workspace-and-scripts.md`
- `crates/ark-lsp/src/lib.rs`
- `crates/arukellt/src/main.rs`