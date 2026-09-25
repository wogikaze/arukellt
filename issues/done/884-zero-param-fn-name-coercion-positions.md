---
Status: done
Created: 2026-09-26
Updated: 2026-09-26
ID: 884
Parent: 874
Track: language-v2
Depends on: "881 (generic-erased slots only)"
Related: "ADR-058, RFC-011, #874, #881, #882, #883"
Orchestration class: implementation-ready
Orchestration upstream: 874
Blocks v{N}: none
Priority: 2
Source: "#883 fn-value probe fleet extension — 2026-09-26"
---

# 884 — bare 0-param fn names still auto-call at scalar positions and generic-erased slots

## Scope

\`lower_core_name_value\` resolves a bare 0-param function name by its
result type, so \`answer\` lowers to \`call answer()\` unless the
lowering site knows a fn value is expected. #883 fixed the call-arg and
struct-literal-field positions (spine param / declared field type
checks). The same root cause still fires at every other position where a
fn value is expected, on both backends.

## Reproduction

### A. scalar let annotation

\`\`ark
let f: fn() -> i32 = answer
stdio::println((f()).to_string())
\`\`

Traps with \`indirect call type mismatch\` on wasm32 linear (probe
let_zp, s2 after a8077fcd3). The annotation \`fn() -> i32\` is
available as the let's \`type_name\` in \`lower_core_let\`, so the
same expected-type guard as #883 applies.

### B. fn-typed return position

\`\`ark
fn producer() -> fn() -> i32 {
    answer
}
\`\`

\`ctx_current_return_type(ctx) == VT_FUNCREF\` identifies the
expected type in \`lower_core_return\`.

### C. assignment to a fn-typed local

\`\`ark
let mut f: fn() -> i32 = zero
f = answer
\`\`

Expected type comes from the target local's stored type name.

### D. generic-erased slots (blocked by #881)

\`\`ark
let o: Option<fn() -> i32> = Some(answer)
let v: Vec<fn() -> i32> = Vec::new()
v.push(answer)
\`\`

Variant-payload and generic-field positions cannot see the concrete fn
type until the monomorphized payload/field type resolves — same generic
erasure family as #881. Fix A–C first, then re-test D after #881.

## Acceptance

- [x] \`let f: fn() -> i32 = answer\` binds the fn pair on wasm32
      linear and wasm32-gc
- [x] \`return answer\` in a \`-> fn() -> i32\` function returns
      the fn pair on both targets
- [x] assignment \`f = answer\` to a fn-typed local stores the pair
- [x] \`Some(answer)\` stores the pair in \`Option<fn() -> i32>\`
      (may ride on the #881 resolution)
- [x] No regression on the fn-pair probe fleet or closure_capture
      fixtures

## Required evidence

- implementation commit SHA
- fixture covering let / return / assign (and Option payload if fixed)
- validate output for the GC core module

## Progress receipt — 2026-09-26 (A–D done; issue closing)

Commit: \`8f16d24a9\` on \`docs/adr-058-language-v2-semantics\`.

All four reproduction families verified with the stage-2 compiler
(\`.build/selfhost/arukellt-s2-runtime.wasm\` built from this commit):

- A. let annotation: \`let f: fn() -> i32 = answer\` prints 42 on
  wasm32 linear memory32 and wasm32-gc component. The NK_TYPE_FN
  annotation kind marks the binding as a VT_FUNCREF pair slot on both
  the AST (\`lower_let_stmt\` via node type_ann) and CoreHIR
  (\`lower_core_let\` via resolved type name) paths.
- B. return position: implicit tail (\`fn producer() -> fn() -> i32
  { answer }\`) and explicit \`return answer\` both print 42 on both
  targets — \`ctx_current_return_type == VT_FUNCREF\` gates the
  coercion on the AST return, CoreHIR return, and both bare-name body
  tail entries (\`mir_lower_entry_body_core_value\` /
  \`mir_lower_legacy_body_value\`).
- C. assignment: \`let mut f: fn() -> i32 = zero; f = answer\` prints
  42 on both targets — the target local's stored VT_FUNCREF decides
  (\`lower_assign_expr\` + \`lower_core_assign\`).
- D. generic-erased slots: \`Option<fn() -> i32> = Some(answer)\`
  prints 42 on both targets; \`Result<fn() -> i32, String> =
  Ok(answer)\` and \`Result<String, fn() -> i32> = Err(answer)\` print
  42 (expected enum type-arg decoded from
  \`ctx.expected_struct_inst\` across Option_/option:/Option<...> and
  Result_/result:/Result<...> spellings, with an arrow-aware
  top-level splitter so \`fn(A,B) -> R\` args do not mis-split).
  \`push(v, answer)\` on \`Vec<fn() -> i32>\` prints 42 — the
  expected type comes from the receiver local's Vec element type
  since the generic push spine reports the elem param as T.
  \`hashmap_i32_fn\` specialized adapters still print 42.

Evidence:

- Implementation commit \`8f16d24a9\` (this commit's code parent).
- New fixture \`tests/fixtures/functions/fn_zero_value_positions.ark\`
  covers let / implicit + explicit return / assign / Option payload /
  Vec push; expected output 42 x6 matched on wasm32 linear memory32
  and wasm32-gc component. \`wasm-tools validate\` clean on the GC
  component (\`.build/reg/fn_zero_value_positions.gc.wasm\`).
- Regression: option_fn_call, option_fn_mut_assign, vec_fn_push_call,
  struct_fn_field_call, closure_basic, closure_capture,
  hashmap_i32_fn_zero_call all produce expected output on wasm32
  linear; option_fn_call / vec_fn_push_call /
  hashmap_i32_fn_zero_call / escape_store verified identical on
  wasm32-gc component (validate OK).
- \`python3 scripts/manager.py verify lane\` PASS (ark lint smoke,
  touched-code quality ratchet 14 files, lint ratchet 15 files).

Residual (out of #884 scope): fn-valued positions inside other
generic-erased containers (nested Option<Result<fn, E>>, HashMap<K,
fn> through the generic — not specialized-adapter — surface) were not
part of the reproduction fleet; specialized hashmap_i32_fn adapters
and the Result Ok/Err payload positions verified above cover the
concrete reported cases.
