---
Status: open
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

#264 established the category-oriented CI job wiring. Its remaining future
bullet is a separate reporting concern: aggregate category job results and show
a quality-area summary that reviewers can scan without opening every CI log.

## Why this matters

- Category jobs identify failing layers, but the final CI view still requires
  humans to inspect individual jobs.
- A generated summary would make regressions across fixture, component,
  package-workspace, bootstrap, LSP, and extension surfaces easier to triage.
- The summary contract should live in the repo instead of being implied by CI UI
  conventions.

## Acceptance

- [ ] CI emits a category result summary artifact or job summary.
- [ ] The summary includes each named verification category and its pass/fail
  state.
- [ ] The summary links or points to the responsible job/log for each category.
- [ ] Documentation explains where reviewers find the category summary.
- [ ] `python scripts/manager.py verify` passes.

## Primary paths

- `.github/workflows/ci.yml`
- `scripts/manager.py`
- `docs/test-strategy.md`
- `docs/testing/test-categories.md`

## Close gate

All acceptance items checked with repo-internal evidence; #264 does not regain
unchecked future bullets.
