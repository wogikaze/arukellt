---
Status: open
Created: 2026-09-25
Updated: 2026-09-25
ID: 883
Parent: 874
Track: language-v2
Depends on: none
Related: "ADR-058, RFC-011, #872, #874, #881, #882"
Orchestration class: implementation-ready
Orchestration upstream: 874
Blocks v{N}: none
Priority: 2
Source: "#874 fn-pair probe fleet — 2026-09-25"
---

# 883 — fn values in aggregate slots: GC field-init type index / linear 0-param indirect call

## Scope

Function values stored inside aggregates miscompile on both backends in
two distinct ways. Both were found by the #874 fn-pair probe fleet and
sit next to (but are independent of) the \`Option<T>\` field type-index bug
#881 and the generic-impl family #882.

## Reproduction

### A. wasm32-gc: funcref field init after scalar fields

\`\`\`ark
pub struct ScanBox {
    index: i32,
    length: i32,
    state: i32,
    f: fn(i32, i32) -> Option<i32>,
}

let it = ScanBox { index: 0, length: 3, state: 0, f: sum2 }
\`\`\`

The \`f: sum2\` init emits \`i32.wrap_i64\` on the fn-pair value and a
\`struct.set\` whose type index disagrees with the managed-pair object:
\`wasm-tools validate\` rejects the core with
\`type mismatch: expected (ref null $type), found (ref (exact $type))\`.
The same shape validates when a reference-typed field (e.g. \`Vec<i32>\`)
precedes \`f\`, so field ordering perturbs the chosen type index.

### B. wasm32 linear: zero-param fn value through HashMap

\`\`\`ark
let m = hash_fn::hashmap_i32_fn_new()
let _ = hash_fn::hashmap_i32_fn_set(m, 1, answer)
match hash_fn::hashmap_i32_fn_get(m, 1) {
    Some(f) => stdio::println((f()).to_string()),
    None => {},
}
\`\`\`

\`(f())\` traps at runtime with \`indirect call type mismatch\`. The
artifact is byte-identical to one built before the c3e63d813 fn-pair
fixes, so this is a pre-existing linear defect, not a regression.

## Acceptance

- [ ] A record literal storing a multi-param fn value validates and runs
      on wasm32-gc regardless of the field's position
- [ ] A zero-param fn value retrieved from \`HashMap<i32,fn()->i32>\`
      invokes correctly on wasm32 linear and wasm32-gc
- [ ] No regression on the existing fn-pair probe fleet or the
      \`closure_capture\` fixtures

## Required evidence

- implementation commit SHA
- positive fixtures for both reproductions
- validate output for the GC core module
