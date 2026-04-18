# Issues Index

This directory tracks work queue and completion status.

## Organization

- **`open/`** - Active work items waiting to be completed
- **`done/`** - Completed work for reference and history

## Issue Format

Each issue is a markdown file with machine-readable header fields plus human-readable sections:

```markdown
# [TITLE]

**Status**: open | done
**Created**: YYYY-MM-DD
**Updated**: YYYY-MM-DD
**ID**: 001
**Depends on**: none | 001, 002
**Orchestration class**: implementation-ready | design-ready | verification-ready | blocked-by-upstream | unsupported-in-this-run
**Orchestration upstream**: — | free-form (e.g. `#039`, `external:037`)

## Summary
Brief description of what needs to be done.

## Acceptance Criteria
- [ ] Criterion 1
- [ ] Criterion 2
- [ ] Criterion 3

## Notes
Any context, blockers, or implementation notes.
```

The generator reads these exact header fields. `Depends on` must contain comma-separated numeric IDs or `none`.

Generated artifacts:
- `issues/open/index.md` — dependency-aware issue index (includes orchestration columns when set in issue headers)
- `issues/open/index-meta.json` — machine-readable export (deps, acceptance counts, orchestration)
- `issues/open/dependency-graph.md` — Mermaid graph + adjacency lists
- `scripts/gen/generate-issue-index.sh` — shell entrypoint for regeneration
- `scripts/gen/sync-issue-orchestration-fields.py` — optional bulk import of orchestration fields from a TSV into issue headers

## Workflow

1. Add new issues to `open/`
2. Reference in AGENTS.md as active focus
3. When verification passes and work is complete, move to `done/`
4. Maintain timestamp consistency for traceability

## Queue Operations

- **Start work**: Note in issue, update status
- **Blocked**: Document reason in Notes section
- **Complete**: Move file to `done/` with completion timestamp
