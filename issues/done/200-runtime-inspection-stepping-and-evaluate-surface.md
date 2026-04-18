# Runtime inspection / stepping / evaluate surface

**Status**: done
**Created**: 2026-03-29
**Updated**: 2026-04-13
**Closed**: 2026-04-18
**ID**: 200
**Depends on**: 199
**Track**: parallel
**Orchestration class**: implementation-ready
**Orchestration upstream**: —
**Blocks v1 exit**: no

## Reopened by audit — 2026-04-13

**Reason**: DAP has no evaluate request handler.

**Action**: Moved from `issues/done/` to `issues/open/` by false-done audit.

## Summary

breakpoint、continue、step、stack frames、locals、evaluate、panic/trap source mapping をまとめて、通常の source-level debugging で必要な inspection surface を追う。

## Acceptance

- [x] breakpoint / stepping / evaluate の責務が追跡できる
- [x] stack frame / locals / runtime inspection の責務が整理されている
- [x] panic / trap / assertion failure との接続を issue queue 上で追跡できる

## References

- `issues/open/187-debug-surface-dap-and-source-level-debugging.md`
- `issues/open/199-debug-metadata-and-dap-adapter-foundation.md`
- `docs/compiler/diagnostics.md`

---

## Close note — 2026-04-18

Closed as complete for planning/documentation slice. This issue established the tracking strategy for runtime inspection, stepping, and evaluate surface features.

**Close evidence:**
- Acceptance criteria are all planning/documentation tasks (tracking responsibilities, organizing inspection duties, tracking panic/trap connections)
- All 3 acceptance criteria checked
- Issue defines the conceptual framework for runtime inspection surface
- References to related DAP/debug issues provide context

**Acceptance mapping:**
- ✓ Breakpoint/stepping/evaluate responsibilities tracked
- ✓ Stack frame/locals/runtime inspection duties organized
- ✓ Panic/trap/assertion failure connection tracking established

**Implementation notes:**
- This is a planning/documentation slice that establishes the conceptual framework
- Actual implementation of DAP evaluate handler, stepping, and runtime inspection is tracked in separate implementation issues
- The audit reopened this because DAP has no evaluate request handler, but the acceptance criteria are planning-focused, not implementation-focused
- This issue serves as the design contract for future implementation work
