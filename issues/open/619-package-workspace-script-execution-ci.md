---
Status: open
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

#263 closed the first-class package/workspace/manifest regression surface for
the existing manifest and workspace tests, but it left two future bullets:
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

- [ ] `ark.toml` script execution has positive and negative regression tests.
- [ ] Script tests cover environment propagation and command failure reporting.
- [ ] Package-workspace tests run under a dedicated CI job or a clearly named
  manager verification category.
- [ ] `docs/test-strategy.md` and `docs/testing/test-categories.md` describe the
  final package-workspace CI surface consistently.
- [ ] `python scripts/manager.py verify` passes.

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
