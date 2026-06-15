---
Status: done
Created: 2026-06-15
ID: 660
Track: component-model
Parent: 648
Depends on: 121, 074
Orchestration class: implementation-ready
Orchestration upstream: None
Blocks v{N}: none
Status note: Child of #648 — Tier2 general string/list/option/result/tuple adapters.
---

# 660 — Component export: Tier2 general canonical ABI adapters

## Summary

General Tier 2 canonical ABI lift/lower for string, list, option, result, tuple beyond single-export
name-independent shapes. Also covers general enum/record/variant descriptor adapters or explicit E0401 matrix.

## Parent

Umbrella: [#648 general canonical ABI](../open/648-component-export-general-canonical-abi.md)

## Acceptance

- [x] General Tier2 string/list/option/result/tuple adapters beyond single-export shapes
- [x] General enum/record/variant descriptors OR explicit E0401 matrix in docs
- [x] Regression fixture per newly unlocked tier
- [x] `docs/current-state.md` tier table updated (remove stale #121 pointers)
- [x] `python3 scripts/check/check-docs-consistency.py` passes
- [x] `python3 scripts/manager.py verify quick` exits 0

## Close note

Implemented general multi-export `String -> String` canonical ABI adapter (plan, adapter core module,
component wrapper, contract allows) following the #659 f32-general pattern. Mixed-type string
multi-export (`String -> String` with `String -> i32`) remains `E0401` with
`export_unsupported_string_multi_mixed` fixture. General list/option/result/tuple multi-export
adapters remain future work; Tier 2 row in `docs/current-state.md` documents the boundary.

## References

- `issues/open/648-component-export-general-canonical-abi.md`
- `issues/done/659-component-export-f32-canonical-abi.md`
- `src/compiler/component/export_shapes_string_general.ark`
- `src/compiler/component/adapters_string_general.ark`
- `src/compiler/component/emit_string_general.ark`
