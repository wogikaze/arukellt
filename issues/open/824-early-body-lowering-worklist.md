---
Status: open
Created: 2026-07-17
Updated: 2026-07-17
ID: 824
Parent: 823
Track: selfhost-infra
Depends on: "823"
Related: "#730, #823, docs/research/selfhost-compile-latency-root-cause.md"
Orchestration class: design
Blocks v4 exit: False
---

# Early body lowering (worklist; design first)

## Summary

Post-MIR prune (#823) still runs after every body is lowered
(`fns before≈8748 after≈7991`, ~8.7% omitted). Early body lowering would skip
MIR body emit for FunctionIds never reached from roots. **Implementation is
blocked** until #823 has real phase-ms evidence that `decl_emit` dominates wall
time (KEEP_CLOCK / clock intrinsic validate is currently broken).

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
12. **Measurement gate.** Land code only after #823 phase-ms receipt shows
    `decl_emit` (or equivalent body-lower time) as the dominant share; then
    re-run wall / RSS / fns/blocks/insts before→after.

## Non-goals

- AST cache repair (#825)
- Symbol/path interning (#826)
- Phase arena (#827)
- Changing public API / ABI / language semantics

## Acceptance

- [ ] Design section above remains the implementation contract
- [ ] Implementation plan lists root seeding, deterministic order, mono/closure
      rules, fallback edges, keep-reason counters, prune safety net,
      prune-disabled + stage-2 overlay keep-all behavior
- [ ] Implementation starts only after #823 phase-ms re-judge selects decl_emit
- [ ] `python3 scripts/manager.py verify quick` + selfhost build-compiler smoke
      when code lands

## Evidence / parent receipt

See #823 A/B: BFS wall 124 s vs legacy 134 s on stubbed s2-runtime; prune
8748→7991 with matching block/inst deltas; phase ms still 0ms (KEEP_CLOCK
blocked). No decl_emit majority claim yet.

## References

- `issues/open/823-selfhost-compile-latency-quadratic-mir.md`
- `docs/research/selfhost-compile-latency-root-cause.md` (原因5 / P1.1)
