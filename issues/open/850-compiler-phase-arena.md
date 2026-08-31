---
Status: open
Created: 2026-08-31
Updated: 2026-09-01
ID: 850
Parent: 827
Track: selfhost-infra
Depends on: "827"
Related: "#827, #823, #730, #824, #834, docs/research/selfhost-phase-arena-ownership.md, docs/research/selfhost-compile-latency-root-cause.md"
Orchestration class: architecture-implementation
Orchestration upstream: 827
Blocks v4 exit: False
Priority: 1
Source: "#827 design-only close required a new issue before product arena code"
---

# 850 — Compiler phase-arena implementation (wasm32-gc host overlay)

## Summary

Implement the compiler-bootstrap phase arena decided in
[`docs/research/selfhost-phase-arena-ownership.md`](../../docs/research/selfhost-phase-arena-ownership.md).
`#827` is **design-only** and stays closed. This issue is the product-code tracker.

ADR-002 (user-program Wasm GC) does not change. This is bump / handle storage
inside `src/compiler/**` so the **gc-typed selfhost compiler** does not keep a
~1.5–1.8GB Copying-GC live set while compiling itself.

## Why this is the remaining path

Official goal: `BOOTSTRAP_EMIT_* = wasm32-gc` + `wasi-p2`, `sha256(s2)==sha256(s3)`,
cacheless flattened overlay of `src/compiler/main.ark` on that gc host **≤10s**.

Measured on HEAD `886581d1d` plus tick 80 (do not treat wasm32 ≤10s as this goal):

| Host | Overlay | Notes |
|---|---:|---|
| wasm32 + wasi-p1 | ~10s | superseded as the *goal*; still the fast emit host |
| wasm32-gc + wasi-p2 (Copying) | **239s loaded** (tick 90) / **208s quiet** (tick 80) | same tick-80 binary; s2=s3, RSS ~1.76GB |
| wasm32-gc Null collector | ~23–26s then trap | mutator floor; discard does not help Null |
| wasm32-gc DeferredRC | ~279s | worse |

Closed families (do not retry): AST empty-fallback (`tick 35`, AST is not the 2GB),
lazy per-fn HIR (`tick 36`), discard/pack MIR (`ticks 37–41`), generic/skip/intern
micro-opts (`ticks 43–62` except landed 42/44/49), thin-shell discard (`tick 63`:
**97s trap, RSS still 1.52GB**). `#824` early-body-only-reachable is wontfix.

10s needs **both** live set **≪ 1GB** (function shells / LowerCtx / TypeTable, not
just insts) **and** less mutator work than today's lower. i32 handles + phase reset
are the structural way to get there without flipping `BOOTSTRAP_EMIT_*` early.

## Design constraints (from the #827 memo)

1. Reset only at phase boundaries: parse→typecheck, typecheck→lower, lower→emit.
2. No raw references across resettable arenas; cross-phase values are durable
   handles or copies into a non-reset durable bump.
3. Final Wasm bytes and durable tables (types, signatures, export names) must not
   live in a resettable phase arena.

## Acceptance

- [ ] Product arena / handle table exists in `src/compiler/**` and is used on a
      hot lower or emit path (not a stub, not stdlib `Arena<T>` demo-only)
- [ ] Reset points match the memo; no cross-arena raw refs
- [ ] Cacheless gc-host overlay of flattened `src/compiler/main.ark` stays
      `sha256(s2)==sha256(s3)` (or the *new* compiler is itself a fixpoint)
- [ ] Receipt: wall + RSS vs tick 90 loaded **239s** / tick 80 quiet **208s** / ~1.76GB
- [ ] Do **not** set `BOOTSTRAP_EMIT_*` to wasm32-gc until overlay is ≤10s
- [ ] `python3 scripts/manager.py verify lane` on the implementation slice
- [ ] No `--to-memory64` widen of memory32 wasm32-gc (`#834`)

## First slice (suggested)

Lower-phase durable handles for MIR temporaries (insts / locals / block
instruction vecs) so Copying GC scans i32 tables instead of record graphs.
Do not add fields to `MirFunction`. Side tables belong on `LowerCtx` or a
parallel session-owned bump. Do **not** store locals on a module-lifetime
pack that outlives reachability prune (tick 131: RSS 3.87GB). Do **not**
retry a function-scoped slim-handle `MirLocal` pack (tick 132: 275s /
1.72GB wash). Do **not** replace `MirFunction.locals` with columns and
reconstruct a fat `MirLocal` on `local_at` (tick 133: 250s / 1.75GB wash).
Do **not** retry locals-pack + partial scalar rewrite (tick 134: 275s /
1.72GB wash). Do **not** skip `compute_fn_source_locations` (tick 135:
242s wash, hello 2308B, s2≠s3). Do **not** unroot flatten
`source_text` from `input` / `LowerCtx` after location compute
(tick 136: 230s s2=s3 hello-ok, RSS wash). Do **not** intern
identical GET/SET/control `MirInst` records on LowerCtx (tick 137:
241s s2=s3, RSS wash). Do **not** in-place typed-MIR local sync
(tick 138: 245s s2=s3, RSS wash). Do **not** drop dump-only
`MirBlock` phi/pred/dom vec fields (tick 139: 245s s2=s3, RSS wash).
Do **not** retarget `SET_from` dest onto the last producer
(tick 140: 234s wash, s2≠s3, RSS 1.75GB). Do **not** CSE
block-scoped `CONST_I32` by `int_val` (tick 141: 286s worse,
s2≠s3, RSS wash). Do **not** skip leftover-statement `NOP`
(tick 142: 240s floor wash, s2≠s3, RSS wash). Do **not**
share `params[i]` with `locals[i]` when type names match
(tick 143: 287s worse, s2=s3, RSS wash).

**Do not reconstruct a fat `MirInst` on every `MirBlock_inst_at`.** Ticks 64–65
did that and timed out. Tick 66 used a handle `MirInst` (`hid` + 1-element host
vec) plus column walks for async scan, in-place resolve, propagate producers,
emit neighbors, reachability, and GC local-cache seed. Overlay still **killed
at 315s** (RSS **1.11GB**). Handle `inst_at` is still an allocation per walk.

Tick 68 tried a third SoA shape: intern fat→columns **without bind-on-push**,
keep original fat accessors, **one scratch `MirInst` per block** filled from
columns for `emit_mir_inst_ctx`, plus column walks for resolve / propagate /
scan / neighbors. Hello **2312B sha256 matched tick49**. Overlay still
**timeout 320s** (no output; `/usr/bin/time` footer lost). Scratch-fill is
still a fat record write per instruction. Do **not** retry ticks 64–68.

Tick 69 made `emit_mir_inst_ctx(block, hid)` and emitted LOCAL_GET /
LOCAL_SET / CONST_I32 from columns (no fill). CALL / arith / struct still
filled a scratch `MirInst`. First SoA overlay to **finish**: **263.51s**,
`s2=s3`, hello 2312B matched tick49, RSS **2.21GB**. Worse wall than
242s/255s and RSS jumped vs ~1.75GB. Hybrid fill still allocates fat
records; intern still `clone`s every `str_val`. Do **not** retry ticks
64–69.

Tick 70 skipped the default 1-element `result_types` Vec on
`MirInst_new` and ran enrich only for CALL / WIT_CALL. Overlay
**269.91s**, `s2=s3`, hello 2312B matched, RSS **1.72GB**. Worse wall
than 242s/255s; RSS wash. Do **not** retry empty-default `result_types`.

Next slice must emit CALL / arith / struct from columns with **no fill**
(no hybrid fat record on the emit hot path). Do not keep a live fat
`MirInst` beside SoA columns. Do not retry ticks 64–76 as they were.

Tick 71 skipped the enrich rewrite of constructor `result_types` on
non-CALL (avoids a second Vec+MVT per LOCAL_GET/CONST). Overlay
**259.53s**, `s2=s3`, hello 2312B matched, RSS **1.71GB**. Wash vs
255s loaded (4s). Do **not** retry this enrich skip.

Tick 72 skipped `ctx_enrich_inst_result_types` on every `ctx_emit`
(CALL included). Hello 2312B matched tick49. Overlay **264.35s**,
**s2≠s3**, RSS **1.71GB**. CALL spine enrich is required for fixpoint.
Do **not** skip all enrich.

Tick 73 made `mir_inst_with_func_id_raw` mutate in place instead of
copying the record. Overlay **264.87s**, `s2=s3`, hello 2312B matched,
RSS **1.71GB**. Worse wall than 255s loaded. Do **not** retry in-place
func_id attach. Emit probes with the existing gc host.

Tick 74 used the cached `LowerCtx.is_gc_target` bool in
`ctx_gc_enum::ctx_is_gc_target` and `ctx_edge_record_inst` instead of
`clone(lower_target)` + `emit_target::is_gc_target` / string compare.
Overlay **266.22s**, `s2=s3`, hello 2312B matched, RSS **1.76GB**.
Worse wall than 255s loaded. Do **not** retry cached-bool gc-target
reads. Did **not** skip name intern when `fid >= 0` (fid-only edges
are closed).

Tick 75 skipped `core_op_shadow_observe_call` and
`registry_audit_observe_call` (emit diagnostics only) and stopped
storing unused `LowerCtx.source_text` after source-location compute.
Overlay **257.30s**, `s2=s3`, hello 2312B matched, RSS **1.72GB**.
Wash vs 255s loaded. Do **not** retry emit-audit / unused-source_text
skips. Next slice is still SoA CALL/arith/struct **no fill**.

