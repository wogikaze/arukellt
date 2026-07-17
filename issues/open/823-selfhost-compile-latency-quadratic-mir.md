---
Status: open
Created: 2026-07-17
Updated: 2026-07-17
ID: 823
Track: selfhost-infra
Depends on: "730"
Related: "#730, #824, #825, #826, #827"
Orchestration class: implementation
Blocks v4 exit: False
---

# Selfhost compile latency: remove quadratic MIR vector rebuilds

## Summary

Selfhost full compile latency is dominated by MIR sync/propagate and
reachability that rebuild `Vec` on every element update, amplified by the
bump allocator (no free). Memory64 avoids OOM but does not fix wall time.
Research: [`docs/research/selfhost-compile-latency-root-cause.md`](../../docs/research/selfhost-compile-latency-root-cause.md).

## Acceptance

### P0 (this issue — land first)

1. `mir_function_set_local_at` / `mir_function_set_param_at` update in place
   (no full `Vec` rebuild per element).
2. `MirModule_set_function_at` updates in place.
3. Prefer differential / single typed sync after propagate where safe
   (may be a follow-up commit under this issue).
4. Phase timers for lower/reachability/sync/propagate/emit (or a linked slice)
   so the next regression has a receipt.

### P1 (MIR reachability queue BFS — landed under this issue)

1. Explicit `FunctionId.raw → MirFunction index` map (never assume raw == mir index).
2. Queue BFS reachability (each function body walked once).
3. `MIR_CALL` / `MIR_REF_FUNC` prefer `func_id_raw`; name + normal-call fallback retained.
4. `--time` prints `lower.reachability_fns` / `_blocks` / `_insts` before/after.
5. Gate: `python3 scripts/check/check-mir-reachability-bfs.py` (wired into verify quick).
6. Same-artifact A/B vs legacy fixpoint (`MIR_REACHABILITY_LEGACY_FIXPOINT=1`).

### P1+ (child issues — do not implement under #823)

- Early body lowering: #824 (design only until phase ms re-judge)
- AST cache format repair: #825
- Intern + clone audit: #826
- Phase arena (after ADR-002 / #730 ownership): #827

## Required verification

- `python3 scripts/manager.py verify quick`
- After emitter/MIR changes: `python3 scripts/manager.py selfhost build-compiler`
  and hello / reachability gate smoke.
- Prefer a before/after note of stage-2 or stage-3 wall + peak RSS when measurable.

## Notes

- Do not treat lean-bootstrap or Memory64 page size as the primary latency fix.
- `docs/compiler/bootstrap.md` ~45s stage-2 vs #730 ~10–11 min stage-3 are
  different pipelines; keep receipts labeled by artifact/target.

## Progress (2026-07-17)

P0.1–P0.2: in-place MIR `Vec` updates (prior commits).

P0.3 typed sync fuse: propagate / enum-normalize / callee-lookup write
`type_name` via `mir_local_with_type_name_synced` (TypeTable re-sync on write).
Lower entry runs one full `mir_module_sync_all_value_types` then propagate;
`ctx_sync_typed_value_types` no longer does a second full-module sync.

P0.4 phase timers (CLI flag is `--time`):
- `--time` / `MIR_LOWER_PHASE_TIMING=1` →
  `lower.decl_emit` / `lower.reachability` / `lower.sync` / `lower.propagate`
- `--time` pipeline → `lower` / `mir_opt` / `mir_verify` / `emit`
- pinned→s2 overlay stubs `clock::monotonic_now` to 0, so stage-2 receipts
  print labels with `0ms`.
- Do not call `time::duration_ms` from driver/mir timing paths under pinned→s2
  (lowers to `unreachable`); inline ns→ms with `i64_to_i32`.

### P1 reachability queue BFS

Landed:
- `mir/reachability_index.ark`: `NameIndex` + `fid_to_mir` (core-op aliases unmapped)
- queue BFS entry/walk/roots/names; CALL/REF_FUNC prefer `func_id_raw`
- `mir/reachability_legacy.ark`: old fixpoint behind `MIR_REACHABILITY_LEGACY_FIXPOINT=1`
- REF_FUNC-only fixture + MIR dump asserts:
  `tests/fixtures/reachability/ref_func_only_target.ark`
  (`ref_only_target` kept, `truly_dead` pruned; not CALL-reachable from export)
- Gate: `scripts/check/check-mir-reachability-bfs.py` (builds s2 if needed; skip = fail)

### KEEP_CLOCK / phase-ms blocker

`ARUKELLT_OVERLAY_KEEP_CLOCK=1` (even limited to `mir/lower/entry_timing.ark`)
builds a module that fails `wasm-tools validate` with
`expected i64, found i32` (Memory64/GC clock intrinsic lowering).
`build_clock_capable_s2` documents this and rejects invalid artifacts.
**Phase ms remain unavailable** until that intrinsic path is fixed.
Do not treat stubbed `0ms` labels as evidence of phase cost.

### A/B receipt (2026-07-17) — same stubbed s2-runtime, full selfhost

Artifact: `.build/selfhost/arukellt-s2-runtime.wasm` (clock-stubbed, validated)  
Workload: overlay `src/compiler/main.ark` → wasm32-gc / wasi-p2, `--time`  
Runner: `/usr/bin/time -v` via `ARUKELLT_REACHABILITY_AB=1`  
Local receipt: `.build/selfhost/reachability-bfs-receipt.json`

| Mode | Wall (s) | Peak RSS (KiB) | fns before→after | blocks before→after | insts before→after |
|------|---------:|---------------:|------------------:|--------------------:|-------------------:|
| queue BFS (default) | 124 | 1,385,968 | 8748→7991 | 17496→15982 | 373771→358123 |
| legacy fixpoint | 134 | 1,385,828 | 8748→7991 | 17496→15982 | 373771→358123 |

- Prune results match → BFS is not dropping edges vs fixpoint.
- Wall delta ≈ **−10 s (~7.5% of 134 s)**; RSS ≈ unchanged.
- Phase `--time` labels all `0ms` (clock stub).
- Omitable bodies ≈ 757 / 8748 ≈ **8.7%** of functions after full decl emit.

### Bottleneck status (not a decl_emit verdict)

With phase ms still stubbed, **do not claim** that `decl_emit` is the majority of
the ~124–134 s wall, and **do not start #824 implementation** on that basis.

Known facts only:

1. P1 BFS saves ~10 s vs legacy fixpoint on this workload.
2. Post-MIR prune removes ~8.7% of functions after every body was already lowered.
3. Remaining wall could still be sync, propagate, emit, or decl_emit — unknown
   until KEEP_CLOCK / clock-intrinsic validate is fixed and real phase ms exist.

Next: fix Memory64 clock intrinsic lowering (or equivalent timer ABI), re-run
`--time`, then choose #824 vs sync/propagate/emit from actual ms shares.
