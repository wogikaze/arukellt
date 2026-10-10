---
Status: done
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

## Emitted-module evidence (wasm-tools print)

Broken program (`use std::test`, unannotated dual instantiation), gc:

```
(func $identity (;19;) (type 12) (param $x i64) (result i64)
...  call $identity        // both call sites, i32 sig
```

The user's generic `identity<T>` and both expected instances
(`identity__i32`, `identity__String`) never materialize as wasm
functions at all. Control program (`use std::host::stdio`, no
`std::core::identity` import), gc - valid:

```
(func $identity__i32    (param i64) (result i64)
(func $identity__String (param (ref null 14)) (result (ref null 14))
...  call $identity__i32 / call $identity__String
```

So the user's decl/instances are dropped upstream of emit and the call
sites bind the first same-name MirFunction (std::core's i64-signature
`identity`). On linear this is a silent mis-bind that happens to return
correct results for `identity` specifically (i32 sig accepts the String
pointer and returns it unchanged).

## Root-cause hypothesis

Two name-keyed single-slot tables collide:

- `env.fn_sig_index` maps one sig per unqualified name
  (`build_fn_sig_index`, first-inserted wins). With `std::core` in
  scope, `identity` resolves to the imported i32 sig - the annotated
  `let` path checks against it and reports E0200.
- The MIR function / mono-index namespace keeps unqualified decl names,
  so the imported `identity` and the local generic `identity` share one
  key; the user's decl and its recorded `identity__i32`/`identity__String`
  instances are then never published into the emit function table, and
  `mir_resolve_duplicate_call_target`'s exact-name first-scan binds the
  call to whichever `identity` appears first in module order
  (std modules emit first).

Either resolution must make local definitions win over imports, or
function/instance keys must be fully qualified (module + name + type
args) end to end - the same qualified-identity redesign #912 calls for.

## Acceptance

- Both minimal repros compile on wasm32 and wasm32-gc; gc output passes
  `wasm-tools validate`; linear output runs to exit 0 with asserts.
- Regression fixture in the dual-instantiation family covering a user
  generic whose name collides with an imported std function.
- The prelude `assert_eq` warning and the `assert_eq_debug` rename
  rationale are revisited; if the generic `assert_eq` works post-fix,
  a follow-up may restore the name (tracked separately).

## Resolution

Two name-keyed single-slot tables were colliding; both fixed:

1. `build_fn_sig_index` (typechecker/entry.ark): imported decls are
   listed before entry-module decls and `name_index_insert_first` keeps
   the first writer, so `std::core::identity(i32)` permanently claimed
   the bare `identity` slot. Entry-module decls (empty qualified key)
   now register bare names first; imports still fill qualified keys and
   remaining bare slots.
2. Mono-variant ownership (mir/lower): `mono_index` groups instances by
   bare name, so every same-named decl claimed the entry generic's
   variants. Once the call sites minted `identity__i32`/`identity__String`
   again, the imported concrete decl registered/emitted them with its
   own i64 signature (param-i64/result-ref Frankenstein func). Variant
   publishing, signature-registry registration, and mono-body emission
   are now gated by `decl_is_generic` - a decl only owns instances when
   its typed signature still carries type variables
   (`fn_index_source::mir_fn_index_source_decl_is_generic`,
   `entry_fns_mono::mir_entry_decl_is_generic`).

Regression fixture: `tests/fixtures/functions/generic_shadow_import_name.ark`
(run: + t3-compile: manifest entries) covers annotated and unannotated
lets at i32 and String instantiations under `use std::test`.

Verified on a freshly built s2 (built post-fix):

- All six previously failing probe shapes (annotated/unannotated lets,
  `use std::test`/`std::core`/`std::core::cmp`) compile and
  `wasm-tools validate` on wasm32-gc; wasm32 compiles and runs to
  exit 0.
- `identity__i32` and `identity__String` now emit as separate funcs
  with correct signatures; call sites bind them by mangled name.
- Spot fixtures `generic_fn`, `generics/two_params`,
  `generics/vec_generic` compile + validate unchanged;
  `check-fresh-s2-smoke` 4/4 PASS.

Remaining scope (not this issue): the flat name-keyed tables still allow
two generic decls sharing a bare name to claim each other's instances,
and the no-arg-unification escape in `infer_concrete_call_expr` still
silently accepts arg mismatches when a concrete sig wins the slot.
Both belong to the qualified-identity redesign tracked by #912.

## Related

- #908 - the same mis-bind class surfaced through std::seq wrappers
  (fixed for that shape; this is the unresolved general case).
- #912 - monomorphization demand-closure boundary; fully-qualified
  instance identity is part of that design.
- #913 - wasm32-gc corrupt string-table record; a different emit
  corruption shape (flaky, unlike this deterministic one).
