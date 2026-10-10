---
Status: open
Created: 2026-10-10
Updated: 2026-10-10
ID: 914
Track: compiler-internal
Depends on: none
Orchestration class: implementation-ready
Orchestration upstream: none
Blocks v{N}: language-v2
Priority: 1
Source: 911 lane probe bisection 2026-10-10 - minimal user-side repro of the prelude-documented same-named-generic mis-lower
---

# 914 - A generic user function colliding with an imported concrete function of the same name mis-binds resolve and mono instances

## Summary

`std::core::identity(x: i32)` is a public concrete function. When a user
program `use`s any module that pulls `std::core`'s root value namespace
into scope (`use std::core`, `use std::core::cmp`, or transitively
`use std::test`) and also defines a generic `fn identity<T>(x: T) -> T`,
the call `identity(String_from("ok"))` is mis-bound against the i32
signature. Two deterministic manifestations, same collision:

1. With a type-annotated `let`, typecheck rejects the program:
   `error[E0200|typecheck]: type mismatch: i32 vs String` (both targets).
2. Without an annotation, typecheck accepts the program, but wasm32-gc
   emit produces a deterministically invalid module
   (`expected i64, found (ref null $type)` in `main`): the String
   instantiation is emitted with the i32 instance's storage/binding.
   wasm32 (linear) output validates and runs correctly, so the mis-bind
   is silent on the fallback target for this shape.

The same bug class is already worked around in-tree: `std/prelude.ark`
documents that "a generic named exactly `assert_eq` mis-lowers scalar
monomorphizations", and `std/test/mod.ark` renamed its generic to
`assert_eq_debug` for that reason. This issue is the underlying defect;
the workarounds are removable once fixed.

## Minimal reproduction

Typecheck manifestation (fails on both wasm32 and wasm32-gc):

```ark
use std::core

fn identity<T>(x: T) -> T {
    x
}

fn main() {
    let b: String = identity(String_from("ok"))
}
```

Emit manifestation (typechecks, but invalid wasm32-gc):

```ark
use std::test

fn identity<T>(x: T) -> T {
    x
}

fn main() {
    let a = identity(7)
    let b = identity(String_from("ok"))
}
```

`wasm-tools validate` on the gc output:
`type mismatch: expected i64, found (ref null $type)` (function `main`),
byte-identical across repeated emits (md5-stable, not flaky).

## Control matrix (all under the same s2 runtime, ~12s per probe)

| shape | use | let | result |
|-------|-----|-----|--------|
| annotated, single `String` call | `std::test` / `std::core` / `std::core::cmp` | `let b: String` | E0200 i32 vs String |
| unannotated, dual i32+String | `std::test` | `let a`, `let b` | typecheck OK; gc emit invalid; linear valid + runs |
| annotated, dual i32+String | `std::host::stdio` | annotated | valid on both targets |
| unannotated, dual i32+String | `std::host::stdio` | unannotated | valid on both targets |
| annotated | `std::text` / `std::core::convert` / `std::prelude::{contains}` | annotated | valid |

So the trigger is the presence of `std::core::identity` (an i32
concrete fn) in the resolved value namespace, and the annotated-let path
also resolves against it while the unannotated path resolves the user
generic correctly but downstream mono/emit re-binds by name to the i32
instance.

## Root-cause hypothesis

Function lookup and/or mono-instance registration is keyed by
unqualified name somewhere in the resolve -> mono pipeline: an imported
concrete `identity` collides with a local generic `identity<T>`. At
typecheck the annotated-let expected-type path picks the imported i32
signature; at emit the `identity__String` call site binds the
`__i32` instance's body/storage. Either resolution must make local
definitions win over imports, or instance keys must be fully qualified
(module + name + type args).

## Acceptance

- Both minimal repros compile on wasm32 and wasm32-gc; gc output passes
  `wasm-tools validate`; linear output runs to exit 0 with asserts.
- Regression fixture in the dual-instantiation family covering a user
  generic whose name collides with an imported std function.
- The prelude `assert_eq` warning and the `assert_eq_debug` rename
  rationale are revisited; if the generic `assert_eq` works post-fix,
  a follow-up may restore the name (tracked separately).

## Related

- #908 - the same mis-bind class surfaced through std::seq wrappers
  (fixed for that shape; this is the unresolved general case).
- #912 - monomorphization demand-closure boundary; fully-qualified
  instance identity is part of that design.
- #913 - wasm32-gc corrupt string-table record; a different emit
  corruption shape (flaky, unlike this deterministic one).
