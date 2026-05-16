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
