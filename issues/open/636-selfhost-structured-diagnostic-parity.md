---
Status: open
Created: 2026-06-12
Updated: 2026-06-12
ID: 636
Track: selfhost-frontend
Depends on: 566
Orchestration class: implementation-ready
Blocks v1 exit: none
Source: docs-to-issues audit — docs/process/docs-gap-inventory-2026-06-12.md
---

# 636 — Selfhost structured diagnostic parity (codes, spans, warnings)

## Summary

Selfhost compiler diagnostics lack structured error codes, line/column spans, and warning severity that the Rust compiler and docs/compiler/diagnostic-parity.md contract require. CI diag-parity gate exists but structured field parity remains incomplete.

## Evidence source

docs/compiler/diagnostic-parity.md, src/compiler/diagnostics.ark, src/compiler/main.ark

## Primary paths

src/compiler/diagnostics.ark, src/compiler/main.ark, tests/fixtures/diagnostics/, scripts/manager.py

## Non-goals

Exact message text wording match between Rust and selfhost compilers

## Acceptance

- [ ] Representative diagnostics fixtures emit error codes (EXXXX) from selfhost compiler
- [ ] Primary span line numbers present in selfhost diagnostic output for representative fixtures
- [ ] Warning severity distinguishable from error in selfhost output
- [ ] python scripts/manager.py selfhost diag-parity passes with expanded structured checks
- [ ] docs/compiler/diagnostic-parity.md case table updated (❌ → ✅ where parity achieved)

## Required verification

```bash
python3 scripts/manager.py selfhost diag-parity
python3 scripts/manager.py verify quick
```

## Close gate

diag-parity gate green; diagnostic-parity.md reflects achieved parity; no stale ❌ rows for implemented cases.
