---
Status: done
Created: 2026-10-14
Updated: 2026-10-14
ID: 921
Track: compiler
Depends on: none
Orchestration class: design-ready
Orchestration upstream: 851
Blocks v4 exit: False
Priority: 2
Source: "#851 per-push emit debug session: TEMP debug eprintln triggered latent codegen bug"
Related: "#851, ADR-053"
---

# 921 — Nested if-expression as call argument emits invalid wasm

## Summary

An `if`-expression used **directly as a function-call argument** produces
wasm that fails validation. The `if` arms deliver their result via the
frame-slot convention (`local.get frame; i32.const N; i32.sub; local.get T;
call <slot-store-helper>`), while the call-argument binding code expects the
value on the operand stack (`local.set` immediately after the `if`/`end`).

## Reproduction

```ark
use std::prelude::{to_string}
use std::host::stdio
fn main() {
    let b = true
    let s = to_string(if b { 1 } else { 0 })
    stdio::println(s)
}
```

Compiled with the **pinned bootstrap compiler** (pre-existing, unrelated to
the #851 emitter changes):

```
scripts/run/arukellt-selfhost.sh compile --target wasm32-gc -o out.wasm t.ark
wasm-tools validate out.wasm
# error: func 16 failed to validate
#   type mismatch: expected i64 but nothing on stack (at offset 0x652)
```

The let-bound equivalent compiles and validates fine:

```ark
let n = if b { 1 } else { 0 }
let s = to_string(n)
```

## Impact

Low-to-moderate: the pattern is uncommon in existing code (the compiler
itself had zero occurrences until a temporary debug line introduced one),
but any user writing `f(if c { a } else { b })` silently gets a corrupt
module — the compile reports "compilation succeeded" and only
`wasm-tools validate` or a runtime trap reveals it.

Observed in the wild inside `emit_gc_local_enum_like` (selfhost compile of
`src/compiler/wasm/code_locals_struct.ark`): `func 5279` failed with
`expected i32 but nothing on stack` at the `local.set` following the
nested `if`/`end`.

## Acceptance

- [ ] `to_string(if b { 1 } else { 0 })` (and nested if-as-arg generally)
      validates and runs correctly
- [ ] Root cause identified: whether the if-result lowering picks the wrong
      delivery convention, or the arg-binding assumes stack delivery
- [ ] Regression fixture covering if-expression in call-arg position
- [ ] Verify match/nested-block expressions in arg position as well

## Resolution (2026-10-14)

Root cause: if/match/block-tail expressions deliver their value via a
result local — mir_store_if_branch_result writes each arm into the
if-result local and mir_end_if_expr deliberately does not emit a trailing
local.get ("callers that need the value load the local themselves").
Unstaged call arguments (stage_call_args=false: single-arg calls and
println-like callees) are consumed straight from the operand stack, so a
result-local arg left nothing for the callee — local.set on an empty stack.
Staged calls were immune because mir_stage_call_arg copies the local.

Fix: mir_method_arg_leaves_no_stack now also reports MIR_END (structured
control terminator) as a no-stack producer — matching the existing
STRUCT_SET/ARRAY_SET/GC_STRUCT_SET result-local cases — and
lower_direct_call_expr / lower_core_direct_call_args emit
local.get arg_idx for unstaged args whose lowering ended at a no-stack
producer. Method-call receivers/args benefit through the same helper.

Evidence:

- to_string(if b {1} else {0}), id(if b {2} else {3}),
  id(match r {2=>4,_=>5}), id({ let q=7; if b {q} else {8} }) all
  compile + wasm-tools validate clean on wasm32-gc (s2-runtime build);
  component emit + run prints expected values.
- Fixture: tests/fixtures/control/if_expr_call_arg.ark (+ .expected,
  manifest run: entry). selfhost-fixture-test --filter-dir control
  PASS 14/14.
- Regression: nested id(id(5)), staged add2(if…, 9), let-bound
  let n = if … and 7 control/functions fixtures all remain VALID.

Follow-up (filed separately): component emit (i32-scalar mode) still fails
"expected i64, found i32" when an i32-typed source feeds an i64 parameter —
pre-existing arg-width widening gap, reproduces with plain id(n: i32) on
the pinned bootstrap; unrelated to this fix.
