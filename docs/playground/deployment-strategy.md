# Playground Deployment Strategy

**Status**: DRAFT
**Created**: 2026-06-12
**Scope**: Playground (web), CI/CD, hosting, caching, preview environments
**Related ADRs**: ADR-017 (execution model), ADR-021 (share URL format), ADR-022 (deployment & caching)

> **Status note (2026-04-14, updated for #437):** This document records deployment architecture. The following surfaces are now repo-proved:
> - Browser entrypoint: `docs/playground/index.html` (issue #466 — done)
> - Docs route wiring: `docs/_sidebar.md` links to playground page (issue #467 — done)
> - Build & publish path: `.github/workflows/pages.yml` runs `npm run build:app` and deploys `./docs` including playground assets (issue #468 — done)
> - Wasm content-hash cache busting: `scripts/gen/stamp-playground-assets.sh` produces `ark_playground_wasm_bg-<hash12>.wasm` and `wasm/asset-manifest.json`; `docs/playground/index.html` reads the manifest at runtime to load the content-addressed Wasm URL (issue #437 — done)
>
> The following remain **target-state only** (not yet repo-proved):
> - `npm run dev` (no local dev-server script exists in `playground/package.json`)
> - `npm run build:full` (does not exist; use `npm run build:app` instead)
> - PR preview deployment workflow (no per-PR preview exists in any workflow file)
> - JS bundle content-hash filenames (requires a real bundler, e.g., esbuild/Vite; JS files are served under fixed names with GitHub Pages ~10-minute TTL)
>
> The following are now **repo-proved** (added by issue #491):
> - CI Wasm size gate: `.github/workflows/playground-ci.yml` job `playground-wasm-size` (≤ 300 KB)
> - CI bundle size gate: `.github/workflows/playground-ci.yml` job `playground-bundle-size` (≤ 512 KB)
>
> The following are now **repo-proved** (added by issue #498):
> - Lighthouse CI performance audit: `.github/workflows/playground-ci.yml` job `playground-lighthouse` (LCP ≤ 2.5 s, CLS ≤ 0.1, accessibility ≥ 90)

---

## 1. Overview

This document defines the deployment, preview environment, and asset caching
strategy for the Arukellt web playground. The playground is a **static web
application** (client-side only per ADR-017) that compiles Arukellt source to
diagnostics, parse trees, and formatted output via a Wasm module running in the
browser. There is no server-side component. Share URLs are fragment-based and
require no backend (per ADR-021).

### Constraints from upstream ADRs

| Constraint | Source | Implication |
|------------|--------|-------------|
| Client-side only (v1) | ADR-017 | Static hosting is sufficient; no server, container, or FaaS required |
| No server-side executor | ADR-017 | No compute backend to manage, scale, or secure |
| Fragment-based share URLs | ADR-021 | No URL rewriting, no server-side routing, no database |
| Wasm module ≈ 247 KB (post wasm-opt) | #379 | Dominant asset; caching strategy revolves around it |
| `wasm32-unknown-unknown` target | ADR-017 | No WASI; standard browser Wasm instantiation |

---

## 2. Hosting Platform

### 2.1 Target production host once repo proof exists

**Production hosting**: GitHub Pages, served from a dedicated branch
(`gh-pages`) or a `docs/` directory on `main`, via the project's existing
GitHub infrastructure.

**Rationale**:

1. **Zero operational overhead**: No separate hosting account, billing, or
   infrastructure to manage. GitHub Pages is free for public repositories.
2. **Ecosystem alignment**: The project is GitHub-hosted; docs already use
   GitHub Pages conventions (`.nojekyll`, `docs/index.html`).
3. **Static-only fit**: The playground requires no server-side logic, SSR,
   edge functions, or API routes. GitHub Pages' static-only model is not a
   limitation — it is the exact requirement.
4. **Custom domain support**: GitHub Pages supports custom domains with
   automatic HTTPS via Let's Encrypt.

**Alternatives considered**:

| Platform | Pros | Cons | Verdict |
|----------|------|------|---------|
| Cloudflare Pages | Global CDN, excellent preview deploys, analytics | Separate account/billing, additional vendor | Viable upgrade path if CDN perf needed |
| Netlify | Good DX, preview deploys, redirect rules | Free tier limits, vendor lock-in | Over-featured for static-only app |
| Vercel | Fast builds, edge functions | Opinionated framework assumptions | Poor fit for vanilla static app |
| Self-hosted (S3 + CloudFront) | Full control | Operational burden, cost | Overkill for v1 |

### 2.2 Upgrade path

If GitHub Pages performance proves insufficient (e.g., high latency in
Asia-Pacific, need for edge caching), the migration path is:

1. Switch CI deploy target from GitHub Pages to Cloudflare Pages
2. Update DNS CNAME
3. No code changes required (static assets are platform-agnostic)

---

## 3. CDN and Static Hosting Approach

### 3.1 GitHub Pages CDN characteristics

GitHub Pages serves content via Fastly CDN with the following properties:

- Global edge caching (Fastly PoPs)
- Automatic HTTPS
- HTTP/2 support
- `Cache-Control` headers set by GitHub (10-minute TTL for most assets)
- No custom header configuration (GitHub controls response headers)

### 3.2 Working within GitHub Pages cache constraints

GitHub Pages does not allow custom `Cache-Control` headers. To achieve
effective long-term caching, we use **content-hashed filenames** for all
static assets. The browser treats each unique filename as a new resource,
so cache invalidation is automatic on content change.

```
playground/
├── index.html                          ← always fetched (short GitHub TTL)
├── assets/
│   ├── playground-<hash>.js            ← immutable (hash changes on rebuild)
│   ├── playground-<hash>.css           ← immutable
│   ├── ark-playground-<hash>.wasm      ← immutable (247 KB, hash changes on Rust rebuild)
│   └── worker-<hash>.js               ← immutable (Web Worker script)
└── examples/
    └── examples-<hash>.json            ← immutable (curated example set)
```

### 3.3 Content-hash generation

The build pipeline (see §4) generates content hashes for all assets:

- **JS/CSS**: Bundler (esbuild/Vite) outputs hashed filenames by default
- **Wasm**: Build script computes SHA-256 of `.wasm` file, appends first
  12 hex characters to filename
- **Examples JSON**: Same hash approach as Wasm

`index.html` references all assets by their hashed filenames. Since
`index.html` itself is not hashed, browser always fetches the latest
version (subject to GitHub's ~10-minute TTL), which in turn references
the correct hashed assets.

---

## 4. CI/CD Pipeline

> **Current state (2026-04-14, updated for #471):** `.github/workflows/pages.yml` builds the playground TypeScript package and deploys the `./docs/` directory (which includes `docs/playground/dist/` and `docs/playground/wasm/`) to GitHub Pages on push to `master`. It does **not** compile Wasm from Rust source, run size gates, or create PR preview deployments. The target-state build pipeline in §4.1–§4.3 (Rust Wasm build, wasm-opt, size gate, smoke test) has not yet landed.
>
> **Available `playground/package.json` scripts:**
>
> ```bash
> cd playground && npm run build          # tsc — produces dist/
> cd playground && npm run build:wasm     # wasm-pack build --target web --release
> cd playground && npm run build:all      # build:wasm && build
> cd playground && npm run build:app      # build + copy dist/ and wasm pkg/ into docs/playground/
> cd playground && npm run typecheck      # tsc --noEmit
> cd playground && npm run test           # node --test dist/tests/*.test.js
> cd playground && npm run test:typecheck # tsc --noEmit (alias)
> cd playground && npm run clean          # rm -rf dist/
> ```
>
> **Current pages.yml steps** (docs + playground deploy):
> 1. `actions/checkout@v4`
> 2. `actions/setup-node@v4` (Node.js 20)
> 3. `npm install && npm run build:app` in `playground/`
> 4. `actions/configure-pages@v5`
> 5. `actions/upload-pages-artifact@v3` — uploads `./docs`
> 6. `actions/deploy-pages@v4`
>
> Trigger: `push` to `master` on paths `docs/**` or `.github/workflows/pages.yml`; `workflow_dispatch`.
>
> **Scripts that do NOT exist** (referenced in target-state sections): `npm run dev`, `npm run build:full`.

### 4.1 Build pipeline (target state)

The intended playground build pipeline, once issue #468 lands, is:

```
┌─────────────┐    ┌──────────────┐    ┌───────────────┐    ┌──────────────┐
│ Rust source  │───▶│ cargo build  │───▶│ wasm-opt -Oz  │───▶│ .wasm artifact│
│ (crates/)    │    │ --target     │    │ (size pass)    │    │ (247 KB)     │
│              │    │ wasm32-      │    │               │    │              │
│              │    │ unknown-     │    │               │    │              │
│              │    │ unknown      │    │               │    │              │
└─────────────┘    └──────────────┘    └───────────────┘    └──────────────┘
                                                                    │
┌─────────────┐    ┌──────────────┐    ┌───────────────┐            │
│ TS source   │───▶│ tsc + bundle │───▶│ hashed output │◀───────────┘
│ (playground/ │    │ (esbuild)    │    │ (dist/)       │
│  src/)       │    │              │    │               │
└─────────────┘    └──────────────┘    └───────────────┘
                                              │
                                              ▼
                                    ┌───────────────────┐
                                    │ Deploy to hosting  │
                                    │ (GitHub Pages)     │
                                    └───────────────────┘
```

### 4.2 GitHub Actions workflow (target state)

The target playground CI/CD workflow (does not yet exist in repo):

**Target trigger conditions**:

| Event | Branch/Pattern | Action |
|-------|---------------|--------|
| `push` | `master` | Build + deploy to production |
| `pull_request` | any | Build + deploy preview + comment PR with URL |
| `workflow_dispatch` | `master` | Manual production deploy |

**Target job steps** (production):

1. **Checkout** — `actions/checkout@v4`
2. **Rust toolchain** — Install stable Rust + `wasm32-unknown-unknown` target
3. **Cache Cargo** — `actions/cache@v4` for `~/.cargo` and `target/`
4. **Build Wasm** — `cargo build --release --target wasm32-unknown-unknown -p ark-playground-wasm`
5. **Optimize Wasm** — `wasm-opt -Oz` on output `.wasm`
6. **Size gate** — Assert `.wasm` ≤ 300 KB (headroom above current 247 KB)
7. **Node.js setup** — `actions/setup-node@v4`
8. **Install TS deps** — `npm ci` in `playground/`
9. **Build frontend** — `npm run build` in `playground/` (produces `dist/`)
10. **Assemble dist** — Copy Wasm + frontend assets into deploy directory, apply content hashes
11. **Smoke test** — Run headless browser test (Playwright) against built assets
12. **Deploy** — Push assembled assets to GitHub Pages via `actions/deploy-pages@v4`
13. **Verify** — `python scripts/manager.py verify quick`

**Estimated CI time** (once implemented): 3–5 minutes (Rust build cached), 6–8 minutes (cold cache).

### 4.3 Size gate

> **Enforcement status (updated 2026-04-15, issues #491, #498):**
> - **Enforced in CI:** Wasm binary size gate and JS bundle size gate — both in `.github/workflows/playground-ci.yml`.
> - **Enforced in CI:** Lighthouse CI performance audit — `.github/workflows/playground-ci.yml` job `playground-lighthouse`; budgets in `.github/lighthouserc.json`.

The CI pipeline enforces binary size budgets with two dedicated jobs in
`.github/workflows/playground-ci.yml`. These jobs trigger on push to `master`
and on pull requests touching `playground/**`, `crates/ark-playground-wasm/**`,
or `docs/playground/**`.

| Metric | Current | Budget | CI job | Action on exceed |
|--------|---------|--------|--------|------------------|
| `.wasm` post-opt (uncompressed) | ≈247 KB | **300 KB** | `playground-wasm-size` | CI fails — update `PLAYGROUND_WASM_LIMIT` in workflow and document reason |
| JS bundle (uncompressed total) | ≈90 KB | **512 KB** | `playground-bundle-size` | CI fails — update `PLAYGROUND_BUNDLE_LIMIT` in workflow and document reason |
| Gzipped `.wasm` | ~100 KB (est.) | 150 KB | _(not yet automated)_ | Manual tracking |
| Total initial payload (gzipped) | ~180 KB (est.) | 250 KB | _(not yet automated)_ | Manual tracking |

Budget bumps require a comment in the PR explaining the size increase and
updating the threshold environment variable in `.github/workflows/playground-ci.yml`.

**Automated checks** (`scripts/check/check-playground-size.sh`):
- Called by both CI jobs; also runnable locally.
- `--wasm <file>` mode: checks a single `.wasm` file against `PLAYGROUND_WASM_LIMIT`.
- `--bundle-dir <dir>` mode: sums all `.js` files in the directory against `PLAYGROUND_BUNDLE_LIMIT`.

**Lighthouse CI budgets** (`.github/lighthouserc.json`, enforced by `playground-lighthouse` CI job, issue #498):

| Metric | Budget | Threshold type | CI behaviour on exceed |
|--------|--------|---------------|------------------------|
| Largest Contentful Paint (LCP) | ≤ 2.5 s | hard-error | CI fails |
| Cumulative Layout Shift (CLS) | ≤ 0.1 | hard-error | CI fails |
| Accessibility score | ≥ 90 | hard-error | CI fails |
| Performance score | ≥ 70 | warn only | CI warns (CI env latency is noisy) |

The `playground-lighthouse` job builds the JS bundle, serves `docs/playground/` on
`localhost:3000` via `npx serve`, then runs `treosh/lighthouse-ci-action@v11` against it.
To update a threshold: edit `.github/lighthouserc.json` and document the reason in the PR.

---

## 5. Preview Environments

### 5.1 PR preview deployments (target state, not current repo proof)

Every pull request that modifies `playground/`, `crates/ark-playground-wasm/`,
or playground-related docs gets an automatic preview deployment.

**Mechanism**: GitHub Actions builds the playground and deploys to a
predictable URL derived from the PR number:

```
https://<project>.github.io/playground-preview/pr-<number>/
```

Alternative: Use artifact-based previews with `actions/upload-artifact` and
a deployment step, or leverage a service like Cloudflare Pages (which
provides automatic per-PR preview URLs at no cost).

**Preview lifecycle**:

| Event | Action |
|-------|--------|
| PR opened/updated | Build + deploy preview |
| PR merged | Delete preview deployment |
| PR closed (not merged) | Delete preview deployment after 7 days |

**PR comment**: The CI bot comments on the PR with:

```
🎮 Playground preview: https://...preview.../pr-123/
📦 Wasm size: 247 KB (budget: 300 KB) ✅
⏱️ Build time: 3m 42s
```

### 5.2 Development preview (local)

The browser entrypoint is `docs/playground/index.html`. To build the playground
locally and populate assets into the docs directory:

```bash
# Build the TypeScript playground package and copy into docs/playground/dist/
cd playground && npm run build:app

# Build the Wasm package and JS bindings (requires wasm-pack)
cd crates/ark-playground-wasm && wasm-pack build --target web --release
```

After running `build:app`, open `docs/playground/index.html` directly in a
browser (e.g., via `python3 -m http.server` in the `docs/` directory) to see
the playground with parse/diagnostics. When Wasm modules are available in
`docs/playground/wasm/`, full parse functionality works; otherwise the page
degrades gracefully with an informational message.

**Not yet available:** There is no `npm run dev` hot-reload dev-server script
in `playground/package.json`. A local dev workflow with file watching is
target-state work, not a current capability.

### 5.3 Staging

There is no staging environment for the playground. This is by design:

- The app is static and client-side only — there is no database migration,
  server state, or backend configuration to stage
- Preview deployments on PRs provide pre-merge validation
- Rollback is a single `git revert` + CI redeploy (< 5 minutes)

---

## 6. Asset Caching Strategy

### 6.1 Cache tiers

| Asset | Filename pattern | Cache behavior | Rationale |
|-------|-----------------|----------------|-----------|
| `index.html` | Fixed name | GitHub Pages default (~10 min TTL) | Must always resolve to latest version to pick up new hashed asset references |
| JS bundle | `playground-<hash>.js` | Immutable (hash = cache key) | Content-addressed; any change produces new hash, new URL |
| CSS | `playground-<hash>.css` | Immutable | Same as JS |
| Wasm module | `ark-playground-<hash>.wasm` | Immutable | Largest asset; critical to cache effectively |
| Worker script | `worker-<hash>.js` | Immutable | Runs in Web Worker; same caching as main JS |
| Examples JSON | `examples-<hash>.json` | Immutable | Changes infrequently; curated example set |
| Fonts/icons | `<name>-<hash>.<ext>` | Immutable | If any are added in future |

### 6.2 Wasm module caching — detailed strategy

The Wasm module is the dominant asset at 247 KB (≈100 KB gzipped). Caching
it effectively is the single largest performance lever.

**Content-hash derivation**:

```
SHA-256(ark_playground_wasm.wasm) → first 12 hex chars → ark-playground-<hash>.wasm
```

**Cache invalidation**: Automatic. When the Rust source changes, the Wasm
binary changes, the hash changes, and the filename changes. The old file
is simply never referenced again. No explicit cache purge is needed.

**Browser caching lifecycle**:

1. First visit: Browser downloads `ark-playground-abc123def456.wasm` (247 KB)
2. Subsequent visits (same version): Browser serves from disk cache (0 KB transfer)
3. New version deployed: `index.html` references `ark-playground-789xyz...wasm`;
   browser downloads new file, old file eventually evicted from cache

**Wasm compilation caching**: Modern browsers (Chrome 91+, Firefox 89+,
Safari 15+) cache compiled Wasm modules in a separate code cache. After
the first load, subsequent page loads skip Wasm compilation entirely,
reducing Time to Interactive by 100–300 ms.

### 6.3 Versioned URLs

All asset URLs include a content hash, making them effectively versioned:

```html
<!-- index.html (not versioned, always fresh) -->
<script type="module" src="assets/playground-a1b2c3d4e5f6.js"></script>
<link rel="stylesheet" href="assets/playground-f6e5d4c3b2a1.css">
```

```javascript
// Inside playground-<hash>.js
const wasmUrl = new URL('ark-playground-9f8e7d6c5b4a.wasm', import.meta.url);
```

The version is implicit in the hash — there is no separate version string
in the URL. This avoids query-string cache-busting (`?v=1.2.3`), which
some CDN configurations handle poorly.

### 6.4 Cache invalidation summary

| Scenario | What changes | Cache effect |
|----------|-------------|-------------|
| Rust source change | Wasm hash changes | New Wasm downloaded, old cached |
| TS source change | JS hash changes | New JS downloaded, Wasm still cached |
| CSS change | CSS hash changes | New CSS downloaded, JS+Wasm still cached |
| Example set change | Examples JSON hash changes | New JSON, everything else cached |
| No change (redeploy) | No hashes change | All assets served from cache |

Key property: **Independent invalidation**. A TS-only change does not
invalidate the cached Wasm module, and vice versa. Users who already have
the Wasm cached (the expensive asset) will not re-download it for a
CSS-only fix.

---

## 7. Performance Budget

### 7.1 Initial load budget

| Metric | Budget | Measurement |
|--------|--------|-------------|
| Total transfer size (gzipped) | ≤ 250 KB | Sum of all assets on first load |
| Wasm module (gzipped) | ≤ 150 KB | Single largest asset |
| JS bundle (gzipped) | ≤ 60 KB | Application logic |
| CSS (gzipped) | ≤ 20 KB | Styles |
| HTML | ≤ 10 KB | Entry point |
| Time to Interactive (4G, mid-tier phone) | ≤ 3.0 s | Lighthouse CI |
| Time to Interactive (broadband, desktop) | ≤ 1.0 s | Lighthouse CI |
| Wasm compile time | ≤ 500 ms | `performance.measure()` in app |
| First parse result | ≤ 800 ms from TTI | User types, sees diagnostics |

### 7.2 Subsequent load budget (cached)

| Metric | Budget | Rationale |
|--------|--------|-----------|
| Transfer size | ≤ 10 KB | Only `index.html` (revalidation); all else cached |
| Time to Interactive | ≤ 500 ms | Wasm from code cache, JS from disk cache |

### 7.3 Budget enforcement (target state)

The playground build workflow exists (`.github/workflows/pages.yml` runs
`npm run build:app`), but performance budgets are **not yet enforced in CI**:

1. **Binary size gate** (§4.3): Not yet implemented in any workflow
2. **Bundle size gate**: Not yet implemented
3. **Lighthouse CI** (stretch goal): Not yet implemented

**Current state**: No CI enforcement of size budgets exists. The build
workflow runs `npm run build:app` but does not gate on asset sizes.

### 7.4 Current asset inventory (estimated)

| Asset | Raw size | Gzipped (est.) |
|-------|----------|----------------|
| `ark-playground-<hash>.wasm` | 247 KB | ~100 KB |
| `playground-<hash>.js` | ~80 KB | ~25 KB |
| `worker-<hash>.js` | ~15 KB | ~5 KB |
| `playground-<hash>.css` | ~20 KB | ~5 KB |
| `index.html` | ~5 KB | ~2 KB |
| `examples-<hash>.json` | ~10 KB | ~3 KB |
| **Total** | **~377 KB** | **~140 KB** |

The estimated total of ~140 KB gzipped is well within the 250 KB budget,
providing ~110 KB of headroom for future growth.

---

## 8. Security Considerations

### 8.1 Content Security Policy

The playground should serve a strict CSP via `<meta>` tag (since GitHub
Pages does not support custom headers):

```html
<meta http-equiv="Content-Security-Policy"
  content="default-src 'self';
           script-src 'self' 'wasm-unsafe-eval';
           style-src 'self' 'unsafe-inline';
           connect-src 'self';
           img-src 'self' data:;
           font-src 'self';
           object-src 'none';
           base-uri 'self';">
```

Key points:
- `'wasm-unsafe-eval'` is required for Wasm instantiation
- No `'unsafe-eval'` — no dynamic JS evaluation
- No external script/style sources
- `object-src 'none'` — no plugins

### 8.2 Subresource Integrity

Hashed filenames provide content-addressing but not tamper detection. For
defense-in-depth, SRI hashes should be added to `<script>` and `<link>` tags
in `index.html`:

```html
<script type="module"
  src="assets/playground-a1b2c3d4e5f6.js"
  integrity="sha384-...">
</script>
```

SRI hashes are generated at build time and embedded in `index.html`.

---

## 9. Rollback Procedure

> **Note**: This section describes the rollback procedure. The current `.github/workflows/pages.yml` builds playground JS via `npm run build:app` and deploys the `./docs` directory (including `docs/playground/dist/`) via `actions/deploy-pages@v4`. Rollback is a standard git revert + CI redeploy.

| Step | Action | Time |
|------|--------|------|
| 1 | Identify bad deploy (monitoring, user report) | — |
| 2 | `git revert <commit>` on `master` | 30 s |
| 3 | CI builds and deploys reverted state | 3–5 min |
| 4 | GitHub Pages CDN propagates | ≤ 10 min |
| **Total** | | **< 15 min** |

For faster rollback (target state, once playground CI lands), maintain the
previous known-good deployed state as a tagged ref. Emergency rollback:

```bash
# For faster rollback, use the GitHub Pages deployment history (via Actions UI)
# to re-run the previous successful deploy job.
```

---

## 10. Monitoring and Observability

### 10.1 v1 scope (minimal, no backend)

Since the playground has no backend, monitoring is limited to:

- **GitHub Pages status**: Monitored via GitHub's status page
- **CI pipeline health**: GitHub Actions workflow success/failure
- **Binary size tracking**: Size recorded per build, graphed over time

### 10.2 Future (v2, optional)

- **Client-side error reporting**: `window.onerror` / `reportError()` to a
  lightweight error aggregation service (e.g., Sentry free tier)
- **Usage analytics**: Privacy-respecting analytics (e.g., Plausible,
  Umami) — opt-in, no cookies, GDPR-compliant
- **Web Vitals**: Report LCP, FID, CLS via `web-vitals` library

---

## 11. Summary of Decisions

| Decision | Choice | Rationale |
|----------|--------|-----------|
| Hosting platform | GitHub Pages | Zero cost, ecosystem alignment, static-only fit |
| CDN | Fastly (via GitHub Pages) | Included with GitHub Pages, global PoPs |
| Cache strategy | Content-hashed filenames | Immutable assets, automatic invalidation |
| Wasm caching | Hash in filename + browser code cache | Largest asset, most important to cache |
| Preview deploys | Per-PR deployment (target state — no workflow exists) | Pre-merge validation |
| Staging | None (PR previews planned, production only today) | No backend state to stage |
| Performance budget | ≤ 250 KB gzipped, ≤ 3s TTI (4G) | User experience baseline |
| Size gate | Wasm ≤ 300 KB, total ≤ 250 KB gzip (target state — not yet in CI) | CI enforcement of budgets |
| CSP | Strict, via meta tag | Security baseline for static app |
| Rollback | Git revert + CI redeploy (< 15 min) | Simple, reliable |

---

## Appendix A: File Layout (target-state deployed playground)

> **Note**: This is the intended layout of the deployed playground directory.
> `pages.yml` deploys `./docs`, and `npm run build:app` populates `docs/playground/dist/`
> with TypeScript build output. Content-hashed filenames are target-state (not yet implemented).

```
/ (GitHub Pages root — playground subdirectory once deployed)
├── index.html
├── assets/
│   ├── playground-<hash>.js
│   ├── playground-<hash>.css
│   ├── worker-<hash>.js
│   └── ark-playground-<hash>.wasm
├── examples/
│   └── examples-<hash>.json
└── favicon.ico
```

## Appendix B: Related Documents

- [ADR-017: Playground Execution Model](../adr/ADR-017-playground-execution-model.md)
- [ADR-021: Playground Share URL Format](../adr/ADR-021-playground-share-url-format.md)
- [ADR-022: Playground Deployment and Caching](../adr/ADR-022-playground-deployment-and-caching.md)
- [playground/README.md](../../playground/README.md)
