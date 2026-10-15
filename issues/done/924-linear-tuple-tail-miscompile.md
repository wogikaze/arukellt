---
Status: done
Created: 2026-10-15
Updated: 2026-10-15
ID: 924
Track: compiler
Depends on: ""
Related: "#922, #851, #926"
Orchestration class: implementation
Blocks v4 exit: False
---

# wasm32-linear: bare tuple literal in tail position emits drop+unreachable

## Summary

On the wasm32 (linear-memory) backend, a function whose body ends in a
bare tuple literal `(a, b)` — rather than `return (a, b)` — was
miscompiled: element values were dropped, `unreachable` emitted, and
the tuple construction became dead code.  Depending on shape the result
was either a runtime trap, invalid wasm (operand-stack underflow at
validation), or a bogus `undefined name` resolve error.

Found while repairing the AST cache codec (#825): every codec reader
returned tuples via bare tails, which is why the cache never worked on
the linear target.

## Root cause

Not an emitter defect — a **parser** defect.  `parse_expr_bp`
(`src/compiler/parser/expr_bp_loop.ark`) already treated a `(` at the
start of the next line as a new statement, but the guard lived only in
the loop iteration that consumed the newline.  When the previous line
ended in an infix expression, the nested `parse_expr_bp` call for the
infix right-hand side consumed the newline, saw `(`, and broke —
returning with the cursor already past the boundary.  The outer frame
then saw `(` with no newline and applied the call suffix:

    let next = pos + 4
    (word, next)

parsed as `let next = (pos + 4)(word, next)`.  When every argument
name happened to be in scope (e.g. `word`, `pos + 5`), the call
"resolved" and the typechecker accepted it — a call of a non-function
i32 is silently inferred as unknown (filed as #926) — so the emitters
received a call node over an i32 callee and produced garbage.  On
wasm32-linear that garbage took the shape of dropped elements,
`unreachable`, and stack-underflow-invalid modules.

This also explains why the defect looked emitter-specific to wasm32:
the two emitters lower the malformed call differently — wasm32-gc
emitted validating-but-wrong code while wasm32 emitted invalid code.

## Fix

`expr_bp_loop.ark`: mark the cursor before the newline check; when the
next-line `(` or `[` boundary is detected, restore the mark before
breaking so every enclosing infix frame re-observes the newline and
stops too.  `[` was folded into the same guard: `expr\n[i]` index
continuation had zero in-tree uses (src/compiler, std, tests) and the
mirror-image case — a tail array literal after an infix line — silently
parsed as an index.  Trailing-operator continuation
(`a &&\n(b || c)`) is unaffected — that newline is skipped at frame
top before the prefix parse, a different path.

## Consequence found in-tree

`src/compiler/wasm/intrinsic_vec_set_gc_layout.ark` —
`vec_set_vec_flags` ended with `let is_vec_sref = vec_type == f(ctx)`
followed by a bare 4-tuple tail.  It was being miscompiled into every
selfhost binary as `(vec_type == f(ctx))(is_vec_string, ...)`: the
`is_vec_sref` flag came from a call to a boolean/i32 result.  Wrong
sref flags → wrong GC layout emitted for `vec_set` on reference-typed
Vec elements — a candidate contributor to the #922 linear-GC corruption
signature.

## Evidence

- Pre-fix: `let v = b0 + b1` then `(v, pos + 4)` →
  `error[E0100|resolve]: undefined name: v` (the arg position resolves
  before the let binds — proves the fold into call args).
- Post-fix wasm32 core module: compiles, `wasm-tools validate` clean,
  runs `30 / 7`.
- Post-fix wasm32-gc component: compiles, runs `30 / 7`.
- `check-cross-backend-parity.py --fixture
  functions/tuple_tail_after_infix.ark` → `parity` (both targets
  compile, run, and produce identical output matching `.expected`).
- `selfhost fixture-parity --filter-dir functions` → PASS=21 FAIL=0.

## Resolution

- [x] Bare tuple tails `(a, b)` produce correct tuple construction on
      wasm32 (parity with wasm32-gc).
- [x] Regression fixture:
      `tests/fixtures/functions/tuple_tail_after_infix.ark` +
      `.expected`, registered as `run:`, `t3-compile:`, `t3-run:`;
      `functions/` is inside the cross-backend-parity domain list so
      wasm32-linear compile+run+validate stays gated.
- [x] Sibling audit: record literals (`{` is not a call-suffix starter)
      and nested tuples (same `(` boundary, covered by the fix).  The
      `[`-index analogue silently parsed tail arrays as index
      expressions — closed by folding `[` into the same boundary guard.
- [x] wasm32-gc unaffected: fixture parity + full selfhost self-compile
      by the fixed s2.

## Notes

- Follow-up: #926 (typechecker silently accepts calls of non-function
  callees — the amplifier that let this reach emission).
