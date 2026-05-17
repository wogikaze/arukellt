# Playground Build and Publish Path - Proof Record

**Issue**: #468; updated by #631
**Status**: PROVED
**Current proof date**: 2026-05-17

## Current Build Commands

Run from repo root:

```bash
cd playground
npm run build:app
```

This compiles the TypeScript playground engine and UI package, then copies the
compiled output to `docs/playground/dist/`.

## Current Output Paths

- `playground/dist/` - compiled TypeScript package
- `docs/playground/dist/` - deployable copy used by GitHub Pages

The retired Rust playground Wasm package is no longer part of the build path.
`docs/playground/wasm/` is not populated by CI.

## Publish Path

`.github/workflows/pages.yml` uploads `./docs` to GitHub Pages:

```yaml
- uses: actions/upload-pages-artifact@v3
  with:
    path: ./docs
- uses: actions/deploy-pages@v4
```

The workflow triggers on changes to `docs/**`, `playground/**`, or
`.github/workflows/pages.yml`.

## Toolchain Requirements

- Node.js >= 18

## Historical Note

Earlier proof records used a Rust Wasm bridge and wasm-pack-generated assets.
That path was retired in #631 during the selfhost cleanup phase. Current
playground behavior is provided by `playground/src/engine.ts`.
