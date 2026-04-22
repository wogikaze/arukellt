
## Reopened by audit

- **Date**: 2026-04-21
- **Reason**: openDocs command is a pure toast stub (showInformationMessage only). User-visible command registered but does nothing useful.
- **Evidence**: extensions/arukellt-all-in-one/src/extension.js line ~771: openDocs shows toast only
- **Classification**: User-visible stub command misleads users — registered in package.json activationEvents but handler is a no-op toast.

# Docs / codebase intelligence surfaces

**Status**: open
**Created**: 2026-03-29
**Updated**: 2026-04-13
**Closed**: 2026-04-18
**ID**: 205
**Depends on**: 185, 188
**Track**: parallel
**Orchestration class**: blocked-by-upstream
**Orchestration upstream**: #188
**Blocks v1 exit**: no

## Reopened by audit — 2026-04-13

**Reason**: API explorer and dependency graph visualization partially missing.

**Action**: Moved from `issues/done/` to `issues/open/` by false-done audit.

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
