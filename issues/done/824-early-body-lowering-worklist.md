---
Status: done
Disposition: wontfix
Created: 2026-07-17
Updated: 2026-07-26
ID: 824
Parent: 829
Track: selfhost-infra
Depends on: "829"
Related: "#730, #823, #829, docs/research/selfhost-compile-latency-root-cause.md, docs/research/receipts/824-early-body-decl-emit-defer-receipt.json"
Orchestration class: design
Blocks v4 exit: False
---

# Early body lowering (worklist; design first)

## Summary

Post-MIR prune (#823) still runs after every body is lowered
(`fns before≈8748 after≈7991`, ~8.7% omitted; blocks ~8.7%; insts ~4.2%).
Early body lowering would skip MIR body emit for FunctionIds never reached from
roots.

**This is a candidate under [#829](829-selfhost-latency-phase-reprofile-hotspot.md),
not the default next theme.** Sync / propagate / wasm emit already run on the
pruned graph, so #824 mainly saves omitted-body `decl_emit` (+ temps). It will
not alone turn ~23 min stage-3 into a few minutes unless `decl_emit` dominates
the phase receipt.

## Phase 2 decision (2026-07-26) — **wontfix / defer**

Measurement gate for implementation:

1. `lower.decl_emit` ≥ **35%** of `--time` total, **and**
2. `lower.decl_emit` ≥ **1.5×** the second-largest phase

### Lane L5 receipt (KEEP_CLOCK, this worktree)

Workload: `compile src/compiler/main.ark --target wasm32-gc --wasi-version wasi-p2 --time`  
Host: `arukellt-s2-clock.wasm` via `build_clock_capable_s2`  
Durable receipt: [`docs/research/receipts/824-early-body-decl-emit-defer-receipt.json`](../../docs/research/receipts/824-early-body-decl-emit-defer-receipt.json)

| Phase | ms | share of total |
|---|---:|---:|
| **emit** | 41175 | **40.3%** |
| **lower.reachability** | 35314 | **34.6%** |
| lower.decl_emit | 11311 | **11.1%** |
| resolve | 11104 | 10.9% |
| total | 102131 | 100% |

- `decl_emit` share **11.1%** ≪ 35% → gate fail
- `decl_emit / second` = 11311 / 41175 ≈ **0.27×** ≪ 1.5× → gate fail
- Note: `latency_rss_phase_probe.py` REFUSE'd concurrent bootstraps; phases taken
  via the same KEEP_CLOCK argv under concurrent load (absolute ms inflated;
  shares match the #829 quiet receipt below).

### #829 after receipt (quiet machine, authoritative share)

From `.build/selfhost/selfhost-latency-receipt.json` after CSR producer index:

| Phase | ms | share |
|---|---:|---:|
| lower.reachability | 13180 | **42.1%** |
| lower.decl_emit | 5601 | **17.9%** |
| emit.code.locals | 2782 | 8.9% |
| total | 31343 | 100% |

- `decl_emit` **17.9%** ≪ 35%; ratio to reachability ≈ **0.43×** ≪ 1.5×

**Decision: do not implement early body lowering.** Close as wontfix.  
Next latency focus: `lower.reachability` (and remaining emit work), not #824.

Design lock below remains the contract **if** a future receipt ever meets the
gate (re-open then). No compiler code landed for this issue.

## Design (acceptance for this issue = design lock + no premature impl)

```text
Register all signatures / FunctionIds / layouts / types
  → seed root FunctionIds (main / _start / exports / WIT / conservative set)
  → work queue (deterministic order — see below)
  → lower one function body
  → collect CALL / REF_FUNC / normal-fallback FunctionIds
  → enqueue → until empty
  → never-lowered bodies stay as signatures only
  → existing post-MIR prune remains as safety net
```

### Constraints (must appear in implementation plan)

1. **Do not delete CoreHIR declarations early.** Signature / FunctionId / layout /
   type metadata remain fully registered for the whole program.
2. **Body-only worklist.** Only function bodies are deferred; edges come from
   lowered CALL / REF_FUNC / normal-call fallback FunctionIds.
3. **Deterministic worklist order.** Seed order and enqueue order must be stable
   across runs (e.g. ascending `FunctionId.raw`, then declaration order). No
   hash-map iteration order dependence in roots or edge collection.
4. **Dynamic mono instances.** Monomorphized bodies created during lowering are
   registered into the same worklist (or conservatively lowered immediately).
   A mono instance that appears after its caller was processed must still be
   reachable from the queue.
5. **Closure / function table.** Closures, `REF_FUNC`, and any function-table /
   HOF surface that can be invoked without a direct CALL edge are treated as
   roots or conservative keeps until a proven edge model exists.
6. **Normal-call fallback.** `mir_call_normal_fallback_symbol` (and equivalents)
   must enqueue the fallback FunctionId the same way post-MIR BFS does.
7. **Conservative keep with reason counters.** If method / mono / closure / HOF /
   export / WIT / unknown-indirect roots cannot be proven safe to defer,
   **lower the body** and increment a named counter
   (`keep_reason_method`, `keep_reason_mono`, `keep_reason_closure`,
   `keep_reason_hof`, `keep_reason_export`, `keep_reason_wit`,
   `keep_reason_unknown`). Counters are printed under `--time` for receipts.
8. **Post-MIR prune safety net.** Keep `#823` queue-BFS prune after body
   lowering. Early lowering is an optimization; prune still drops anything that
   slipped through.
9. **Prune-disabled paths.** `lower_program_to_mir` / `*_no_prune` and any
   driver path with `prune_enabled=false` must either lower all bodies or
   clearly document that early lowering is off (no silent partial graphs).
10. **Stage-2 overlay full-emitter keep contract.** Bootstrap overlay paths that
    intentionally disable prune / keep the full emitter graph (pinned→s2
    contracts) must keep early body lowering **off** or force conservative
    keep-all so overlay completeness does not regress.
11. **Separate state from MIR prune map.** `FunctionId → body-lowered?` is not
    the post-MIR `FunctionId → Mir index` map.
12. **Measurement gate.** Land code only after #829 phase-ms receipt shows
    `decl_emit` (or equivalent body-lower time) as the dominant share; then
    re-run wall / RSS / fns/blocks/insts before→after.

## Non-goals

- AST cache repair (#825)
- Symbol/path interning (#826)
- Phase arena (#827)
- Changing public API / ABI / language semantics

## Acceptance

- [x] Design section above remains the implementation contract (if re-opened)
- [x] Implementation plan lists root seeding, deterministic order, mono/closure
      rules, fallback edges, keep-reason counters, prune safety net,
      prune-disabled + stage-2 overlay keep-all behavior
      (`docs/plans/824-early-body-lowering-worklist.md`)
- [x] Implementation starts only after #829 phase-ms re-judge selects decl_emit
      — **gate not met; no implementation**
- [x] Measurement receipt recorded; issue closed wontfix without code land
- [ ] `python3 scripts/manager.py verify quick` + selfhost build-compiler smoke
      when code lands — **N/A (no code)**

## Evidence / parent receipt

See #823 A/B: BFS wall 124 s vs legacy 134 s on stubbed s2-runtime; prune
8748→7991 / blocks 17496→15982 / insts 373771→358123.  
#829 after: `decl_emit` 5601 ms (17.9%), `lower.reachability` 13180 ms (42.1%).  
Lane L5 2026-07-26: `decl_emit` 11.1%, dominant `emit` 40.3% / `reachability` 34.6%.

## References

- `issues/done/829-selfhost-latency-phase-reprofile-hotspot.md`
- `issues/done/823-selfhost-compile-latency-quadratic-mir.md` (if present) / open archive
- `docs/research/selfhost-compile-latency-root-cause.md`
- `docs/research/receipts/824-early-body-decl-emit-defer-receipt.json`
- `docs/plans/824-early-body-lowering-worklist.md`
