# Interactive compiler pipeline + inline profiling

**Status**: open
**Created**: 2026-03-29
**Updated**: 2026-04-13
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
