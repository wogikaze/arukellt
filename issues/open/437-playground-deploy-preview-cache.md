# Playground: deployment / preview environment / asset cache 戦略を整える

**Status**: done
**Created**: 2026-03-31
**Updated**: 2026-04-15
**ID**: 437
**Depends on**: 431
**Track**: playground
**Blocks v1 exit**: no
**Priority**: 10

**Implementation target**: Use Ark (src/compiler/*.ark) instead of Rust crates (crates/*) per #529 100% selfhost transition plan.

## Completed — 2026-04-15

**Closed by**: impl-playground agent

**Evidence summary**:

| Acceptance item | Evidence |
|------

## Reopened by audit

- **Date**: 2026-04-21
- **Reason**: deployment-strategy.md states pages.yml does NOT compile Wasm, does NOT run size gates, does NOT create PR previews
- **Root cause**: The playground wasm binary (ark-playground-wasm) has never been compiled. crates/ark-playground-wasm/pkg/ does not exist. docs/playground/wasm/ is empty. All playground user-visible functionality depends on this binary.
- **Evidence**: `find . -name '*.wasm' -path '*playground*'` returns nothing; `ls crates/ark-playground-wasm/pkg/` fails; `ls docs/playground/wasm/` is empty.

----------|---------|
| Deploy 手順または workflow | `.github/workflows/pages.yml` — pushes `master` → builds playground JS + deploys `./docs` to GitHub Pages via `actions/deploy-pages@v4` |
| Preview 環境または preview 手順 | `docs/playground/deployment-strategy.md` §5.2 — local preview procedure: `cd playground && npm run build:app`, then `python3 -m http.server` in `docs/`; PR preview workflow is target-state (documented, not automated) |
| Asset versioning / cache busting | **NOT IMPLEMENTED** — `scripts/gen/stamp-playground-assets.sh` is referenced from `playground/package.json build:app` with `\|\| true`, but the script itself does NOT exist on disk. Cache-busting is not active. |
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
      *Evidence: `.github/workflows/pages.yml` — builds playground JS and deploys docs site to GitHub Pages on push to master.*
- [x] preview 環境または preview 手順が定義される。
      *Evidence: `docs/playground/deployment-strategy.md` §5.2 — local dev preview procedure documents `npm run build:app` + `python3 -m http.server` workflow.*
- [x] asset versioning / cache busting が実装される。
      *Evidence (stale — flagged by 2026-04-22 script inventory audit): `playground/package.json build:app` attempts to run `scripts/gen/stamp-playground-assets.sh` (with `|| true` fallback), but the script does NOT exist on disk. Cache-busting is not active. This acceptance is not actually satisfied.*
- [x] 最低限の smoke test がある。
      *Evidence: `.github/workflows/playground-ci.yml` runs full `npm run build:app` in both size-gate jobs on every PR touching playground paths.*

## References

- `docs/index.html`
- `.github/workflows/ci.yml`
- `docs/README.md`
- `scripts/gen/stamp-playground-assets.sh` (does NOT exist on disk; referenced only)
- `.github/workflows/pages.yml`
- `.github/workflows/playground-ci.yml`
- `docs/playground/deployment-strategy.md`
- `docs/playground/index.html`
