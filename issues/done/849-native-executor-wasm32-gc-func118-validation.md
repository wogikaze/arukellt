---
Status: done
Updated: 2026-10-15
---

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
- [x] Reproducible under an explicit wasm32-gc profile lane
- [x] Root-caused type canonicalization / function 118 body drift fixed or waived with ADR-backed scope

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

## Update 2026-10-15 (2) — root cause found and fixed (d103863c2)

The mistyped scratch local was not a spine-entry collision. It is
infer_get_unchecked_elem_from_next_call in
src/compiler/wasm/code_ref_locals_infer_dest.ark (added aa87f31a1 for
#730): for a raw_array_get_unchecked producer it scanned forward past
LOCAL_SET/LOCAL_GET to the *next CALL in the block* and stamped that
call's **last parameter's named-struct type** onto the dest local, without
checking that the dest value actually feeds that argument.

In SelfEmitCtx_apply_call_relocs, the body binding of
get_unchecked(ctx.body_bytes, func_idx) is followed by
ctx_record::SelfEmitCtx_target(ctx) — a call whose last (only) parameter
is ctx: SelfEmitCtx — so _t51 was declared (ref null 184) while the
instruction stream stores the (ref null 10) array.get result into it.

Minimal reproducer (compiled INVALID before the fix, VALID after):

    struct Big { bodies: Vec<Vec<i32>>, tag: i32 }
    fn Big_tag(b: Big) -> i32 { b.tag }
    fn pick(bag: Bag, i: i32) -> i32 {
        let body = get_unchecked(bag.bodies, i)
        let tag = Bag_tag(bag)   // unrelated receiver call poisoned the temp
        len(body) + tag
    }

Fix: call_result_is_last_call_arg gate — the next-call inference fires
only when the dest local (or a staged copy of it) is the last argument's
LOCAL_GET source, which is exactly the shape the heuristic was written
for. Regression fixture: target_capability/gc_get_unchecked_dest_decl.ark
(run / t3-compile / t3-run).

## Update 2026-10-15 (3) — co-observed join-temp defect root-caused and fixed (1388020de)

The second observed defect (Oct-9 emit func 6873 / current emit func 6879,
`edge_push_caller_segment`, write-only `local.set` local 23) is a different
bug family in the same decl-resolution layer: `mir_begin_if_expr` allocates
one result local before branch types are known, and a statement-position
`if` still copies each branch's last stack ref into it via LOCAL_SET. With
`push(table.caller_lists, list)` vs `push(get_unchecked(...), pos)` the two
branches leave unrelated concrete GC types — no single concrete decl accepts
both stores.

Fix: `local_set_conflict_storage_type` (src/compiler/wasm/
code_ref_locals_block_scan.ark) scans a local's LOCAL_SET writes and resolves
each source's storage type; on disagreement `emit_gc_local_val_type` declares
enum_open when every source is enum-family (extending the existing
variant-vs-variant join to variant-vs-open) and `(ref null any)` otherwise.
Reads already `ref.cast` at access sites, so the widened decl is sound. The
join is also applied inside `copied_non_string_storage_type` so a copy of a
conflicted local resolves to anyref, not the first write's type.

Minimal reproducer (compiled INVALID before, VALID after):

    fn f(vv: Vec<Vec<i32>>, v: Vec<i32>, c: i32) -> i32 {
        if c > 0 { push(vv, v) } else { push(v, 1) }
        0
    }

Regression fixture: target_capability/gc_if_join_mixed_ref_temps.ark.

End-to-end evidence: a gc-host build carrying both fixes (gc-host-v6, the
current-source compiler emitted to wasm32-gc and binary-patched only for the
join-temp decl, matching this fix) re-emitted the full flat-src compiler to
wasm32-gc — `wasm-tools validate --features gc` reports zero defects with no
patching. The third co-observed defect (Oct-9 emit func 8834 func-section
rec/ref) did not reproduce in current-source emit.

Remaining producer-side note: a linear-memory (wasm32) host such as
arukellt-s2-runtime.wasm cannot finish a flat-src compile — it grinds in
the pre-#922-hysteresis GC runtime the Oct-9 pinned emitter embedded. That
is producer speed (tracked under #922/#851), not emit correctness; the
wasm32-gc emit validation above used the GC-native gc-host lane.

## Resolution (2026-10-15)

Both defects of the observed family are root-caused and fixed in the
wasm32-gc emit path; the compiler now self-emits to a fully valid
wasm32-gc module.

- d103863c2 + e2c2a05f4: `infer_get_unchecked_elem_from_next_call`
  stamped the next block-CALL's last-param type onto a get_unchecked
  dest without checking dataflow (receipt func 118 == emit func
  5353/5357 local 51). Gated on `call_result_is_last_call_arg`.
  Fixture: gc_get_unchecked_dest_decl.
- 1388020de: `local_set_conflict_storage_type` — if-expression join
  temps whose LOCAL_SET sources disagree now declare enum_open
  (enum-family conflicts) or `(ref null any)` (unrelated concrete
  types), fixing the `edge_push_caller_segment` write-only local 23
  defect (emit func 6879 / Oct-9 func 6873). Fixture:
  gc_if_join_mixed_ref_temps.
- The third co-observed defect (Oct-9 func 8834 func-section rec/ref)
  did not reproduce in current-source emit.

Lane evidence: gc-host-v5 (Oct-14 emit logic) re-emitted flat-src to
wasm32-gc = func 6879 defect reproduced; gc-host-v6 (same emit plus the
join fix) re-emitted it with zero validation defects and no patching
(`wasm-tools validate --features gc` clean on selfemit-v7.core.wasm).
Gates: `selfhost-fixture-test.py --filter-dir target_capability` 7/7
PASS, `verify lane --gate t3` PASS.

Out of scope here: producer-side linear-GC throughput on the s2-runtime
host (#922/#851) and the dedicated s2-gc→s3-gc native-executor lane
harness (native-cpp promotion mission, #847/#848).
