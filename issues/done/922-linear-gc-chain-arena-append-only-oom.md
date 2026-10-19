---
Status: open
Created: 2026-10-14
Updated: 2026-10-16
ID: 922
Track: compiler
Depends on: none
Orchestration class: implementation-ready
Orchestration upstream: none
Blocks v{N}: none
Priority: 1
Source: "#851 selfhost-scale verification — s3-worklist flat-src compile exits silently after ~17 min"
Related: "#909, #896, #851, ADR-053"
---

# 922 — linear-GC page-chain arena is append-only; high-churn workloads exit CHAIN OOM

## Summary

The #909 page-table chain that speeds up `mark_candidate` allocates one
{record,next} node per committed record per covered 64KiB page into a
32 MiB bump arena (LINEAR_GC_CHAIN_BYTES = 33554432, ~4M nodes). Nothing
ever reclaims it: sweep filters stale nodes by range but the bump cursor
only moves forward, so the arena bounds *total commits since process
start*, not the live set. A long-running or high-churn program —
including the selfhost compiler compiling itself — exhausts the arena
and dies via `proc_exit(122)` with no diagnostic output.

## Reproduction

Any workload committing > ~4M page-records. Minimal:

```ark
fn main() -> i32 {
    let mut i = 0
    while i < 6000000 {
        let v = Vec::new<i32>()
        push(v, i)
        i = i + 1
    }
    i
}
```

`wasmtime run out.wasm` → exit status 122 (LINEAR_GC_OOM_EXIT_CHAIN) in
under a second. The flat-src selfhost compile (~17 min on s3-worklist)
dies the same way — confirmed by exit code capture, since proc_exit
produces no stderr.

## Root cause

