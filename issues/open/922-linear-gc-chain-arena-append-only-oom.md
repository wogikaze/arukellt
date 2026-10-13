---
Status: open
Created: 2026-10-14
Updated: 2026-10-14
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

Verified: the 6M-allocation churn loop exits 122 on the buggy emitter
and completes (exit 0) on the fixed emitter. Single-activation bursts
larger than the arena (no safepoint between commits) still exit CHAIN
OOM — that remains the documented bounded-arena contract.

## Acceptance

- [ ] s2/s3-equivalent selfhost compile (flat-src) completes without
      silent exit under the fixed emitter.
- [ ] Linear-target fixture set passes; no new runtime-trap regressions.
- [ ] Churn reproducer added as a regression fixture if a lightweight
      variant exists.
