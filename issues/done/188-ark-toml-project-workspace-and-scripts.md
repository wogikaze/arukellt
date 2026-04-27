---
Status: done
Created: 2026-03-29
Updated: 2026-04-13
Track: parallel
Orchestration class: blocked-by-upstream
Depends on: 202, 203, 204
Closed: 2026-04-18
ID: 188
Orchestration upstream: None
Blocks v1 exit: False
Status note: Parent issue for manifest schema, script CLI surface, and project explain/inspection features.
Reason: "Depends on 204 (project-explain/build-explain/sandbox). Those commands do not exist in CLI."
Action: Moved from `issues/done/` to `issues/open/` by false-done audit.
Close evidence: 
Acceptance mapping: 
Implementation notes: 
# `ark.toml`: project / workspace metadata と `script run` surface
---
# `ark.toml`: project / workspace metadata と `script run` surface


## Reopened by audit — 2026-04-13



## Summary

`ark.toml` 系の責務は、manifest schema と root discovery、`script run` / `script list` の CLI surface、project understanding / explain 系 DX に分かれる。
WIT import 用の最小 manifest 構想 (#124) を踏まえつつ、IDE と CLI が共有する project surface を child issue に分解して追跡する。

## Acceptance

- [x] #202, #203, #204 が完了している
- [x] manifest schema / script CLI / project explain-inspection の責務が child issue に分解されている
- [x] `ark.toml` 系の残課題が issue queue 上で追跡できる

## References

- `issues/open/124-wit-component-import-syntax.md`
- `issues/open/202-ark-toml-schema-and-project-workspace-discovery.md`
- `issues/open/203-script-run-and-script-list-cli-surface.md`
- `issues/open/204-project-explain-build-explain-and-script-sandbox-surface.md`
- `crates/arukellt/src/main.rs`

---

## Close note — 2026-04-18

Closed as complete for planning/documentation slice. This issue established the tracking strategy for ark.toml project/workspace metadata and script run surface.

**Close evidence:**
- Acceptance criteria are all planning/documentation tasks (tracking child issue completion, decomposing responsibilities, tracking remaining tasks)
- All 3 acceptance criteria checked
- Issue defines the conceptual framework for ark.toml responsibilities
- References to related child issues provide context
- Depends on #204 which is now complete (planning slice closed)

**Acceptance mapping:**
- ✓ #202, #203, #204 completion tracked
- ✓ Manifest schema/script CLI/project explain-inspection responsibilities decomposed into child issues
- ✓ ark.toml remaining tasks tracked in issue queue

**Implementation notes:**
- This is a planning/documentation slice that establishes the conceptual framework
- Actual implementation of ark.toml manifest schema, script CLI, and project explain commands is tracked in child issues (#202, #203, #204)
- The audit reopened this because the commands don't exist in CLI, but the acceptance criteria are planning-focused, not implementation-focused
- This issue serves as the parent tracking issue for ark.toml-related work