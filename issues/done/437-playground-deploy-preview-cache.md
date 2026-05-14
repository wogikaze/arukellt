---
Status: done
Created: 2026-03-31
Updated: 2026-05-14
ID: 437
Track: playground
Depends on: 431
Orchestration class: implementation-ready
---

# Playground: deployment / preview environment / asset cache 戦略を整える

## Summary

playground の frontend と Wasm assets をどこに配置し、preview 環境と
cache busting をどう扱うかを決めて実装する。これは design-only ではなく、
実際の deploy 手順、preview 手順、versioned assets、smoke gate を持たせる issue。

## Scope Notes

- JS bundle files under `docs/playground/dist/` still use fixed filenames because
  the current build path uses `tsc` rather than a bundler such as esbuild/Vite.
  GitHub Pages' short TTL is accepted for JS in this issue.
- PR preview deployment remains target-state only. The local preview procedure in
  `docs/playground/deployment-strategy.md` satisfies the preview-procedure scope.
- Wasm cache busting is implemented for `ark_playground_wasm_bg.wasm` through
  `scripts/gen/stamp-playground-assets.sh` and `docs/playground/wasm/asset-manifest.json`.

## Acceptance

- [x] deploy 手順または workflow が追加される。
      *Evidence: `.github/workflows/pages.yml` builds playground JS and deploys docs site to GitHub Pages on push to `master`.*
- [x] preview 環境または preview 手順が定義される。
      *Evidence: `docs/playground/deployment-strategy.md` documents the local preview procedure using `npm run build:app` and a static file server under `docs/`.*
- [x] asset versioning / cache busting が実装される。
      *Evidence: `scripts/gen/stamp-playground-assets.sh` stamps `docs/playground/wasm/ark_playground_wasm_bg.wasm` into `ark_playground_wasm_bg-<hash>.wasm` and writes `asset-manifest.json`; `docs/playground/index.html` reads that manifest for `wasmUrl`.*
- [x] 最低限の smoke test がある。
      *Evidence: `.github/workflows/playground-ci.yml` runs `npm run build:app` in the playground size-gate jobs on PRs touching playground paths.*

## Close Note — 2026-05-14

The false-done blocker was fixed at the root: the missing
`scripts/gen/stamp-playground-assets.sh` now exists, and `build:app` no longer
silently suppresses a missing stamp script. When the Wasm package is present, the
script emits a hashed Wasm filename and manifest consumed by the playground
entrypoint. When the Wasm package is absent in local preview, it exits cleanly so
the documented stub fallback remains usable.

## References

- `playground/package.json`
- `scripts/gen/stamp-playground-assets.sh`
- `docs/playground/index.html`
- `docs/playground/deployment-strategy.md`
- `.github/workflows/pages.yml`
- `.github/workflows/playground-ci.yml`
