# ADR-019: Anchor / Permalink Naming Convention and Redirect Policy

**Status**: DECIDED
**Created**: 2026-04-14
**Scope**: Language documentation (`docs/language/`), all Markdown docs under `docs/`, docs site (`docs/index.html`)

---

## Context

As the language documentation grows and reorganization occurs (section splits, renames, hierarchy changes),
anchor links and document permalinks become unstable. Readers who bookmark URLs or external tools (issue
trackers, RFCs, external blogs) that link into the docs will encounter broken references. Additionally,
the existing `scripts/check-links.sh` checks internal file references but does not validate anchor
fragments — this gap should be documented and a policy set for v1 scope.

This ADR covers three related concerns:

1. **Anchor naming convention** — how in-page section anchors must be formed.
2. **Redirect / alias policy** — how to handle stable external URLs when docs are moved or split.
3. **Link-check coverage** — what the existing harness covers and what is deferred.

---

## Decision

### 1. Anchor Naming Convention

#### 1.1 Auto-generated anchors (GFM / Docsify)

The docs site uses [Docsify](https://docsify.js.org/) with `routerMode: 'hash'`.
Docsify auto-generates per-heading anchors from heading text using GitHub Flavored Markdown (GFM) rules:

- Convert heading text to lowercase.
- Replace spaces with hyphens (`-`).
- Remove all characters except `[a-z0-9-]` (i.e., strip punctuation, diacritics, non-ASCII).
- Deduplicate anchors on the same page by appending `-1`, `-2`, … as needed.

These auto-generated anchors are **implicitly stable** only when the heading text does not change.
Heading renames are therefore **breaking changes** for any external link targeting that anchor.

#### 1.2 Anchor stability tiers

| Tier | Applicability | Requirement |
|------|--------------|-------------|
| **S1 — fixed** | Top-level `##` headings in normative docs and ADRs | MUST NOT change after the document is merged; renames require a redirect alias |
| **S2 — stable** | Sub-section `###` headings in normative docs | SHOULD NOT change without a migration note in the same PR |
| **S3 — advisory** | `####` and deeper in any doc; all headings in transitional or explanatory docs | MAY change freely |

#### 1.3 Explicit stable anchors for S1 headings

For **S1** headings that are likely to be externally linked (e.g., referenced from issues, from `docs/adr/`,
or from external documentation), place an explicit anchor element **before** the heading:

```markdown
<a id="error-result-type"></a>
## Error Handling: Result Type
```

The `id` attribute MUST follow the same lowercase-hyphenated form and MUST be unique within the document.
Explicit anchors take precedence over the auto-generated form; both remain valid in Docsify.

#### 1.4 Anchor naming rules

All `id` values (explicit or auto-generated) MUST conform to:

- Pattern: `[a-z][a-z0-9-]*`
- No uppercase letters.
- No underscores — use hyphens instead.
- No leading hyphens.
- Maximum 80 characters.
- Derive the `id` from the canonical English form of the heading (even when the heading text is in another language).

Examples:
- `##型変換` → explicit anchor: `<a id="type-conversion"></a>`
- `## Result / Option エラー処理` → explicit anchor: `<a id="result-option-error-handling"></a>`
- `## Memory Model — GC native` → auto-anchor: `memory-model--gc-native` (two hyphens from ` — `); explicit stable: `<a id="memory-model-gc-native"></a>`

#### 1.5 ADR document anchors

Each ADR section heading (Context, Decision, Consequences, Alternatives, References) is **S1**
and MUST use the GFM-generated anchor form (no special characters in those headings, so GFM and
explicit are equivalent). Cross-ADR section links use the pattern:

```markdown
[ADR-018 § Decision](ADR-018-language-docs-classification.md#decision)
```

---

### 2. Redirect / Alias Policy for Doc Reorganization

#### 2.1 Scope

This policy applies whenever a document in `docs/` is:

- **Moved** to a different directory or filename.
- **Renamed** (filename change, even within the same directory).
- **Split** (one doc becomes two or more).
- **Merged** (two docs become one, eliminating a URL).

#### 2.2 Docsify alias entries

The docs site uses Docsify's `alias` config in `docs/index.html` to provide path-level redirects.
When a document is moved or renamed, a Docsify alias MUST be added to `window.$docsify.alias`
before the old URL is removed:

```js
alias: {
  // Format: 'old-path': 'new-path'  (relative to docs root, without leading slash)
  'language/old-name.md': 'language/new-name.md',
  // Wildcard aliases use regex syntax:
  '/language/old-section/(.*)': '/language/new-section/$1',
}
```

#### 2.3 Retention period

Redirect aliases MUST be kept for a minimum of **two minor releases** after the old path is removed.
After that period, aliases MAY be removed in a PR that documents the cleanup.

#### 2.4 Stub files for moved normative documents

When a **normative** document is moved, in addition to the Docsify alias, leave a one-paragraph
stub at the old path for at least **one minor release**:

```markdown
# <Old Title> (Moved)

> This document has moved to [new-name.md](../new-name....md).
> Please update your bookmarks. This stub will be removed after vX.Y.
```

Stub files are exempt from classification requirements (they are not added to
`docs/data/language-doc-classifications.md`).

#### 2.5 Split policy

When a document is split into two or more files:

1. The original URL MUST redirect (Docsify alias) to the primary/canonical replacement.
2. The original S1 section anchors MUST be preserved in the target files using explicit `<a id="">` elements.
3. A `SPLIT_FROM` comment at the top of each new file documents the origin:

```markdown
<!-- SPLIT_FROM: old-doc.md (split in #NNN, vX.Y) -->
```

#### 2.6 Merge policy

When two documents are merged:

1. Both old URLs MUST redirect to the merged document.
2. Section anchors from both source documents MUST be preserved using explicit `<a id="">` elements.
3. A `MERGED_FROM` comment at the top of the new file documents the origin:

```markdown
<!-- MERGED_FROM: doc-a.md, doc-b.md (merged in #NNN, vX.Y) -->
```

---

### 3. Link-Check Coverage

#### 3.1 Existing coverage: `scripts/check-links.sh`

A link-check script already exists at `scripts/check-links.sh`. It:

- Scans all Markdown files under `docs/` and `issues/`.
- Validates that relative file references in `](path...)` link targets resolve to existing files.
- **Intentionally skips** pure anchor-only references (`#anchor`) and anchors appended to file paths
  (`path.md#anchor`) — only the file existence is checked.
- **Does not** check external URLs (`http://`, `https://`).

This script is the **v1 canonical link-checker**. Do not add a second link-checker unless the
scope of this script is demonstrably insufficient for a specific assigned work order.

#### 3.2 Anchor fragment checking (deferred to v2)

Anchor fragment validation requires either:
- A live Docsify-rendered site, or
- A Markdown heading extraction pass (extracting all `##...` headings and resolving `path.md#anchor`).

This is deferred to v2. For v1, the policy is:

- **Authors are responsible** for verifying anchor targets when adding cross-document links.
- **PRs that rename S1 headings** in normative docs MUST include a search for inbound anchor references
  and either update them or add explicit `<a id="">` elements to preserve the target.

#### 3.3 Summary of v1 link-check guarantees

| Check | Tool | v1 Status |
|-------|------|-----------|
| Internal file references (e.g. `path/to/file....md`) | `scripts/check-links.sh` | ✅ Covered |
| Anchor fragments (e.g. `file....md#section-id`) | — | ❌ Not covered (v2) |
| Pure in-page anchors (e.g. `#section-id`) | — | ❌ Not covered (v2) |
| External URLs (`https://...`) | — | ❌ Out of scope |

---

## Consequences

- Every PR that renames an S1 heading in a normative document MUST either (a) update all inbound
  anchor links, or (b) add an explicit `<a id="old-anchor">` to preserve the old target.
- Every PR that moves or removes a document MUST add a Docsify alias entry to `docs/index.html`.
- `scripts/check-links.sh` is the v1 canonical link checker; no additional tool is required for v1.
- Anchor fragment checking is a v2 enhancement (separate work order).
- ADR-018 classification banners are orthogonal to this policy; both apply independently.

---

## Alternatives Considered

**Embed redirect headers in Markdown front matter**
Rejected: The docs site (Docsify) does not process YAML front matter for redirects. Docsify aliases
in `docs/index.html` are the appropriate mechanism.

**Add anchor-fragment checking to `check-links.sh` now**
Rejected for v1: Requires Markdown heading extraction logic that is non-trivial to maintain correctly
across heading level nesting. Deferred to v2 as a standalone enhancement.

**Use a separate `_redirects` file (Netlify-style)**
Rejected: The project is not currently deployed to Netlify or any service that reads `_redirects`.
Docsify aliases in `docs/index.html` are self-contained and do not require a hosting-specific feature.

---

## References

- `docs/index.html` — Docsify configuration including alias entries
- `scripts/check-links.sh` — existing internal file reference checker
- `docs/data/language-doc-classifications.toml` — per-document classification data (ADR-018)
- ADR-018: Language Docs Classification — Normative / Explanatory / Transitional
- ADR-016: Breaking Change Process — Three-Piece Set
- Issue #412: Language Docs: 安定した anchor / permalink 体系を整える