`emit_linear_gc_runtime_commit_top_body` pushes chain nodes on every
commit — including re-commits of records reused off the free list — and
no path resets GLOBAL_LINEAR_GC_CHAIN_CURSOR. The design comment in
constants.ark already acknowledges nodes are append-only ("stale nodes
are filtered ... so the arena bounds total committed page-records
between OOM"); what was not anticipated is how quickly a compiler-scale
workload reaches that bound.

## Fix (implemented)

1. `emit_linear_gc_rebuild_chains` runs at the head of each collection:
   clears the 65536-entry page table, resets CHAIN_CURSOR to the arena
   base (CHAIN_LIMIT - LINEAR_GC_CHAIN_BYTES), and re-registers every
   committed record's page span — bounding the arena to live + per-cycle
   churn.
2. The fn-entry safepoint gate gains a third disjunct: collect when
   CHAIN_LIMIT - CHAIN_CURSOR < LINEAR_GC_PRESSURE_FREE_BYTES, so the
   reclaim fires before a burst can outrun the arena.
3. The same pressure check now also runs at the head of
   `emit_linear_gc_runtime_commit_top_body`: intrinsic inline
   allocations (Vec::new / push etc.) inside a single activation emit
   no fn-entry safepoints, so the safepoint gate alone could never see
   a burst like the reproducer. The record being committed is not yet
   linked into the live list when the check fires, so collection cannot
   sweep it.

Verified: the 6M-allocation churn loop exits 122 on the buggy emitter
and completes (exit 0, ~0.26s) on the fixed emitter — including the
single-activation burst case after fix 3. linear_gc_metadata_pressure /
linear_gc_entry_collect / linear_gc_hashmap_survives all run exit 0 on
wasm32 + wasm-tools validate clean. The residual bounded-arena contract
now covers only genuinely-huge live sets (live page-records alone
exceeding ~4M nodes).

## Acceptance

- [ ] s2/s3-equivalent selfhost compile (flat-src) completes without
      silent exit under the fixed emitter.
- [ ] Linear-target fixture set passes; no new runtime-trap regressions.
- [ ] Churn reproducer added as a regression fixture if a lightweight
      variant exists.

## Update 2026-10-15 — second pathology layer found and fixed

The first fix (rebuild_chains + safepoint/commit-edge pressure trigger)
removed the append-only OOM, but a flat-src compile on the new emitter
still did not terminate: RSS went flat at ~304MB / 100% CPU and the run
had to be killed at 60min. Stack samples (wasmtime -W timeout) showed
the guest inside normal parse/codegen frames, so the burn was inside GC
maintenance, not emit logic. Deeper analysis found two more issues:

1. Commit-edge gate fired collect whenever CHAIN_LIMIT - CHAIN_CURSOR
   < 4MiB. rebuild_chains re-registers the whole live set, so when the
   live footprint itself sits inside the band, every commit re-fired a
   futile full collect (the live set can never shrink below its own
   size). Fix: CHAIN_COLLECT_MARK records the post-rebuild cursor; the
   chain term additionally requires a full band-width of new nodes
   (cursor - mark >= 4MiB) before refiring.
2. The metadata band had the same shape: (ROOT - META < 4MiB) AND
   free-list empty => collect on every fn-entry AND (new) commit. A
   collect that frees nothing leaves both conditions true, so the term
   refires per commit = the same livelock. Bench probe: 8.15M live
   records + 500k churn stalled >300s pre-fix; with a 512KiB
   (PRESSURE/8) METADATA_COLLECT_MARK retry window it completes in 179s;
   a fully saturated set still exits 120 cleanly instead of hanging.
   (PRESSURE-wide hysteresis alone was verified too coarse: it consumed
   the whole band before the first retry and turned the recoverable
   churn case into OOM 120 at 77s.)

Also in the same commit: the commit-edge gate now mirrors the entire
fn-entry gate (watermark OR meta-pressure OR chain-pressure) instead of
only chain pressure, and both arenas were enlarged to 128MiB
(metadata ~8.39M records, chain ~16M nodes) because a compiler-scale
workload holds several million live records.

Verification: fixtures target_capability 6/6 + collections 20/20 +
generics_v1 23/23 + functions 20/20 + for_loops 4/4 +
match_extensions 11/11 + enums 13/13 all executed PASS on a freshly
built s2-runtime; probes live_edge/meta_band/meta_edge complete.
Commits 73d5bfd2e + 9bfa2b280.

## Update 2026-10-15 (b) — flat-src s4 attempt: OOB memory fault at MIR lowering

A full s4 attempt (s3-final.wasm, gc-host-emitted wasm32 build with the
#922 fixes) reached MIR lowering after ~2.5h of parse/check, then died
on an out-of-bounds linear-memory read — the first concrete evidence
that a guest-side UAF window remains:

    memory fault at wasm address 0x211aca80 in linear memory of size 0x205e0000
    name_index_find_slot <- name_index_get <- type_table_find_existing
    <- TypeTable_intern <- mir_value_type_from_type_name <- ...

get_unchecked(index.buckets, slot) read ~11.3MB past the memory end —
either the NameIndex record or its buckets Vec was swept while still
reachable, i.e. a live ref existed only in a wasm operand-stack slot or
unmirrored scratch local when the commit-edge collect ran. The fn-entry
safepoint is safe by construction (empty operand stack); the commit-edge
collect added for #922 fires mid-expression where that invariant does
not hold.

A second attempt (s3-v2.wasm -> s4-v2.wasm) died during module loading
with 59 "failed to load module" errors — diagnosed as environmental:
verify quick ran _prepare_flattened_selfhost_source_locked (which does
_remove_tree(flat-src) under flat-overlay.lock) at 08:12, while the s4
guest was mid-load of the same tree. Not a compiler defect; artifacts
dropped into flat-src do not survive a re-flatten.

## In flight — amplified-collect detonation build

To localize the unrooted window without 2.5h per datapoint, a debug
flat-src snapshot was emitted with:

- LINEAR_GC_COLLECT_THRESHOLD_BYTES 64MiB -> 64KiB (both collect
  gates fire ~1000x more often)
- sweep poison: first payload word of each swept record overwritten
  with 0x7EADBEEF so a stale ref faults at a recognizable address

s3-debug.wasm (11.4MB) contains both (verified in the binary) and is
running the same flat-src -> wasm32 compile. If a mid-expression ref
window exists and is reachable at compiler scale, it should detonate
far earlier and name the consumer in the backtrace. Repro status:
50k-insert HashMap churn + full readback on wasm32 passes clean on the
normal build (.build/repro_922.ark), so the window needs either
compiler-scale allocation density or a specific emitter path.

## Update 2026-10-15 (c) — deterministic emitter miscompile found via #825

While repairing the AST cache codec (#825) a **100% deterministic**
wasm32-linear emitter defect surfaced that is almost certainly the same
family as the UAF above:

- `fn f(...) -> (i32, i32)` ending in a bare tuple tail `(a, b)` emits
  `local.get a; local.get b; drop; drop; unreachable` plus the real
  tuple-build in dead code — i.e. tail-position tuple literals are
  scrapped instead of returned.  Every bare-tuple-tail function traps at
  runtime.  `return (a, b)` emits correctly.  (Compare `read_i32` vs its
  early-return path in .build/astc-probe.wasm, func ~706.)
- With certain shapes the emission is worse than a trap: operand values
  are lost mid-expression and the function ships *invalid* wasm
  ("expected i32 but nothing on stack" at validation).  In one instance
  the `b3` local's store landed after the `unreachable` — a scratch-local
  def/use inversion under the shadow-stack spill pass (locals 69+).
- Confirmed on both current s2 and the pinned bootstrap — pre-existing,
  not introduced by #825.

This gives a tiny, fast repro for the class of bug suspected in the s4
UAF: **the linear backend's value tracking around bare tuple tails /
scratch spill is unreliable** — a live value can be dropped or
re-materialized after a terminator.  If the same machinery unroots a ref
across a collect safepoint, that is the UAF window.  The codec now
avoids the pattern; the emitter fix itself belongs to this issue (or the
emitter-audit slice of #851).

Minimal repro sketch: any `fn -> (i32,i32)` whose body ends in
`(x, y)` without `return`, compiled with --target wasm32.  Every
existing codec function hit it; nothing in the fixture suite exercises
bare tuple tails on wasm32 (all tuple-returning code paths in src/
either return early explicitly or were never reached at runtime — the
AST cache was disabled for exactly this class of breakage).

## Update 2026-10-15 (d) — the "emitter defect" was a parser defect; a live in-tree miscompile found

Correction to (c): the bare-tuple-tail garbage was not emitted by the
wasm32 backend — the **parser** folded the tail tuple into call
arguments on the previous line's infix expression (#924, fixed).  With
all-bound argument names the fold resolved cleanly and the typechecker
accepted a call of an i32 callee (#926), so malformed call nodes
reached emission.  wasm32-gc emitted validating-but-wrong code; wasm32
emitted the drop/unreachable shape — hence the target asymmetry.

Two consequences for this issue:

1. **A real miscompile shipped inside the compiler.**
   `src/compiler/wasm/intrinsic_vec_set_gc_layout.ark::vec_set_vec_flags`
   ends with `let is_vec_sref = vec_type == SelfEmitCtx_vec_struct_ref_type(ctx)`
   followed by a bare 4-tuple tail.  Under the parse bug it compiled as
   `(vec_type == f(ctx))(is_vec_string, is_vec_i64, is_vec_f64, is_vec_sref)`
   — `is_vec_sref` was produced by calling an i32/bool as a function.
   Every selfhost binary built with the buggy parser therefore emitted
   **wrong GC layout flags for vec_set on reference-typed Vec
   elements**.  The name index, type table, and signature spine are
   Vec<String>/Vec<record> structures — the same element class this
   function misclassifies.

2. **The s4-debug detonation landed.**  s3-debug.wasm (64KiB collect
   threshold + 0x7EADBEEF sweep poison) crashed after ~1h20m in
   `name_index_get` → `type_table_find_existing` → `TypeTable_intern`
   ← `vt_to_mir_value` ← `sig_reg_register_fn` ←
   `mir_build_signature_spine_from_view`: out-of-bounds read at
   0x29f38fb8 against a 0x20370000 linear memory — a wild index, i.e. a
   stale/poisoned slot value used as an index into the name index.
   Deterministic, during MIR signature construction — squarely the
   sref-vec corruption class that (1) describes.

Next probe: rebuild the s2->s3->s4 chain with the #924-fixed parser and
re-run the same flat-src compile.  If the detonation no longer
reproduces, the miscompiled `vec_set_vec_flags` was the corruptor and
this issue's "unrooted mid-expression window" hypothesis narrows to
"sref vec_set layout flags" rather than a generic emitter defect.

## Update 2026-10-16 — third pathology: quadratic free list + bootstrap deadlock

With the #924 parser fix in, the next flat-src attempt (hopB / s3-newrt)
was not a cycle or a UAF — a `wasmtime -D coredump` of the stalled run
shows ALLOC_HEAD = 1,469,551 nodes and FREE_HEAD = 781,122 nodes with an
average free span of ~19 bytes.  Three compounding defects in the linear
runtime:

1. **Unbounded first-fit scan.**  Every `AstNode_new` alloc (func 11188)
   walks the entire free list until a fit; at ~781K sliver nodes each
   alloc is O(781K) pointer chases — the compile is effectively infinite.
2. **Remainder leak.**  A fit truncated `hi = lo + size` and dropped the
   tail bytes outside every record — permanently leaking the remainder,
   which is why the list fills with ~19B slivers.
3. **commit_top destroys free spans.**  Popping a free record for a new
   alloc-record slot consumed whatever span sat at the head, including
   large reusable spans.

Fix implemented (uncommitted->committed in `fix(#922)` patch): the free
list is split into three size classes — SMALL (<=32B), MID (<=512B),
HUGE — as globals 2/14/15.  Sweep classifies each freed record once by
its full recorded span; alloc runs a same-class first-fit bounded to 64
nodes and escalates to the next larger bucket whose head is a
guaranteed fit (no walk); a reused span is adopted whole (no truncation
leak); commit_top pops small->mid->huge before bumping metadata, so
slivers are recycled for record slots and large spans survive.  The
fn-entry safepoint and commit-edge gates now require all three buckets
empty for the free-list term.  Verified: linear_gc_* fixtures +
collections vec_string/hashmap compile via s2-runtime -> wasm32,
wasm-tools validate clean, run exit 0.

## Update 2026-10-16 (b) — bootstrap deadlock blocks a corrected binary

A compiled compiler's codegen = its own source, but its *runtime* = the
emitter of its parent.  No live binary currently pairs the #922-fixed
allocator codegen with a usable runtime:

- `s2-runtime.wasm` (new codegen, OLD runtime) stalls on flat-src at the
  #927 fixpoint-mark cliff — RSS flat ~158MB, 10min/35min timeouts.
- `gchost-v5m32.component.wasm` (GC runtime, OLD codegen) compiles
  flat-src in ~3.5min but emits OLD-runtime wasm32 binaries that grind
  on the quadratic free list (s3-newrt killed at 30min).
- `gclean-v2.component.wasm` (GC runtime + NEW codegen vehicle) is
  **invalid wasm**: func 6888 `edge_push_caller_segment` fails
  validation at offset 0x2b9060 — `local.get $_t39` yields
  `(ref null 25)` (Vec<Vec<i32>>) where `local.set $_t23` expects
  `(ref null 10)` (Vec<i32>).  gc-emit type-mapping bug in
  `push(table.caller_lists, list)` grow path
  (`mir/lower/edge_discovery.ark:1343` ->
  `wasm/intrinsic_vec_push_gc.ark` emit_gc_push_store_finish/growth).
  Pre-existing emitter defect, unrelated to the allocator fix.

Next: fix the gc-emit Vec<Vec<T>> push typing bug -> rebuild gclean-v2
-> it emits new-runtime wasm32 binaries in minutes -> flat-src compile
bounded-check -> pin refresh candidate (unblocks #927 and the #851
flat-src acceptance run).


## Update 2026-10-16 (c) — coredump forensics: the "UAF" is a deterministic emitter arg/type confusion, not a GC sweep

Instrumented s2-linear-dbg*.wasm builds (root-push/update calls inlined; debug
globals 16/17/18 capture find_slot's name/index/last key) die at the
same deterministic address every run: memory fault at 0x2de1c997 inside
name_index_find_slot <- name_index_get <- type_table_find_existing <-
TypeTable_intern <- mir_value_type_from_type_name <- vt_to_mir_value <-
sig_reg_register_fn <- mir_build_signature_spine_from_view.

Coredump state (/tmp/dbg3.core + dbg3segs.pkl/dbg3mem.bin/dbg3spans.pkl,
scripts /tmp/dbgforen*.py):

- index = 0x20178c45 = a fully consistent NameIndex
  {capacity:1024, buckets*:{len:1024,cap:1024,data:0x221d2255},
  keys*:{len:315,cap:512,data:0x221d5b81}, values*:{315,512,0x221d6381}}.
  No bucket entry > 315; buckets[381]=291 -> keys[290]="SelfEmitCtx".
- name = 0x1dd63cc1 = a 44-byte record {len:0, cap:8, data:self+12} —
  byte-identical to the Vec::new<TypeId>() emission (alloc 44 =
  12B header + 32B inline buffer) inside mir_value_type_from_type_name.
  i.e. the name param slot received the freshly-created type_param_ids
  empty Vec pointer, not a String data pointer.
- Exact fault math: len(name) reads u32(name-4) = 0x201719ad
  (the word before the record, not a real length), then
  byte_at(name, name_len/2) = 0x1dd63cc1 + 0x201719ad/2 = 0x2de1c997
  — the fault address to the byte. Deterministic across dbg/dbg2/dbg3.
- name's record is LIVE and referenced: two parallel big element buffers
  (0x20b004a9 32KB, 0x20b284a9 64KB — Vec<Vec<..>> element arrays) hold
  it at the same slot (off +22544), and the adjacent 12-byte record at
  0x1dd63ced — shape {tag:1, f4:name, f8:name} — matches a TypeInfo
  whose name and type_args fields both point at the same Vec record
  (field aliasing).

Verified clean at every level of the forwarding chain in dbg3.wat:
name_index_get forwards (index,name) verbatim, type_table_find_existing
passes (name_head, name), TypeTable_intern passes (name,type_param_ids)
in order, and the STRUCT branch of mir_value_type_from_type_name stores
 into  and the fresh vec into / separately.
So the Vec arrived *as*  — corruption is upstream of the
find_slot call: either (a) a TypedFn/TypeInfo produced during typecheck whose
name field holds a Vec record (e.g. {tag, name:String_new(),
type_args:Vec::new()} fields aliased to the same temp), or (b) the
push(returns, vt_to_mir_value(table, ret_vt, ret_type_name)) staging in
sig_reg_register_fn leaked the returns/Vec::new scratch into the
type_name arg slot — the empty-Vec shape matches both.

Note the existing comment at signature_registry_entry.ark:37:
"TypedFn.TypeInfo values are unsafe across the duplicate TypeInfo/TypedFn
structs in the flat selfhost overlay" — this crash is exactly that class
surfacing as an OOB on wasm32.

Next: minimal repro — a record literal whose fields are two call results
(String + Vec), or the sig_reg path on wasm32; check whether the emitted
field stores alias a shared scratch temp. Related: the gc-emit
Vec<Vec<T>> push typing bug (gclean-v2 func 6888 edge_push_caller_segment
invalid: local.get  (ref 25, Vec<Vec<i32>>) into local.set 
(ref 10, Vec<i32>)) is a *separate* pre-existing defect blocking the
gclean-v2 "new codegen + GC runtime" bootstrap vehicle for the flat-src
acceptance run.

## Update 2026-10-19 — the gc-emit Vec<Vec<T>> typing bug is fixed

Root cause: nested-vec type names collapsed to bare "vec:Vec" at three
producer sites, so downstream consumers fell back to a vec:String guess
and typed Vec<Vec<i32>> element/dest slots with the wrong GC struct.

Fix (all verified on rebuilt s2-runtime):

- mir/lower/return_typeinfo_names.ark: TY_VEC elements now recurse →
  vec:vec:i32 (was vec:Vec); vec:Vec kept as the no-name fallback.
- corehir/type_ann_param_name.ark: type_ann_shape_param_type_name
  recurses for Vec-annotated elements.
- corehir/param_shape_local.ark: shared vec_ann_local_type_name helper;
  fn_param_local_type_name and method_param_local_type_name both use it
  (fnref/tuple element cases unchanged).
- wasm/intrinsic_vec_access_gc.ark: resolve_container_elem_gc_type is now
  pub and maps the Vec<T> angle form through the vec: spelling.
- wasm/code_ref_locals_infer_dest.ark: is_vec_push_call_callee case in
  infer_ref_local_from_producer resolves the receiver's storage type via
  find_call_source_local / wasm_ref_type_idx_for_local (fallback
  resolve_vec_type_for_call), so if-join temps fed by differently-typed
  push dests resolve both arms and under-declare as (ref null any).
  infer_get_unchecked_elem_from_vec_arg maps vec:-prefixed elements to
  the nested vec family's GC layout.
- lexer/char.ark + wasm/intrinsic_vec_set_gc_layout.ark: explicit return
  on bare tuple tails (vec_set_vec_flags was the in-tree tuple-tail
  miscompile corruptor per item (d)).

vec:vec:i32 maps to the same structref-vec layout as the old vec:Vec
spelling — the ABI is unchanged; mono keys improve (Vec<Vec<String>> now
specializes on Vec<String> instead of a bogus Vec<i32>).

Verification (s2-runtime built 2026-10-19 03:35):
- .build/p922/gcrepro/vv_push.wasm, .build/p922/vv/r2.wasm:
  wasm-tools validate clean (previously invalid).
- fixtures run: collections/vec_vec_i32 → 4, vec_vec_i32_get → 9,
  vec_vec_string_get → ok (all match .expected).
- target_capability gc_if_join_mixed_ref_temps (wasm32-gc+p2) and
  gc_get_unchecked_dest_decl (wasm32 linear) compile clean.

The speculative i32-scratch root-mirroring diff (~50 wasm emitter files,
chasing the disproved mid-expression-sweep hypothesis) was reverted;
archive at .build/p922/scratch-root-mirror-speculative.patch if the
coverage question resurfaces.

Next: rebuild gclean-v2 (gc runtime + new codegen vehicle, previously
invalid at func 6888 edge_push_caller_segment — the bug fixed here),
then bounded flat-src compile check and pin refresh to unblock #927 and
the #851 acceptance run.

## Update 2026-10-19 (b) — real mark-phase miss found and fixed: boundary-handle shadowing

A dump-the-heap-graph diagnostic emitted into collect (post-mark,
pre-sweep verifier over the live list) proved a genuine mark-phase miss
in mark_candidate: a marked LoadState-shaped record held a word
equal to a dead record's lo (ref=0x08040100, word==dead.lo).

Mechanics: the page-chain walk returned on the *first* record whose
range matched the candidate. Two edges combined to make that wrong:

1. (hi-1)>>16 page-span registration (both in commit_top and
   rebuild_chains) meant a {len@-4}-style handle equal to hi was only
   reachable while hi sat inside the record's page; a hi on a page
   boundary dropped the registration for the page the handle actually
   lives on.
2. With the range check closed to [lo,hi] (required, because minimal
   4-byte records produce handle == hi), a boundary word
   cand == prev.hi == next.lo matched the *earlier* chain node and
   returned before reaching the record that actually owns it — so the
   owner stayed unmarked and was swept while live.

Fix: mark_candidate keeps the closed interval, marks on boundary hits
(cand == lo or cand == hi) but keeps walking the chain, and returns
early only on strict interior hits (lo < cand < hi), which are unique
by construction since live payload ranges are disjoint. Both
registration paths now cover the hi page (hi>>16, not (hi-1)>>16).

Refinement (2026-10-19 (c)): the boundary mark is asymmetric. A cand
equal to lo always marks (head pointers are the common handle shape).
A cand equal to hi only marks minimal {len@-4} records (hi - lo == 4,
the one layout where hi is a valid handle); for larger spans the word
is really the adjacent record's lo and marking there only retains a
dead predecessor. On bump-adjacent heaps the unrestricted hi mark
turned every scanned head pointer into extra phantom retention.

Deterministic verification harness (all since reverted):
emit_linear_gc_verify_transitive dumped {dead, dead_lo, dead_hi,
ref, ref_lo, ref_hi, slot_addr, word} plus the full page chain to fd 2
and proc_exit(99) on the first marked-referrer-to-dead-record edge.
Pre-fix builds fired it immediately; post-fix the same workloads run
clean. New fixture linear_gc_boundary_handles constructs the
adjacent-record collision deterministically (string header immediately
followed by a Vec::with_capacity external backing whose lo equals
the header's hi) — fails on the half-fixed (closed-interval +
early-return) emitter, passes on both the pre-closed-interval and the
fixed emitters. The full sweep-miss class is also exercised by the
LoadState name/slot rehash loop at the tail of the same fixture.

The earlier ~50-file i32 scratch-root mirror diff remains reverted and
archived at .build/p922/scratch-root-mirror-speculative.patch: the
verifier above showed the observed corruption came from the mark phase
dropping transitive references, not from unmirrored locals, and the
post-fix runtime passes the reproducer without it.

## Update 2026-10-19 (c) — scale validation: flat-src compile reaches
## fixpoint after boundary refinement + capacity + two latent OOB bugs

With the mark-phase fix in place, the flat-src selfhost compile
(lin_comp, wasm32 + linear GC, compiling src/compiler/main.ark) ran
past all earlier corruption points and exited ROOTS OOM (121) at the
MIR-lowering stage. An instrumented OOM encoded the arena fill as the
exit status: the record arena was ~127 MiB of the 128 MiB capacity, so
the OOM was genuine saturation — not a leak (the root stack uses ~1 MiB
and is balanced by frame_enter/leave on every return path).

Three further changes were needed to complete the compile:

1. Boundary-hit refinement: marking on every cand == hi over-retained,
   because a scanned head pointer (vec handle, backing ptr, struct ref
   — all exact `lo` values) also matched the adjacent predecessor's hi
   on bump-adjacent heaps, keeping its dead subgraph alive. The mark
   now treats the two edges asymmetrically: cand == lo always marks;
   cand == hi only marks minimal {len@-4} records (hi - lo == 4, the
   one layout where hi is a valid handle). Either way the walk never
   returns early on a boundary — strict interior hits remain the only
   early exit.
2. Arenas doubled to 256 MiB each (LINEAR_GC_METADATA_BYTES /
   LINEAR_GC_CHAIN_BYTES): the measured ~127 MiB watermark at MIR
   lowering gives the ~16.7M-record arena roughly 2x headroom.
3. Two latent OOB bugs the larger heap_start exposed:
   - initial memory was a fixed 512 MiB; heap_start (~525 MiB now)
     crossed it, so the first fn-entry root_push wrote out of bounds.
     emit_memory_section now declares max(512 MiB, ceil(heap_start))
     for linear targets — arena-reserved pages stay untouched so the
     cost is address space only.
   - emit_fs_read_loop / emit_wasi_read_stdin_string wrote FS_BUF_SIZE
     bytes into the bump heap top before advancing it; the bump ensure
     only covers the pointer byte, so a heap top within 4096 of the
     memory edge faulted in the WASI region check. Both loops now
     pre-grow headroom before the host write (this also repaired
     emit_heap_pre_grow_headroom, which had a latent `(result i32)` if
     that produced nothing — dead code until these call sites).

Result: lin_comp_v28 (wasm32 + linear GC, built by the fixed emitter)
compiles flat-src main.ark to completion in ~30 min (EXIT=0), and the
output is byte-identical to the compiler binary that produced it —
the s2==s3 fixpoint for the linear target (ADR-029), the strongest
available evidence that marking no longer corrupts live records at
selfhost scale. Fixture linear_gc_boundary_handles passes under the
final emitter and still fails on the half-fixed (closed-interval +
early-return) emitter.

Landing note (2026-10-19): the fixture's original draft called the
generic `clone(Vec<T>)` intrinsic, which under wasm32-gc resolves the
bare `clone` callee to `String::clone` and emits a module that fails
`wasm-tools validate` (reproduces on the pinned bootstrap, so it is
a pre-existing emitter bug — filed as #928). The landed fixture keeps
the same heap shape via element-wise clone helpers instead; it passes
the wasm32-gc parity path, the wasm32 linear run, and still traps on
the half-fixed emitter.