Tick 76 re-landed SoA columns, emitted numeric arith from columns
(no fill), and reused one scratch `MirInst` per function for leftover
ops (CALL/struct still filled). Overlay **300.28s**, `s2=s3`, hello
2312B matched, RSS **2.07GB**. Worse wall than tick 69 (263s) and
255s loaded. Do **not** retry SoA + arith-from-cols + reused-scratch
fill. CALL/struct still need column emit with **no fat record**.

Tick 80 capped `mir_function_propagate_local_types` at 2 iterations
(was 8). Same-day HEAD-today control (max=8, tick49 host) was
**242.08s**. Cap-2 overlay **213.36s** then remasure **208.40s**,
`s2=s3`, hello 2312B matched, RSS **1.76GB**. Extra scans after the
second pass did not change compiler wasm. Keep.

Tick 81 capped the same fixpoint at 1 iteration. Overlay **230.55s**,
`s2=s3`, hello 2312B matched, RSS **1.75GB**. Worse wall than tick 80
(208s remasure). One scan still produces matching compiler wasm, but
the second scan is cheaper than whatever incomplete types cost later.
Do **not** retry max=1.

Tick 83 skipped the post-propagate `mir_module_sync_all_value_types`
(tick 50 skipped the pre-propagate sync). Overlay **233.98s**, `s2=s3`,
hello 2312B matched, RSS **1.75GB**. Worse wall than tick 80 (208s).
The second sync does not change compiler wasm, but skipping it costs
more later in layout/emit. Do **not** retry skipping either full
module sync.

Tick 84 skipped `mir_function_normalize_multi_variant_locals` after
propagate (O(locals×insts) per function). Hello 2312B matched.
Overlay **225.66s**, **s2≠s3**, RSS **1.75GB**. Worse wall than
tick 80 and the skip changes compiler wasm. Do **not** skip enum
normalize.

Tick 85 skipped the O(insts) multi-variant scan for locals whose
type_name is a non-enum and `variant_slot < 0`. Hello 2312B matched.
Overlay **226.10s**, **s2≠s3**, RSS **1.75GB**. The filter drops
joins that still change compiler wasm. Do **not** retry this
type_name/slot filter.

Tick 86 replaced the per-local recursive inst scan with one inst
walk plus 8-iteration LOCAL_SET edges. Hello 2312B matched.
Overlay **223.55s**, **s2≠s3**, RSS **1.75GB**. The rewrite is not
equivalent to the recursive collect. Do **not** retry this one-pass
slot table.

Tick 87 counted payload-extract GET/other uses in one inst walk
instead of per `enum:` local. Overlay **236.97s**, `s2=s3`, hello
2312B matched, RSS **1.75GB**. Worse wall than tick 80 (208s). The
count tables plus a full inst walk cost more than the old per-local
scans. Do **not** retry this payload-extract count pass.

Tick 88 delayed `stdlib_resolve_normal_call_block` output-vec
construction until the first specialized fallback. Overlay
**237.48s**, `s2=s3`, hello 2312B matched, RSS **1.75GB**. Worse
than tick 80. Do **not** retry lazy resolve output.

Tick 89 dropped `stdlib_resolve_normal_calls` from the overlay
`optimize_module` stub. Overlay **240.83s**, `s2=s3`, hello 2312B
matched, RSS **1.75GB**. Resolve does not change compiler wasm, but
skipping it made emit slower. Do **not** retry stub-skip resolve.

Tick 90 remeasured the tick-80 binary on current HEAD flatten
(no product change). Overlay **239.11s**, `s2=s3` `2ee7c360…`,
RSS **1.77GB**. Same wasm as tick 80's 208s remasure. Treat **239s
loaded** as the current compare floor; 208s is quiet-best, not
today's machine. Ticks 81/83/87–89 walls of 230–241s vs 208s were
not proven product losses against this remasure (still do **not**
retry those edits). Ticks 84–86 stay closed (`s2≠s3`). Tick 91
emit-loop sliding window is closed (wash wall + RSS jump). Tick 92
opcode-first emit dispatch is closed (wash). Tick 93 line-start
source-map index is closed (first cut `s2≠s3`; remasure wash).
Tick 135 skip `compute_fn_source_locations` is closed (hello
mismatch, `s2≠s3`, wash). Tick 136 unroot flatten `source_text`
after location compute is closed (230s s2=s3, RSS wash). Tick 137
intern identical GET/SET/control `MirInst` is closed (241s s2=s3,
RSS wash). Tick 138 in-place local sync is closed (245s s2=s3, RSS
wash). Tick 139 slim `MirBlock` (drop dump-only vecs) is closed
(245s s2=s3, RSS wash).
Tick 94 gcsref run-copy rewrite is closed (wash). Tick 95
producer-index payload/vec scans is closed (wash). Tick 96
has_ref miss memo is closed (wash). Tick 97 skip layout-plan
validator is closed (wash). Tick 98 `local_feeds_return` return-type
guard is closed (wash). Tick 99 feeds_return cache seed is closed
(wash). Tick 100 skip unused producer-index build is closed
(wash). Tick 101 has_ref propagate cap-1 is closed (wash).
Tick 102 i64-scan type_name guard is closed (wash).
Tick 103 skip unused def-site cache is closed (wash).
Tick 104 fold has_ref into first cache walk is closed (wash).
Tick 105 payload-extract container type_name guard is closed (wash).
Tick 106 vec-access enum/option/result guard is closed (wash).
Next slice is still SoA CALL/struct
**no fill** (do not add a new helper family). Compare new walls
to ~239s same-day, not 208s.

Tick 91 reused already-read `MirInst`s in
`emit_function_instructions` (block-local cur/nxt/n2 window)
instead of four `inst_at` peeks per instruction. Overlay
**233.40s**, `s2=s3`, hello 2312B matched, RSS **2.16GB**. Wall
is wash vs today's 239s floor; RSS jumped 1.77→2.16GB. Do **not**
retry this emit-loop window.

Tick 92 routed hot GET/CONST/SET/CALL/struct opcodes in
`emit_mir_inst_ctx` straight to the existing family (skip failed
`try_emit` probes). Overlay **234.23s**, `s2=s3`, hello 2312B
matched, RSS **1.71GB**. Wall is wash vs today's 239s floor.
Do **not** retry this opcode-first dispatch.

Tick 93 replaced per-function `offset_to_line` / `offset_to_column`
scans with one line-start index plus binary search. First cut
treated `offset < 0` as unmapped and did not cap past `len(source)`
(old helpers yield line 1 col 1 / end-of-file). Overlay **237.45s**,
**s2≠s3**, hello 2312B matched, RSS **1.71GB**. Remasure after
matching those edge cases: **237.35s**, `s2=s3`, RSS **1.71GB**.
Wall is wash vs today's 239s floor. Do **not** retry this
line-start index.

Tick 94 rewrote `gc_struct_rewrite_gcsref_slots` to copy plain
runs instead of one-char `concat`, and matched `gcsref` with
`char_at` instead of `substring`+`eq`. Overlay **232.13s**,
`s2=s3`, hello 2312B matched, RSS **1.71GB**. Wall is wash vs
today's 239s floor. Do **not** retry this run-copy rewrite.

Tick 95 walked `#829` producer-index sites for payload-extract and
vec-access storage instead of a full body scan per local. Overlay
**227.35s**, `s2=s3`, hello 2312B matched, RSS **1.72GB**. ~12s
under today's 239s floor but inside same-day noise vs ticks 91–94
(232–234s) and added emit helpers. Do **not** retry this
producer-index scan rewrite.

Tick 96 memoized `local_has_any_ref_assignment` misses (`cached==2`)
so later queries skip the body scan. Overlay **230.63s**, `s2=s3`,
hello 2312B matched, RSS **1.71GB**. Wall is wash vs today's 239s
floor. Do **not** retry this has_ref miss memo.

Tick 97 skipped the O(n²) `mir_gc_layout_plan_valid` check after
binding. Overlay **231.27s**, `s2=s3`, hello 2312B matched, RSS
**1.67GB**. Wall is wash vs today's 239s floor. Do **not** retry
this validator skip. Next slice is still SoA CALL/struct **no fill**.

Tick 98 checked `return_type_name` starts with `vec:` and
`mir_function_returns_enum_open` before `local_feeds_return` (no new
helpers). Overlay **225.24s**, `s2=s3`, hello 2312B matched, RSS
**1.67GB**. ~14s under today's 239s floor but inside same-day noise
vs ticks 91–97 (225–237s). Do **not** retry this return-type guard.
Next slice is still SoA CALL/struct **no fill**.

Tick 99 seeded `local_feeds_return` flags in the existing
`begin_function_local_gc_cache` walk (RETURN arg0 / dest-less
LOCAL_GET). Overlay **239.50s**, `s2=s3`, hello 2312B matched, RSS
**1.67GB**. Same as today's 239s floor; compiler grew ~2KB. Do
**not** retry this feeds_return cache. Next slice is still SoA
CALL/struct **no fill**.

Tick 100 skipped `build_local_producer_index` in
`begin_function_local_gc_cache` (no readers after tick 95 revert).
Overlay **244.45s**, `s2=s3`, hello 2312B matched, RSS **1.65GB**.
~5s over today's 239s floor; wasm 6895312 B (−5.7KB). Do **not**
retry this unused-index skip. Next slice is still SoA CALL/struct
**no fill**.

Tick 101 capped `propagate_has_ref_assignments` at 1 pass (was 4).
Overlay **235.04s**, `s2=s3`, hello 2312B matched, RSS **1.67GB**.
~4s under today's 239s floor; inside same-day noise. Do **not**
retry this has_ref cap-1. Next slice is still SoA CALL/struct
**no fill**.

