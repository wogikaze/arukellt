---
Status: open
Created: 2026-06-26
Updated: 2026-07-04
ID: 707
Track: language-design
Depends on: "688"
Orchestration class: design-required
Orchestration upstream: None
Blocks v{N}: "689, 691, 708, 710"
Priority: 1
Source: Trait dispatch parity — `Self` return types for trait methods
---

# 707 — `Self` return type support for trait method dispatch

## Summary

Trait methods that return `Self` (for example `Clone::clone`, builder
patterns, and associated-type-style APIs) require the compiler to resolve
the concrete implementing type at monomorphization / dispatch sites instead
of treating `Self` as an opaque placeholder.

## Current state

- ADR-040 signature registry / mono-instance spine records instantiated
  return types for trait dispatch and monomorphized calls.
- Fixture `tests/fixtures/stdlib_trait/self_return_clone.ark` exercises
  `fn dup<T: Clone>(x: T) -> T { x.clone() }`.
- Remaining open work is tracked by downstream issues #689, #691, #708,
  and #710 that depend on broader trait/stdlib surfaces.

## Acceptance

- [x] Compiler resolves `Self` return types for trait method dispatch.
- [x] Mono / generic call sites use signature registry return metadata.
- [ ] Downstream trait surfaces (#689, #691, #708, #710) remain open.
