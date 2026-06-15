---
Status: open
Created: 2026-06-15
Updated: 2026-06-15
ID: 648
Track: component-model
Depends on: 121, 074
Orchestration class: implementation-ready
Orchestration upstream: None
Blocks: 659
Blocks v{N}: none
Status note: Umbrella; dispatch child slices #659 (f32 general), #660 (Tier2 general adapters).
Source: docs/current-state.md Known v2 carry-over limitations (Tier 1/2 partial adapters post-#121)
---

# 648 — Component export: general canonical ABI adapters (post-#121)

## Summary

Issue #121 closed with name-independent adapters for a fixed fixture surface (101/101 component
interop fixtures). `docs/current-state.md` still documents partial Tier 1/2 coverage:

- f32: bit-reinterpret adapters for narrow shapes; broader f32 preservation remains open
- enum/record/variant: name-independent adapters for specific export shapes; general
  descriptors/adapters remain open
- Tier 2 string/list/option/result/tuple: many single-export adapters exist; general
  adapters still reject with E0401

This issue tracks the **general** canonical ABI lift/lower path beyond the #121 fixture
matrix — not another round of one-off name-independent adapters.

## Evidence

- `docs/current-state.md` Component export type tiers table references #121 for carry-over
- `issues/done/121-wasi-p2-canonical-abi-hardening.md` Close note: general first-class
  f32 / string / list / option / result lowering deferred

## Non-goals

- WIT function import binding (#124)
- Resource/stream/future handles (#473, #474)
- WIT flags (#651)

## Acceptance

- [x] General (non-name-independent) f32 export/import preservation or documented permanent rejection
- [ ] General enum/record/variant descriptor adapters OR explicit E0401 matrix in docs
- [x] Tier 2 general string/list/option/result/tuple adapters beyond single-export shapes
- [x] Regression fixtures for at least one newly unlocked general shape per tier
- [x] `docs/current-state.md` tier table updated (remove stale #121 carry-over pointers)
- [ ] `python3 scripts/manager.py verify quick` exits 0

## Required verification

```bash
python3 scripts/manager.py verify quick
python3 scripts/check/check-docs-consistency.py
```

## Close gate

General canonical ABI coverage is either implemented with fixtures or explicitly bounded
with docs + diagnostics; current-state no longer points at closed #121 for open work.
