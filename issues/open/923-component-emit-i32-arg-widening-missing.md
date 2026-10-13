---
Status: open
Created: 2026-10-14
Updated: 2026-10-14
ID: 923
Track: compiler
Depends on: none
Orchestration class: design-ready
Orchestration upstream: 921
Blocks v4 exit: False
Priority: 2
Source: "#921 verification: component emit (i32-scalar mode) rejects i32 source to i64 parameter"
Related: "#807, #921"
---

# 923 — Component emit misses i32→i64 argument widening

## Summary

In component emit mode (i32-scalar codegen) an argument held in an
i32-typed local is passed to an i64 parameter without any widening —
the produced core wasm fails validation with
"expected i64, found i32".

## Reproduction

```ark
use std::prelude::{to_string}
use std::host::stdio
fn id(x: i64) -> i64 { x }
fn main() {
    let n: i32 = 2
    let r = id(n)
    stdio::println(r.to_string())
}
```

Compile with component emit (reproduces on the pinned bootstrap, so it
predates the #921 fix):

    compile --target wasm32-gc --emit component -o out.wasm t.ark
    wasm-tools validate out.wasm
    # error: func 17 failed to validate
    #   type mismatch: expected i64, found i32

Plain wasm32-gc compile validates — scalars are i64-wide there, so the
mismatch cannot materialize. Integer literals are forced to i64 by
mir_should_force_int_lit_i64; the gap is for non-literal i32 sources
(locals, if/match results, narrow vars).

## Acceptance

- [ ] i32-typed arg to i64 param emits i64.extend (or an equivalent
      coercion) in component emit / i32-scalar mode
- [ ] The same widening verified for if/match result args and declared
      i32 locals
- [ ] Regression fixture exercising component emit arg width coercion
