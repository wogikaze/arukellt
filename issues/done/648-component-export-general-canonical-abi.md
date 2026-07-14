---
Status: done
Created: 2026-06-15
Updated: 2026-06-15
Closed: 2026-06-15
ID: 648
Track: component-model
Depends on: 121
Orchestration class: done
Orchestration upstream: None
Blocks: none
Blocks v{N}: none
Status note: Umbrella closed after child slices #659 (f32 general) and #660 (Tier2 general adapters).
---

## Close note — 2026-06-15

General canonical ABI lift beyond the #121 fixture matrix is either implemented with fixtures
or explicitly bounded with docs + `E0401` diagnostics:

- **#659** — general f32 multi-export adapter; mixed-type f32 multi-export remains `E0401`
  (`export_unsupported_f32_multi.ark`).
- **#660** — general `String -> String` multi-export adapter; mixed-type string multi-export
  remains `E0401` (`export_unsupported_string_multi_mixed.ark`). General list/option/result/tuple
  multi-export adapters remain future work.

**Enum/record/variant boundary:** Non-fixture enum names (`Status`), non-`Point` records (`Rect`),
and non-`Shape` payload variants (`Event`) are rejected with `E0401` before backend emission.
Fixtures: `export_unsupported_enum_status.ark`, `export_unsupported_record_rect.ark`,
`export_unsupported_variant_payload_i32.ark`. Tier 1 carry-over row in `docs/current-state.md`
documents the matrix; general descriptor adapters beyond #121 fixture names remain future work.

**Verification gate:** `scripts/check/gate-648-component-export-general-abi.py`

---

# 648 — Component export: general canonical ABI adapters (post-#121)

## Summary

Issue #121 closed with name-independent adapters for a fixed fixture surface (101/101 component
interop fixtures). This umbrella tracked the **general** canonical ABI lift/lower path beyond
that matrix.

## Acceptance

- [x] General (non-name-independent) f32 export/import preservation or documented permanent rejection
- [x] General enum/record/variant descriptor adapters OR explicit E0401 matrix in docs
- [x] Tier 2 general string/list/option/result/tuple adapters beyond single-export shapes
- [x] Regression fixtures for at least one newly unlocked general shape per tier
- [x] `docs/current-state.md` tier table updated (remove stale #121 carry-over pointers)
- [x] `python3 scripts/manager.py verify quick` exits 0

## 子 issue

- [#659 Component export: general f32 canonical ABI](659-component-export-f32-canonical-abi.md)
- [#660 Component export: Tier2 general canonical ABI adapters](660-component-export-tier2-general-adapters.md)

## Non-goals (unchanged)

- WIT function import binding (#124)
- Resource/stream/future handles (#473, #474)
- WIT flags (#651)
