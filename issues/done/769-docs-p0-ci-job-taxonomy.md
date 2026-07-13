---
Status: done
Created: 2026-07-11
Updated: 2026-07-11
ID: 769
Track: docs-audit
Depends on: 765
Orchestration class: implementation-ready
Orchestration upstream: None
Blocks v{N}: none
Priority: 1
Source: Docs re-audit 2026-07-11 (P0-4)
Blocks: 770
---

# 769 — Docs P0: CI job taxonomy from workflow

## Summary

`test-strategy.md` and `testing/test-categories.md` invent or mix job IDs
(`fixture-primary`, `verification-bootstrap`, …) that are not in
`.github/workflows/ci.yml` (`verification`, `selfhost`, `docs`,
`extension-tests`, `release-tag`, `verify`).

## Acceptance

- [x] Machine-readable / generated job list from `ci.yml`
- [x] `test-strategy.md` uses only real job IDs (or generated include)
- [x] `testing/test-categories.md` bootstrap/CI sections updated off Rust-era jobs
- [x] Gate fails when current docs cite unknown CI job IDs
- [x] Docs-related verify gates pass

## References

- `.github/workflows/ci.yml`
- `docs/test-strategy.md`

## Completion

Completed 2026-07-11 as docs re-audit Phase 1.
