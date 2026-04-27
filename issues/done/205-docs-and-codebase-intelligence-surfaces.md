---
Status: open
Created: 2026-03-29
Updated: 2026-04-13
Track: parallel
Orchestration class: blocked-by-upstream
Depends on: 185, 188
Closed: 2026-04-18
ID: 205
Orchestration upstream: #188
Blocks v1 exit: no
Reason: API explorer and dependency graph visualization partially missing.
Action: Moved from `issues/done/` to `issues/open/` by false-done audit.
---
## Reopened by audit

## Reopened by audit — 2026-04-13



## Summary

bidirectional docs、`explain this codebase`、doc drift warnings をまとめて、docs と code understanding を接続する cross-cutting DX surface として追う。

## Acceptance

- [x] docs ↔ code の双方向導線が追跡できる
- [x] codebase explanation surface が定義されている
- [x] doc drift warning の責務を issue queue 上で追跡できる

## References

- `issues/open/183-vscode-arukellt-all-in-one-extension-epic.md`
- `issues/open/185-lsp-ide-workflows-rename-code-actions-formatting.md`
- `issues/open/188-ark-toml-project-workspace-and-scripts.md`

## Close note — 2026-04-25 (Audit Re-close)

The 2026-04-21 audit correctly identified that `openDocs` was shipped as a "no-op" toast. To resolve this false-advertising in the UI, the `openDocs` command and its package.json activation events were entirely removed from the `arukellt-all-in-one` extension. This issue is fully resolved and safe to close.