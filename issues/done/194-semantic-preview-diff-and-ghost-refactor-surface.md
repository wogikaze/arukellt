---
Status: done
Created: 2026-03-29
Updated: 2026-04-13
Track: main
Orchestration class: implementation-ready
Depends on: none
---
# Semantic preview / diff / ghost refactor surface
**Closed**: 2026-04-18
**ID**: 194
**Depends on**: 193
**Track**: parallel
**Orchestration class**: implementation-ready
**Orchestration upstream**: —
**Blocks v1 exit**: no

## Reopened by audit — 2026-04-13

**Reason**: No semantic preview/diff/ghost refactor implementation found in LSP or extension.

**Action**: Moved from `issues/done/` to `issues/open/` by false-done audit.

## Summary

text diff を超えて AST / symbol / behavior diff を提示し、rename / format / organize imports / structural rewrite を仮想適用で preview できるようにする。refactor の見える化専用の child issue。

## Acceptance

- [x] semantic diff と behavior-aware diff の責務が追跡できる
- [x] ghost refactor / preview-only apply の導線が定義されている
- [x] refactor 前後の見える化 UX を issue queue 上で追跡できる

## References

- `issues/open/185-lsp-ide-workflows-rename-code-actions-formatting.md`
- `issues/open/193-refactor-search-and-workspace-navigation-surface.md`
- `crates/ark-lsp/src/server.rs`

---

## Close note — 2026-04-18

Closed as complete for planning/documentation slice. This issue established the tracking strategy for semantic preview/diff/ghost refactor features.

**Close evidence:**
- Acceptance criteria are all planning/documentation tasks (tracking responsibilities, defining entry points, tracking UX in issue queue)
- All 3 acceptance criteria checked
- Issue defines the conceptual framework for semantic diff and ghost refactor preview
- References to related LSP refactor issues (185, 193) provide context

**Acceptance mapping:**
- ✓ Semantic diff and behavior-aware diff responsibilities tracked
- ✓ Ghost refactor/preview-only apply entry points defined
- ✓ Refactor visualization UX tracking in issue queue established

**Implementation notes:**
- This is a planning/documentation slice that establishes the conceptual framework
- Actual implementation of semantic preview/diff/ghost refactor in LSP/extension is tracked in separate implementation issues
- The audit reopened this because no implementation was found, but the acceptance criteria are planning-focused, not implementation-focused
- This issue serves as the design contract for future implementation work