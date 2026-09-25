---
Status: open
Created: 2026-09-25
Updated: 2026-09-25
ID: 882
Parent: 691
Track: language-v2
Depends on: none
Related: "ADR-058, RFC-011, #691, #874, #881"
Orchestration class: implementation-ready
Orchestration upstream: 691
Blocks v{N}: none
Priority: 1
Source: "#691 ScanIter probe fleet — 2026-09-25"
---

# 882 — generic impl method bodies: bare-T field reads, sibling calls, fn-typed fields

## Scope

\`impl Box<T>\` method bodies miscompile in three ways, blocking generic
iterator state machines (generic \`ScanIter<T>\`, \`PeekIter<T>\` with a real
\`Option<T>\` field, and arbitrary adapter state) required by #691/#860.
All three reproduce on current s2 and are independent of the \`Option<T>\`
record-field type-index bug tracked in #881.

## Reproduction

### A. bare-T field reads lower to an empty body

\`\`\`ark
pub struct ScanBox<T> {
    state: T,
}

impl ScanBox<T> {
    fn get(self: ScanBox<T>) -> T {
        self.state
    }
}
\`\`\`

The MIR body of \`get\` is empty (\`bb1 instructions=0\`). On wasm32 the
module fails translation with \`type mismatch: expected i32 but nothing on
stack\`; assigning through a local (\`let item = self.state; item\`) fails
the same way. Concrete \`ScanBox\` (non-generic) works.

### B. sibling generic method calls vanish

\`self.step_item(item)\` inside \`impl ScanBox<T> { fn next(...) }\` emits no
call to a monomorphized \`step_item\`; the callee is dropped from the
module and \`next\` traps on \`unreachable\` at runtime (linear wasm32,
exit 134).

### C. fn-typed fields in generic impls produce invalid GC modules

\`\`\`ark
pub struct AccBox<T> {
    state: T,
    f: fn(T, T) -> Option<T>,
}
\`\`\`

Calling \`(self.f)(self.state, x)\` inside \`impl AccBox<T>\` validates and
runs on wasm32 linear, but the wasm32-gc core fails
\`wasm-tools validate\` with \`type mismatch: expected i64, found
(ref null $type)\`: the fnsig carries unresolved \`T\`, so the
\`call_ref\` ref result is typed into an i64 dest local. The equivalent
concrete-struct shape (#874 fn-pair ABI) passes on both backends.

## Acceptance

- [ ] A generic impl method returning \`self.<T-field>\` emits a
      \`struct.get\`-equivalent read and runs on both targets
- [ ] \`self.<method>(...)\` inside a generic impl resolves to the
      monomorphized sibling and runs on both targets
- [ ] A generic struct with an \`fn(T, ...) -> R\` field invoked through
      \`(self.f)(...)\` validates on wasm32-gc and runs with output
      identical to linear
- [ ] \`ScanIter<T>\` can be expressed generically (the i32-only
      \`ScanIter\` workaround in \`std/core/iter.ark\` becomes removable)

## Required evidence

- implementation commit SHA
- positive fixtures for each reproduction on wasm32 linear and
  wasm32-gc
- no regression on the existing fn-pair probe fleet
