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
4. `--time` prints `lower.reachability_fns: before=N after=M`.

### P1+ (child issues — do not implement under #823)

- Early body lowering: #824
- AST cache format repair: #825
- Intern + clone audit: #826
- Phase arena (after ADR-002 / #730 ownership): #827

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

P0.4 phase timers (CLI flag is `--time`):
- `--time` / `MIR_LOWER_PHASE_TIMING=1` →
  `lower.decl_emit` / `lower.reachability` / `lower.sync` / `lower.propagate`
- `--time` pipeline → `lower` / `mir_opt` / `mir_verify` / `emit`
- pinned→s2 overlay stubs `clock::monotonic_now` to 0, so stage-2 receipts
  print labels with `0ms`; use a clock-capable artifact for wall numbers.
- Do not call `time::duration_ms` from driver/mir timing paths under pinned→s2
  (lowers to `unreachable`); inline ns→ms with `i64_to_i32`.

### P1 reachability queue BFS (2026-07-17)

Landed:
- `mir/reachability_index.ark`: `NameIndex` + `fid_to_mir` (core-op aliases unmapped)
- `reachability_entry` / `walk` / `roots` / `names`: queue BFS; CALL/REF_FUNC prefer
  `func_id_raw`; normal-call fallback via `mir_call_normal_fallback_symbol`
- `lower/call_func_id.ark`: attach `func_id_raw` for CALL and REF_FUNC
- fixture: `tests/fixtures/reachability/call_export_roots.ark`
- REF_FUNC keep smoke: `scripts/tests/test_mir_reachability_bfs.py`

### Measurement receipt (2026-07-17) — clock-stubbed s2-runtime full compile

Artifact / runner:
- Compiler: `.build/selfhost/arukellt-s2-runtime.wasm` (valid, overlay clock stub)
- Workload: overlay workspace full selfhost (`src/compiler/main.ark` → wasm32-gc / wasi-p2)
- Flags: `--time`; outer `/usr/bin/time -v`
- `ARUKELLT_OVERLAY_KEEP_CLOCK=1` keep_clock builds fail `wasm-validate`
  (`func 10: expected i32, found i64`), so **phase ms remain 0** on this path.
  Wall / RSS / `reachability_fns` are the reliable numbers.

| Metric | Value |
|--------|------:|
| Wall (`Elapsed`) | 1:42.18 (~102 s) |
| User time | 100.08 s |
| Peak RSS | 1,379,560 KiB (~1.32 GiB) |
| `lower.reachability_fns` | before=8737 after=7980 (Δ −757) |
| Phase `--time` labels | all `0ms` (clock stub) |

Smaller smokes (same stubbed s2, `--time`):
- hello: `before=81 after=1`
- `wasm_dead_fn_elim`: `before=83 after=3`
- reachability CALL fixture: `before≈84 after≥3` (keeps export CALL chain; prunes `truly_dead`)
- REF_FUNC edge keep: see `test_mir_reachability_bfs` (`after≥4`)

### Bottleneck conclusion (after P1 BFS)

Prune still runs **after** full MIR decl emit (`8737` bodies already lowered).
Queue BFS + FunctionId maps make reachability itself cheaper, but the remaining
selfhost wall is dominated by **lowering unreached bodies** (and subsequent
sync/propagate/emit on the still-large kept set ~7980). Next implementation
target is **early body lowering** (#824), not further MIR-only name-scan tweaks.
AST cache (#825) / intern (#826) / arena (#827) stay deferred; no product code
for AST cache or arena under this slice.
