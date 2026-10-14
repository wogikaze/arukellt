# 849 — native-executor wasm32-gc S3 validation drift (func 118)

## Summary
When the native executor lane historically forced `--target wasm32-gc`, Stage-3
validation failed with a nominal reference-type mismatch in `func 118`.

## Reproduction (historical receipt)
- Command: `python3 scripts/manager.py selfhost native-executor --build` with S3 target hardcoded to `wasm32-gc`
- Receipt validation_error:
  `invalid compiler wasm arukellt-s3-native.wasm: error: func 118 failed to validate`
  `type mismatch: expected (ref null $type), found (ref null $type) (at offset 0x452d7)`
- Saved artifacts (origin workspace snapshot):
  - `.recovery-snapshot/raw/.build/selfhost/native/native-executor-receipt.json`
  - `.recovery-snapshot/raw/.build/native-cpp-executor/selfhost/native-cpp/arukellt-s3-native-debug.wasm`

## Impact on native-cpp MVP
Not a blocker for the canonical native executor lane, which inherits the
comparison S2 build profile (`wasm32` + `wasi-p1` today). Handle under a
separate `s2-gc[wasm32-gc] -> s3-gc[wasm32-gc]` validation lane.

## Acceptance
- Reproducible under an explicit wasm32-gc profile lane
- Root-caused type canonicalization / function 118 body drift fixed or waived with ADR-backed scope

## Update 2026-10-15 — defect reproduces in current-source emit

The same nominal (ref null $type) mismatch is not confined to the
historical pinned binary. A wasm32-gc self-emit produced by the current
source fails wasm-tools validate:

- Producer: gc-host overlay build of current src/compiler (patched
  Oct-9-emitted core), run:
  gc-host.component.wasm compile src/compiler/main.ark --target wasm32-gc
  --wasi-version wasi-p2 -o gchost-selfemit.wasm
- Result: func 5353 failed to validate — type mismatch: expected
  (ref null $type), found (ref null $type) (at offset 0x1f6c25)
- Defect site: SelfEmitCtx_apply_call_relocs — the instruction at
  0x1f6c25 is local.set 51 storing a (ref null 10) (post-cast
  get_unchecked(ctx.body_bytes, func_idx) value) into scratch local
  _t51, which was declared (ref null 184) (SelfEmitCtx's struct
  type). The value should have been declared (ref null 10). The named
  $body local immediately after is correctly (ref null 10), so the
  scratch-local declared heaptype — not the store index — is wrong.
- Same func, same local index, same defect family in the Oct-9 pinned
  emit of the same source (gc-host.core.wasm): local 51 was declared
  ref 198 where ref 24 was required. Type numbers shift between
  compilations; the constant is that scratch local 51 of this function
  gets a wrong declared heaptype.
- Two more Oct-9 pinned-emit defects observed while patching gc-host into
  a working producer (same "wrong heaptype in local decl / func sig"
  family): func 6873 write-only local.set 23 typing, and func 8834
  (hashmap_type_name) func-section type index naming the wrong rec/ref.
- Small programs are unaffected: a trivial Vec::new loop compiles to
  wasm32-gc and validates clean. The bug needs the large type space /
  spine state of a compiler-scale function.

Likely locus: the named-local decl path resolves scratch _tN ref types
through SelfEmitCtx_wasm_ref_type_idx_for_local (spine-only lookup) in
emit_gc_local_inferred_type / emit_gc_local_val_type; a stale or
cross-function-colliding spine entry would produce exactly this
signature. See src/compiler/wasm/code_locals_struct.ark and the #910
reanalysis work.

Also observed this session: s2-runtime (memory64 heap-patched host)
cannot finish a flat-src to wasm32-gc compile within 15 min — consistent
with the known quadratic-mark pathology that motivated the gc-host lane;
that is a producer-speed property, not emit correctness.
