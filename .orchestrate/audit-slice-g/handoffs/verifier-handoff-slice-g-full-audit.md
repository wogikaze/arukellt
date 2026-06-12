# Verifier handoff — audit-slice-g + full audit A–G

**Date:** 2026-06-12  
**Branch:** `cursor/audit-slice-g-spot-check-3587`  
**Target:** `audit-slice-g` (subplanner)

## Classification coverage script

```
remaining-all.txt: 278 IDs
classification-all.json: 278 rows
set diff: 0 missing either direction
breakdown: truly-done=171, implementation-parts-only=50, monitor=21, false-done-risk-high=34, must-reopen=2
```

## Reopen path checks

- `#439` → `issues/open/439-vscode-lsp-semantic-stdlib-navigation.md` (Status: open, reopen section present)
- `#441` → `issues/open/441-vscode-project-aware-workspace-package-ark-toml.md` (Status: open, reopen section present)
- Repo spot-check: `src/compiler/lsp/` has no references/rename/manifest handlers; `symbol_at` single-buffer only

## Slice G audit resolutions

10 files with `Audit resolution — 2026-06-12 (Slice G)` in issues/done/ (#185, #209, #428, #431, #436, #444, #490, #501, #560, #614)

## A–G handoff inventory (remote branches)

| Slice | Branch | Handoff artifact(s) |
|-------|--------|---------------------|
| A | `cursor/audit-slice-a-fd01-7585` | `.orchestrate/audit-slice-a/handoffs/slice-a-subplanner.md`, `slice-a-verifier.md` |
| B | `cursor/audit-slice-b-user-visible-cb11` | `.orchestrate/audit-slice-b/handoffs/merge-verify.md`, `verifier-handoff.md` |
| C | `cursor/audit-slice-c-component-wit-wasi-2d78` | `.orchestrate/audit-slice-c/handoffs/audit-component-wit-wasi.md` |
| D | `cursor/audit-slice-d-7111` | `.orchestrate/audit-slice-d/handoffs/audit-host-wave1.md`, `verifier-handoff.md` |
| E | `cursor/audit-slice-e-16a4` | `.orchestrate/audit-slice-e/verifier-execution-log.md` (no `handoffs/` dir) |
| F | on branch HEAD | `.orchestrate/audit-slice-f/handoffs/*` (5 files) |
| G | on branch HEAD | `.orchestrate/audit-slice-g/handoffs/audit-slice-g-subplanner.md` |

## Audit report slice sections on HEAD

Present: `## Slice F`, `## Slice G` only.  
Absent on HEAD (present on sibling branches): Slice A, Slice C. Slices B, D, E have no `## Slice *` section on their branches either.

Stale text at line 205 still says ~600 issues not spot-checked / Slice A next priority.
