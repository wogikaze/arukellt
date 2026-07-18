---
Status: open
Created: 2026-07-15
Updated: 2026-07-16
ID: 820
Parent: 729
Track: compiler-internal
Depends on: "798, 816"
Related: "818, 821, 822, ADR-042, docs/plans/intrinsic-layer-separation"
Orchestration class: ready
Orchestration upstream: none
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

- [x] Eligibility is limited to compiler-shipped core/stdlib functions
- [x] Recursive and over-budget candidates remain normal calls
- [x] Effectful, trapping, and allocating bodies preserve their declared behavior
- [x] Positive and negative inliner fixtures pass
- [ ] `python3 scripts/manager.py verify quick` passes

## Evidence

- Eligibility is keyed by `FunctionId -> SignatureEntry -> CoreOpId` and requires
  `semantic_stdlib`, `normal_call`, a resolved non-public implementation symbol,
  and an inline hint.
- The bounded pass rejects recursive/call-containing bodies, non-local branches,
  more than 24 MIR instructions, and an estimated body size above 192 bytes.
- Calls that are not inlined are rewritten to the registered Ark fallback before
  Wasm emission, including at optimization level 0.
- `scripts/tests/test_stdlib_inline.py` verifies the `math.abs` Ark body at O0
  and O1: both validate and return `7` for `probe(-7)`; O0 retains the fallback
  call and O1 removes it.
- The bootstrap overlay keeps the pre-existing LICM/GC isolation while shipping
  only the bounded stdlib inliner and normal-call resolver.

## References

- `issues/open/729-intrinsic-layer-separation.md`
- `issues/done/816-prelude-compilation-restoration.md`
- `issues/open/818-core-op-production-scaffold-exit.md`
- `docs/adr/ADR-042-intrinsic-layer-separation.md`
