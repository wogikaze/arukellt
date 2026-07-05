# ADR-019: Link-Check Coverage Policy

ステータス: **DECIDED** — リンクチェックカバレッジポリシーを採用
**Created**: 2026-04-14
**Revised**: 2026-07-06 — ポリシーを柱3（リンクチェック）のみに縮小。柱1（アンカー命名規則）・柱2（リダイレクトポリシー）は運用実態に合わないため削除。
**Scope**: Language documentation (`docs/language/`), all Markdown docs under `docs/`, docs site (`docs/index.html`)

---

## Context

As the language documentation grows, internal links (`path.md#anchor`) and file references (`path/to/file.md`) can break when documents are moved, renamed, or reorganized. A link-check harness is needed to catch drift on CI and in `verify quick`.

This ADR covers **link-check coverage** — what the harness checks and what is out of scope.

> **Historical note:** This ADR originally also specified an anchor naming convention (S1/S2/S3 tiers, explicit `<a id="">` anchors) and a redirect/alias policy (Docsify aliases, stub files, `SPLIT_FROM`/`MERGED_FROM` comments). Those policies were not operationally enforced and have been removed. The GFM auto-anchor rules and explicit `<a id="">` anchors remain valid Markdown/Docsify behavior but are no longer mandated by this ADR.

---

## Decision

### 1. Existing coverage: `scripts/check/check-links.sh`

A link-check script exists at `scripts/check/check-links.sh`. It:

- Scans all Markdown files under `docs/` and `issues/`.
- Validates that relative file references in `](path...)` link targets resolve to existing files.
- **Intentionally skips** pure anchor-only references (`#anchor`) and anchors appended to file paths (`path.md#anchor`) — only the file existence is checked.
- **Does not** check external URLs (`http://`, `https://`).

This script is the **v1 canonical link-checker**. Do not add a second link-checker unless the scope of this script is demonstrably insufficient for a specific assigned work order.

### 2. Anchor fragment checking (implemented)

Anchor fragment validation is implemented in `scripts/check/check-anchor-fragments.py` and wired into `python3 scripts/manager.py verify quick` (static pass, immediately after `scripts/check/check-links.sh`).

The checker:

- Scans Markdown under `docs/` and `issues/`, plus `README.md` and `AGENTS.md`.
- Validates relative links of the form `path.md#anchor` and same-file `#anchor` references.
- Resolves targets using GFM heading slug rules plus explicit `<a id="">` anchors.
- Skips external URLs (`http://`, `https://`, `mailto:`) and Docsify router paths (`#/...`).
- Supports an optional allowlist at `scripts/check/anchor-allowlist.txt` for known exceptions.

Authors adding cross-document anchor links should still verify targets locally; the harness catches drift on CI and in `verify quick`.

### 3. Summary of link-check guarantees

| Check | Tool | Status |
|-------|------|--------|
| Internal file references (e.g. `path/to/file.md`) | `scripts/check/check-links.sh` | ✅ Covered |
| Anchor fragments (e.g. `file.md#section-id`) | `scripts/check/check-anchor-fragments.py` | ✅ Covered |
| Pure in-page anchors (e.g. `#section-id`) | `scripts/check/check-anchor-fragments.py` | ✅ Covered |
| External URLs (`https://...`) | — | ❌ Out of scope |

---

## Consequences

- `scripts/check/check-links.sh` validates internal file references; `scripts/check/check-anchor-fragments.py` validates anchor fragments — both run in `verify quick`.
- ADR-018 classification banners are orthogonal to this policy; both apply independently.
- Document moves/renames/splits/merges do not require Docsify aliases, stub files, or `SPLIT_FROM`/`MERGED_FROM` comments (the previously mandated policies have been removed). Authors should update inbound links manually; the link-check harness will catch broken references.

---

## Alternatives Considered

**Embed redirect headers in Markdown front matter**
Rejected: The docs site (Docsify) does not process YAML front matter for redirects.

**Add anchor-fragment checking to `check-links.sh` now**
Rejected: heading extraction and slug deduplication are easier to maintain in a dedicated `check-anchor-fragments.py` script (implemented in issue #644).

**Use a separate `_redirects` file (Netlify-style)**
Rejected: The project is not currently deployed to Netlify or any service that reads `_redirects`. Docsify aliases in `docs/index.html` are self-contained and do not require a hosting-specific feature.

---

## References

- `docs/index.html` — Docsify configuration
- `scripts/check/check-links.sh` — internal file reference checker
- `scripts/check/check-anchor-fragments.py` — anchor fragment checker (GFM slugs + explicit ids)
- ADR-018: Language Docs Classification — Normative / Explanatory / Transitional
- Issue #644: Docs anchor fragment link-check (ADR-019 v2 delivery)
- Issue #412: Language Docs: 安定した anchor / permalink 体系を整える
