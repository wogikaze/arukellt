# Release: Extension Activation Tests

> **Status:** open
> **Track:** release
> **Type:** Verification

## Scope

Ensure extension activation tests pass for release verification.

## Checklist Source

docs/release-checklist.md — Extension distribution section

## Acceptance

- [ ] `cd extensions/arukellt-all-in-one && npm ci && npm run build` succeeds
- [ ] VSIX package generated (`.vsix` file exists)
- [ ] Extension activation tests pass (`xvfb-run -a npm test`)

## Required Verification

- Build extension from source
- Verify VSIX package generation
- Run activation tests in headless X environment

## Close Gate

Extension build, VSIX generation, and activation tests must all pass.

## Primary Paths

- `extensions/arukellt-all-in-one/` (extension source)
- Extension build configuration
- VSIX package output
- Activation test suite

## Non-Goals

- Cross-platform testing (Linux only for now)
- Performance optimization
