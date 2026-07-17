---
Status: open
Created: 2026-07-17
Updated: 2026-07-17
ID: 823
Track: selfhost-infra
Depends on: "730"
Related: "#730"
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

### P1+ (track here or child issues)

- Early reachability before MIR emit; FnId/index reachability walk.
- AST cache restore; phase arena / interning (ties to #730 heap model).

## Required verification

- `python3 scripts/manager.py verify quick`
- After emitter/MIR changes: `python3 scripts/manager.py selfhost build-compiler`
  and a hello / small fixture compile smoke.
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

P0.4 phase timers:
- `--show-timing` / `MIR_LOWER_PHASE_TIMING=1` →
  `lower.decl_emit` / `lower.reachability` / `lower.sync` / `lower.propagate`
- `--show-timing` pipeline → `lower` / `mir_opt` / `mir_verify` / `emit`
