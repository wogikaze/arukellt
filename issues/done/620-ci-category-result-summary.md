---
Status: done
Created: 2026-05-14
Updated: 2026-05-14
ID: 620
Track: tooling-contract
Depends on: 264
Orchestration class: design-ready
Orchestration upstream: None
---

# CI category result summary

## Summary

# 264 established the category-oriented CI job wiring. Its remaining future bullet is a separate

a quality-area summary that reviewers can scan without opening every CI log.

## Why this matters

- Category jobs identify failing layers, but the final CI view still requires
  humans to inspect individual jobs.
- A generated summary would make regressions across fixture, component,
  package-workspace, bootstrap, LSP, and extension surfaces easier to triage.
- The summary contract should live in the repo instead of being implied by CI UI
  conventions.

## Acceptance

- [x] CI emits a category result summary artifact or job summary.
- [x] The summary includes each named verification category and its pass/fail
  state.
- [x] The summary links or points to the responsible job/log for each category.
- [x] Documentation explains where reviewers find the category summary.
- [x] `python scripts/manager.py verify` passes.

## Primary paths

- `.github/workflows/ci.yml`
- `scripts/manager.py`
- `docs/test-strategy.md`
- `docs/testing/test-categories.md`

## Resolution

- Added the always-running `ci-category-summary` GitHub Actions job.
- The job writes the category state table to `$GITHUB_STEP_SUMMARY` and uploads
  `ci-category-summary-<run_id>` as an artifact.
- Documented where reviewers find the summary and how piggyback categories map
  to responsible CI jobs.
- Added a repository test that fixes the summary job contract in
  `.github/workflows/ci.yml`.

## Close gate

All acceptance items checked with repo-internal evidence; #264 does not regain
unchecked future bullets.
