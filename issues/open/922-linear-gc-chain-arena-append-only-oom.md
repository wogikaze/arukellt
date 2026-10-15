---
Status: open
Created: 2026-10-14
Updated: 2026-10-15
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
