---
Status: open
Created: 2026-06-29
Updated: 2026-06-29
ID: 708
Track: stdlib-api
Depends on: "692, 707"
Orchestration class: blocked-by-upstream
Orchestration upstream: "#692 From/Into traits, #707 Self return type support"
Blocks v{N}: none
Priority: 2
Source: Issue #692 deferral — TryFrom/TryInto deferred pending Self + Result return type resolution
---

# 708 — `TryFrom` / `TryInto` traits for fallible conversions

## Summary

Issue #692 implemented `Clone`, `Default`, `From`, and `Into` traits for
scalar types. `TryFrom` and `TryInto` were explicitly deferred because their
method signatures return `Result<T, E>` — a generic enum type that the
trait method dispatch typechecker cannot yet resolve in the return position
of a trait method.

`Result<T, E>` already exists in the prelude (`std/prelude.ark`) with `Ok`
and `Err` constructors. The blocker is not the `Result` type itself but the
typechecker's ability to resolve a generic enum type as a trait method
return type during trait dispatch. This is closely related to #707 (Self
return type support) — both issues require the typechecker to substitute
type variables in trait method return positions.

Rust's `TryFrom` / `TryInto` provide fallible conversions with error
types:

```rust
trait TryFrom<T> {
    type Error;
    fn try_from(value: T) -> Result<Self, Self::Error>;
}

trait TryInto<T> {
    type Error;
    fn try_into(self) -> Result<T, Self::Error>;
}
```

## Current state

- `Result<T, E>` and `Option<T>` exist in the prelude with constructors.
- `From<T>` and `Into<T>` traits implemented in #692 for numeric widening.
- `TryFrom` / `TryInto` not defined — #692 deferred them.
- Trait method dispatch (#688) supports methods with `self` parameter.
- **Blocker**: Trait method return types that are generic enums
  (`Result<T, E>`) are not correctly resolved during trait dispatch.
  The typechecker's `infer_trait_method_call` returns the raw
  `FnSig_return_type(sig)`, which for a `Result<T, E>` return would need
  the type arguments `T` and `E` substituted from the trait's type
  parameters — this substitution is not implemented.
- **Blocker**: `TryFrom::try_from` has no `self` parameter (it's an
  associated function, like `From::from` and `Default::default`). The
  current trait dispatch model only supports methods with `self`.
  This is tracked in #701 (associated function syntax).

## Rust baseline

```rust
// std::convert::TryFrom
trait TryFrom<T>: Sized {
    type Error;
    fn try_from(value: T) -> Result<Self, Self::Error>;
}

// std::convert::TryInto
trait TryInto<T>: Sized {
    type Error;
    fn try_into(self) -> Result<T, Self::Error>;
}

// Blanket impl: impl<T, U> TryInto<U> for T where U: TryFrom<T>
```

Standard library impls:
- `i32::try_from(s: &str)` → `Result<i32, ParseIntError>`
- `i64::try_from(x: i32)` → `Result<i64, Infallible>` (infallible widening)
- `i32::try_from(x: i64)` → `Result<i32, TryFromIntError>` (narrowing)
- `f64::try_from(x: i32)` → `Result<f64, Infallible>`

## Required work

### Prerequisites (blocked by other issues)

- [ ] **#707**: Self return type support — required for `try_from` to
      return `Result<Self, E>` where `Self` is the implementing type.
- [ ] **#701**: Associated function syntax — `TryFrom::try_from` has no
      `self` parameter; it's called as `i32::try_from("42")` or
      `TryFrom::try_from("42")`. The current trait dispatch only supports
      `self` methods.
- [ ] **Generic enum return type resolution**: The typechecker must
      substitute type arguments in trait method return types that are
      generic enums (`Result<T, E>`). This is an extension of #707's
      type variable substitution to nested generic positions.

### Stdlib

- [ ] Define `trait TryFrom<T> { fn try_from(v: T) -> Result<TryFrom, String> }`
      in `std/core/convert.ark`.
      *(Note: Arukellt has no associated types yet, so `Error` is fixed to
      `String` initially. Once associated types land, this can be
      generalized.)*
- [ ] Define `trait TryInto<T> { fn try_into(self: TryInto) -> Result<T, String> }`
      in `std/core/convert.ark`.
- [ ] Provide impls:
  - `impl TryFrom<String> for i32` — delegates to `parse_i32`
  - `impl TryFrom<String> for i64` — delegates to `parse_i64`
  - `impl TryFrom<String> for f64` — delegates to `parse_f64`
  - `impl TryFrom<i64> for i32` — narrowing with overflow check
  - `impl TryInto<i32> for String` — delegates to `TryFrom`
  - `impl TryInto<i32> for i64` — delegates to `TryFrom`

### Fixtures

- [ ] `tests/fixtures/stdlib_trait/try_from_parse.ark` —
      `let n: Result<i32, String> = "42".try_into()`,
      match on `Ok(n) => stdio::println(i32_to_string(n))`.
- [ ] `tests/fixtures/stdlib_trait/try_from_narrow.ark` —
      `let r: Result<i32, String> = 999999999999i64.try_into()`,
      match on `Err(e) => stdio::println(e)`.
- [ ] `python3 scripts/manager.py verify quick` exits 0.

## Acceptance

- [ ] `TryFrom` / `TryInto` traits defined with scalar impls.
- [ ] `"42".try_into()` returns `Ok(42)` as `Result<i32, String>`.
- [ ] `999999999999i64.try_into()` returns `Err` as `Result<i32, String>`.
- [ ] `python3 scripts/manager.py verify quick` exits 0.

## References

- Depends on: #692 (From/Into — base conversion traits),
  #707 (Self return type — `try_from` returns `Result<Self, E>`),
  #701 (associated function syntax — `try_from` has no `self`)
- Related: #690 (`?` operator — `?` desugars to `match { Ok(v) => v,
  Err(e) => return Err(From::from(e)) }`, which benefits from `TryFrom`),
  #694 (Error trait — unified error ecosystem uses `TryFrom` for
  error conversion)
- `std/core/convert.ark` — existing From/Into definitions
- `std/prelude.ark` — `Result<T, E>`, `parse_i32`, `parse_i64`, `parse_f64`
- `src/compiler/typechecker/call_method.ark` — trait method dispatch
