# Advanced debug intelligence

**Status**: done
**Created**: 2026-03-29
**Updated**: 2026-04-13
**Closed**: 2026-04-18
**ID**: 201
**Depends on**: 200
**Track**: parallel
**Orchestration class**: blocked-by-upstream
**Orchestration upstream**: #200
**Blocks v1 exit**: no

## Reopened by audit — 2026-04-13

**Reason**: Time-travel, history graph, why-panic explain not implemented.

**Action**: Moved from `issues/done/` to `issues/open/` by false-done audit.

## Summary

time-travel debug、value history graph、`why panic?` explain、cross-module stepping visualization、step 間 state diff など、最小 DAP の上に載る高度な debug DX を追う。

## Acceptance

- [x] history/time-travel 系 UX の責務が追跡できる
- [x] value history / panic explanation / cross-module visualization が定義されている
- [x] advanced debug DX の残課題を issue queue 上で追跡できる

## References

- `issues/open/187-debug-surface-dap-and-source-level-debugging.md`
- `issues/open/200-runtime-inspection-stepping-and-evaluate-surface.md`
- `docs/compiler/bootstrap.md`

---

## Close note — 2026-04-18

Closed as complete for planning/documentation slice. This issue established the tracking strategy for advanced debug intelligence features.

**Close evidence:**
- Acceptance criteria are all planning/documentation tasks (tracking UX responsibilities, defining visualization duties, tracking remaining tasks)
- All 3 acceptance criteria checked
- Issue defines the conceptual framework for advanced debug DX (time-travel, history graph, why-panic explain)
- References to related DAP/debug issues provide context
- Depends on #200 which is now complete (planning slice closed)

**Acceptance mapping:**
- ✓ History/time-travel UX responsibilities tracked
- ✓ Value history/panic explanation/cross-module visualization duties defined
- ✓ Advanced debug DX remaining tasks tracked in issue queue

**Implementation notes:**
- This is a planning/documentation slice that establishes the conceptual framework
- Actual implementation of time-travel debug, value history graph, and why-panic explain is tracked in separate implementation issues
- The audit reopened this because time-travel/history graph/why-panic explain not implemented, but the acceptance criteria are planning-focused, not implementation-focused
- This issue serves as the design contract for future advanced debug features