Tick 102 checked `local_has_result_option_enum_type_name` before
`local_body_has_i64_stack_set` (no new helpers). Overlay
**238.18s**, `s2=s3`, hello 2312B matched, RSS **1.68GB**. Same as
today's 239s floor. Do **not** retry this i64-scan guard. Next
slice is still SoA CALL/struct **no fill**.

Tick 103 stopped filling unused `local_def_block` / `local_def_inst`
caches (`cached_def_*` has no callers). Overlay **239.84s**,
`s2=s3`, hello 2312B matched, RSS **1.66GB**. Same as today's
239s floor; wasm 6899359 B (−1.7KB). Do **not** retry this
def-site skip. Next slice is still SoA CALL/struct **no fill**.

Tick 104 called `local_inst_marks_ref` in the existing cache walk
and dropped `propagate_has_ref_assignments` (was 4 extra passes).
Overlay **238.29s**, `s2=s3`, hello 2312B matched, RSS **1.67GB**.
Same as today's 239s floor. Do **not** retry this has_ref fold.
Next slice is still SoA CALL/struct **no fill**.

Tick 105 skipped `local_payload_extract_storage_type` when
`type_name` starts with `vec:` / `hashmap:` / `struct:`. Overlay
**239.09s**, `s2=s3`, hello 2312B matched, RSS **1.67GB**. Same as
today's 239s floor. Do **not** retry this extract-scan guard. Next
slice is still SoA CALL/struct **no fill**.

Tick 106 skipped `local_vec_access_storage_type` when
`local_has_result_option_enum_type_name`. Overlay **245.15s**,
`s2=s3`, hello 2312B matched, RSS **1.68GB**. ~6s over today's
239s floor. Do **not** retry this vec-access guard. Next slice is
still SoA CALL/struct **no fill**.

Tick 107 ran `local_set_non_string_storage_type` only when
`gc_local_type_name_is_string_or_empty` (sticky String/empty).
Hello 2312B matched tick49. Overlay **204.92s**, **s2≠s3**,
RSS **1.69GB**. s2 6901066 B `20d13a2d…` ≠ s3 6899719 B
`d74afe8e…`. Named non-string locals still need the SET-follow
copy infer. Do **not** retry this copy-scan guard. Next slice is
still SoA CALL/struct **no fill**.

Tick 108 replaced `Vec<MirInst>` with SoA columns. Emit neighbors
and CALL/struct opcode checks read columns; CALL children and
GET/SET/CONST/arith still `inst_at` reconstruct. Hello 2312B
matched tick49. Emit 6.91MB (266.15s). Overlay **timeout 320s**,
RSS **1.08GB**. Same rematerialize tax as ticks 64–68. Do **not**
retry this hybrid reconstruct. Next slice must emit GET/SET/CONST
and CALL/struct **from columns with no `inst_at`** on those
leaves (change existing signatures; do not add a new
`try_emit_*_from_cols` pile).

Tick 109 did that for GET/SET/CONST/REF_FUNC (existing signatures,
column reads, no `inst_at` on those leaves) plus leftover
reconstruct for CALL/arith/convert/control/struct. Hello 2312B
matched tick49. First emit was invalid wasm (`should_skip_store_after_early_tee`
dropped `arg0` — undefined local, stack underflow). Second emit
6.93MB (272.03s) validated. Overlay **timeout 320s**, no output.
RSS not captured (wrapper crashed after the timeout return).
GET/SET/CONST column emit is **not enough** while leftover
families still reconstruct. Do **not** retry this leftover-reconstruct
hybrid. Next slice must emit CALL/struct/arith/convert/control
**from columns with no `inst_at`** (change existing signatures).
Also convert remaining hot `inst_at` in `code_ref_locals_block_scan`
/ infer_dest / payload-extract body scans.

Tick 110 did GET/SET/CONST **and** arith/control/convert from
columns (existing signatures; `inst_at` only after those miss, for
CALL/struct/gc/future). Overlay-hot walks used columns (in-place
resolve, propagate, enum, scan, reachability, ssa `replace_inst`).
Hello 2312B matched tick49. Emit 6.93MB (258.65s) validated.
Overlay **timeout 320s**, RSS **1.45GB**, no output. Arith/control
column emit is **not enough** while CALL/struct still reconstruct.
Do **not** retry this leftover-CALL reconstruct. Next slice must
emit CALL/struct **from columns with no `inst_at`** (change
existing signatures; do not add a `try_emit_*_from_cols` pile).
Also convert remaining CALL children / infer_dest `inst_at`. When
scalarizing `should_skip_store_after_early_tee`, **keep `arg0`**.

Tick 111 re-landed SoA columns and mechanically rewrote ~158 emit
files from `inst: MirInst` to `(block, hid)` so CALL/struct would
read columns with no `inst_at` on the emit path. Tick77 host emit
**256.75s**, 6939700 B, `compilation succeeded (phase 6)`.
`wasm-tools validate` **failed** at func 8133
`lookup_struct_byte_size_in_func`: `expected i32, found (ref null
$type)`. The rewriter replaced `MirInst_op(inst)` with
`MirBlock_op_at(block, hid)` even in loops whose index is `ii`
and whose signature has **no `hid`**. The type checker did not
reject the unbound name; WAT shows a one-arg call to a two-arg
accessor. Hello and overlay were not run. Do **not** retry a
global `inst`→`(block, hid)` rewriter. Convert CALL/struct **by
hand, file-by-file**, passing the real loop index (`ii`) as hid.
Do **not** retry leftover-reconstruct hybrids (ticks 108–110).

Tick 112 re-landed SoA and converted emit functions whose
signature is `inst: MirInst` (function-scoped; skipped any fn
that contains `MirBlock_inst_at`). Tick77 host emit **265.04s**,
6939781 B, validated. Hello 2312B sha256 `1dbf14ca…` matched.
Overlay **timeout 320s**, RSS **1.08GB**, no output. CALL/struct
column emit is **not enough** while `code_ref_locals_*` /
enum-normalize / SSA still `inst_at` (SSA also `replace_inst`
after a full reconstruct). Do **not** retry a global rewriter.
Do **not** retry leftover-reconstruct hybrids. Next slice must
convert remaining overlay-hot scans to columns with the real
loop index (`ii`), especially `code_ref_locals_*` and SSA
in-place column writes (no reconstruct-then-`replace_inst`).

Tick 113 re-landed SoA + function-scoped emit (tick 112 shape) and
converted overlay-hot scans: SSA writes columns in place, `fn_cache`
seed / producer index / `has_ref` use `op_at`/`dest_at`,
`local_feeds_return` / enum-normalize / `local_inst_assigns_ref` /
scalar-producer helpers read columns. Tick77 host emit **287.10s**,
6940767 B, validated. Hello 2312B sha256 `1dbf14ca…` matched.
Overlay **timeout 320s**, RSS **1.18GB**, no output. Column emit +
fn_cache + SSA is **not enough** while `code_ref_locals_block_scan` /
infer_dest / leftover `inst_defines_scalar_i64` / resolve CALL
still `inst_at`. Do **not** retry this partial scan convert. Next
slice must finish remaining overlay-hot `inst_at` (block_scan
helpers, infer_dest, payload-extract body, resolve CALL) using the
real loop index — no global rewriter, no leftover-reconstruct
hybrid.

Tick 114 re-landed SoA + function-scoped emit and finished the
leftover overlay-hot `inst_at`: `infer_ref_local_from_producer` /
block_scan / scan helpers / enum SET / fn_cache marks read columns,
reachability `op_at` then reconstruct only CALL/REF, propagate
`inst_at` only when `dest >= 0`, and overlay resolve writes
`str_val` / `func_id_raw` / `result_types` in place (no output
`Vec<MirInst>`). Tick77 host emit **265.97s**, 6940720 B, validated.
Hello 2312B sha256 `1dbf14ca…` matched. Overlay **finished**
**250.64s**, RSS **1.84GB** — first SoA finish since ticks 69/76 —
but **s2≠s3** (s2 `51a1c6c8…` 6940720 B ≠ s3 `7bb04a85…` 6940630 B,
90 bytes). Wall is worse than the 239s loaded floor. In-place
resolve unblocks overlay; it is **not** byte-identical to the
tick77 reconstruct path. Do **not** retry this exact combo. Next
slice must make column resolve match reconstruct (or keep CALL-only
reconstruct) before measuring again. Do **not** land a 250s wash
even if s2=s3. Still no leftover-reconstruct hybrid and no global
rewriter.

Tick 115 re-landed the tick 114 SoA + leftover scan columns but kept
HEAD resolve semantics: reconstruct only insts with `func_id_raw >= 0`,
then `mir_inst_with_call_target` + `replace_inst` (no whole-block
`set_instructions`, no in-place field writes). Tick77 host emit
**278.96s**, 6941288 B, validated. Hello 2312B sha256 `1dbf14ca…`
matched. Overlay **260.64s**, RSS **1.84GB**, **s2≠s3** (s2
`89be2294…` 6941288 B ≠ s3 `56f6bd4c…` 6941198 B, **90 bytes**).
Same section shape as tick 114: type **+21**, code **−47**, name
**−64**. In-place resolve was **not** the 90-byte cause. Column
scans / function-scoped emit still insert extra GC types. Wall is
worse than 239s. Do **not** retry 114 or 115 as they were. Next
slice must isolate the extra type-section entries (scan infer vs
emit) before another full overlay. Do **not** land a 250–260s wash.

Tick 116 decoded saved tick-115 probes (no product re-land, no
overlay). `wasm-tools print --skeleton`: s2 **2340** types / s3
**2343** (+3, +21 B). Root is **not** leftover TARGET `code_ref_locals`
scans. Three user GC structs interned at lower time widen trailing
`i32` fields to `i64`:

