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

- [x] A record literal storing a multi-param fn value validates and runs
      on wasm32-gc regardless of the field's position
- [ ] A zero-param fn value retrieved from \`HashMap<i32,fn()->i32>\`
      invokes correctly on wasm32 linear and wasm32-gc
- [x] No regression on the existing fn-pair probe fleet or the
      \`closure_capture\` fixtures

## Required evidence

- implementation commit SHA
- positive fixtures for both reproductions
- validate output for the GC core module

## Progress receipt — 2026-09-26 (A done; B open)

Commit: \`f56c263a3\` on \`docs/adr-058-language-v2-semantics\`.

A is fixed by two independent root causes, both verified:

- \`mir/lower/stmt_let_store.ark\` — \`sync_let_local_type_name\`
  only propagated initializer type names longer than 1 char, so a
  let-bound record named e.g. \`B\` lost its ctx type.
  Field-access callees \`(it.f)(a,b)\` then could not find the fn
  field and lowered to an unresolved \`call f fid=-1\` (drop/drop/
  unreachable + i64 Option dest).
- \`wasm/code_ref_locals_block_scan.ark\` — \`local_set_source_local\`
  read \`MirInst_dest\` of the producer inst for stack-valued
  LOCAL_SETs, but \`MIR_STRUCT_SET\` packs field_idx * 100000 +
  byte_size into dest and \`MIR_GC_STRUCT_SET\` stores the value
  local there.  The fn-pair temp after scalar fields inherited an
  unrelated local's storage type (array i8), mistyping every downstream
  ref.cast/struct.set.  Now mirrors inst_dispatch_local's reload table
  (STRUCT_SET→arg0, GC_STRUCT_SET→arg1).

Evidence:

- New fixture \`tests/fixtures/functions/struct_fn_field_call.ark\`
  (run + t3-compile + t3-run in manifest): sum2 stored at field
  positions last/first/mid, invoked via \`(it.f)(a,b)\`.
  wasm-tools validate clean on wasm32-gc; prints 7/3/30 identically on
  wasm32-gc component and wasm32 linear memory32.
- Probes: c_read (store + read-back) validates + runs 0 on both
  targets; generic \`ScanBox<T>.next(5)\` probe (\`(self.f)\` inside
  impl) validates and runs 5 on both.
- Regression: 4-dir batch (functions, closure_capture, collections,
  stdlib_trait) = 219 compile-validates, 25 FAIL — identical list on
  baseline s2 without the change; verify lane PASS.

Residuals:

- B remains open: \`HashMap<i32,fn()->i32>\` zero-param indirect call
  traps on wasm32 linear (pre-existing, artifact byte-identical to
  before the fn-pair fixes).
- Generic \`Vec<T>\` record fields (ScanBox<T>.vec) still fail GC
  validation — field gets erased-generic \`{array structref,i32}\`
  layout vs concrete \`Vec<i32>\`.  Same family as #881; second
  repro noted there.
