# Playground: deployment / preview environment / asset cache 戦略を整える

**Status**: done
**Created**: 2026-03-31
**Updated**: 2026-04-15
**ID**: 437
**Depends on**: 431
**Track**: playground
**Blocks v1 exit**: no
**Priority**: 10


## Completed — 2026-04-15

**Closed by**: impl-playground agent

**Evidence summary**:

| Acceptance item | Evidence |
|----------------|---------|
| Deploy 手順または workflow | `.github/workflows/pages.yml` — pushes `master` → builds playground JS + deploys `./docs` to GitHub Pages via `actions/deploy-pages@v4` |
| Preview 環境または preview 手順 | `docs/playground/deployment-strategy.md` §5.2 — local preview procedure: `cd playground && npm run build:app`, then `python3 -m http.server` in `docs/`; PR preview workflow is target-state (documented, not automated) |
| Asset versioning / cache busting | `scripts/gen/stamp-playground-assets.sh` — computes SHA-256 of `ark_playground_wasm_bg.wasm` (first 12 hex chars), copies to `ark_playground_wasm_bg-<hash12>.wasm`, writes `docs/playground/wasm/asset-manifest.json`; `docs/playground/index.html` fetches the manifest at runtime and passes the hashed URL to `createPlayground` |
| Smoke test | `.github/workflows/playground-ci.yml` jobs `playground-bundle-size` and `playground-wasm-size` both run `npm run build:app` and gate on asset size — pass = build compiles clean within budget |

**Gap note**: JS bundle files (served from `docs/playground/dist/`) use fixed filenames because there is no bundler (esbuild/Vite) in the pipeline. GitHub Pages' ~10-minute TTL provides adequate cache freshness for JS. Full JS content-hash filenames require a proper bundler; that work is not in this issue's scope.

**PR preview**: No per-PR automated preview deployment exists (documented explicitly as target-state in `deployment-strategy.md` §5.1). The local preview procedure (§5.2) satisfies "preview 手順が定義される."

## Reopened by audit — 2026-04-13

**Reason**: No preview deploy or smoke test.

**Action**: Moved from issues/done/ to issues/open/ by false-done audit.

## Summary

playground の frontend と Wasm assets をどこに配置し、preview 環境と cache busting をどう扱うかを決めて実装する。これも design-only ではなく、実際の deploy 手順と versioned assets を持たせる。

## Current state

- frontend package も deploy pipeline も存在しない。
- Wasm asset はキャッシュされやすく、更新反映が難しい。
- preview を持たないと docs との統合検証がしにくい。

## Acceptance

- [x] deploy 手順または workflow が追加される。
      _Evidence: `.github/workflows/pages.yml` — builds playground JS and deploys docs site to GitHub Pages on push to master._
- [x] preview 環境または preview 手順が定義される。
      _Evidence: `docs/playground/deployment-strategy.md` §5.2 — local dev preview procedure documents `npm run build:app` + `python3 -m http.server` workflow._
- [x] asset versioning / cache busting が実装される。
      _Evidence: `scripts/gen/stamp-playground-assets.sh` — content-hashes Wasm binary, writes `asset-manifest.json`; `docs/playground/index.html` reads manifest to load hashed Wasm URL. Called from `playground/package.json` `build:app`._
- [x] 最低限の smoke test がある。
      _Evidence: `.github/workflows/playground-ci.yml` runs full `npm run build:app` in both size-gate jobs on every PR touching playground paths._

## References

- `docs/index.html`
- `.github/workflows/ci.yml`
- `docs/README.md`
- `scripts/gen/stamp-playground-assets.sh`
- `.github/workflows/pages.yml`
- `.github/workflows/playground-ci.yml`
- `docs/playground/deployment-strategy.md`
- `docs/playground/index.html`
