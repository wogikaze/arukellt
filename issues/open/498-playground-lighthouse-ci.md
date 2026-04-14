# 498 — Playground CI: Lighthouse performance audit

**Track:** playground
**Status:** open
**Created:** 2026-04-14
**Updated:** 2026-04-14
**Source:** explicit-defer — docs/playground/deployment-strategy.md §4.3 (issue #491)

## Summary

Issue #491 added the wasm binary and JS bundle size gates to CI
(`.github/workflows/playground-ci.yml`). Lighthouse CI — a headless browser
performance audit that checks Core Web Vitals (LCP, FID, CLS, TTI) and
accessibility scores against configured budgets — was explicitly deferred.

This issue tracks adding Lighthouse CI to the playground CI workflow once
the playground has a stable browser entrypoint that can be meaningfully audited.

## Background

The deployment-strategy.md §4.3 originally listed Lighthouse CI as a stretch
goal. It is not yet integrated because:

1. The playground's headless test infrastructure (Playwright) is target-state
   only — no smoke test step exists in the current workflow.
2. A meaningful Lighthouse audit requires a fully assembled static site;
   the current CI only builds the JS bundle and wasm binary.
3. Budget thresholds (e.g., LCP ≤ 2.5 s, accessibility score ≥ 90) need
   to be agreed before automation is useful.

## Acceptance

- [ ] A Lighthouse CI step is added to `.github/workflows/playground-ci.yml`
      (or a new dedicated workflow)
- [ ] Lighthouse budgets are documented in `docs/playground/deployment-strategy.md §4.3`
- [ ] Step fails when Lighthouse performance score falls below configured threshold
- [ ] `docs/playground/deployment-strategy.md` updated to mark Lighthouse CI as implemented

## Primary paths

- `.github/workflows/playground-ci.yml`
- `docs/playground/deployment-strategy.md`

## Non-goals

- Changing playground build toolchain
- Full E2E Playwright test suite (separate concern)

## Required verification

- `bash scripts/run/verify-harness.sh --quick` passes
- Lighthouse CI step runs on a built playground artifact and produces a report

## Close gate

All acceptance items checked; deployment-strategy.md marks Lighthouse CI as enforced.
