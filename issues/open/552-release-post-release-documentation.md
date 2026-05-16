# Release: Post-Release Documentation

> **Status:** open
> **Track:** release
> **Type:** Documentation

## Scope

Ensure post-release documentation is updated for release verification.

## Checklist Source

docs/release-checklist.md — Post-release section

## Acceptance

- [ ] Release notes written and attached to GitHub release
- [x] `docs/current-state.md` reflects shipped capabilities

## Progress — 2026-05-14

- Added repository-side draft release notes at `docs/release-notes-v0.1.0-draft.md`.
- Updated `docs/data/project-state.toml` and regenerated marker-backed docs so
  `docs/current-state.md`, `README.md`, and `docs/README.md` report the current
  verification entry point and 22/22 quick gate.
- Updated `docs/current-state.md` benchmark snapshot from
  `tests/baselines/perf/baselines.json` generated on 2026-05-14, including the
  new `file_io` benchmark and the current per-workload pass/fail/skip statuses.

Not closed yet: the release notes are written in-repo, but they have not been
attached to a GitHub release because no release tag / GitHub release exists in
this checkout.

## Recheck — 2026-05-16

### Results

- **Criterion 1** ``[ ]`` Release notes written and attached to GitHub release
  - `docs/release-notes-v0.1.0-draft.md` exists and contains the draft release
    notes. Written part is satisfied.
  - No release tag exists in the repository. `gh release list` confirms zero
    GitHub releases on the remote. Attachment part **cannot be satisfied** until
    an actual release is cut.
- **Criterion 2** ``[x]`` `docs/current-state.md` reflects shipped capabilities
  - Already verified and marked complete on 2026-05-14. No new commits or
    changes since then that would invalidate this.

### Verdict

**Do not close.** Criterion 1 is partially met (notes exist as a draft) but the
"attached to GitHub release" sub-criterion requires an actual tagged GitHub
release, which does not yet exist in this repository. The issue should remain
open until the release is published and the draft notes are attached to it.

## Required Verification

- Release notes document changes in this release
- docs/current-state.md accurately reflects current capabilities

## Close Gate

All post-release documentation must be updated and accurate.

## Primary Paths

- `docs/current-state.md` (current state documentation)
- GitHub release notes
- Extension documentation

## Non-Goals

- Marketing materials
- External blog posts
