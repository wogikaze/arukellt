---
Status: open
Created: 2026-06-12
Updated: 2026-06-12
ID: 639
Track: cli
Depends on: 487
Orchestration class: implementation-ready
Blocks v1 exit: none
Source: docs-to-issues audit — docs/process/docs-gap-inventory-2026-06-12.md
---

# 639 — HTTP package registry resolution

## Summary

File-based registry mock is implemented (#487). docs/module-resolution.md §5.4 documents network (HTTP) registries as planned follow-up. E0120 covers unreachable registry endpoints.

## Evidence source

docs/module-resolution.md §5.4, docs/compiler/error-codes.md E0120, src/compiler/loader/

## Primary paths

src/compiler/loader/, src/compiler/manifest.ark, docs/module-resolution.md, tests/fixtures/

## Non-goals

Operating a public registry infrastructure at registry.arukellt.dev

## Acceptance

- [ ] ark.toml [registry] url = "https://..." resolves packages over HTTP in test harness
- [ ] E0120 distinguishes network unreachable from package-not-found (E0121)
- [ ] At least one positive fixture (mock HTTP server) and one negative fixture (unreachable host)
- [ ] docs/module-resolution.md §10 Open Work table updated (#234/#235 marked done; HTTP registry tracked as #639)

## Required verification

```bash
python3 scripts/manager.py verify fixtures
python3 scripts/manager.py verify quick
```

## Close gate

HTTP registry fixture green; module-resolution §10 reflects current issue states.
