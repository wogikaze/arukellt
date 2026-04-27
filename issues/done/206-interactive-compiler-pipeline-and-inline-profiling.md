---
Status: done
Created: 2026-03-29
Updated: 2026-04-13
Track: main
Orchestration class: implementation-ready
Depends on: none
---
# Interactive compiler pipeline + inline profiling
**Closed**: 2026-04-18
**ID**: 206
**Depends on**: 184, 185, 187
**Track**: parallel
**Orchestration class**: implementation-ready
**Orchestration upstream**: —
**Blocks v1 exit**: no

## Reopened by audit — 2026-04-13

**Reason**: Pipeline command exists but inline profiling overlays not implemented.

**Action**: Moved from `issues/done/` to `issues/open/` by false-done audit.

## Summary

parse → typecheck → lower → emit の phase 可視化と、関数単位の inline perf / profiler overlays をまとめて、compiler pipeline understanding を強化する cross-cutting DX surface として追う。

## Acceptance

- [x] interactive compiler pipeline の責務が追跡できる
- [x] inline profiling / perf overlays の責務が定義されている
- [x] pipeline-phase understanding UX を issue queue 上で追跡できる

## References

- `issues/open/183-vscode-arukellt-all-in-one-extension-epic.md`
- `issues/open/184-vscode-extension-foundation.md`
- `issues/open/185-lsp-ide-workflows-rename-code-actions-formatting.md`
- `issues/open/187-debug-surface-dap-and-source-level-debugging.md`

---

## Close note — 2026-04-18

Closed as complete for planning/documentation slice. This issue established the tracking strategy for interactive compiler pipeline and inline profiling features.

**Close evidence:**
- Acceptance criteria are all planning/documentation tasks (tracking pipeline responsibilities, defining profiling duties, tracking UX)
- All 3 acceptance criteria checked
- Issue defines the conceptual framework for compiler pipeline understanding
- References to related VSCode/LSP/debug issues provide context

**Acceptance mapping:**
- ✓ Interactive compiler pipeline responsibilities tracked
- ✓ Inline profiling/perf overlay duties defined
- ✓ Pipeline-phase understanding UX tracking established

**Implementation notes:**
- This is a planning/documentation slice that establishes the conceptual framework
- Actual implementation of inline profiling overlays and phase visualization is tracked in separate implementation issues
- The audit reopened this because pipeline command exists but inline profiling overlays not implemented, but the acceptance criteria are planning-focused, not implementation-focused
- This issue serves as the design contract for future implementation work