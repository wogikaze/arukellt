---
Status: open
Created: 2026-08-31
Updated: 2026-08-31
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
parallel session-owned bump.

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

## Non-goals

- Reopening `#827` or `#097` (rejected Rust `bumpalo` rewrite)
- Language-level GC heap / ADR-002 change
- Coupling into `#824` early body lowering
- Official fixpoint on native-cpp
- Treating wasm-opt or AST cache as the 10s path
