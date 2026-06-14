---
Status: open
Created: 2026-06-15
ID: 659
Track: component-model
Parent: 648
Depends on: 121, 074
Orchestration class: implementation-ready
Orchestration upstream: None
Blocks v{N}: none
Status note: Child of #648 — general f32 canonical ABI preservation slice.
---

# 659 — Component export: general f32 canonical ABI

## Summary

Implement or explicitly bound general (non-name-independent) f32 export/import canonical ABI adapters
beyond the #121 fixture matrix. Addresses `docs/current-state.md` f32 carry-over limitation.

## Parent

Umbrella: [#648 general canonical ABI](648-component-export-general-canonical-abi.md)

## Acceptance

- [ ] General f32 export/import preservation OR documented permanent E0401 rejection matrix
- [ ] Regression fixture for at least one newly unlocked f32 shape
- [ ] Diagnostics updated if rejection is intentional
- [ ] `docs/current-state.md` f32 tier row updated
- [ ] Component interop fixtures pass for f32 slice
- [ ] `python3 scripts/manager.py verify quick` exits 0

## References

- `issues/open/648-component-export-general-canonical-abi.md`
- `issues/done/121-wasi-p2-canonical-abi-hardening.md`
- `src/compiler/component/`
