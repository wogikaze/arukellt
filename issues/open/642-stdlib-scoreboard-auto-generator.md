---
Status: open
Created: 2026-06-12
Updated: 2026-06-12
ID: 642
Track: docs/ops
Depends on: none
Orchestration class: implementation-ready
Blocks v1 exit: none
Source: docs-to-issues audit — docs/process/docs-gap-inventory-2026-06-12.md
---

# 642 — Stdlib scoreboard auto-generator

## Summary

docs/stdlib/scoreboard.md and docs/directory-ownership.md state the module maturity scoreboard is hand-maintained; auto-generator is not yet implemented.

## Evidence source

docs/stdlib/scoreboard.md, docs/directory-ownership.md, docs/stdlib/README.md

## Primary paths

scripts/gen/, docs/stdlib/scoreboard.md, std/manifest.toml, tests/fixtures/manifest.txt

## Non-goals

Changing stdlib API surface, manifest schema redesign

## Acceptance

- [ ] Generator script under scripts/gen/ produces scoreboard data from manifest + fixture coverage
- [ ] docs/stdlib/scoreboard.md generated or semi-generated with documented regen command
- [ ] docs/directory-ownership.md updated (hand-maintained → generated)
- [ ] docs/stdlib/README.md #613 reference corrected (#613 is done in issues/done/)

## Required verification

```bash
python3 scripts/gen/generate-docs.py
python3 scripts/check/check-docs-consistency.py
```

## Close gate

Regen command documented; scoreboard reflects manifest; README #613 drift fixed.
