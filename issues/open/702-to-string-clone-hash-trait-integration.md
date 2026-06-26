---
Status: open
Created: 2026-06-27
Updated: 2026-06-27
ID: 702
Track: stdlib-api
Depends on: "688, 700, 692"
Orchestration class: blocked-by-upstream
Orchestration upstream: "#688 trait method dispatch, #700 builtin type method syntax, #692 Clone/Default/From/Into"
Blocks v{N}: none
Priority: 1
Source: Method-syntax-first stdlib direction 2026-06-27
---

# 702 — Integrate `to_string` / `clone` / `hash` builtins into trait dispatch

## Summary

Arukellt has `Display`, `Hash`, and (planned via #692) `Clone` trait
definitions with scalar impls, but the **compiler-handled builtin rewrites**
for `to_string`, `hash`, and `clone` bypass the trait dispatch path:

- `mir_rewrite_to_string` (`src/compiler/mir/lower/call_rewrite_string.ark`)
  inspects the first argument's type and rewrites `to_string(x)` to
  `i32_to_string` / `f64_to_string` / `bool_to_string` / `char_to_string`
  or identity for `String`.
- `hash_i32` / `clone` are free functions calling intrinsics directly.

This creates a **dual-dispatch inconsistency**: trait method calls
(`x.to_string()` inside `<T: Display>`) go through trait dispatch, but
free-function calls (`to_string(x)`) go through builtin rewrite. The two
paths can drift (as seen in the `to_string(String_from("hello"))` drift
fixed in `core_literals.ark`).

The goal is to **unify** these so that:

```ark
// After — single dispatch path via trait
42.to_string()         // Display::to_string(42)
String_from("hi").to_string()  // Display::to_string(s)
x.to_string()          // inside <T: Display> — same path

// Free functions become thin wrappers
pub fn i32_to_string(x: i32) -> String { x.to_string() }
pub fn to_string<T: Display>(x: T) -> String { x.to_string() }
```

## Current state

- `mir_rewrite_to_string` in `src/compiler/mir/lower/call_rewrite_string.ark`
  handles `to_string(x)` by type inspection of the first argument local.
- `Display` trait defined in `std/core/convert.ark` with scalar impls
  (`impl Display for i32 { fn to_string(self) -> String {
  __intrinsic_i32_to_string(self) } }`).
- Trait method dispatch (#688) works inside generic functions
  (`<T: Display> x.to_string()`).
- **Free-function `to_string(x)` does NOT go through trait dispatch** —
  it goes through `mir_rewrite_to_string`.
- `hash_i32` / `clone` are free functions, not trait-dispatched.
- The `to_string(String)` drift bug (returning pointer value `140`
  instead of `"hello"`) was caused by this dual-path inconsistency.

## Required work

### Compiler

- [ ] **Remove `mir_rewrite_to_string`** from
      `src/compiler/mir/lower/call_rewrite_string.ark` — replace with
      trait method dispatch lowering.
- [ ] **MIR lowering**: `to_string(x)` free-function call should resolve
      to `Display::to_string` trait dispatch (same path as `x.to_string()`).
      This may require:
      - (a) Making `to_string` a generic function `fn to_string<T: Display>(x: T) -> String { x.to_string() }` in prelude, OR
      - (b) Compiler-recognized lowering of `to_string(x)` to
            `Display::to_string(x)` directly.
      Approach (a) is preferred for uniformity.
- [ ] **Verify**: The `to_string(String_from("hello"))` case correctly
      dispatches to `Display::to_string` for `String` (identity impl).

### Stdlib

- [ ] Rewrite `i32_to_string` / `f64_to_string` / `bool_to_string` /
      `char_to_string` as thin wrappers delegating to `Display::to_string`:
      ```ark
      pub fn i32_to_string(x: i32) -> String { x.to_string() }
      ```
- [ ] Rewrite `hash_i32` as thin wrapper delegating to `Hash::hash`:
      ```ark
      pub fn hash_i32(x: i32) -> i32 { x.hash() }
      ```
- [ ] Rewrite `clone` (String) as thin wrapper delegating to `Clone::clone`
      (once #692 lands `Clone` trait).

### Fixtures

- [ ] Update `tests/fixtures/stdlib_core/to_string_i32.ark` to verify
      `to_string(String_from("hello"))` returns `"hello"` (regression
      test for the drift bug).
- [ ] `tests/fixtures/trait/display_dispatch.ark` —
      `42.to_string()`, `String_from("hi").to_string()`,
      `to_string(42)` all return correct values via same dispatch path.
- [ ] `tests/fixtures/trait/hash_dispatch.ark` —
      `42.hash()`, `hash_i32(42)` return same value.
- [ ] `python3 scripts/manager.py verify quick` exits 0.

## Acceptance

- [ ] `mir_rewrite_to_string` removed or reduced to a thin alias.
- [ ] `to_string(x)` and `x.to_string()` go through the same trait
      dispatch path.
- [ ] `to_string(String_from("hello"))` returns `"hello"` (no drift).
- [ ] `i32_to_string` / `hash_i32` / `clone` are thin wrappers over
      trait impls.
- [ ] `python3 scripts/manager.py verify quick` exits 0.

## References

- Depends on: #688 (trait method dispatch), #700 (builtin type method
  syntax — `42.to_string()` requires `impl i32`),
  #692 (Clone trait — for `clone` wrapper)
- Related: #689 (operator overload — same unification pattern for
  operators)
- `src/compiler/mir/lower/call_rewrite_string.ark`
- `src/compiler/mir/lower/core_literals.ark` (site of the drift fix)
- `std/core/convert.ark`, `std/core/hash.ark`, `std/prelude.ark`
- ADR-036 D5 (prelude thin wrapper化)
