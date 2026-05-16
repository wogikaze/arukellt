---
Status: done
Created: 2026-05-14
Updated: 2026-05-14
ID: 621
Track: tooling-contract
Depends on: 265
Orchestration class: design-ready
Orchestration upstream: None
Resolution: Failure metadata emitted by verification harness and local gate; docs updated.
---

# Verification failure category reporting

## Summary

#265 defined the categories and naming structure needed to identify failure
layers, but its future reporting bullets were not their own open work item.
This issue tracks runner/report-level category annotations when verification
fails.

## Why this matters

- A failing command should state the affected category without requiring the
  reviewer to infer it from file paths.
- Category annotations make local verification failures and CI logs consistent.
- This complements #620: #620 aggregates CI job status, while this issue
  improves the failure messages emitted by runners and reports.

## Acceptance

- [x] Verification runner output includes the category for failing checks.
- [x] Failure reports include category, responsible command, and primary path.
- [x] Existing package-workspace, fixture, component, bootstrap, LSP, and
  extension checks have category labels.
- [x] Documentation describes the category labels used in failure reports.
- [x] `python scripts/manager.py verify` passes.

## Primary paths

- `scripts/manager.py`
- `scripts/run/`
- `docs/test-strategy.md`
- `docs/testing/test-categories.md`

## Close gate

All acceptance items checked with repo-internal evidence; #265 does not regain
unchecked future bullets.
