---
Status: done
Created: 2026-05-14
Updated: 2026-05-14
ID: 619
Track: tooling-contract
Depends on: 263
Orchestration class: design-ready
Orchestration upstream: None
---

# Package workspace script execution tests and CI lane

## Summary

# 263 closed the first-class package/workspace/manifest regression surface for the existing manifest

`ark.toml` script execution tests and a dedicated package-workspace CI lane.
This issue tracks those as a separate implementation slice.

## Why this matters

- Manifest and workspace resolution can stay green while `ark.toml` script
  execution regresses.
- `docs/test-strategy.md` currently describes package-workspace as partially
  covered by broader jobs, so failures are less visible than the category model
  intends.
- CI naming should make package/workspace regressions attributable without
  reading the full harness log.

## Acceptance

- [x] `ark.toml` script execution has positive and negative regression tests.
- [x] Script tests cover environment propagation and command failure reporting.
- [x] Package-workspace tests run under a dedicated CI job or a clearly named
  manager verification category.
- [x] `docs/test-strategy.md` and `docs/testing/test-categories.md` describe the
  final package-workspace CI surface consistently.
- [x] `python scripts/manager.py verify` passes.

## Primary paths

- `tests/package-workspace/`
- `crates/arukellt/src/`
- `.github/workflows/ci.yml`
- `scripts/manager.py`
- `docs/test-strategy.md`
- `docs/testing/test-categories.md`

## Close gate

All acceptance items checked with repo-internal evidence; #263 does not regain
unchecked future bullets.

## Completion evidence — 2026-05-14

- `crates/arukellt/src/main.rs` now handles `arukellt script list`,
  `arukellt script list --json`, and `arukellt script run <name> [args...]`
  from the nearest `ark.toml` before forwarding other commands to the selfhost
  compiler.
- `scripts/run/test-package-workspace.sh` covers script listing, JSON listing,
  environment propagation, argument passthrough, command failure reporting, and
  missing-manifest diagnostics.
- `.github/workflows/ci.yml` adds the merge-blocking
  `verification-package-workspace` job.
- Docs updated: `docs/test-strategy.md` and
  `docs/testing/test-categories.md`.
- Verification:
  - `bash scripts/run/test-package-workspace.sh` → PASS (9/9)
  - `cargo test --workspace --lib --bins -- --nocapture` → PASS
  - `python3 scripts/check/check-docs-consistency.py` → PASS
  - `python scripts/manager.py verify` → PASS (22/22)
