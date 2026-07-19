---
Status: open
Created: 2026-07-20
Updated: 2026-07-20
ID: 829
Track: selfhost-infra
Depends on: "730"
Related: "#730, #813, #823, #824, #825, #826, #827, docs/research/selfhost-compile-latency-root-cause.md"
Orchestration class: architecture-investigation
Blocks v4 exit: False
---

# Selfhost latency: phase re-profile and dominant-hotspot removal

## Summary

After Memory64 unblocks selfhost scale, the development loop is still blocked by
**cold stage-3 wall times on the order of tens of minutes** (~23.5 min observed
2026-07-20 with s2 fingerprint hit). #823 already landed in-place MIR updates,
typed-sync fuse, and queue-BFS reachability; **P0 is not the remaining story**.

This issue is the next theme: restore real `--time` receipts, identify the
dominant phase, and remove that hotspot. It is **not** “implement #824 by
default.”

Research: [`docs/research/selfhost-compile-latency-root-cause.md`](../../docs/research/selfhost-compile-latency-root-cause.md).

## Sequence (do not skip)

```text
1. mem64 / fixpoint green (#730 / #813)
2. KEEP_CLOCK s2 validates; --time prints real ms (#730 completion criterion)
3. Lock a phase receipt on one artifact + target
4. Halve the dominant phase
5. Cold stage-3: <5 min, then <2 min
```

Incremental edit-loop targets (module cache → **5–10 s**) are a **later** stage.
Do not use 5–10 s as acceptance for cold full selfhost (~118k LOC).

## Required phase receipt

Same compiler wasm, same overlay, same `--target` / `--wasi-version`, no
concurrent selfhost compiles. Capture wall ms for:

`frontend / lower.decl_emit / reachability / sync / propagate / mir_opt / mir_verify / wasm emit`

Prefer RSS at each boundary (final RSS alone conflates “slow” vs “allocator
growth”).

## Decision table (after receipt)

| Dominant | Next work |
|---|---|
| `decl_emit` | Consider [#824](824-early-body-lowering-worklist.md) |
| `propagate` | Fixpoint / stack-producer search (new slice or extend #823 notes) |
| `wasm emit` | Section/function rebuild, clone, name-lookup audit |
| RSS-only growth across phases | [#826](826-symbol-path-intern-clone-audit.md) |
| `mir_opt` / `mir_verify` | Split dedicated issues |

### Why #824 is only a candidate

Post-MIR prune removes ≈8.7% functions / ≈4.2% instructions **after** bodies are
already lowered; sync/propagate/emit run on the pruned graph. Early body
lowering mainly saves omitted-body `decl_emit` work unless that phase dominates
the receipt.

## Acceptance

- [ ] #730 includes KEEP_CLOCK validate + real `--time` as completion criteria
- [ ] Clock-capable s2 (or equivalent) validates; stage-3 `--time` receipt attached
      under `.build/selfhost/` or issue notes (artifact hash, target, wall, RSS)
- [ ] Dominant phase named from that receipt (not assumed)
- [ ] One hotspot change lands with before/after wall on the same workload
- [ ] Cold stage-3 wall under 5 min on the labeled receipt machine/config
- [ ] Follow-up plan for under 2 min cold stage-3 (may be child issues)
- [ ] Docs: research memo + #823/#824 point here; no “P0 still needed for 5–10s”

## Non-goals

- Treating lean-bootstrap / page size as the primary latency fix
- Implementing #824 before a decl_emit-majority receipt
- Claiming 5–10 s cold full selfhost in this issue’s acceptance

## References

- `#823` A/B: BFS 124 s vs legacy 134 s; prune 8748→7991; phase ms still 0
- `#730` Memory64 / fixpoint
- Live profile 2026-07-20: stage-3 ≈ 23.5 min, check ≈ 9.1 s, hello ≈ 0.03 s
