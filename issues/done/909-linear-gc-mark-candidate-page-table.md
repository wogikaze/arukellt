---
Status: done
Created: 2026-10-10
Updated: 2026-10-10
ID: 909
Track: gc
Depends on: none
Orchestration class: implementation-ready
Orchestration upstream: none
Blocks v{N}: language-v2
Priority: 1
Source: 908 lane profiling 2026-10-10 - mark_candidate at 99.9% of stalled compiles
---

# 909 - Linear GC mark_candidate O(records) scan makes collection quadratic

## Summary

The wasm32 fallback runtime's mark_candidate walks the allocation-record
linked list for every candidate pointer. collect() calls it once per root
slot word and once per payload word of every marked record, in a fixpoint
loop - so one collection costs O(marked_words x records). Once a compile
passes the 64 MiB watermark or the 4 MiB metadata-pressure margin, every
function-entry safepoint runs a collection that can take minutes at
selfhost-compiler record counts. Fresh s2 builds appeared to hang
indefinitely while emitting ~300 monomorphic bodies (perf: 99.9% inside
mark_candidate).

## Fix implemented: 64 KiB page table + per-page record chains

Memory layout becomes
[data][page table 256 KiB][chain arena 32 MiB][metadata arena 64 MiB][heap],
with three new globals (indices 8/9/10) holding the page-table base, the
chain-node bump cursor, and the chain-arena limit.

- commit_top pushes one {record, next} node (8 bytes) onto the chain of
  every 64 KiB page the committed range touches
  (lo>>16 .. (hi-1)>>16). Chain nodes are bump-allocated and never freed.
- mark_candidate computes candidate>>16, follows that page's chain, and
  checks the record's CURRENT [lo,hi) fields. Cost per candidate is
  O(records covering that page) instead of O(all committed records).
- Stale nodes (record reused for a different range, or record dead and
  free-listed) are filtered by the [lo,hi) range check: a page's chain
  may contain records that no longer cover it, but a covering record
  always appears in the chain of every page it covers, since commit_top
  registers the full current range unconditionally.

## Rejected alternative: binary search over the metadata arena

A first attempt binary-searched [METADATA_BASE, METADATA_CURSOR) assuming
records stay sorted by lo. Free-list reuse rewrites rec.lo in the stale
slot position, which breaks the ordering; the search then returns the
wrong record and live payloads were freed early. An s3-equivalent build
under that runtime corrupted memory and crashed inside vec_has. Reverted;
the page table does not depend on record ordering at all.

## Known limitations

- A stale chain node can still mark a dead record whose last range covers
  the candidate. This is the same conservative-marking false-positive
  class the fallback already tolerates (bounded leak, mark cleared next
  sweep) - not a corruption vector.
- The chain arena is bump-only across collections. 32 MiB holds ~4M
  page-registrations; exhaustion exits 122 (explicit OOM, not a trap).
  Workloads that churn many collection cycles could need arena recycling
  later.
- mark_transitive still re-scans to fixpoint; a worklist would make a
  single pass sufficient (optional, not required by current workloads).

## Verification

- Fresh s2 (ARUKELLT_SELFHOST_WASM=.build/selfhost/arukellt-s2-runtime.wasm)
  emits the new layout: globals 8/9/10 initialized to page-table base,
  chain cursor, and chain limit; the emitted metadata cursor equals the
  chain limit, so arena ordering is consistent.
- tests/fixtures/target_capability/linear_gc_{entry_collect,hashmap_survives,metadata_pressure}
  compile via the fresh s2 and run clean (exit 0) under wasmtime.
- gcstress probe (8000 live strings + 400 x 256KiB churn, multiple
  collections, all assertions pass): 6.59s with the list walk -> 0.80s
  with the page-table chains on the identical workload.

## Remaining

- [x] Fresh s2 rebuild emits the page-table runtime;
      tests/fixtures/target_capability/linear_gc_* compile and run clean.
- [x] gcstress probe re-run against a program emitted by the new s2:
      0.80s vs 6.59s on the identical workload.
- [ ] s2 -> s3 build carries the new runtime into the runnable compiler;
      confirm previously-stalling std::seq probes compile promptly under
      s3. In progress via a big-arena + collect-nop binary patch on a
      scratch copy of s2-runtime.wasm (the s2's own baked runtime is the
      old one until the next re-pin; see #911).

      Done 2026-10-10: an s3-equivalent compiler was produced by running
      the fresh s2 under a scratch binary patch that enlarges the
      metadata arena to ~2 GiB and no-ops the old collect body (zero
      collections during the build; the old runtime cannot be replaced
      in-place until re-pin). The resulting s3 wasm carries the
      page-table runtime (globals 8/9/10 present) and compiled the
      previously-stalling std::seq probes in seconds each. Evidence
      artifact: .build/selfhost/flat-src/bootstrap-out-bigarena.wasm.
