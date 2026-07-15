---
Status: open
Created: 2026-07-15
Updated: 2026-07-15
ID: 820
Parent: 729
Track: compiler-internal
Depends on: "798, 816"
Related: "818, 821, 822, ADR-042, docs/plans/intrinsic-layer-separation"
Orchestration class: blocked
Orchestration upstream: "816"
Blocks v{N}: none
Priority: 2
Source: ADR-042 bounded stdlib inliner ownership split
---

# 820 — Bounded stdlib-only inliner

## Summary

Implement the limited inliner required for compiler-shipped Ark core/stdlib
fallbacks without creating a general user-code optimizer contract.

## Scope

- Inline only compiler-shipped core/stdlib bodies with a resolved CoreOp entry.
- Reject recursion and enforce MIR instruction and code-size budgets.
- Treat `inline.policy = "always"` as a strong hint, not a semantic guarantee.
- Preserve effect, trap, and allocation behavior across inlining.

## Non-goals

- Do not inline arbitrary user code.
- Do not migrate individual stdlib operation bodies in this issue.

## Acceptance

- [ ] Eligibility is limited to compiler-shipped core/stdlib functions
- [ ] Recursive and over-budget candidates remain normal calls
- [ ] Effectful, trapping, and allocating bodies preserve their declared behavior
- [ ] Positive and negative inliner fixtures pass
- [ ] `python3 scripts/manager.py verify quick` passes

## References

- `issues/open/729-intrinsic-layer-separation.md`
- `issues/open/816-prelude-compilation-restoration.md`
- `issues/open/818-core-op-production-scaffold-exit.md`
- `docs/adr/ADR-042-intrinsic-layer-separation.md`
