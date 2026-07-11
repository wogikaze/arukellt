# ADR-033: クロージャ呼び出しを call_ref に移行

**Status**: ACCEPTED — phased migration; `call_indirect` remains baseline until table-free patterns land
**Date**: 2026-06-14
**Track**: wasm-feature
**Issue**: [#069](../../issues/done/069-wasm-typed-func-ref.md)
**Supersedes**: none (refines GC-native closure notes in issues #019, #025)

---

## Context

Arukellt closures and higher-order functions (HOF) currently lower to Wasm
`call_indirect` with a function table (`docs/current-state.md` Closures row).
The WebAssembly Typed Function References proposal adds `ref.func`, `call_ref`,
`br_on_null`, and `br_on_non_null`, enabling table-free typed dispatch when the
callee signature is known at compile time.

> **2026-07 update**: Typed Function References is now Phase 5 shipped in
> Wasm 3.0 (`typedFunctionReferences`). wasmtime 46 and V8 14.6 (Chrome 146 /
> Node.js 26) enable it by default. See ADR-043 for the Post-MVP survey.

Historical issues (#019, #025, #024) planned a GC-native `call_ref` path; the
selfhost emitter still uses `call_indirect` for generic HOF dispatch on the
current T3 path. Issue #069 tracks closing the gap. Issue #722 tracks the
detailed phase plan (emitter audit, nullable refs, benchmark gate).

## Decision

1. **Baseline (now)**: Keep `call_indirect` for generic closure/HOF dispatch.
   No user-visible regression while migration is incremental.
2. **Phased migration**: Move from `call_indirect` to `call_ref` in phases,
   gated by benchmark results. The detailed phase plan (emitter audit,
   nullable refs, benchmark gate) is tracked in **issue #722**.
3. **Out of scope here**: `return_call_ref` tail calls (#492), eliminating all
   `call_indirect` before v5, Table/Elem removal before escape analysis proves
   table-free coverage.

## Consequences

- `docs/current-state.md` must qualify the Closures row: `call_indirect` is the
  current default; `call_ref` adoption is phased per this ADR.
- New emitter work lands behind fixtures that prove `call_ref` bytes in output
  before claiming full HOF migration.
- MIR may gain `FnRef`/`call_ref` lowering hooks without removing table-based
  paths until Phase C benchmark gate passes.

## References

- `docs/spec/spec-3.0.0/proposals/function-references/Overview.md`
- `issues/done/025-gc-native-closures.md`
- `issues/done/492-t3-return-call-ref.md`
