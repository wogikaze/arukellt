---
Status: done
Created: 2026-04-14
Updated: 2026-04-15
Track: playground
Orchestration class: implementation-ready
Depends on: none
Closed: 2026-04-15
Source: "explicit-defer — docs/playground/deployment-strategy.md §4.3 (issue #491)"
JS bundle, serves `docs/playground/` at `localhost: 3000` via `npx serve@14`,
---

# 498 — Playground CI: Lighthouse performance audit
- `.github/workflows/playground-ci.yml` job `playground-lighthouse`: builds
- `.github/lighthouserc.json`: "Lighthouse CI config with assertions:"
- `categories: "performance` ≥ 0.7 / 70% (warn)"
- `docs/playground/deployment-strategy.md §4.3`: status note updated to
# 498 — Playground CI: Lighthouse performance audit
**Closed:** 2026-04-15
**Source:** explicit-defer — docs/playground/deployment-strategy.md §4.3 (issue #491)

## Summary

Issue #491 added the wasm binary and JS bundle size gates to CI
(`.github/workflows/playground-ci.yml`). Lighthouse CI — a headless browser
performance audit that checks Core Web Vitals (LCP, CLS) and accessibility
scores against configured budgets — was explicitly deferred.

This issue tracked adding Lighthouse CI to the playground CI workflow once
the playground has a stable browser entrypoint that can be meaningfully
audited.

## Background

The deployment-strategy.md §4.3 originally listed Lighthouse CI as a stretch
goal. It was not integrated because:

1. The playground's headless test infrastructure (Playwright) is target-state
   only — no smoke test step existed in the current workflow.
2. A meaningful Lighthouse audit requires a fully assembled static site;
   the previous CI only built the JS bundle and wasm binary.
3. Budget thresholds (LCP ≤ 2.5 s, accessibility score ≥ 90) needed to be
   agreed before automation is useful.

## Acceptance

- [x] A Lighthouse CI step is added to `.github/workflows/playground-ci.yml`
      (job `playground-lighthouse`)
- [x] Lighthouse budgets are documented in `docs/playground/deployment-strategy.md §4.3`
- [x] Step fails when Lighthouse performance score falls below configured threshold
      (LCP ≤ 2.5 s → error; CLS ≤ 0.1 → error; accessibility ≥ 90 → error;
       performance score ≥ 70 → warn)
- [x] `docs/playground/deployment-strategy.md` updated to mark Lighthouse CI as implemented

## Close evidence

- `.github/workflows/playground-ci.yml` job `playground-lighthouse`: builds
  JS bundle, serves `docs/playground/` at `localhost:3000` via `npx serve@14`,
  waits for readiness with `wait-on@7`, then runs `treosh/lighthouse-ci-action@v11`
- `.github/lighthouserc.json`: Lighthouse CI config with assertions:
  - `largest-contentful-paint` ≤ 2500 ms (error)
  - `cumulative-layout-shift` ≤ 0.1 (error)
  - `categories:accessibility` ≥ 0.9 / 90% (error)
  - `categories:performance` ≥ 0.7 / 70% (warn)
- `docs/playground/deployment-strategy.md §4.3`: status note updated to
  "enforced in CI"; Lighthouse budgets table added; `playground-lighthouse`
  job reference added

## Primary paths changed

- `.github/workflows/playground-ci.yml`
- `.github/lighthouserc.json` (created)
- `docs/playground/deployment-strategy.md`

## Required verification

- `bash scripts/run/verify-harness.sh --quick` — passed (exit 0)

## Non-goals

- Changing playground build toolchain
- Full E2E Playwright test suite (separate concern)