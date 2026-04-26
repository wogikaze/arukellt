# Playground Privacy, Telemetry, and Error Reporting Policy

**Status**: DECIDED
**Created**: 2026-07-09
**Scope**: Playground (web), privacy, telemetry, error reporting, GDPR
**Related ADRs**: ADR-017 (execution model), ADR-021 (share URL format), ADR-022 (deployment & caching)

---

## 1. Overview

This document defines the privacy, telemetry, and error reporting policy for the
Arukellt web playground. It covers:

- What data is and is not collected
- Error reporting approach (client-side only)
- Telemetry opt-in/opt-out mechanisms
- GDPR and privacy compliance considerations
- The v1 vs future (v2+) boundary for each area

The playground is a **static, client-side-only web application** (ADR-017). There
is no backend server, no database, and no user accounts. Share URLs are
fragment-based — code is never transmitted to the server in HTTP requests
(ADR-021). Hosting is on GitHub Pages (ADR-022). These architectural constraints
form the foundation of the privacy model: **privacy by architecture, not by
policy alone.**

---

## 2. Foundational Privacy Principles

1. **User code never leaves the browser** — parsing, formatting,
   and diagnostics all execute in a Wasm module running locally in the user's
   browser. No source code is transmitted to any server at any time during
   normal operation. (Type-checking is not yet available; tracked by `issues/open/472`.)

2. **Share URLs are fragment-based** — when a user shares a snippet, the code is
   encoded in the URL fragment (`#`), which is never sent to the HTTP server
   (RFC 3986 §3.5). The server sees only the base URL path, not the shared
   content.

3. **No user accounts, no sessions** — the playground does not authenticate
   users, create sessions, or store persistent user state on any server.

4. **No cookies** — the playground does not set, read, or require HTTP cookies.
   Not in v1; not in any planned future version.

5. **Minimal data surface** — when additional data collection is considered in
   future versions, the default is to not collect. Collection requires explicit
   justification, user consent, and this document to be updated.

---

## 3. Data Collection Scope

### 3.1 Data the playground DOES NOT collect (v1)

| Category | Detail | Guarantee |
|----------|--------|-----------|
| Source code | User-written or pasted `.ark` source | Never transmitted to any server |
| Parse trees / AST | Result of parsing user code | Computed and discarded in browser only |
| Type-check results | Errors, warnings, diagnostics | Not yet available (planned; will run in browser only when implemented — tracked by `issues/open/472`) |
| Formatted output | Result of the formatter | Computed in browser only |
| Keystrokes / input | User typing behavior, timing | Not captured |
| Clipboard contents | Paste events | Not intercepted beyond normal editor behavior |
| Personal information | Name, email, IP (by the app) | Not collected by the playground application |
| Browser fingerprints | Canvas hash, WebGL renderer, fonts | Not collected |
| Local storage data | Playground does not persist to `localStorage` in v1 | No persistent local state |

### 3.2 Data the playground DOES expose (v1)

| Category | Detail | User control |
|----------|--------|-------------|
| Share URL fragment | Encoded source code in URL `#` portion | User explicitly clicks "Share" to generate; code is in the URL only when shared |
| Shared URL in clipboard | When the user copies a share URL | Explicit user action (copy button) |

**Important**: When a user shares a URL via chat, email, or social media, the
encoded source code is visible to anyone who receives the URL. This is by
design — sharing is an explicit, intentional user action. The playground
displays a clear indication that the share URL contains the source code.

### 3.3 Data collected by the hosting infrastructure (outside playground control)

