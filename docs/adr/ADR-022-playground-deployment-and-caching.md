# ADR-022: Playground Deployment and Asset Caching Strategy

**Status**: DECIDED
**Created**: 2026-06-12
**Scope**: Playground (web), deployment, CDN, caching, CI/CD, performance budget

---

## Context

ADR-017 established that the playground v1 is a **client-side-only static web
application** — parse, format, check, and diagnostics via Wasm in the browser,
with no server-side component. ADR-021 established that share URLs use
**fragment-based encoding** (no server roundtrip). The Wasm module is the
dominant asset at 247 KB after `wasm-opt` (from #379).

Before implementation begins, the project needs clear answers to:

1. **Where is the playground hosted?** Static hosting options range from
   GitHub Pages (free, integrated) to dedicated CDN platforms (Cloudflare
   Pages, Netlify, Vercel). The choice affects cache control, preview
   deployments, and operational overhead.

2. **How are assets cached?** The Wasm module is expensive to download
   (247 KB raw, ~100 KB gzipped) and expensive to compile. Effective caching
   is the single largest performance lever. The strategy must handle cache
   invalidation without manual purges.

3. **How do preview environments work?** Contributors need to validate
   playground changes before merge. PR preview deployments reduce the
   feedback loop from "merge and hope" to "click and verify."

4. **What is the performance budget?** Without an explicit budget, asset
   sizes tend to grow unchecked. A CI-enforced budget prevents regressions.

---

## Decision

### D1: Host on GitHub Pages (production)

The playground is deployed as a static site to **GitHub Pages**, served from
the `gh-pages` branch via GitHub's Fastly CDN.

**Rationale**: Zero operational cost, zero additional vendor accounts, and the
project already uses GitHub for source hosting and CI. The playground has no
server-side requirements — GitHub Pages' static-only model is the exact fit.

**Upgrade path**: If CDN performance proves insufficient (e.g., high latency
in specific regions), migration to Cloudflare Pages requires only a CI
target change and DNS CNAME update — no code changes.

### D2: Content-hashed filenames for all static assets

All assets except `index.html` use content-hashed filenames:

```
playground-<hash>.js
playground-<hash>.css
ark-playground-<hash>.wasm
worker-<hash>.js
examples-<hash>.json
```

The hash is derived from the file content (SHA-256, first 12 hex characters).
`index.html` is the only mutable entry point; it references all other assets
by their hashed names.

**Rationale**: Content-hashing provides **automatic cache invalidation** without
explicit purges. When content changes, the hash changes, the filename changes,
and browsers fetch the new version. When content does not change (e.g., a
TS-only fix does not change the Wasm), browsers continue serving the cached
Wasm — **independent invalidation** across asset types.

This strategy works within GitHub Pages' cache constraints (no custom
`Cache-Control` headers) because the browser treats each unique URL as a
distinct cacheable resource.

### D3: PR preview deployments via GitHub Actions

Every PR that modifies playground source (`playground/`, `crates/ark-playground-wasm/`)
receives an automatic preview deployment. The CI bot comments on the PR with
the preview URL, Wasm size, and build time.

Preview deployments are cleaned up when the PR is merged or closed.

**Rationale**: Pre-merge validation of playground changes reduces risk and
enables visual review without local builds.

### D4: Performance budget with CI enforcement

| Metric | Budget |
|--------|--------|
| Wasm module (raw) | ≤ 300 KB |
| Total initial payload (gzipped) | ≤ 250 KB |
| Time to Interactive (4G, mid-tier device) | ≤ 3.0 s |
| Time to Interactive (broadband, desktop) | ≤ 1.0 s |

The Wasm size gate and total payload gate are enforced in CI as blocking checks.
TTI is initially advisory (Lighthouse CI) and promoted to blocking once the
baseline is stable.

**Rationale**: The current Wasm module (247 KB) plus estimated JS/CSS/HTML
totals ~140 KB gzipped, well within the 250 KB budget. The 300 KB Wasm
ceiling provides ~20% headroom for feature growth while preventing
unchecked bloat.

---

## Consequences

### Positive

- **Zero hosting cost**: GitHub Pages is free for public repositories.
- **Automatic cache invalidation**: Content-hashed filenames eliminate
  manual cache purges and "clear your cache" support requests.
- **Independent asset caching**: A CSS fix does not force re-download of
  the 247 KB Wasm module.
- **Pre-merge confidence**: PR previews catch visual and functional
  regressions before merge.
- **Enforced performance discipline**: CI gates prevent gradual size creep.

### Negative

- **No custom `Cache-Control` headers**: GitHub Pages controls caching
  headers. `index.html` has a ~10-minute TTL (not instant). This is
  acceptable for a playground (not latency-critical content).
- **Preview deployment complexity**: PR previews require CI workflow
  configuration and artifact management.
- **No server-side analytics**: Usage data requires a client-side
  analytics solution (deferred to v2).

### Neutral

- **Rollback is git-based**: Reverting a bad deploy requires `git revert`
  - CI redeploy (~15 minutes). Emergency rollback via force-push to
  `gh-pages` is faster (< 1 minute + CDN propagation).
- **Migration to another host is low-friction**: All assets are static
  files with no platform-specific configuration.

---

## Alternatives Rejected

### A1: Cloudflare Pages as primary host

Cloudflare Pages offers superior CDN, custom headers, and native preview
deployments. However, it introduces a separate vendor account, billing
relationship, and operational surface. The project can migrate to Cloudflare
Pages later if GitHub Pages performance proves insufficient.

### A2: Query-string cache busting (`?v=1.2.3`)

Some projects use `app.js?v=1.2.3` for cache busting. This is fragile:
some CDN configurations and proxies strip or ignore query strings for
caching purposes. Content-hashed filenames are universally reliable.

### A3: Service Worker for offline caching

A Service Worker could provide offline support and more aggressive caching.
This adds complexity (SW lifecycle management, update flows) and is deferred
to v2. Content-hashed filenames provide sufficient caching without a SW.

### A4: No performance budget

Without a budget, the Wasm module could grow to 500+ KB as features are
added. An explicit budget forces conscious trade-offs and prevents
"death by a thousand cuts" size regression.

---

## Verification

- `bash scripts/verify-harness.sh --quick` — all harness checks pass
- `python3 scripts/check-docs-consistency.py` — docs consistency verified
- Deployment strategy document: `docs/playground/deployment-strategy.md`

---

## References

- ADR-017: Playground Execution Model and v1 Product Contract
- ADR-021: Playground Share URL Format
- `docs/playground/deployment-strategy.md` — full operational detail
- `playground/README.md` — architecture overview
