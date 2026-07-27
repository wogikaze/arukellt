---
Status: done
Created: 2026-07-15
Updated: 2026-07-21
ID: 821
Parent: 729
Track: stdlib
Depends on: "798, 816, 820"
Related: "709, 718, 817, 818, 822, ADR-036, ADR-042, docs/plans/intrinsic-layer-separation"
Orchestration class: done
Orchestration upstream: "816, 820"
Blocks v{N}: none
Priority: 2
Source: ADR-042 pure stdlib migration ownership split
---

# 821 — Pure semantic stdlib migration

## Summary

Move non-allocating semantic operations from emitter handlers into Ark stdlib
bodies while retaining CoreOp meaning for optimization and validation.

## Scope

- Migrate pure scalar, range, search, comparison, fold, and ordering operations
  that do not require representation-specific allocation.
- Use trait/method/associated APIs as the public surface.
- Retain only true primitives and target-specific raw operations in the backend.
- Add fallback-versus-legacy differential tests using each CoreOp equivalence rule.

## Non-goals

- Do not migrate allocating or representation-dependent Vec/String operations.
- Do not implement runtime/WIT host lowering.

## Acceptance

- [x] The assigned pure CoreOps have Ark implementation symbols and `normal_call` lowering
- [x] No assigned operation retains a `legacy_emitter` lowering
- [x] Public bindings follow ADR-044/ADR-046 trait/method/associated forms
- [x] Differential tests pass for every migrated CoreOp
- [x] Issue-scoped verification green (`verify lane` + domain tests); full `verify quick` remains the merge/CI gate

## Migration progress

- Assigned pure set: `math.abs`, `math.min`, `math.max`, `math.clamp`,
  `math.gcd`, `math.pow_i32`, `core.range_contains`, and `core.range_len`.
  Each uses a non-public Ark implementation symbol, `normal_call` lowering,
  and a bounded inline hint. O0/O1 differential execution is covered by
  `scripts/tests/test_stdlib_inline.py`.
- The public canonical forms are the existing `i32` methods and the new
  `Range.contains` / `Range.len` methods. Free functions remain only as
  compatibility entry points tracked by #718.
- The remaining legacy-emitter inventory is intentionally not treated as one
  homogeneous pure set: Vec search/fold/sort and String/parse operations are
  representation-dependent and belong to #822; `sqrt` and f64 bit extraction
  are backend primitives; portable SIMD remains governed by ADR-037; host
  operations belong to #819.

## References

- `issues/open/729-intrinsic-layer-separation.md`
- `issues/open/818-core-op-production-scaffold-exit.md`
- `issues/open/820-stdlib-only-inliner.md`
- `data/core-ops.toml`
- `docs/adr/ADR-042-intrinsic-layer-separation.md`

## Close evidence (2026-07-21)

Assigned pure CoreOp migration acceptance items 1–4 were already `[x]`.
Re-verified via `scripts.tests.test_stdlib_inline` (3/3 OK) and `verify lane` PASS
after closing upstream #820 in the same wave. Moving to done.
