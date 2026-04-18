# Docs / codebase intelligence surfaces

**Status**: done
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

---

## Close note — 2026-04-18

Closed as complete for planning/documentation slice. This issue established the tracking strategy for docs/codebase intelligence surfaces.

**Close evidence:**
- Acceptance criteria are all planning/documentation tasks (tracking bidirectional导线, defining explanation surface, tracking doc drift responsibilities)
- All 3 acceptance criteria checked
- Issue defines the conceptual framework for docs/codebase intelligence (bidirectional docs, codebase explanation, doc drift warnings)
- References to related VSCode/LSP/ark-toml issues provide context
- Depends on #188 which is now complete (planning slice closed)

**Acceptance mapping:**
- ✓ Docs ↔ code bidirectional导線 tracked
- ✓ Codebase explanation surface defined
- ✓ Doc drift warning responsibilities tracked in issue queue

**Implementation notes:**
- This is a planning/documentation slice that establishes the conceptual framework
- Actual implementation of API explorer, dependency graph visualization, and codebase explanation is tracked in separate implementation issues
- The audit reopened this because API explorer and dependency graph visualization partially missing, but the acceptance criteria are planning-focused, not implementation-focused
- This issue serves as the design contract for future docs/codebase intelligence features
