# Playground Deployment Strategy

**Status**: DRAFT
**Updated**: 2026-05-17
**Scope**: Playground (web), CI/CD, hosting, caching, preview environments
**Related ADRs**: ADR-017 (execution model), ADR-021 (share URL format), ADR-022 (deployment & caching)

## Current Repo-Proved Surface

- Browser entrypoint: `docs/playground/index.html`
- Docs route wiring: `docs/_sidebar.md` links to the playground page
- Build and publish path: `.github/workflows/pages.yml` runs
  `npm run build:app` in `playground/` and deploys `./docs`
- Browser engine: `playground/src/engine.ts`, bundled into
  `docs/playground/dist/`
- JS bundle size gate: `.github/workflows/playground-ci.yml` job
  `playground-bundle-size`
- Lighthouse audit: `.github/workflows/playground-ci.yml` job
  `playground-lighthouse`

The former Rust playground Wasm bridge and wasm-pack asset path were retired in
issue #631. Historical references to that path should not be used as current
build or deployment instructions.

## Overview

The Arukellt web playground is a static, client-side-only web application. It
parses, formats, tokenizes, and reports diagnostics in the browser through the
TypeScript playground engine. There is no server-side executor, no database, and
no API backend. Share URLs remain fragment-based per ADR-021.

## Build Path

From repo root:

```bash
cd playground
npm install --no-audit --no-fund --package-lock=false
npm run build:app
```

`build:app` compiles the TypeScript package and copies `playground/dist/` into
`docs/playground/dist/`. Generated build outputs are not committed.

## GitHub Pages Deployment

`.github/workflows/pages.yml` deploys the `./docs` directory to GitHub Pages on
pushes to `master` that touch:

- `docs/**`
- `playground/**`
- `.github/workflows/pages.yml`

## CI Gates

`playground-bundle-size` builds the playground package and checks the total JS
size under `docs/playground/dist/` with
`scripts/check/check-playground-size.sh --bundle-dir docs/playground/dist`.

`playground-lighthouse` serves `docs/playground/` locally and runs Lighthouse
against the built static page.

## Cache Strategy

Current deploy output uses fixed JS filenames produced by `tsc`; GitHub Pages
HTTP caching is the active cache layer. A future bundler migration may add
content-hashed JS filenames.

## Historical Retired Path

Before #631, the playground built a Rust `wasm32-unknown-unknown` package with
wasm-pack, copied generated assets into `docs/playground/wasm/`, and stamped a
content-hashed Wasm filename. That path no longer exists in the active
workspace. `scripts/gen/stamp-playground-assets.sh` is retained as a no-op
compatibility hook for older build scripts.