GitHub Pages (the hosting platform) collects standard HTTP server logs as part
of normal web server operation. This is **outside the playground application's
control** and governed by [GitHub's Privacy Statement](https://docs.github.com/en/site-policy/privacy-policies/github-general-privacy-statement).

| Data | Collected by | Playground control |
|------|-------------|-------------------|
| IP address | GitHub Pages / Fastly CDN | None — standard HTTP |
| User-Agent header | GitHub Pages / Fastly CDN | None — standard HTTP |
| Request URL (path only, not fragment) | GitHub Pages / Fastly CDN | None — standard HTTP |
| Request timestamp | GitHub Pages / Fastly CDN | None — standard HTTP |
| Referrer header | GitHub Pages / Fastly CDN | Mitigated by `referrerpolicy="no-referrer"` on outbound links |

**The URL fragment (containing shared source code) is NOT included in HTTP
requests** — this is a property of the HTTP/URL specification (RFC 3986 §3.5),
not a playground-specific implementation choice. GitHub's servers never see
shared source code.

---

## 4. Error Reporting

### 4.1 v1: Browser-local error reporting only

In v1, all error reporting is **local to the user's browser**:

- **Compiler diagnostics** (parse errors, type errors) are displayed in the
  playground's diagnostics panel. They are computed in the Wasm module and
  rendered in the browser DOM. They are never transmitted anywhere.

- **Application errors** (Wasm instantiation failure, JS runtime exceptions)
  are logged to the browser's developer console via `console.error()`. They
  are never transmitted anywhere.

- **Wasm panics** (unexpected compiler crashes) are caught by the Wasm
  error-handling boundary and displayed as a user-facing error message
  ("The compiler encountered an unexpected error. Please try again or
  report a bug."). The panic message is shown to the user but not
  transmitted to any server.

**No error aggregation service is used in v1.** No Sentry, no Bugsnag, no
custom error endpoint. Zero outbound network requests are made by the
playground application (beyond the initial asset load from GitHub Pages).

### 4.2 v2+: Optional client-side error aggregation (future)

If client-side error aggregation is added in a future version:

1. **Opt-in only** — error reporting to an external service MUST be opt-in.
   The default is OFF. Users must explicitly enable it (e.g., toggle in
   playground settings).

2. **No source code in error reports** — error reports MUST NOT include
   user source code, parsed AST, or diagnostic messages derived from user
   code. Reports are limited to:
   - Error type / class name
   - Stack trace (playground application code only, not user code)
   - Browser and OS identification (user-agent string)
   - Playground version

3. **Lightweight service** — if deployed, use a self-hosted or free-tier
   aggregation service (e.g., Sentry free tier) with data residency in a
   GDPR-compliant region (EU or equivalent).

4. **Retention limit** — error data retained for a maximum of 90 days,
   then automatically deleted.

5. **This document must be updated** before any error aggregation is shipped.

### 4.3 Error reporting to the project (manual)

Users who encounter bugs can report them via GitHub Issues. The playground
MAY offer a "Report Bug" button that pre-fills an issue template with:

- Playground version
- Browser and OS (from `navigator.userAgent`)
- The error message displayed

The button MUST NOT auto-include user source code in the report. If the
user wants to include their code for reproduction, they do so manually
(by pasting or including a share URL).

---

## 5. Telemetry

### 5.1 v1: No telemetry

The v1 playground collects **zero telemetry**. There are:

- No analytics scripts (no Google Analytics, no Plausible, no Umami)
- No usage tracking (no event logging, no feature-flag reporting)
- No performance metric collection (no Web Vitals reporting)
- No A/B testing infrastructure
- No outbound network requests from the application

The playground is a fully self-contained static application. After the
initial page load (HTML, JS, CSS, Wasm from GitHub Pages), no further
network communication occurs.

### 5.2 v2+: Privacy-respecting analytics (future)

If usage analytics are added in a future version, they MUST comply with
the following requirements:

| Requirement | Detail |
|-------------|--------|
| **Opt-in by default** | Analytics collection is OFF until the user enables it |
| **No cookies** | Must use cookie-free analytics (e.g., Plausible, Umami) |
| **No PII** | Must not collect personally identifiable information |
| **No source code** | Must not include user code in any analytics event |
| **Aggregated only** | Individual-level tracking is prohibited; only aggregate metrics |
| **Consent banner** | If EU users are detected (via timezone heuristic, NOT IP geolocation), show a clear consent notice |
| **Easy opt-out** | One-click disable in playground settings; preference stored in `localStorage` |
| **Data residency** | Analytics data must be stored in a GDPR-compliant jurisdiction |
| **Open-source analytics** | Prefer self-hosted open-source solutions (Umami, Plausible self-hosted) over proprietary SaaS |
| **Minimal event set** | Collect only: page views, feature usage counts (format, check, share — no content), and errors. No session recording, no heatmaps, no scroll tracking |

### 5.3 Telemetry opt-in/opt-out mechanism (v2+ specification)

When analytics are introduced, the opt-in/opt-out mechanism works as follows:

```
┌──────────────────────────────────────────┐
│  First visit (or cleared localStorage)   │
│  → Analytics: OFF (default)              │
│  → No consent banner shown unless EU     │
│    timezone detected                     │
└────────────┬─────────────────────────────┘
             │
             ▼
┌──────────────────────────────────────────┐
│  User opens Settings panel               │
│  → Toggle: "Help improve the playground  │
│    by sharing anonymous usage data"      │
│  → Default: OFF                          │
│  → Link to this privacy policy document  │
└────────────┬─────────────────────────────┘
             │ (user enables)
             ▼
┌──────────────────────────────────────────┐
│  Analytics: ON                           │
│  → Preference stored: localStorage      │
│    key: "ark-playground-analytics"       │
│    value: "on" | "off"                   │
│  → Cookie-free analytics script loaded   │
│  → Aggregate events collected            │
└────────────┬─────────────────────────────┘
             │ (user disables or clears storage)
             ▼
┌──────────────────────────────────────────┐
│  Analytics: OFF                          │
│  → Analytics script not loaded           │
│  → No events sent                        │
│  → localStorage key removed or "off"     │
└──────────────────────────────────────────┘
```

**Key properties:**
- The analytics script is **never loaded** unless the user has opted in.
  This means zero tracking code executes for users who have not consented.
- Clearing `localStorage` (or using private/incognito browsing) resets
  the preference to OFF.
- The preference is per-browser, not per-user (no user accounts exist).

---

## 6. Content Security Policy and Network Isolation

The playground's Content Security Policy (defined in ADR-022 §8.1) enforces
network isolation at the browser level:

```
default-src 'self';
script-src 'self' 'wasm-unsafe-eval';
style-src 'self' 'unsafe-inline';
connect-src 'self';
img-src 'self' data:;
font-src 'self';
object-src 'none';
base-uri 'self';
```

Privacy implications:

- `connect-src 'self'` — the playground cannot make XHR/fetch requests to
  any domain other than its own origin. This enforces that no data
  exfiltration is possible even if a dependency is compromised.
- `script-src 'self'` — no third-party scripts can be loaded (no analytics
  scripts, no tracking pixels). In v2, if analytics are added, `connect-src`
  would be expanded to include the specific analytics endpoint only.
- `object-src 'none'` — no plugins, no Flash, no Java applets.

**In v1, the CSP makes network-based data collection architecturally
impossible**, not just policy-prohibited.

---

## 7. GDPR and Privacy Compliance

### 7.1 v1 GDPR posture

The v1 playground has a strong GDPR compliance posture due to its architecture:

| GDPR requirement | v1 status | Rationale |
|-----------------|-----------|-----------|
| **Lawful basis for processing** | Not applicable | No personal data is processed by the application |
| **Data minimization** | Satisfied | Zero data collected by the application |
| **Purpose limitation** | Satisfied | No data collection, therefore no purposes to limit |
| **Storage limitation** | Satisfied | No data stored server-side; no `localStorage` used in v1 |
| **Right to access** | Not applicable | No personal data held |
| **Right to erasure** | Not applicable | No personal data held |
| **Right to portability** | Not applicable | No personal data held |
| **Data Protection Impact Assessment** | Not required | No high-risk processing |
| **Cookie consent** | Not required | No cookies used |
| **Consent banner** | Not required | No data collection to consent to |

**Note on hosting infrastructure**: GitHub Pages' server-side logging (IP
addresses, request timestamps) is governed by GitHub's own GDPR compliance
and Data Processing Agreement. This is outside the playground's control and
is not addressed by this policy. Users concerned about GitHub's data
practices should consult [GitHub's Privacy Statement](https://docs.github.com/en/site-policy/privacy-policies/github-general-privacy-statement).

### 7.2 v2+ GDPR requirements

If analytics or error reporting are added in a future version:

1. **Consent before collection** — no analytics data may be collected before
   the user explicitly opts in (legitimate interest is NOT a valid basis
   for analytics on a playground tool).

2. **Cookie-free implementation** — use analytics that do not require cookies,
   eliminating the need for cookie consent banners under ePrivacy Directive.

3. **Data Processing Agreement** — if a third-party analytics provider is
   used, a DPA must be in place before deployment.

4. **Privacy policy link** — the playground must link to this document (or a
   user-facing summary) from the settings panel and footer.

5. **Data residency** — analytics data must be stored in the EU or a
   jurisdiction with an EU adequacy decision, unless the analytics provider
   processes no PII (cookie-free, IP-anonymized solutions like Plausible
   may qualify regardless of residency).

6. **Annual review** — this privacy policy must be reviewed at least annually
   or when any data-touching feature is added.

---

## 8. Third-Party Dependencies and Supply Chain Privacy

### 8.1 v1 dependency posture

The v1 playground loads only first-party assets from its own origin:

- `index.html` — entry point
- `playground-<hash>.js` — application bundle (first-party code)
- `playground-<hash>.css` — styles (first-party)
- `ark-playground-<hash>.wasm` — compiler frontend (first-party Rust code)
- `worker-<hash>.js` — Web Worker (first-party)
- `examples-<hash>.json` — curated examples (first-party)

No external CDN resources (fonts, icon libraries, analytics scripts) are
loaded. No `<script>` or `<link>` tags reference external origins.

### 8.2 Future dependency rules

If external resources are added in future versions:

1. **Self-host over CDN** — prefer self-hosting dependencies (fonts, icons)
   over loading from external CDNs, to avoid leaking referrer/timing data.
2. **SRI required** — any externally-loaded resource must include Subresource
   Integrity hashes.
3. **CSP update** — any new external origin must be explicitly added to the
   CSP; no wildcard origins.
4. **Privacy review** — adding an external dependency that makes network
   requests requires updating this document and reviewing GDPR implications.

---

## 9. v1 vs Future Summary

| Capability | v1 | v2+ (future) |
|-----------|-----|-------------|
| User code transmission | ❌ Never | ❌ Never |
| Cookies | ❌ None | ❌ None (policy) |
| Analytics | ❌ None | ⚠️ Opt-in only, cookie-free |
| Error aggregation | ❌ None (console only) | ⚠️ Opt-in only, no source code |
| Web Vitals reporting | ❌ None | ⚠️ Opt-in, aggregated |
| User accounts / sessions | ❌ None | ❌ Not planned |
| `localStorage` usage | ❌ None | ⚠️ Settings preferences only |
| Third-party scripts | ❌ None | ⚠️ Only with CSP update + review |
| GDPR consent banner | Not required | Required if analytics added |
| Outbound network requests | ❌ Zero (after initial load) | ⚠️ Only analytics endpoint if opted in |

---

## 10. Verification Checklist

Before any version is deployed, verify:

- [ ] No outbound `fetch()` or `XMLHttpRequest` calls exist in the application
      code (v1 gate; v2 allows analytics endpoint only if opted in)
- [ ] No cookies are set (check `document.cookie` and `Set-Cookie` headers)
- [ ] CSP meta tag is present and matches §6
- [ ] No third-party scripts are loaded (check network tab in browser DevTools)
- [ ] Share URLs use fragment encoding only (no query parameters with user data)
- [ ] `referrerpolicy="no-referrer"` is set on outbound links
- [ ] If analytics are present (v2+): toggle defaults to OFF, no events fire
      before opt-in

---

## 11. Document Maintenance

This document MUST be updated when:

- Any form of data collection is added to the playground
- Any external dependency that makes network requests is added
- The hosting platform changes (different privacy implications)
- The share URL format changes in a way that affects privacy
- Any `localStorage` or `sessionStorage` usage is added
- Annually, even if no changes are made (review and re-confirm)

**Owner**: Playground maintainers
**Review cycle**: Annual or on data-touching changes (whichever is sooner)

---

## References

- [ADR-017: Playground Execution Model](../adr/ADR-017-playground-execution-model.md) — client-side only, no server executor
- [ADR-021: Playground Share URL Format](../adr/ADR-021-playground-share-url-format.md) — fragment-based encoding, privacy by default
- [ADR-022: Playground Deployment and Caching](../adr/ADR-022-playground-deployment-and-caching.md) — GitHub Pages hosting, CSP
- [Deployment Strategy](deployment-strategy.md) — operational detail, monitoring roadmap
- [RFC 3986 §3.5](https://datatracker.ietf.org/doc/html/rfc3986#section-3.5) — URI fragment not sent in HTTP requests
- [GitHub Privacy Statement](https://docs.github.com/en/site-policy/privacy-policies/github-general-privacy-statement) — hosting platform privacy
- [GDPR — Regulation (EU) 2016/679](https://eur-lex.europa.eu/eli/reg/2016/679/oj) — General Data Protection Regulation
- [ePrivacy Directive 2002/58/EC](https://eur-lex.europa.eu/eli/dir/2002/58/oj) — Cookie consent requirements
