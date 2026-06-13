---
Status: done
Created: 2026-06-12
Updated: 2026-06-12
ID: 644
Track: docs/ops
Depends on: none
Orchestration class: implementation-ready
Blocks v1 exit: none
Source: docs-to-issues audit — docs/process/docs-gap-inventory-2026-06-12.md
---

# 644 — Docs anchor fragment link-check (ADR-019 v2)

## Summary

ADR-019 defers anchor fragment checking to v2. Internal docs links with #fragments are not validated by current harness.

## Evidence source

docs/adr/ADR-019-anchor-permalink-policy.md §3.2

## Primary paths

scripts/check/, docs/adr/ADR-019-anchor-permalink-policy.md

## Non-goals

External URL checking, stub file policy changes

## Acceptance

- [x] CI or verify check validates internal markdown anchor fragments resolve
- [x] ADR-019 §3.2 updated from deferred to implemented with harness reference
- [x] False positive rate acceptable (allowlist for generated docs if needed)

## Required verification

```bash
python3 scripts/manager.py verify quick
```

## Close gate

Anchor check in verify quick; ADR-019 reflects v2 delivery.
