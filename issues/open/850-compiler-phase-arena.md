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
| wasm32-gc + wasi-p2 (Copying) | **208s remasure / 213s first** (tick 80) | was 242s quiet / 255s loaded; s2=s3, RSS ~1.76GB |
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
- [ ] Receipt: wall + RSS vs tick 80 ~208–213s / same-day HEAD-today 242s / ~1.76GB
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
Do **not** retry max=1. Next slice is still SoA CALL/struct **no fill**
(do not add a new helper family).

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

## Non-goals

- Reopening `#827` or `#097` (rejected Rust `bumpalo` rewrite)
- Language-level GC heap / ADR-002 change
- Coupling into `#824` early body lowering
- Official fixpoint on native-cpp
- Treating wasm-opt or AST cache as the 10s path
