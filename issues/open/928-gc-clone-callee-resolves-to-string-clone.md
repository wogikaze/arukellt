---
Status: open
Created: 2026-10-19
Updated: 2026-10-19
ID: 928
Track: compiler
Depends on: none
Orchestration class: implementation-ready
Orchestration upstream: none
Blocks v{N}: none
Priority: 1
Source: "#922 fixture landing — new linear_gc_boundary_handles fixture failed the wasm32-gc fixture-parity path on pinned and current emitters"
Related: "#730, #922"
---

# 928 — bare `clone` callee resolves to `String::clone`; wasm32-gc emits invalid wasm for `clone(Vec<T>)`

## Summary

`clone` is a generic `clone(T) -> T` intrinsic. Under wasm32 (linear
GC) `clone(v)` on `v: Vec<String>` / `Vec<i32>` produces a correct
deep clone. Under wasm32-gc the emitted call goes to `String::clone`
`(param (ref null $string))` with the `Vec<T>` argument still on the
operand stack, so the module fails `wasm-tools validate`:

    error: func 22 failed to validate
    Caused by:
      type mismatch: expected (ref null $type), found (ref null $type)

The result-local type sync added for #730
(`src/compiler/mir/post_pass_callee_lookup.ark` — "Cache often maps
bare `clone` to String and poisons GC struct temps") fixes the
*local's* declared type, but the *callee* is still resolved through
the same poisoned table, so the call itself is emitted as
`String::clone`.

## Evidence (2026-10-19)

- Minimal repro (compiles and runs correctly under
  `--target wasm32`; invalid under `--target wasm32-gc`):

      use std::test
      use std::prelude::{clone, eprintln}

      pub fn main() {
          let mut v: Vec<String> = Vec::new<String>()
          push(v, String_from("hello"))
          let c = clone(v)
          test::assert_eq_i32(len(c), 1)
          eprintln(String_from("ok"))
      }

- Reproduces identically on the pinned bootstrap compiler
  (`bootstrap/arukellt-selfhost.wasm`, 2026-10-09) and on current
  s2-runtime — pre-existing bug, not a #922 regression.
- In the emitted wat the call is literally `call $String::clone`
  directly after `local.get` of the `Vec<T>` param, while the
  destination local is declared `(ref null $vec_struct)` — both the
  argument and the result are type-mismatched; the first mismatch
  traps validation.
- `clone` on `String` args works on both targets; the misdispatch only
  shows when the argument is a non-String managed ref.

## Expected

`clone(x)` on a non-String managed argument resolves to the generic
clone lowering (same lowering path the linear emitter reaches), or —
if generic clone is intentionally String-only — a compile-time
diagnostic. Emitting a call whose argument type cannot satisfy the
callee signature must never reach the backend.

## Acceptance

- [ ] `clone(Vec<String>)` / `clone(Vec<i32>)` under `--target wasm32-gc`
      emits a module that passes `wasm-tools validate` and runs with
      the same semantics as the wasm32 (linear) build.
- [ ] If the resolution is tightened to a diagnostic instead, a
      `compile-error`/`diag` fixture covers it and the wasm32-gc path
      no longer emits invalid wasm.
- [ ] Regression fixture exercising `clone` on a non-String managed
      arg under the gc target.
