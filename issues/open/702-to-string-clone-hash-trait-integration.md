---
Status: open
Created: 2026-06-27
Updated: 2026-10-25
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
  is now the resolver-first Display dispatch (5875ce384): free
  `to_string(x)` resolves to the registered `Type::to_string` impl, the same
  path as `x.to_string()`. The builtin intrinsic fallbacks remain only for
  unresolved type variables / fn indexes without the impl.
- `Display` / `Clone` / `Hash` traits and impls live in
  `std/core/convert.ark` / `std/core/hash.ark`; the scalar `to_string`,
  `clone`, `hash` entry points resolve to those impls.
- Trait method dispatch (#688) works inside generic functions
  (`<T: Display> x.to_string()`).
- Free `clone(x)` / `hash(x)` normalize the `string`/`String` receiver
  spelling and resolve to the registered `String::clone` / `String::hash`
  impl (fdd8983e4); the old `hash_i32`/`hash_string` builtin mapping and the
  `i32_to_string` free conversions were removed per ADR-036/046.
- The `to_string(String)` drift bug is covered by the display_dispatch and
  to_string_* fixtures.

## Required work

### Compiler

- [x] `mir_rewrite_to_string` is reduced to the resolver-first Display
      dispatch (5875ce384); no type-inspection-only path remains for the
      concrete impl cases.
- [x] MIR lowering: free `to_string(x)` resolves to `Display::to_string` via
      the registered impl (the same path as `x.to_string()`).
- [x] `to_string(String_from("hello"))` dispatches to the `String` identity
      impl (display_dispatch / to_string_i32 fixtures).

### Stdlib

- [x] The `i32_to_string` / `f64_to_string` / `bool_to_string` /
      `char_to_string` free conversions were removed per ADR-036; `to_string`
      resolves to the Display impls instead of a thin wrapper.
- [x] `hash_i32` was removed per ADR-036/046; free `hash(x)` and `x.hash()`
      resolve to the `Hash` impl (hash_dispatch fixture).
- [x] Free `clone(String)` is the prelude signature shim delegating to
      `s.clone()`; the Clone impl is the dispatch target.

### Fixtures

- [x] `tests/fixtures/stdlib_core/to_string_i32.ark` covers the
      `to_string(String_from("hello"))` drift case.
- [x] `tests/fixtures/trait/display_dispatch.ark` covers free/method parity.
- [x] `tests/fixtures/stdlib_trait/hash_dispatch.ark` and
      `clone_dispatch.ark` cover free/method/generic parity.
- [ ] `python3 scripts/manager.py verify quick` exits 0 (pre-existing
      unrelated manifest-completeness failure at the base; see Remaining).

## Acceptance

- [x] `mir_rewrite_to_string` is reduced to a thin resolver alias.
- [x] `to_string(x)` and `x.to_string()` share the trait dispatch path.
- [x] `to_string(String_from("hello"))` returns `"hello"`.
- [x] The removed free conversions (`i32_to_string`, `hash_i32`) are not
      required; the remaining free `to_string` / `clone` / `hash` resolve to
      their trait impls.
- [ ] `python3 scripts/manager.py verify quick` exits 0.

## Remaining

- `verify quick` has a pre-existing manifest-completeness failure
  (`tests/fixtures/modules/bare_use_root_fallback/sub/{helper,user}.ark`)
  unrelated to this issue; fix it before closing.
- `stdlib_core/hash_i64.ark` traps on an i64-literal `.hash()` receiver
  (pre-existing #700-class literal-receiver bug, compiles/validates).

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
