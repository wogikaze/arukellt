---
Status: open
Created: 2026-10-19
Updated: 2026-10-19
ID: 932
Track: compiler
Depends on: "#851"
Orchestration class: implementation-ready
Orchestration upstream: none
Blocks v{N}: none
Priority: 1
Source: "#851 Phase 3 guest-profile decomposition (2026-10-19)"
Related: "#851, ADR-053, docs/plans/selfhost-compiler-core-rewrite.md"
---

# 932 — fused in-loop emit re-scans growing state once per pushed
# function; incremental indexes and version gates remove the quadratics

## Summary

ctx_run_per_fn_post_pass -> emit_inloop_prepare runs once per pushed
function (~11k pushes on the selfhost compile). Several of its sweeps
re-scanned state that only grows, making the phase quadratic:

- gc_layout_table_drain_plan re-ran mir_gc_layout_plan_valid (an
  O(bindings^2) pairwise type_id_eq scan) on the whole plan per push.
- mir_gc_layout_plan_bind deduped via a linear type_id_eq scan per
  registration.
- ctx_gc_canon_ensure_late re-ensured every late_gc_order entry and
  re-scanned all struct/enum slots per push.
- SelfEmitCtx_finalize_gc_derived_indexes rebuilt three NameIndexes and
  flag vecs (with per-name string derives) per push.
- plan_pending was rescanned per push even when no new canon/base state
  could resolve anything.
- mir_reachability_index_build derived per-fn name ordinals with an O(fi)
  scan and resolved (name, ordinal) with an O(registry) scan, once per fn.
- mir_rewrite_hashmap_constructor built ~4 candidate strings x 6
  overloads per call site.

Guest profile evidence (gc host on flat-src, wasmtime --profile guest):
the old validator/binding accessors held ~38% inclusive under
gc_layout_table_drain_plan; eq held ~10% self.

## Acceptance

- [ ] wall of the gc-overlay selfhost compile drops measurably vs the
      pre-fix host under the same conditions
- [ ] emitted wasm is byte-identical before/after (self-fixpoint) —
      semantics must not move
- [ ] wasm-tools validate passes
- [ ] plan + receipt updated

## Implementation notes

- MirGcLayoutPlan.index_by_raw (TypeId.raw -> binding slot) makes bind
  dedup O(1); the validator now checks index-consistency per binding,
  which also detects duplicates in O(n).
- The drain validates only [plan_seen, count).
- MirModule.gc_canon_version bumps on name pushes and -1 -> final canon
  transitions; GcLayoutTable.canon_version_seen gates the three derived
  index rebuilds, drain_*_seen gates the pending rescan.
- ctx_gc_canon_ensure_late skips slots whose canon idx is already final
  (ensure is idempotent).
- The reachability index build precomputes ordered name->registry-slot
  chains + per-name ordinal counters.
- The HashMap constructor rewrite gates on a starts_with("HashMap")
  prefilter.