- type 132: `{i32, ref 131, ref 36, i32, i32, i32}` → last 3 `i64`
- type 156: `{i32, ref 131, ref 64, i32}` → last `i64`
- type 181: `{i32, ref 131, ref 114, ref 165, i32}` → last `i64`

Then constructors/accessors intern i64 variants; three leftover
functype entries sit at 2340–2342
(`(ref 11)->(ref 98)`, `(ref 43, ref 11)->(ref 11)`, `(ref 11, i32)`).
`gc_struct_sigs` are registered in `mir/lower/ctx_gc_struct.ark`
(`gc_struct_build_sig` / `gc_struct_scalar_field_suffix`) **before**
emit-time leftover scans. s2 = tick77(HEAD) compiling SoA source →
i32 fields stay i32, so the SoA **source** of the lowerer is fine
under HEAD. s3 = SoA host compiling the same source → i64, so the
**SoA-compiled** typecheck/lower wasm (function-scoped emit) is what
widens. Tick 69/76 already proved SoA + reconstruct leftover emit
can fixpoint. Next slice: leftover column scans **plus tick 69/76
emit** (no function-scoped rewrite of `mir/lower/**`). If that
times out, function-scoped emit **excluding** lower/type intern.
Do **not** retry 114/115. Do **not** land a 250–260s wash.

