---
Status: done
Created: 2026-06-12
Updated: 2026-06-14
ID: 636
Track: selfhost-frontend
Depends on: 566
Orchestration class: implementation-ready
Blocks v1 exit: none
Source: docs-to-issues audit — docs/process/docs-gap-inventory-2026-06-12.md
---

# 636 — Selfhost structured diagnostic parity (codes, spans, warnings)

## Close note — 2026-06-14

Structured diagnostic text rendering (`error[Exxxx|phase]:` plus arrow file:line:col spans),
resolve/typecheck/parse `Diagnostic` emission, `W0007` unused-binding warnings, and
expanded diag-parity goldens landed. Gate: `diag-parity PASS=27 FAIL=0`, `verify quick` green.

## Summary

Selfhost compiler diagnostics now carry structured error codes, line/column spans, and
warning severity per `docs/compiler/diagnostic-parity.md`.

## Acceptance

- [x] Representative diagnostics fixtures emit error codes (EXXXX) from selfhost compiler
- [x] Primary span line numbers present in selfhost diagnostic output for representative fixtures
- [x] Warning severity distinguishable from error in selfhost output (`warning[W0007|typecheck]:`)
- [x] python scripts/manager.py selfhost diag-parity passes with expanded structured checks
- [x] docs/compiler/diagnostic-parity.md case table updated (❌ → ✅ where parity achieved)

## Required verification

```bash
python3 scripts/manager.py selfhost diag-parity  # PASS=27 FAIL=0
python3 scripts/manager.py verify quick          # 156/156
```
