---
Status: open
Created: 2026-07-15
Updated: 2026-07-15
ID: 822
Parent: 729
Track: stdlib
Depends on: "798, 816, 817, 820"
Related: "709, 718, 818, 821, ADR-036, ADR-042, docs/plans/intrinsic-layer-separation"
Orchestration class: blocked
Orchestration upstream: "816, 817, 820"
Blocks v{N}: none
Priority: 2
Source: ADR-042 representation-dependent stdlib migration ownership split
---

# 822 — Representation-dependent and allocating stdlib migration

## Summary

Move Vec/String and other allocation-dependent operations from emitter handlers
to Ark stdlib bodies built on the sealed raw API delivered by #817.

## Scope

- Migrate split/join/replace/repeat/padding/lines, Vec mutation and search,
  HashMap/HashSet, and numeric parse/format operation families assigned by the plan.
- Access Vec/String representation only through the sealed raw API.
- Preserve allocation, trap, ordering, and mutation effects declared by CoreOp metadata.
- Add fallback-versus-legacy differential tests for every migrated CoreOp.

## Non-goals

- Do not expose raw representation APIs to general user code.
- Do not implement runtime/WIT host lowering.
- Do not redesign the sealed raw API selected by #817.

## Acceptance

- [ ] Assigned representation-dependent CoreOps have Ark implementation symbols and production lowerings
- [ ] No assigned operation retains a `legacy_emitter` lowering
- [ ] Vec/String representation access is confined to the sealed raw API
- [ ] Allocation, mutation, trap, and ordering effects match CoreOp metadata
- [ ] Differential tests pass for every migrated CoreOp
- [ ] `python3 scripts/manager.py verify quick` passes

## References

- `issues/open/729-intrinsic-layer-separation.md`
- `issues/open/817-sealed-raw-api-module.md`
- `issues/open/818-core-op-production-scaffold-exit.md`
- `issues/open/820-stdlib-only-inliner.md`
- `data/core-ops.toml`
- `docs/adr/ADR-042-intrinsic-layer-separation.md`