Tick 117 did **not** re-land SoA. Cheap same-source fixtures on tick77 vs
tick115 hosts **match** (i32-field 2828 B; i64-field 2325 B, type 43 =
`{i32, i64, i64, i64}`). The 90 B / types 132/156/181 are
`DriverFrontendResult` / `DriverTypecheckResult` / `DriverResolveResult`
timestamp fields. `tick11{4,5}_emit.py` set
`ARUKELLT_OVERLAY_EMIT_TARGET=wasm32` so flatten **narrows** those i64
fields to i32 (`_patch_bootstrap_driver_timing`). Overlay scripts set
`wasm32-gc`, which **skips** the patch (#813). s2 and s3 compiled
different source. Not leftover TARGET scans, not function-scoped emit of
the lowerer. Next SoA overlay must set **the same**
`ARUKELLT_OVERLAY_EMIT_TARGET=wasm32-gc` on emit **and** overlay. Do
**not** retry 114/115 as they were. Do **not** land a 250–260s wash.

Tick 118 re-landed the tick 115 SoA product (columns + function-scoped
`(block, hid)` emit + leftover scans + HEAD CALL reconstruct) with
**matching** `ARUKELLT_OVERLAY_EMIT_TARGET=wasm32-gc` on emit and
overlay. Flatten kept driver timestamps as `i64` (no #813 narrow).
`tick112_rewrite_emit.py` still skips `host_intrinsic_gc_body.ark`;
the first emit (250.95s, 6940326 B) failed `wasm-tools validate` in
`try_emit_env_stdio_gc_adapter_body` (`expected i32, found (ref null
$type)`): callees now take `(block, hid)` but the skipped file still
passed a fat `MirInst` as the second argument. `scratch_call_block`
(push CALL into a SoA `MirBlock`, pass `(block, 0)`) fixed that.
Second emit **249.03s**, 6940277 B, validated. Hello 2312B sha256
`1dbf14ca…` matched. Overlay **timeout 320.25s**, RSS **1.62GB**, no
compiler s3 (`bootstrap-out.wasm` left at the hello 2312 B fixture).
s2 `343f0532…`. Same timeout family as ticks 112/113. 114/115 overlay
finish was on a **wasm32-narrowed host** (i32 timestamps); the same
product on official gc flatten does not finish in 320s. Reverted. Do
**not** retry 114/115/118 as they were. Function-scoped SoA emit +
leftover columns is closed on the official flatten. Do **not** land a
250–260s wash. Next slice is not another emit rewrite; it needs live-set
reduction (phase arena / i32 handles) or SoA **without** function-scoped
rewrite (69/76 already finished s2=s3 and was worse).

Tick 119 split `compile_source` so `prepare_emit_request` (frontend +
lower) returns a durable `DriverEmitRequest` (MIR + precomputed WIT
bindings + wit decls only for `--emit wit`) and `finish_emit_request`
calls emit without AST/HIR/resolve/`CheckedProgram` as live locals.
wasm32-gc flatten on emit and overlay. Tick77 host emit **260.67s**,
6903497 B, validated. Hello 2312B sha256 `1dbf14ca…` matched. Overlay
**268.70s**, **s2=s3** `d81bd387…`, RSS **1.68GB**. Worse wall than the
239s loaded floor; RSS wash. Unrooting frontend graphs at the
lower→emit call boundary does not cut Copying-GC scan tax (tick 35:
AST is not the 1.7GB). Reverted. Do **not** retry this pipeline split.
Do **not** land a 250–270s wash. Next slice must shrink the MIR /
TypeTable / function-shell live set (i32 tables that GC does not walk
as record graphs), not another driver local-unroot.

Tick 120 replaced `TypeTable.entries: Vec<TypeEntry>` with columns
(`kinds` / `names` / flattened `param_raws`). Intern/find no longer
allocate `TypeEntry`; `lookup` / `entry_at` still reconstruct one for
the existing API. wasm32-gc flatten. Tick77 host emit **246.66s**,
6903133 B, validated. Hello 2312B sha256 `1dbf14ca…` matched. Overlay
**264.65s**, **s2=s3** `42210430…`, RSS **1.69GB**. Worse wall than
239s; RSS wash. A small type intern table is not the 1.7GB emit live
set, and reconstruct-on-lookup keeps the record tax. Reverted. Do
**not** retry TypeTable columns that rebuild `TypeEntry` on lookup.
Do **not** land a 250–270s wash. Next slice must cut MIR function-shell
/ inst record graphs (or TypeTable column accessors with **no**
`TypeEntry` reconstruct on the emit hot path).

Tick 121 stopped storing a duplicate `ssa_name` on `MirLocal_new` /
SSA reset (`ssa_version < 0` readers use `name`) and stopped cloning
`name` / `type_name` / `ssa_name` on accessors. wasm32-gc flatten.
Tick77 host emit **252.93s**, 6901036 B, validated. Hello 2312B
sha256 `1dbf14ca…` matched. Overlay **258.39s**, **s2=s3**
`b215d89c…`, RSS **1.68GB**. Worse wall than 239s; RSS wash. Extra
local-name strings are not the emit live set. Reverted. Do **not**
retry this ssa-share / accessor-unclone. Do **not** land a 250–270s
wash. Next slice must cut fat `MirInst` / `MirBlock` record graphs
without the closed SoA rewrite families (64–76, 108–118).

Tick 122 set every inst `result_types` to one shared empty vec at
`emit_wasm_module` start (wasm emit does not read those MVTs).
wasm32-gc flatten. Tick77 host emit **247.97s**, 6901910 B, validated.
Hello 2312B sha256 `1dbf14ca…` matched. Overlay **252.55s**,
**s2=s3** `c1e8aba1…`, RSS **1.69GB**. Worse wall than 239s; RSS wash.
Per-inst MVT vecs are not the 1.7GB, and the emit-start walk is extra
mutator work. Reverted. Do **not** retry this emit-start
`result_types` walk. Do **not** land a 250–270s wash. Next slice must
cut the `MirInst` record itself (not a field strip that leaves the
record graph) without closed SoA families.

Tick 123 nested `str_val` / `float_val` / `result_types` into
`MirInstExtras` and rebound trivial insts to one shared extras per
`MirBlock` at push (no SoA, no emit rewrite, `inst_at` still returns
`MirInst`). Default result MVTs are derived from `val_type` until CALL
enrich. wasm32-gc flatten. Tick77 host emit **252.53s**, 6904229 B,
validated. Hello 2312B sha256 `1dbf14ca…` matched. Overlay
**257.76s**, **s2=s3** `d36b15a6…`, RSS **1.64GB**. Worse wall than
239s; RSS wash. Child-field sharing leaves the per-inst `MirInst`
object graph; bind-at-push is extra mutator work. Reverted. Do **not**
retry shared-extras / bind-at-push. Do **not** land a 250–270s wash.
Next slice must remove the per-inst `MirInst` GC object (or cut
function-shell / `LowerCtx` tables) without closed SoA families.

Tick 124 flattened `MirLocal.value_type` into i32 scalars (`type_id_raw` /
`repr` / `nullability`) and shared one empty i32 vec across unused
`preds` / `dom_set` / `dom_frontier` at `MirBlock_new` (COW on first
push). wasm32-gc flatten. Tick77 host emit **253.54s**, 6902315 B,
validated. Hello 2312B sha256 `1dbf14ca…` matched. Overlay
**255.65s**, **s2=s3** `37bb8442…`, RSS **1.69GB**. Worse wall than
239s; RSS wash. Nested local MVT objects and unused block id vecs are
not the 1.7GB. Reverted. Do **not** retry flatten-MVT-on-local or
shared-empty-block-id-vecs. Do **not** land a 250–270s wash. Next
slice must still remove the per-inst `MirInst` GC object (or cut
`LowerCtx` string tables) without closed SoA families.

Tick 125 interned `fn_return_type_names` / `qualified_fn_return_type_names`
/ `local_types` into `LowerCtx.str_intern` + `NameIndex` (i32 ids; no
`MirFunction` fields). wasm32-gc flatten. Tick77 host emit **259.81s**,
6902771 B, validated. Hello 2312B sha256 `1dbf14ca…` matched. Overlay
**269.49s**, **s2=s3** `b4bc477c…`, RSS **1.72GB**. Worse wall than
239s; RSS wash. Narrow type-name intern is not the 1.7GB, and intern +
lookup is extra mutator work. Reverted. Do **not** retry this narrow
`LowerCtx` type-name intern. Do **not** land a 250–270s wash. Next
slice must still remove the per-inst `MirInst` GC object (or cut
function-shell / remaining `LowerCtx` tables that are not unique
type-name strings) without closed SoA families.

Tick 126 intern-shared duplicate `struct_all_fields` / `struct_field_types`
/ `gc_struct_sigs` / `gc_enum_variant_sigs` String handles at write
(NameIndex + `str_intern`; reads stay `Vec<String>`, no i32 id).
wasm32-gc flatten. Tick77 host emit **264.69s**, 6902217 B, validated.
Hello 2312B sha256 `1dbf14ca…` matched. Overlay **246.78s**, **s2=s3**
`37dac859…`, RSS **1.76GB**. Worse wall than 239s; RSS unchanged.
Write-time intern-share of field/sig text is not the 1.7GB. Reverted.
Do **not** retry this field/sig intern-share. Do **not** land a
246–270s wash. Next slice must still remove the per-inst `MirInst` GC
object (or cut function-shell records) without closed SoA families
and without further `LowerCtx` string intern.

Tick 127 precomputed per-function inst-count / sole-callee on `SelfEmitCtx`
(no `MirFunction` fields) so call resolve does not re-walk other bodies,
then cleared each function's instruction vec after its code body was
written. wasm32-gc flatten. Tick77 host emit **275.90s**, 6903079 B,
validated. Hello 2312B sha256 `1dbf14ca…` matched. Overlay **265.53s**,
**s2=s3** `6c897aed…`, RSS **1.77GB**. Worse wall than 239s; RSS
unchanged (peak is still end-of-lower). The extra cache-fill walk is
mutator; late body release does not cut the 1.7GB. Reverted. Do **not**
retry this resolve-cache + post-emit body release. Do **not** land a
250–270s wash. Next slice must still remove the per-inst `MirInst` GC
object during lower (not after emit) without closed SoA families.

Tick 128 packed each function's insts into block word/str/float/result
columns at `ctx_push_*` (no `MirFunction` fields) and lazily unpacked
on `inst_at`. wasm32-gc flatten. Tick77 host emit **278.07s**, 6907423 B,
`compilation succeeded`. Hello **invalid wasm** func 2065 (ref-null
type mismatch). Overlay not run. Rematerializing `MirInst` from packed
columns lost emit-visible type identity. Reverted. Do **not** retry
pack-at-commit + lazy `inst_at` unpack. Do **not** reconstruct a fat
`MirInst` on read. Next slice must keep packed scalars through emit
without rematerialize, and without closed SoA emit-rewrite families.

Tick 129 stored inst payloads in a per-block `MirInstPack` (AoS words +
shared `str_val` / `result_types` refs). `MirInst` is a slim `{hid, pack}`
handle allocated at construct and rebound at push. `inst_at` returns that
handle (no rematerialize, no emit rewrite). wasm32-gc flatten. Tick77 host
emit **275.36s**, 6910195 B, `compilation succeeded`. Host wasm **invalid**
func 2151 (ref-null type mismatch; same class as tick 128). Overlay not
run. Relocating fields off the fat record still drops emit-visible type
identity even when the `result_types` vec is shared. Reverted. Do **not**
retry slim-handle + shared pack / accessor-through-pack. Do **not** retry
another MirInst field-pack (128–129). Next slice must cut function-shell /
`LowerCtx` tables, or a phase-arena that does not relocate `MirInst`
fields, without closed SoA families.

Tick 130 released LowerCtx scratch (hollow ctor keeping `module`), the HIR
view, fn-index strings, mono-view instances, and entry `decl_nodes` /
`source_text` after prune and **before** sync/propagate. wasm32-gc flatten.
Tick77 host emit **246.42s**, 6902731 B, validated. Hello 2312B sha256
`1dbf14ca…` matched. Overlay **257.46s**, **s2=s3** `d0108e1f…`, RSS
**1.68GB**. Worse wall than 239s; RSS wash. Unrooting lower frontend +
ctx tables before propagate does not cut the 1.7GB (tick 119 only unrooted
at emit). Reverted. Do **not** retry this pre-propagate scratch release.
Do **not** land a 246–270s wash. Next slice must cut MIR function-shell
records (`MirFunction` / `MirLocal` / `MirBlock`) without adding
`MirFunction` fields and without relocating `MirInst` fields.

Tick 131 stored `MirLocal` payloads in a module-lifetime `MirLocalPack`
(SoA columns on `MirModule`). `MirLocal` is `{hid, pack}`; `ctx_alloc_local`
/ `ctx_fresh_local` append to the module pack; `ctx_push_func_param` binds
standalone params into the same pack. Accessors clone strings and copy
`MirValueType` (tick 121 / 129 contracts). No `MirFunction` fields, no
`MirInst` relocate. wasm32-gc flatten. Tick77 host emit **243.01s**,
6907280 B, validated. Hello 2312B sha256 `1dbf14ca…` matched. Overlay
**219.90s**, **s2=s3** `4c371039…`, RSS **3.87GB**. Wall beat the 239s
loaded floor but RSS more than doubled: prune drops `MirFunction` shells
while the module pack keeps every lowered local, including unreachable
ones. Reverted. Do **not** retry a module-lifetime local pack that
outlives reachability prune. Do **not** land a 3GB RSS even if wall is
~220s. Next slice must keep function-aligned local columns that die with
the function (parallel `MirModule` table pruned with `functions`, not a
`MirFunction` field), or cut `MirBlock` shells, without relocating
`MirInst` fields.

Tick 132 stored the same `{hid, pack}` `MirLocal` columns on
`LowerCtx.local_pack`, reset at `ctx_begin_function_frame`, and
saved/restored with the closure frame. Pack is rooted only by that
function's local/param handles (and briefly by ctx), so prune can drop
it. No `MirModule` field, no `MirFunction` field, no `MirInst` relocate.
wasm32-gc flatten. Tick77 host emit **247.05s**, 6907292 B, validated.
Hello 2312B sha256 `1dbf14ca…` matched. Overlay **275.50s**, **s2=s3**
`8fc421d7…`, RSS **1.72GB**. Worse wall than 239s; RSS wash. Slim local
handles remain N GC objects (tick 123). Function-scoped columns do not
cut the 1.7GB. Reverted. Do **not** retry a function-scoped slim-handle
`MirLocal` pack. Do **not** land a 246–276s wash. Next slice must remove
the per-local `MirLocal` record from the emit/propagate hot path (scalar
accessors + empty `f.locals`), or cut `MirBlock` shells, without
relocating `MirInst` fields and without a module-lifetime pack.

Tick 133 replaced `MirFunction.locals: Vec<MirLocal>` with a per-function
`MirLocalPack` (existing field type change, not a new field). `local_at`
reconstructs a fat `MirLocal` from columns; params stay fat records.
wasm32-gc flatten. Tick77 host emit **242.14s**, 6905901 B, validated.
Hello 2312B sha256 `1dbf14ca…` matched. Overlay **249.91s**, **s2=s3**
`a2739e05…`, RSS **1.75GB**. Worse wall than 239s; RSS wash. Stored
local records are gone, but reconstruct-on-read plus leftover `params`
objects keep the 1.7GB and add mutator work (same class as tick 64
`inst_at`). Reverted. Do **not** retry locals-as-pack + `local_at`
reconstruct. Do **not** land a 246–270s wash. Next slice must use
scalar local accessors with **no** `MirLocal` reconstruct on emit or
propagate, or cut `MirBlock` / `MirInst` objects, without closed pack
families (128–133).

Tick 134 kept the per-function `MirLocalPack` on `MirFunction.locals` and
rewrote emit/propagate/lower type writes to scalar accessors (no
`local_at` on those walks). Leftover reconstruct remains on enum
normalize, callee lookup, closures, dump, SSA. wasm32-gc flatten.
Tick77 host emit **265.66s**, 6912857 B, validated. Hello 2312B sha256
`1dbf14ca…` matched. Overlay **275.35s**, **s2=s3** `7fe527ee…`, RSS
**1.72GB**. Worse wall than 239s and than tick 133 reconstruct; RSS
wash. Scalar walks plus leftover fat `params` / reconstruct do not cut
the 1.7GB. Reverted. Do **not** retry locals-pack + partial scalar
rewrite. Do **not** land a 246–276s wash. Next slice must cut
`MirInst` / `MirBlock` objects (not another `MirLocal` pack), without
closed SoA / pack / reconstruct families.

Tick 135 skipped `compute_fn_source_locations` and passed an empty
location vec (debug `source_map` custom section only). wasm32-gc
flatten. Tick77 host emit **226.89s**, 6899192 B, validated. Hello
**2308B** sha256 `d8a8bd11…` (**mismatch** vs 2312B `1dbf14ca…`).
Overlay **242.06s**, **s2≠s3** `cd38e5ff…` (6899192) ≠ `7664f6c7…`
(6833243), RSS **1.64GB**. Wash vs 239s. Tick77 host still writes
mapped locations into s2; the new compiler writes unmapped maps into
s3 (~66KB). One-hop overlay from an old host cannot fixpoint this
skip. Tick 93 already showed source-map compute is not the 239s.
Reverted. Do **not** skip location compute or empty the debug
source_map to chase overlay wall. Do **not** land a hello-hash change
without a new hello gate. Next slice must cut `MirInst` / `MirBlock`
objects, without closed SoA / pack / reconstruct / source-map
families.

Tick 136 kept location compute (hello gate) and cleared
`input.source_text` plus passed `String_new()` into LowerCtx so the
flatten source is not a second root through body_emit / propagate.
wasm32-gc flatten. Tick77 host emit **232.62s**, 6900936 B, validated.
Hello 2312B sha256 `1dbf14ca…` matched. Overlay **230.24s**, **s2=s3**
`40334e53…`, RSS **1.64GB**. ~9s vs 239s loaded, same 225–245s
same-day noise band as ticks 95/98. Extra flatten-string roots are
not the 1.7GB. Driver/session still holds the original source.
Reverted. Do **not** retry this isolated source_text unroot. Do **not**
land a 225–245s noise win. Next slice must cut `MirInst` / `MirBlock`
objects, without closed SoA / pack / reconstruct / source-map /
source_text families.

Tick 137 interned identical operand-free control insts (nop / block /
loop / if / else / end / return-void / drop) and identical GET-to-self /
GET-stack / SET records on LowerCtx. Same `MirInst` object is pushed
into many blocks. Not SoA, not field-pack, not reconstruct. wasm32-gc
flatten. Tick77 host emit **233.81s**, 6906588 B, validated. Hello
2312B sha256 `1dbf14ca…` matched. Overlay **241.05s**, **s2=s3**
`ed705e51…`, RSS **1.70GB**. Wash vs 239s. Duplicate GET/SET/control
objects are not the 1.7GB; unique CALL / CONST / arith insts remain.
Reverted. Do **not** retry identical-`MirInst` intern. Do **not** land
a 225–245s noise win. Next slice must cut unique `MirInst` / `MirBlock`
objects, without closed SoA / pack / reconstruct / intern / source-map /
source_text families.

Tick 138 mutated `MirLocal` value types in place during module sync
instead of `copy_with_value_type` on every local (including already
concrete ones). Two syncs at end of lower. Not SoA, not intern, not
a `MirFunction` field. wasm32-gc flatten. Tick77 host emit
**242.17s**, 6901708 B, validated. Hello 2312B sha256 `1dbf14ca…`
matched. Overlay **245.41s**, **s2=s3** `8d9295b7…`, RSS **1.68GB**.
Worse than 239s. Sync copies are not the 1.7GB or the 239s.
Reverted. Do **not** retry in-place typed-MIR local sync. Do **not**
land a 245s wash. Next slice must cut unique `MirInst` / `MirBlock`
objects, without closed SoA / pack / reconstruct / intern / sync-copy /
source-map / source_text families.

Tick 139 dropped dump/SSA-only `phis` / `preds` / `dom_set` /
`dom_frontier` from `MirBlock` (accessors stub empty; overlay emit
never reads them). `idom` scalar kept. Distinct from tick 124 empty-vec
share. wasm32-gc flatten. Tick77 host emit **245.60s**, 6899997 B,
validated. Hello 2312B sha256 `1dbf14ca…` matched. Overlay
**245.18s**, **s2=s3** `a3cdd24f…`, RSS **1.68GB**. Worse than 239s.
Unused block vecs are not the 1.7GB (tick 124). Reverted. Do **not**
retry dump-only `MirBlock` field removal. Do **not** land a 245s wash.
Next slice must cut unique `MirInst` objects, without closed SoA /
pack / reconstruct / intern / sync-copy / MirBlock-vec / source-map /
source_text families.

Tick 140 skipped emitting `SET_from` when the current block's last
inst dest equals the SET source, and rewrote that dest to the SET
target instead. Elides unique SET objects without SoA / pack /
reconstruct / intern. wasm32-gc flatten. Tick77 host emit
**263.42s**, 6901765 B, validated. Hello 2312B sha256 `1dbf14ca…`
matched. Overlay **233.76s**, **s2≠s3** `6df66768…` (6901765) ≠
`846086ad…` (6394549), RSS **1.75GB**. Wash vs 239s. Fusion shrinks
s3 (~508KB) but one-hop cannot fixpoint; unique CALL / CONST / arith
remain the live set. Reverted. Do **not** retry `SET_from` dest
retarget. Do **not** land a 225–245s noise win. Next slice must cut
unique `MirInst` objects, without closed SoA / pack / reconstruct /
intern / sync-copy / MirBlock-vec / source-map / source_text /
SET-from-retarget families.

Tick 141 reused a recent same-`int_val` `CONST_I32` dest in the
current block (window 32, skip if dest written after) from
`lower_int_literal` / `lower_core_literal_value` i32 paths.
Cuts unique user-literal CONST objects without SoA / pack /
intern-identical-inst / SET retarget. wasm32-gc flatten.
Tick77 host emit **273.93s**, 6902944 B, validated. Hello
2312B sha256 `1dbf14ca…` matched. Overlay **286.39s**,
**s2≠s3** `90e8a309…` (6902944) ≠ `7094d2bb…` (6837830),
RSS **1.68GB**. Worse than 239s (scan tax). Unique CALL /
arith / struct remain. Reverted. Do **not** retry block-scoped
`CONST_I32` CSE. Do **not** land a 246–286s wash. Next slice
must cut unique `MirInst` objects, without closed SoA / pack /
reconstruct / intern / sync-copy / MirBlock-vec / source-map /
source_text / SET-from-retarget / CONST-CSE families.

Tick 142 stopped emitting leftover-statement `NOP` in
`lower_statement_like_expr` (the only production `MirInst_nop`
site). Elides unique NOP objects without a scan. wasm32-gc
flatten. Tick77 host emit **269.99s**, 6900458 B, validated.
Hello 2312B sha256 `1dbf14ca…` matched. Overlay **239.64s**,
**s2≠s3** `3ec8e8bb…` (6900458) ≠ `42398761…` (6900350),
RSS **1.68GB**. Floor wash vs 239s; s3 is 108B smaller.
Leftover NOP is not the 1.7GB. Reverted. Do **not** retry
skip leftover NOP. Do **not** land a 239s floor wash. Next
slice must cut unique `MirInst` objects, without closed SoA /
pack / reconstruct / intern / sync-copy / MirBlock-vec /
source-map / source_text / SET-from-retarget / CONST-CSE /
leftover-NOP families.

Tick 143 pushed the existing `MirLocal` into `params` when
`sig_type_name == mir_type_name` (fn / method / self binds).
Keeps a second object only when the names diverge. Not SoA,
not a `MirFunction` field, not a local pack. wasm32-gc flatten.
Tick77 host emit **279.96s**, 6902051 B, validated. Hello
2312B sha256 `1dbf14ca…` matched. Overlay **286.94s**,
**s2=s3** `3b0d173f…`, RSS **1.68GB**. Worse than 239s.
Duplicate param records are not the 1.7GB. Reverted. Do
**not** retry param/local share. Do **not** land a 246–287s
wash. Next slice must cut unique `MirInst` objects, without
closed SoA / pack / reconstruct / intern / sync-copy /
MirBlock-vec / source-map / source_text / SET-from-retarget /
CONST-CSE / leftover-NOP / param-share families.

## Receipts

| Slice | Overlay | s2=s3 | RSS | Notes |
|---|---:|---|---:|---|
| HEAD `fee7d7588` | 242s quiet / 255s loaded | yes | ~1.75GB | baseline |
| tick 64 SoA + reconstruct-on-read | timeout 320s | — | 1.06GB | hello 2164; reverted |
| tick 65 SoA + in-place resolve | timeout 320s | — | 1.06GB | hello 2164; emit/propagate still reconstruct; reverted |
| tick 66 SoA + handle `inst_at` + column walks | killed 315s | — | 1.11GB | tick49 host emitted 6.92MB gc (259s); hello sha256 matched tick49 (2312B fixture); overlay no output; reverted |
| tick 67 SoA + reuse one handle / block | timeout 320s | — | 1.11GB | hello 2312 matched tick49; emit 6.92MB; overlay no output; reverted |
| tick 68 SoA + intern-no-bind + scratch fill | timeout 320s | — | n/a | tick49 host emitted 6.92MB gc (305.71s); hello sha256 matched tick49 (2312B); overlay no output; reverted |
| tick 69 SoA + column GET/SET/CONST_I32 + fill fallback | **263.51s** | yes | **2.21GB** | first SoA overlay finish; emit 6.92MB (286.62s); hello sha256 `1dbf14ca…` (2312B); s2=s3 `70363e94…`; worse wall + RSS jump; reverted |
| tick 70 skip default result_types; enrich CALL only | **269.91s** | yes | **1.72GB** | emit 6.90MB (257.88s); hello sha256 `1dbf14ca…` (2312B); s2=s3 `c0a2f3a6…`; worse wall; RSS wash; reverted |
| tick 71 keep constructor result_types; skip non-CALL enrich rewrite | **259.53s** | yes | **1.71GB** | emit 6.90MB (261.02s); hello sha256 `1dbf14ca…`; s2=s3 `104d5dac…`; wash vs 255s; reverted |
| tick 72 skip all ctx_emit enrich | **264.35s** | **no** | **1.71GB** | emit 6.90MB (260.00s); hello sha256 `1dbf14ca…`; s2 `36b41db9…` ≠ s3 `141a7833…`; CALL enrich required; reverted |
| tick 73 in-place mir_inst_with_func_id_raw | **264.87s** | yes | **1.71GB** | emit 6.90MB (261.77s); hello sha256 `1dbf14ca…`; s2=s3 `68c5ac45…`; worse wall; reverted |
| tick 74 cached `is_gc_target` bool | **266.22s** | yes | **1.76GB** | emit 6.90MB (262.90s); hello sha256 `1dbf14ca…` (2312B); s2=s3 `62d470c0…`; worse wall; reverted |
| tick 75 skip emit audits + unused source_text | **257.30s** | yes | **1.72GB** | emit 6.89MB (272.81s); hello sha256 `1dbf14ca…` (2312B); s2=s3 `9d6c02fd…`; wash vs 255s; reverted |
| tick 76 SoA + arith-from-cols + reused scratch | **300.28s** | yes | **2.07GB** | emit 6.93MB (267.20s); hello sha256 `1dbf14ca…` (2312B); s2=s3 `860730e9…`; worse than tick 69; reverted |
| tick 80 cap propagate at 2 iterations | **208.40s** remasure / **213.36s** first | yes | **1.76GB** | emit 6.90MB (230.46s); hello sha256 `1dbf14ca…` (2312B); s2=s3 `2ee7c360…`; same-day HEAD-today max=8 control **242.08s**; kept |
| tick 81 cap propagate at 1 iteration | **230.55s** | yes | **1.75GB** | emit 6.90MB (228.36s); hello sha256 `1dbf14ca…` (2312B); s2=s3 `0acbd0af…`; worse than tick 80; reverted |
| tick 83 skip post-propagate module sync | **233.98s** | yes | **1.75GB** | emit 6.90MB (223.60s); hello sha256 `1dbf14ca…` (2312B); s2=s3 `db410b76…`; worse than tick 80; reverted |
| tick 84 skip enum multi-variant normalize | **225.66s** | **no** | **1.75GB** | emit 6.89MB; hello sha256 `1dbf14ca…` (2312B); s2 `5993ddff…` ≠ s3 `00e2bfce…`; worse + broke fixpoint; reverted |
| tick 85 skip multi-variant scan on non-enum locals | **226.10s** | **no** | **1.75GB** | emit 6.90MB (229.21s); hello sha256 `1dbf14ca…` (2312B); s2 `b4e79b34…` ≠ s3 `2d66eee6…`; worse + broke fixpoint; reverted |
| tick 86 one-pass enum slot table + SET edges | **223.55s** | **no** | **1.75GB** | emit 6.90MB (229.68s); hello sha256 `1dbf14ca…` (2312B); s2 `3e80dfab…` ≠ s3 `03ab3b43…`; not equivalent; reverted |
| tick 87 one-pass payload-extract GET/other counts | **236.97s** | yes | **1.75GB** | emit 6.90MB (228.24s); hello sha256 `1dbf14ca…` (2312B); s2=s3 `4601f7fa…`; worse than tick 80; reverted |
| tick 88 lazy resolve output vec | **237.48s** | yes | **1.75GB** | emit 6.90MB (227.38s); hello sha256 `1dbf14ca…` (2312B); s2=s3 `2cd20ab6…`; worse than tick 80; reverted |
| tick 89 overlay stub skip resolve | **240.83s** | yes | **1.75GB** | emit 6.89MB (234.02s); hello sha256 `1dbf14ca…` (2312B); s2=s3 `35fff7cb…`; worse than tick 80; reverted |
| tick 90 remasure tick-80 binary | **239.11s** | yes | **1.77GB** | no product change; s2=s3 `2ee7c360…`; same 6.90MB as tick 80; current loaded floor |
| tick 91 emit-loop sliding window (reuse cur/nxt/n2) | **233.40s** | yes | **2.16GB** | emit 6.90MB (241.09s); hello sha256 `1dbf14ca…` (2312B); s2=s3 `8a276b05…`; wash vs 239s + RSS jump 1.77→2.16GB; reverted |
| tick 92 opcode-first emit dispatch (hot GET/CONST/SET/CALL/struct) | **234.23s** | yes | **1.71GB** | emit 6.90MB (230.11s); hello sha256 `1dbf14ca…` (2312B); s2=s3 `1d63dbf1…`; wash vs 239s; RSS wash; reverted |
| tick 93 line-start source-map index | **237.45s** then **237.35s** | no then yes | **1.71GB** | first: s2 `46e17093…` ≠ s3 `a68b6bf1…` (offset edge ≠ old helpers); remasure s2=s3 `cfbb6175…`; emit 6.90MB (229.81s); hello sha256 `1dbf14ca…` (2312B); wash vs 239s; reverted |
| tick 94 gcsref run-copy rewrite | **232.13s** | yes | **1.71GB** | emit 6.90MB (233.02s); hello sha256 `1dbf14ca…` (2312B); s2=s3 `8860ca6f…`; wash vs 239s; RSS wash; reverted |
| tick 95 producer-index payload/vec scans | **227.35s** | yes | **1.72GB** | emit 6.91MB (226.14s); hello sha256 `1dbf14ca…` (2312B); s2=s3 `4f0dfcd6…`; ~12s vs 239s, noise vs 232–234s same-day; extra helpers; reverted |
| tick 96 has_ref miss memo (`cached==2`) | **230.63s** | yes | **1.71GB** | emit 6.90MB (225.29s); hello sha256 `1dbf14ca…` (2312B); s2=s3 `8d454678…`; wash vs 239s; RSS wash; reverted |
| tick 97 skip layout-plan validator | **231.27s** | yes | **1.67GB** | emit 6.90MB (230.39s); hello sha256 `1dbf14ca…` (2312B); s2=s3 `f8202844…`; wash vs 239s; RSS wash; reverted |
| tick 98 `local_feeds_return` return-type guard | **225.24s** | yes | **1.67GB** | emit 6.90MB (228.97s); hello sha256 `1dbf14ca…` (2312B); s2=s3 `883af61d…`; ~14s vs 239s, noise vs 225–237s same-day; reverted |
| tick 99 seed feeds_return in GC cache walk | **239.50s** | yes | **1.67GB** | emit 6.90MB (242.74s); hello sha256 `1dbf14ca…` (2312B); s2=s3 `84fd0bf9…`; wash vs 239s; +2KB wasm; reverted |
| tick 100 skip unused `build_local_producer_index` | **244.45s** | yes | **1.65GB** | emit 6.90MB (248.35s); hello sha256 `1dbf14ca…` (2312B); s2=s3 `a26d11fb…`; +5s vs 239s; −5.7KB wasm; reverted |
| tick 101 cap has_ref propagate at 1 | **235.04s** | yes | **1.67GB** | emit 6.90MB (236.02s); hello sha256 `1dbf14ca…` (2312B); s2=s3 `da4278bc…`; wash vs 239s; reverted |
| tick 102 enum type_name before i64 body scan | **238.18s** | yes | **1.68GB** | emit 6.90MB (241.06s); hello sha256 `1dbf14ca…` (2312B); s2=s3 `3b47a7fc…`; wash vs 239s; reverted |
| tick 103 skip unused def-site cache fill | **239.84s** | yes | **1.66GB** | emit 6.90MB (237.64s); hello sha256 `1dbf14ca…` (2312B); s2=s3 `4e4273c7…`; wash vs 239s; −1.7KB wasm; reverted |
| tick 104 fold has_ref mark into first cache walk | **238.29s** | yes | **1.67GB** | emit 6.90MB (234.23s); hello sha256 `1dbf14ca…` (2312B); s2=s3 `ddd62f69…`; wash vs 239s; reverted |
| tick 105 skip payload-extract on container type_name | **239.09s** | yes | **1.67GB** | emit 6.90MB (238.05s); hello sha256 `1dbf14ca…` (2312B); s2=s3 `48d4e527…`; wash vs 239s; reverted |
| tick 106 skip vec-access on enum/option/result | **245.15s** | yes | **1.68GB** | emit 6.90MB (243.91s); hello sha256 `1dbf14ca…` (2312B); s2=s3 `4d052a53…`; +6s vs 239s; reverted |
| tick 107 copy-scan only on string/empty type_name | **204.92s** | **no** | **1.69GB** | emit 6.90MB (247.24s); hello sha256 `1dbf14ca…` (2312B); s2 `20d13a2d…` (6901066) ≠ s3 `d74afe8e…` (6899719); named locals still need SET-follow; reverted |
| tick 108 SoA columns + CALL opcode then reconstruct | timeout 320s | — | **1.08GB** | emit 6.91MB (266.15s); hello sha256 `1dbf14ca…` (2312B); overlay no output; leftover GET/SET/CONST/`inst_at` rematerialize; reverted |
| tick 109 SoA + GET/SET/CONST columns; leftover reconstruct | timeout 320s | — | n/a | emit 6.93MB (272.03s); hello sha256 `1dbf14ca…` (2312B); overlay no output; leftover CALL/arith/control `inst_at`; first emit invalid (`arg0` dropped); reverted |
| tick 110 SoA + GET/SET/CONST/arith/control/convert columns; leftover CALL reconstruct | timeout 320s | — | **1.45GB** | emit 6.93MB (258.65s); hello sha256 `1dbf14ca…` (2312B); overlay no output; leftover CALL/struct `inst_at`; reverted |
| tick 111 SoA + mechanical `(block, hid)` emit rewrite | — | — | — | emit 6.94MB (256.75s); **invalid wasm** func 8133 `lookup_struct_byte_size_in_func` (unbound `hid` after rewriter); hello/overlay not run; reverted |
| tick 112 SoA + function-scoped `(block, hid)` emit (skip `inst_at` fns) | timeout 320s | — | **1.08GB** | emit 6.94MB (265.04s) validated; hello sha256 `1dbf14ca…` (2312B); overlay no output; leftover scan/SSA `inst_at`; reverted |
| tick 113 SoA + emit + SSA/fn_cache/enum column scans | timeout 320s | — | **1.18GB** | emit 6.94MB (287.10s) validated; hello sha256 `1dbf14ca…` (2312B); overlay no output; leftover block_scan/infer_dest/`inst_at`; reverted |
| tick 114 SoA + leftover scan columns + in-place resolve | **250.64s** | **no** | **1.84GB** | emit 6.94MB (265.97s) validated; hello sha256 `1dbf14ca…` (2312B); first SoA overlay finish since 69/76; s2 `51a1c6c8…` (6940720) ≠ s3 `7bb04a85…` (6940630); worse than 239s; reverted |
| tick 115 SoA + leftover scans + CALL reconstruct `replace_inst` | **260.64s** | **no** | **1.84GB** | emit 6.94MB (278.96s) validated; hello sha256 `1dbf14ca…` (2312B); same 90B shape as 114 (type +21 / code −47 / name −64); resolve was not the cause; reverted |
| tick 116 type-section decode of tick-115 probes | — | — | — | no overlay; s2 2340 vs s3 2343 types; types 132/156/181 trailing i32→i64; extras 2340–2342; lower-time `gc_struct_sigs` / function-scoped emit of lowerer, not leftover TARGET scans |
| tick 117 flatten-target mismatch (no SoA re-land) | — | — | — | tick77 vs tick115 hosts match on i32/i64 fixtures; 90B = emit `OVERLAY_EMIT_TARGET=wasm32` (fields→i32) vs overlay `wasm32-gc` (keep i64); types 132/156/181 = driver timing records |
| tick 118 SoA + emit rewrite + leftover scans + wasm32-gc flatten + `scratch_call_block` | timeout 320s | — | **1.62GB** | first emit invalid (`try_emit_env_stdio_gc_adapter_body`); scratch emit 6.94MB (249.03s) validated; hello sha256 `1dbf14ca…` (2312B); overlay no compiler s3; s2 `343f0532…`; 114/115 finish was wasm32-narrowed host; reverted |
| tick 119 lower→emit frontend unroot (`DriverEmitRequest`) | **268.70s** | yes | **1.68GB** | emit 6.90MB (260.67s) validated; hello sha256 `1dbf14ca…` (2312B); s2=s3 `d81bd387…`; worse than 239s; RSS wash; AST/HIR locals were not the emit scan tax; reverted |
| tick 120 TypeTable columns (`TypeEntry` reconstruct on lookup) | **264.65s** | yes | **1.69GB** | emit 6.90MB (246.66s) validated; hello sha256 `1dbf14ca…` (2312B); s2=s3 `42210430…`; worse than 239s; RSS wash; type intern is not the emit live set; reverted |
| tick 121 MirLocal skip duplicate ssa_name + accessor unclone | **258.39s** | yes | **1.68GB** | emit 6.90MB (252.93s) validated; hello sha256 `1dbf14ca…` (2312B); s2=s3 `b215d89c…`; worse than 239s; RSS wash; local-name strings are not the emit live set; reverted |
| tick 122 emit-start shared-empty `result_types` | **252.55s** | yes | **1.69GB** | emit 6.90MB (247.97s) validated; hello sha256 `1dbf14ca…` (2312B); s2=s3 `c1e8aba1…`; worse than 239s; RSS wash; per-inst MVT vecs are not the emit live set; reverted |
| tick 123 slim `MirInst` + per-block shared extras | **257.76s** | yes | **1.64GB** | emit 6.90MB (252.53s) validated; hello sha256 `1dbf14ca…` (2312B); s2=s3 `d36b15a6…`; worse than 239s; RSS wash; child-field sharing is not the emit live set; reverted |
| tick 124 flatten MirLocal MVT + shared block id vecs | **255.65s** | yes | **1.69GB** | emit 6.90MB (253.54s) validated; hello sha256 `1dbf14ca…` (2312B); s2=s3 `37bb8442…`; worse than 239s; RSS wash; local MVT / unused block vecs are not the emit live set; reverted |
| tick 125 LowerCtx type-name intern handles | **269.49s** | yes | **1.72GB** | emit 6.90MB (259.81s) validated; hello sha256 `1dbf14ca…` (2312B); s2=s3 `b4bc477c…`; worse than 239s; RSS wash; narrow return/local type-name intern is not the emit live set; reverted |
| tick 126 intern-share field/sig String handles | **246.78s** | yes | **1.76GB** | emit 6.90MB (264.69s) validated; hello sha256 `1dbf14ca…` (2312B); s2=s3 `37dac859…`; worse than 239s; RSS unchanged; field/sig intern-share is not the emit live set; reverted |
| tick 127 resolve-cache + post-emit body release | **265.53s** | yes | **1.77GB** | emit 6.90MB (275.90s) validated; hello sha256 `1dbf14ca…` (2312B); s2=s3 `6c897aed…`; worse than 239s; RSS unchanged; late body drop is after the 1.7GB peak; reverted |
| tick 128 pack-at-commit + lazy inst_at unpack | — | — | — | emit 6.91MB (278.07s) succeeded; hello **invalid wasm** func 2065 ref-null type mismatch; overlay not run; rematerialize lost type identity; reverted |
| tick 129 slim-handle + shared pack accessors | — | — | — | emit 6.91MB (275.36s) succeeded; host **invalid wasm** func 2151 ref-null type mismatch (same class as 128); overlay not run; pack-off-record lost type identity without rematerialize; reverted |
| tick 130 pre-propagate LowerCtx/HIR scratch release | **257.46s** | yes | **1.68GB** | emit 6.90MB (246.42s) validated; hello sha256 `1dbf14ca…` (2312B); s2=s3 `d0108e1f…`; worse than 239s; RSS wash; reverted |
| tick 131 module-lifetime MirLocal pack | **219.90s** | yes | **3.87GB** | emit 6.91MB (243.01s) validated; hello sha256 `1dbf14ca…` (2312B); s2=s3 `4c371039…`; wall below 239s loaded but RSS 1.76→3.87GB (prune leak); reverted |
| tick 132 function-scoped MirLocal pack on LowerCtx | **275.50s** | yes | **1.72GB** | emit 6.91MB (247.05s) validated; hello sha256 `1dbf14ca…` (2312B); s2=s3 `8fc421d7…`; worse than 239s; RSS wash; slim handles still N objects; reverted |
| tick 133 MirFunction.locals column pack + local_at reconstruct | **249.91s** | yes | **1.75GB** | emit 6.91MB (242.14s) validated; hello sha256 `1dbf14ca…` (2312B); s2=s3 `a2739e05…`; worse than 239s; RSS wash; reconstruct-on-read; reverted |
| tick 134 locals pack + scalar emit/propagate | **275.35s** | yes | **1.72GB** | emit 6.91MB (265.66s) validated; hello sha256 `1dbf14ca…` (2312B); s2=s3 `7fe527ee…`; worse than 239s and tick 133; RSS wash; leftover reconstruct; reverted |
| tick 135 skip compute_fn_source_locations | **242.06s** | **no** | **1.64GB** | emit 6.90MB (226.89s) validated; hello **2308B** sha256 `d8a8bd11…` (mismatch); s2 `cd38e5ff…` (6899192) ≠ s3 `7664f6c7…` (6833243); wash vs 239s; old host maps vs new unmapped; reverted |
| tick 136 unroot flatten source_text after locations | **230.24s** | yes | **1.64GB** | emit 6.90MB (232.62s) validated; hello sha256 `1dbf14ca…` (2312B); s2=s3 `40334e53…`; ~9s vs 239s, noise vs 225–245s same-day; RSS wash; reverted |
| tick 137 intern identical GET/SET/control MirInst | **241.05s** | yes | **1.70GB** | emit 6.91MB (233.81s) validated; hello sha256 `1dbf14ca…` (2312B); s2=s3 `ed705e51…`; wash vs 239s; RSS wash; unique insts remain; reverted |
| tick 138 in-place typed MIR local sync | **245.41s** | yes | **1.68GB** | emit 6.90MB (242.17s) validated; hello sha256 `1dbf14ca…` (2312B); s2=s3 `8d9295b7…`; worse than 239s; RSS wash; sync copies are not the live set; reverted |
| tick 139 slim MirBlock drop dump-only vecs | **245.18s** | yes | **1.68GB** | emit 6.90MB (245.60s) validated; hello sha256 `1dbf14ca…` (2312B); s2=s3 `a3cdd24f…`; worse than 239s; RSS wash; confirms tick 124; reverted |
| tick 140 SET_from dest retarget onto last producer | **233.76s** | **no** | **1.75GB** | emit 6.90MB (263.42s) validated; hello sha256 `1dbf14ca…` (2312B); s2 `6df66768…` (6901765) ≠ s3 `846086ad…` (6394549); wash vs 239s; s3 ~508KB smaller; RSS wash; reverted |
| tick 141 block-scoped CONST_I32 CSE by int_val | **286.39s** | **no** | **1.68GB** | emit 6.90MB (273.93s) validated; hello sha256 `1dbf14ca…` (2312B); s2 `90e8a309…` (6902944) ≠ s3 `7094d2bb…` (6837830); worse than 239s; scan tax; RSS wash; reverted |
| tick 142 skip leftover-statement NOP | **239.64s** | **no** | **1.68GB** | emit 6.90MB (269.99s) validated; hello sha256 `1dbf14ca…` (2312B); s2 `3ec8e8bb…` (6900458) ≠ s3 `42398761…` (6900350); floor wash vs 239s; s3 108B smaller; RSS wash; reverted |
| tick 143 share params with locals when type names match | **286.94s** | yes | **1.68GB** | emit 6.90MB (279.96s) validated; hello sha256 `1dbf14ca…` (2312B); s2=s3 `3b0d173f…`; worse than 239s; RSS wash; duplicate params are not the live set; reverted |

## Non-goals

- Reopening `#827` or `#097` (rejected Rust `bumpalo` rewrite)
- Language-level GC heap / ADR-002 change
- Coupling into `#824` early body lowering
- Official fixpoint on native-cpp
- Treating wasm-opt or AST cache as the 10s path
