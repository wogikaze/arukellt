---
Status: done
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

## Deeper root cause (2026-10-14 analysis)

Memory64 stack convention: MIR scalars sit on the operand stack as i64
(array.get on i32 elems is followed by i64.extend_i32_u; emit_extend
emits i32.wrap_i64 + i64.extend_i32_s treating input as i64). But
i32-typed wasm locals produce a REAL i32 on local.get — the stack repr
is not normalized at the get site. Any consumer that needs stack-i64
(call params, i64 local.set) then sees the raw i32: the class of bug,
not just call args.

Fix shape (design-ready): either local.get emit extends i32-typed
locals to stack-i64 repr in memory64 mode (broad, code size cost), or
arg-bind emits i64.extend_i32_s — NOT MirInst_extend_i32_to_i64_stack,
which emits wrap+extend for the stack-i64 convention and would itself
be invalid on a real-i32 input. Staged calls partially dodge this via
push_i64_needs_extend, but that only covers the last arg — a per-param
widen table is the complete fix.

## Acceptance

- [x] i32-typed arg to i64 param emits i64.extend (or an equivalent
      coercion) in component emit / i32-scalar mode
- [x] The same widening verified for if/match result args and declared
      i32 locals
- [x] Regression fixture exercising component emit arg width coercion

## Resolution (2026-10-14, 5c572e93d)

Implemented the per-param widen table. mir_emit_staged_call_args now takes
a widen_i64_args vector parallel to staged_arg_locals and emits
MirInst_extend_i32_to_i64_stack after each flagged staged re-read. Both
direct-call lowering paths (AST body_call_direct, CoreHIR
core_call_direct_args) compute needs_i64_arg_widen =
mir_callee_param_is_i64(callee, param_idx) && ctx_local_vt(arg) ==
VT_I32, emitting the extend inline for unstaged args and flagging the
staged copy otherwise. The legacy push_i64_needs_extend flag now maps
onto the last staged slot, preserving the old vec_push emit order.

Note on the fix-shape caution above: MirInst_extend_i32_to_i64_stack is
the correct instruction for both widths — under memory32 (component
emit) emit_extend_i32_to_i64 emits the bare i64.extend_i32_s this
analysis called for; under memory64 it emits wrap+extend which
normalizes the stack-i64 representation of an i32 local. The invalid
"wrap on a real i32" combination cannot arise because each width emits
the form matching its own local.get representation.

Verified: the issue reproducer under --emit component validates clean
(wasm-tools validate; previously "expected i64, found i32"), and
regression fixture run:functions/i32_arg_to_i64_param.ark (declared i32
local, staged multi-arg both positions, if-result and match-result args)
prints its expected 2/7/7/5/5 through the memory32 component pipeline.
python3 scripts/manager.py verify lane PASS.
