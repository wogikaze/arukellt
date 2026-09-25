---
Status: open
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

- [ ] \`let f: fn() -> i32 = answer\` binds the fn pair on wasm32
      linear and wasm32-gc
- [ ] \`return answer\` in a \`-> fn() -> i32\` function returns
      the fn pair on both targets
- [ ] assignment \`f = answer\` to a fn-typed local stores the pair
- [ ] \`Some(answer)\` stores the pair in \`Option<fn() -> i32>\`
      (may ride on the #881 resolution)
- [ ] No regression on the fn-pair probe fleet or closure_capture
      fixtures

## Required evidence

- implementation commit SHA
- fixture covering let / return / assign (and Option payload if fixed)
- validate output for the GC core module
