---
Status: done
Created: 2026-04-14
Updated: 2026-04-14
Track: playground
Source: audit — docs/playground/deployment-strategy.md §4.3
Orchestration class: implementation-ready
Depends on: none
---

- `.github/workflows/playground-ci.yml`: "`playground-bundle-size` job (threshold: `PLAYGROUND_BUNDLE_LIMIT=524288` = 512 KB)"
- `scripts/check/check-playground-size.sh`: reusable size check script; exits 1 when threshold exceeded
- `docs/playground/deployment-strategy.md §4.3`: updated with enforcement status table, defers Lighthouse CI to issue #498
- `issues/open/498-playground-lighthouse-ci.md`: explicit tracking reference for deferred Lighthouse CI
# 491 — Playground CI performance budget enforcement

## Summary

`docs/playground/deployment-strategy.md` specifies three CI gates that are documented
but not yet implemented in any workflow:

1. Binary size gate (§4.3)
2. Bundle size gate
3. Lighthouse CI (stretch goal)

No CI workflow currently enforces these gates. The deployment-strategy doc presents
them as current-state, but they are aspirational.

## Primary paths

- `.github/workflows/`
- `scripts/check/`
- `playground/package.json`
- `docs/playground/deployment-strategy.md`

## Non-goals

- Changing the playground build toolchain
- Adding new runtime features

## Acceptance

- [x] At least one CI workflow step fails when playground wasm binary exceeds a configured threshold
- [x] At least one CI workflow step fails when playground bundle exceeds a configured threshold
- [x] Lighthouse CI is either implemented or explicitly deferred in the deployment-strategy doc with a tracking reference
- [x] `docs/playground/deployment-strategy.md` accurately reflects which gates are enforced vs aspirational

## Required verification

- `bash scripts/run/verify-harness.sh --quick` passes
- CI workflow file references a concrete size threshold

## Close gate

All acceptance items checked; deployment-strategy doc matches CI reality.

## Close evidence

- `.github/workflows/playground-ci.yml`: `playground-wasm-size` job (threshold: `PLAYGROUND_WASM_LIMIT=307200` = 300 KB)
- `.github/workflows/playground-ci.yml`: `playground-bundle-size` job (threshold: `PLAYGROUND_BUNDLE_LIMIT=524288` = 512 KB)
- `scripts/check/check-playground-size.sh`: reusable size check script; exits 1 when threshold exceeded
- `docs/playground/deployment-strategy.md §4.3`: updated with enforcement status table, defers Lighthouse CI to issue #498
- `issues/open/498-playground-lighthouse-ci.md`: explicit tracking reference for deferred Lighthouse CI