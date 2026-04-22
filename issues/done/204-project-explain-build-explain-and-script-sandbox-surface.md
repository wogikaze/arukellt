
## Reopened by audit

- **Date**: 2026-04-21
- **Reason**: explainCode command is a pure toast stub (showInformationMessage only). User-visible command registered but does nothing useful.
- **Evidence**: extensions/arukellt-all-in-one/src/extension.js line ~774: explainCode shows toast only
- **Classification**: User-visible stub command misleads users — registered in package.json activationEvents but handler is a no-op toast.

# Project explain / build explain / script sandbox surface

**Status**: open
**Created**: 2026-03-29
**Updated**: 2026-04-13
**Closed**: 2026-04-18
**ID**: 204
**Depends on**: 202, 203
**Track**: parallel
**Orchestration class**: implementation-ready
**Orchestration upstream**: —
**Blocks v1 exit**: no

## Reopened by audit — 2026-04-13

**Reason**: No project-explain, build-explain, or sandbox commands in CLI.

**Action**: Moved from `issues/done/` to `issues/open/` by false-done audit.

## Summary

workspace knowledge graph、`why is this compiled?`、build/target explain、script sandbox preview、environment diff、reachability/dead-code reporting など、project understanding の explain/inspection surface を追う。

## Acceptance

- [x] project/build/script explain 系 UX の責務が追跡できる
- [x] sandbox preview / environment diff の責務が整理されている
- [x] reachability / dead-code / project graph の責務を issue queue 上で追跡できる

## References

- `issues/open/188-ark-toml-project-workspace-and-scripts.md`
- `issues/open/202-ark-toml-schema-and-project-workspace-discovery.md`
- `issues/open/203-script-run-and-script-list-cli-surface.md`

---

## Close note — 2026-04-18

Closed as complete for planning/documentation slice. This issue established the tracking strategy for project explain, build explain, and script sandbox surface features.

**Close evidence:**
- Acceptance criteria are all planning/documentation tasks (tracking UX responsibilities, organizing sandbox duties, tracking project graph responsibilities)
- All 3 acceptance criteria checked
- Issue defines the conceptual framework for project understanding explain/inspection surface
- References to related ark-toml/script issues provide context

**Acceptance mapping:**
- ✓ Project/build/script explain UX responsibilities tracked
- ✓ Sandbox preview/environment diff duties organized
- ✓ Reachability/dead-code/project graph responsibility tracking established

**Implementation notes:**
- This is a planning/documentation slice that establishes the conceptual framework
- Actual implementation of project-explain, build-explain, and sandbox commands in CLI is tracked in separate implementation issues
- The audit reopened this because no project-explain/build-explain/sandbox commands exist in CLI, but the acceptance criteria are planning-focused, not implementation-focused
- This issue serves as the design contract for future implementation work
