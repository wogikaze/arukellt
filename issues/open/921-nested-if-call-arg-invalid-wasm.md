---
Status: open
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
