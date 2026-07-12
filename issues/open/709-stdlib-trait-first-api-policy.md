---
Status: open
Created: 2026-07-01
Updated: 2026-07-12
ID: 709
Track: stdlib-api
Depends on: "691, 695, 697, 703"
Orchestration class: blocked-by-upstream
Orchestration upstream: "#691 Iterator, #695 Ord, #697 Vec<T> operations, #703 monomorphic API cutover"
Blocks v{N}: none
Priority: 1
Source: Stdlib trait-first direction 2026-07-01 / ADR-046 eradication 2026-07-12
---

# 709 — Stdlib trait-first API policy and free-function eradication

## Summary

The stdlib public surface must be **trait-first / type-first**. User-reachable
free functions (`func(recv, …)`, monomorphic `*_i32`, prelude thin wrappers)
are **eradicated**, not kept as permanent bridges
([ADR-046](../../docs/adr/ADR-046-free-function-eradication.md)).

Allowed end states:

- reusable traits (`Iterator`, `Ord`, `Clone`, `Display`, `Read`, …)
- coherent modules and public structs/enums
- `impl` methods and associated functions (`v.push(x)`, `Vec::new()`)
- no-receiver globals as associated on namespace/handle types
  (`Env::args()`, `Process::exit(c)`, …)

Classification for every remaining free / monomorphic / prelude_wrapper
symbol (replaces the old “low-level allowed bridge” bucket):

| Class | Meaning |
|-------|---------|
| `must delete` | Public or user-reachable free; migrate then remove (ADR-014 if stable) |
| `associated-or-method` | Replacement is inherent / trait / associated (owner issue required) |
| `intrinsic-only` | Non-public `__intrinsic_*` / manifest `kind = "intrinsic"` only |

There is **no** lasting `allowed bridge` class for public or prelude wrappers.

## Current state

- Policy ACCEPTED: ADR-046 (2026-07-12). ADR-036 D5 (prelude thin wrappers
  as permanent) is withdrawn.
- `std::seq` and many prelude helpers still expose free / monomorphic APIs.
- Generated docs still teach concrete helper names over methods.
- Inventory execution: #718. Monomorphic cutover: #703.

## Required work

- [ ] Document the eradication policy in stdlib docs (link ADR-046).
- [ ] Inventory all public free / `*_i32` / `*_i64` / `*_f64` /
      representation-specific helpers in `std/manifest.toml`.
- [ ] Classify each as `must delete` / `associated-or-method` /
      `intrinsic-only` (no public bridge class).
- [ ] Link each replacement to an owning issue (#691, #695, #697, #702,
      #703, #718 tiers, …).
- [ ] Define a generated scorecard: public free count, `prelude_wrapper`
      count, remaining monomorphic helpers — **zero** is the goal for
      user-reachable free symbols.
- [ ] Update stdlib docs so high-level examples use method / associated /
      trait APIs only.

## Acceptance

- [ ] Stdlib docs state free-function eradication (ADR-046).
- [ ] All public free / monomorphic helpers have an explicit class and owner.
- [ ] Public docs no longer present `i32` helpers as the primary API.
- [ ] Scorecard tracks remaining helper count toward zero.
- [ ] #703 / #718 have enough inventory to delete or hide obsolete helpers.

## References

- ADR-046 (free-function eradication)
- ADR-044 (trait / method syntax)
- ADR-036 (D5 withdrawn; redesign still PROPOSED)
- #691, #695, #697, #702, #703, #718
- `std/manifest.toml`
- `std/seq/mod.ark`
- `std/collections/`
