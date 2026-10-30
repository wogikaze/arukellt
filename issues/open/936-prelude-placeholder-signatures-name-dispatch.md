---
Status: open
Created: 2026-10-27
Updated: 2026-10-30
ID: 936
Track: stdlib
Depends on: "#934"
Orchestration class: blocked-by-upstream
Orchestration upstream: "#934"
Blocks v{N}: none
Priority: 0
Source: "P0 design/implementation audit (2026-10-27)"
Related: "ADR-042, ADR-046, RFC-005, #917, #928, #933, #935"
---

# 936 — prelude core functions have placeholder signatures; checker and emitter dispatch them by name

## Summary

Core collection / string functions in `std/prelude.ark` declare signatures
that do not describe their real contract, and their bodies are
`panic(...)`. The comment says "polymorphic in practice; emitter dispatches
by name". The typechecker special-cases these names and skips argument
checking; the emitter recognizes the names and emits intrinsics. As a
result, ill-typed calls are accepted and become memory reinterpretation on
`wasm32` and invalid Wasm on `wasm32-gc`.

## Evidence (2026-10-27)

- `std/prelude.ark` L663–700: `pub fn len(s: String) -> i32`,
  `pub fn push(v: Vec<i32>, val: i32)`, `pub fn get(v: Vec<i32>, idx: i32)`,
  `pub fn is_empty(v: Vec<i32>)`, each with a `panic(String_new())` body.
  L79–84: `pub fn clone(s: String) -> String` is used for every type (#928).
- `src/compiler/typechecker/builtins.ark` `builtin_call_return_type`
  returns `i32` for any `len(...)` and `TY_UNKNOWN` for unknown names; the
  arguments are not checked.

Repro 1 (`ty_push.ark`):

```ark
use std::host::stdio

fn main() {
    let v = Vec::new<i32>()
    push(v, String_from("oops"))
    stdio::println(len(v))
    let x: i32 = get_unchecked(v, 0)
    stdio::println(x)
}
```

Repro 2 (`ty_len.ark`):

```ark
use std::host::stdio

fn main() {
    let n = 42
    stdio::println(len(n))
}
```

Observed with `.build/selfhost/arukellt-s2-runtime.wasm`
(sha256 `1e11e8e7f2c5…`):

- `wasm32`: both compile; `wasmtime run` prints `1` / `140` (the string
  pointer read as `i32`) and `0`.
- `wasm32-gc`: both report "compiler produced invalid core Wasm".

## Expected

`push(Vec<i32>, String)` and `len(i32)` are compile-time type errors. Core
operations have real signatures (generic functions or trait methods per
ADR-046), and backend lowering is bound to them through FunctionId →
CoreOp (ADR-042), not through the call-site spelling.

## Acceptance

- [ ] No prelude function declares a placeholder signature; the remaining
      `panic` placeholder bodies are replaced by real bodies or by sealed
      CoreOp bindings.
- [ ] The typechecker has no name-keyed return-type or argument exemptions
      for these operations (`builtins.ark`, `call_method.ark`,
      `vec_core_mono.ark`, `hashmap_methods.ark` name lists removed or made
      signature-driven).
- [ ] The emitter selects intrinsics through the resolved FunctionId / CoreOp,
      not by callee-name prefix (`starts_with(callee, "push__")` etc.).
- [x] Compile-error fixtures for both repros pass on `wasm32` and
      `wasm32-gc`.
- [ ] #928 (`clone(Vec<T>)` on `wasm32-gc`) is fixed by the same change.

## Update (2026-10-30, `wave/issue-936`) — signature-driven Vec free checks

Landed the first increment: the stable Vec prelude frees no longer take an
unchecked, name-only path in the typechecker.

- `std/prelude.ark`: `push<T>(mut Vec<T>, T)`, `get<T>(Vec<T>, i32) -> Option<T>`,
  and `pop<T>(mut Vec<T>) -> Option<T>` now declare the contract the manifest
  already records (`Vec<T>` + element type), so `push(Vec<i32>, String)` is
  E0200 at the call instead of memory reinterpretation/invalid Wasm.
  `get_unchecked<T>` / `set<T>` already had real generic signatures.
  Generic argument inference also adapts an unsuffixed integer literal to an
  already-bound `i64` parameter (`push(Vec<i64>, 10)`), so the stricter
  signatures do not reject valid literal calls; typed `i32`/`i64` values still
  follow ADR-067.
- `src/compiler/typechecker/call_fn.ark`: `len` is the one stable free with
  two owners (`String`, `Vec<T>`). Its declaration must keep the String
  spelling until the committed pin is refreshed (the pin compiles the source
  tree and would reject `len(String)` against a Vec-only declaration), so the
  checker selects the receiver's owner contract: `String` and `Vec<T>` return
  `i32`, any other concrete first argument is E0200
  (`argument 1 has type i32, expected String or Vec<T>`). The
  `builtin_call_return_type` `len` → `i32` name exemption is removed.
- Fixtures: `diagnostics/prelude_vec_push_arg_type.ark` and
  `diagnostics/prelude_len_arg_type.ark` (plus `_wasm32` companions with
  `--target wasm32 --wasi-version wasi-p1`) assert the two repros fail on both
  targets; `stdlib_vec/prelude_generic_signatures.ark` runs `len` on `Vec` and
  `String` plus generic `push`/`get`/`pop` on both targets.

## Update (2026-10-30, `wave/issue-936-2`) — `len` owner must be decided by the end of the body

The first increment still accepted a `len` receiver whose type was a type
variable, because the variable might bind to `String` or `Vec<T>` later in the
body. That also let an unconstrained `T` reach lowering, where bare `len` is
bound to `string.len` by name, instead of being rejected.

- `src/compiler/typechecker/call_fn.ark`: a `len` call on a type variable now
  records the receiver on `TypeEnv.deferred_len_receiver_types` instead of
  accepting it. `finalize_deferred_len_receivers` resolves each receiver after
  the body has run: `String` / `Vec<T>` pass (a later binding may still supply
  the owner), a still-unresolved variable is E0200
  ("cannot infer the type of the `len` receiver: argument 1 must be String or
  Vec<T>"), and a variable that resolved to another concrete type reuses the
  call-site mismatch message. `TY_UNKNOWN` receivers stay quiet because they
  are error-recovery values; the general unknown-type rule stays with #939.
- `src/compiler/typechecker/body.ark`: function and test bodies finalize the
  deferred receivers right after statement checking. The return-contract pass
  re-infers the body afterwards; those additional entries are discarded with
  the per-function env.
- Fixtures: `diagnostics/prelude_len_unconstrained_receiver.ark` and its
  `_wasm32` companion assert the unconstrained-`T` case on both targets.
  `stdlib_vec/prelude_generic_signatures.ark` now also runs `len` inside a
  generic `Vec<T>` helper for `Vec<i32>` and `Vec<String>`.
- The deferred-accept path (a fresh receiver variable that a later call binds
  to `String`) has no run fixture: a `len` inside an unannotated closure still
  emits invalid Wasm because lowering re-infers the parameter as an integer
  and dispatches `string.len` by name. The pinned compiler fails the same way,
  so this is the existing #933/#934 lowering gap, not a regression from the
  checker finalizer.

Remaining acceptance work (updated):

- The emitter still selects the intrinsic through the callee-name tables.
  Bare `len` is bound to `string.len` in `data/core-ops.toml`, and MIR rewrites
  it to `raw_array_len` only from the first argument's local type; `push` /
  `get` / `pop` still lower through `call_rewrite_vec*` name/predicate
  dispatch. Binding the call to the resolved `FunctionId` → `CoreOpId` needs
  the #933 handoff.
- `len`, `push`, `get`, and `pop` still have `Std::panic`/placeholder bodies.
  They are deferred signature-only decls, not sealed CoreOp bindings.
- The typechecker still keeps name-keyed tables for concrete prelude
  conversions (`builtins::is_concrete_prelude_builtin`,
  `builtin_call_return_type`) and for `eq`/`assert_eq_i64`; those are outside
  this increment's Vec-operation scope.
- #928 is not changed by this diff; it remains tracked separately.

## Notes

#917 is converting stdlib free functions to methods; this issue owns the
type-contract part (signatures must be true) so that #935 can be enabled.
