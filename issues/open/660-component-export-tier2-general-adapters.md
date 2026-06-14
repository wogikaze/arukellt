---
Status: open
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

Umbrella: [#648 general canonical ABI](648-component-export-general-canonical-abi.md)

## Acceptance

- [ ] General Tier2 string/list/option/result/tuple adapters beyond single-export shapes
- [ ] General enum/record/variant descriptors OR explicit E0401 matrix in docs
- [ ] Regression fixture per newly unlocked tier
- [ ] `docs/current-state.md` tier table updated (remove stale #121 pointers)
- [ ] `python3 scripts/check/check-docs-consistency.py` passes
- [ ] `python3 scripts/manager.py verify quick` exits 0

## References

- `issues/open/648-component-export-general-canonical-abi.md`
- `issues/open/659-component-export-f32-canonical-abi.md`
- `src/compiler/component/`
