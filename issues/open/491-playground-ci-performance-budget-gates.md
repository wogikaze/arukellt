# 491 — Playground CI performance budget enforcement

**Track:** playground
**Status:** open
**Created:** 2026-04-14
**Updated:** 2026-04-14
**Source:** audit — docs/playground/deployment-strategy.md §4.3

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

- [ ] At least one CI workflow step fails when playground wasm binary exceeds a configured threshold
- [ ] At least one CI workflow step fails when playground bundle exceeds a configured threshold
- [ ] Lighthouse CI is either implemented or explicitly deferred in the deployment-strategy doc with a tracking reference
- [ ] `docs/playground/deployment-strategy.md` accurately reflects which gates are enforced vs aspirational

## Required verification

- `bash scripts/run/verify-harness.sh --quick` passes
- CI workflow file references a concrete size threshold

## Close gate

All acceptance items checked; deployment-strategy doc matches CI reality.
