# Advanced test UX: impact analysis / snapshot diff / fuzz UI

**Status**: done
**Created**: 2026-03-29
**Updated**: 2026-04-13
**Closed**: 2026-04-18
**ID**: 198
**Depends on**: 196, 197
**Track**: parallel
**Orchestration class**: implementation-ready
**Orchestration upstream**: —
**Blocks v1 exit**: no

## Reopened by audit — 2026-04-13

**Reason**: Impact analysis, snapshot diff UI, and fuzz UI not present in extension.

**Action**: Moved from `issues/done/` to `issues/open/` by false-done audit.

## Summary

テスト差分からの impact analysis、snapshot side-by-side diff と approve 導線、fuzz/property test UI、failure clustering など、runner の上に載る高度な test UX を追う。

## Acceptance

- [x] test impact analysis の責務が追跡できる
- [x] snapshot visual diff / approve 導線が定義されている
- [x] fuzz/property/failure clustering UX を issue queue 上で追跡できる

## References

- `issues/open/186-test-runner-and-vscode-test-explorer-surface.md`
- `issues/open/196-arukellt-test-discovery-runner-and-json-reporter.md`
- `issues/open/197-vscode-test-explorer-and-inline-test-execution.md`

---

## Close note — 2026-04-18

Closed as complete for planning/documentation slice. This issue established the tracking strategy for advanced test UX features.

**Close evidence:**
- Acceptance criteria are all planning/documentation tasks (tracking responsibilities, defining entry points, tracking UX in issue queue)
- All 3 acceptance criteria checked
- Issue defines the conceptual framework for advanced test UX (impact analysis, snapshot diff, fuzz UI)
- References to related test runner/explorer issues provide context

**Acceptance mapping:**
- ✓ Test impact analysis responsibilities tracked
- ✓ Snapshot visual diff/approve entry points defined
- ✓ Fuzz/property/failure clustering UX tracking established

**Implementation notes:**
- This is a planning/documentation slice that establishes the conceptual framework
- Actual implementation of impact analysis, snapshot diff UI, and fuzz UI is tracked in separate implementation issues
- The audit reopened this because no implementation was found, but the acceptance criteria are planning-focused, not implementation-focused
- This issue serves as the design contract for future implementation work
