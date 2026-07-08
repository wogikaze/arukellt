---
Status: done
Created: 2026-06-26
Updated: 2026-07-08
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
- [~] Downstream trait surfaces (#689, #691, #708, #710) remain open.
      **Updated 2026-07-08**:
      - #689 (operator overload): **done** — all fixtures pass T1 execution.
      - #691 (Iterator trait): open (depends on #688, #707).
      - #708 (TryFrom/TryInto): open (depends on #692, #707).
      - #710 (linear collections): open (depends on #691, #697, #701, #707, #709).
      #707's core implementation is complete; downstream issues are tracked
      independently and no longer blocked by #707.

## Fixture verification (2026-07-08)

- `tests/fixtures/stdlib_trait/self_return_add.ark` — **pass** T1 execution.
  `fn add_any<T: Add>(x: T, y: T) -> T { x.add(y) }` works for i32 and f64.
- `tests/fixtures/stdlib_trait/self_return_clone.ark` — **partial**.
  i32 clone works (output "42"), but String clone traps with `unreachable`
  instruction. Root cause: `impl Clone for String` in `std/core/clone.ark`
  calls `clone(self)` which dispatches to `Clone::clone` (infinite recursion)
  instead of the prelude `clone` free function. Attempted fix to call
  `__intrinsic_string_clone` directly caused T3 validate-fail regression
  (389→388 pass), so the fix was reverted. The String clone via trait
  dispatch bug is a separate issue (trait dispatch for GC ref types).
  No `.expected` file exists for this fixture.
