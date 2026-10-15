---
Status: done
Created: 2026-10-15
Updated: 2026-10-15
ID: 926
Track: compiler
Depends on: ""
Related: "#924, #922, #825"
Orchestration class: implementation
Blocks v4 exit: False
---

# Typechecker accepts calls of non-function values (silent unknown)

## Summary

Calling a value whose type is known and is not a function — e.g.
`let x = a(b)` with `a: i32` — compiles without any diagnostic on
both wasm32-gc and wasm32.  The emitter then produces garbage: on
wasm32 the observed shape was `local.get a; drop; drop; unreachable`
plus dead code, which either traps at runtime or fails wasm validation
(operand-stack underflow).

## Evidence

- Minimal repro (compiles cleanly today):
      fn main() {
          let a = 3
          let b = 4
          let x = a(b)
      }
  Verified 2026-10-15 with the s2 selfhost compiler on both targets:
  "compilation succeeded" — no diagnostic.
- Code path: `src/compiler/typechecker/call_fn.ark` — when the callee
  is an ident that is not function-typed in scope and has no signature
  in `fn_sigs`, inference falls through to
  `call_unknown::infer_unknown_ident_call`, which returns an unknown
  type with no diagnostic.
- This is what turned the #924 parse mis-shape (`expr\\n(args)` folded
  into a call) into silent invalid output instead of a compile error:
  `intrinsic_vec_set_gc_layout.ark` shipped exactly such a miscompiled
  call into s2.

## Expected

A resolve/typecheck diagnostic along the lines of
`E0xxx: 'a' (i32) is not callable`.  Legal dynamic callees that must
keep working: function-typed locals/parameters (TY_FN), enum variant
constructors, Vec associated ctors, builtin rewrites — all are resolved
before the unknown fallback, so a diagnostic at the fallback point only
fires for genuinely non-callable callees.

## Acceptance

- [x] `a(b)` with `a` bound to a non-function type is a compile error
      naming the callee and its type.
- [x] Function-typed locals, closures, enum-variant ctors and builtins
      still typecheck (no new false positives).
- [x] `compile-error` fixture covering the diagnostic:
      `tests/fixtures/diagnostics/noncallable_callee.ark`.

## Resolution

Fixed in `3a6ad31f1` (`fix(#926)`).

`typechecker/call_unknown.ark::infer_unknown_ident_call` now emits
`'<name>' (<type>) is not callable` (E0200) when the callee resolves to
a concrete non-function binding.  TY_UNKNOWN and TY_TYPE_VAR stay
silent — the resolver owns undefined-name diagnostics, and an unbound
type var may still unify with a function type.  Function-typed
locals/parameters keep returning their result type via the existing
TY_FN path; builtins return before the check.

Verified on the rebuilt s2: `a(b)` with `a: i32` reports
`a (i32) is not callable`; `apply(add1, 41)` through a function-typed
parameter and `Option::Some(3)` still compile; diag-parity PASS with
new `noncallable_callee` fixture.

## Notes

Found while closing #924.  Until fixed, malformed or mistyped call
nodes reach the emitters and produce invalid wasm or runtime traps with
no source-level diagnostic.
