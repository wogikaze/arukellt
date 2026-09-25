---
Status: done
Created: 2026-09-25
Updated: 2026-09-26
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

- [x] A generic impl method returning \`self.<T-field>\` emits a
      \`struct.get\`-equivalent read and runs on both targets
- [x] \`self.<method>(...)\` inside a generic impl resolves to the
      monomorphized sibling and runs on both targets
- [x] A generic struct with an \`fn(T, ...) -> R\` field invoked through
      \`(self.f)(...)\` validates on wasm32-gc and runs with output
      identical to linear
- [x] \`ScanIter<T>\` can be expressed generically (the i32-only
      \`ScanIter\` workaround in \`std/core/iter.ark\` becomes removable)

## Required evidence

- implementation commit SHA
- positive fixtures for each reproduction on wasm32 linear and
  wasm32-gc
- no regression on the existing fn-pair probe fleet


## Progress receipt — 2026-09-25 (literal-driven generic-arg inference)

- Commit `fba962c16` adds `module_structs::infer_generic_literal_args`:
  an unannotated literal on a generic struct (e.g. `ScanBox { state: 7 }`)
  now unifies each field value against the declared generic field type and
  attaches the resolved arguments to the literal's `TypeInfo`. The
  previous bare-name result left `it.get()` unable to bind `T`, so no
  mono instance was recorded and calls targeted the un-emitted generic
  template — the empty MIR body behind reproductions A and B.
- Acceptance A verified: `self.state` (`T` read) and a let-bound
  equivalent both run on wasm32 linear and wasm32-gc.
- Acceptance B verified: `self.get()` from `get_twice` inside the same
  generic impl resolves to the monomorphized sibling
  (`ScanBox__get__i32` emitted and called) on both targets; a
  `ScanBox<String>` instantiation and a two-parameter `Pair<A, B>`
  literal also pass.
- Positive fixture `tests/fixtures/generics_v1/generic_inferred_struct_lit.ark`
  registered as `run:` and `t3-run:` (manifest count 3006 → 3008);
  output 7/hi/two/9 identical on linear and GC component runs.
- `verify lane` green on the implementation commit; a 238-fixture
  regression batch (structs/generics/ownership/iterator/stdlib_trait)
  against this s2 shows 29 failures, all reproduced identically on the
  pre-change s2 baseline (rmw_noop, enum_return, buf_read, debug/eq/ord
  i64 lanes, io_copy, seek, and others), so none are attributed here.
- Acceptance C remains open: a generic struct with an
  `fn(T, T) -> Option<T>` field still fails `wasm-tools validate` on
  wasm32-gc (`expected i64, found (ref null $type)`); the `Option<T>`
  variant payload in a monomorphized body keeps the unresolved self-enum
  ref instead of the substituted item type. This is the same generic
  enum payload lowering gap the pinned bootstrap works around with the
  structural Box/Slice overlay. Acceptance D (generic `ScanIter<T>`)
  depends on it. This is not a close receipt.

## Progress receipt — 2026-09-26 (mono-resolved Option payload binds)

- Commit `6e284a174` teaches `mir_infer_payload_bind_type` and
  `emit_payload_field_bind_gc_resolve_type` to resolve the payload
  element type and the bind's declared name through
  `ctx_resolve_mono_type_name`. A monomorphized body keeps generic
  spellings on its locals (`stepped : Option_T`, `v : T`); the
  extracted element `T` was never mapped, so the bind stayed
  `VT_GC_REF` named `T` and selected the `_f1_ref0`
  `Option::Some` variant (self-enum-ref payload) instead of the
  concrete `_f1_i32` layout.
- Acceptance C verified end-to-end: `AccBox<T> { state: T, f: fn(T, T) -> Option<T> }`
  with `(self.f)(self.state, x)` followed by a `match` on the
  result validates under `wasm-tools validate --features gc,function-references,memory64`
  and runs identically on both targets (prints 15).
- Positive fixture
  `tests/fixtures/generics_v1/generic_fn_field_option_step.ark`
  registered as `run:` and `t3-run:` (manifest count 3008 → 3010).
- `verify lane` green on the implementation commit. A 228-fixture
  compile+validate batch (generics, generics_v1, iterator, stdlib_trait,
  ownership, match_extensions, enums) shows 28 failures, all reproduced
  identically on the pre-change s2 at the same byte offsets
  (ord/partial_ord/debug/eq i64 lanes, read_write, seek, adt_probe,
  struct_pattern, test_enum_return, and others), so none are attributed
  here.
- Residual: `ScanBox<T>`-shaped literals where scalar fields precede
  the `fn` field still fail wasm32-gc validation — the fn-value
  temporary is typed as the enum base ref instead of the fnpair struct
  (`expected (ref null $type), found (ref (exact $type))`). That is
  the fn-value aggregate-slot typing tracked in #883, not this issue's
  Option-payload path. Acceptance D (generic `ScanIter<T>`) depends
  on it plus the `Option<T>` record-field gap in #881. This is not
  a close receipt.

## Progress receipt — 2026-09-26 (D done; issue closing)

Commit: \`9ed271f83\` on \`docs/adr-058-language-v2-semantics\`.

D verified: the i32-only \`ScanIter\` workaround is removed and
\`ScanIter<T>\` is expressed generically.

- \`fn scan\` moved from \`impl VecIter<i32>\` into
  \`impl VecIter<T>\` (\`init: T\`,
  \`f: fn(T, T) -> Option<T>\`, returning \`ScanIter<T>\`).
- \`pub struct ScanIter<T>\` carries \`vec: Vec<T>\`,
  \`state: T\`, \`f: fn(T, T) -> Option<T>\`;
  \`impl ScanIter<T>\` and \`impl Iterator for ScanIter<T>\`
  (\`Item = T\`) replace the concrete impls.
- The i32 convenience surface is unchanged: \`scan_iter\`,
  \`collect_scan_iter\`, and \`v.iter().scan(...)\` keep
  their existing call shapes with \`ScanIter<i32>\` results
  (manifest.toml + generated stdlib docs synced).

Evidence:

- Existing fixture \`tests/fixtures/stdlib_trait/iterator_step_scan.ark\`
  (\`iter::scan_iter(v, 0, running_sum)\` + fused-exhaustion
  checks) validates on wasm32-gc and runs identically on wasm32-gc
  component and wasm32 linear memory32.
- New fixture \`tests/fixtures/iterator/generic_scan_iter.ark\`
  (run + t3-compile + t3-run): \`Vec<String>\` scan with a
  \`fn(String, String) -> Option<String>\` fold — prints
  \`a ab abcc none4\` identically on both targets;
  wasm-tools validate clean.  This exercises the full generic path:
  \`state: T\` field read, the fn-typed field call
  \`(self.f)(self.state, item)\`, and the \`Option<T>\`
  match inside a monomorphized body.
- Regression: iterator 2/2, generics 8/8, functions 16/16,
  collections 15/15, closure_capture 6/6 compile-validates;
  stdlib_trait FAIL=25 - identical pre-existing failure list.
- verify lane PASS (quality changed).
- \`docs regenerate\` PASS; the run also repaired a pre-existing
  fixture-remainder drift (\`fixture_not_in_last_harness_snapshot\`
  had not been bumped alongside \`fixture_manifest_count\` in
  earlier fixture commits; canonical value is count-observed=1434).

Residuals:

- Binary \`+\` on \`String\` operands (no \`Add\` impl)
  compiles and diverges between backends - concat on wasm32-gc,
  pointer arithmetic on linear.  Filed as #885; the fixture uses
  \`concat\`, the supported spelling.
- \`PeekIter<T>\`'s Vec-based \`peeked\` workaround
  remains (unblocked by #881, optional revert).


