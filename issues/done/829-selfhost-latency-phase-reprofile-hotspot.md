---
Status: done
Created: 2026-07-20
Updated: 2026-07-24
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
Executable plan: [`docs/plans/selfhost-latency-phase-reprofile.md`](../../docs/plans/selfhost-latency-phase-reprofile.md).

## Start gate (2026-07-20 probe)

Do **not** start Works 2–5 until all are true:

* `selfhost fixpoint --build` produces validating s2 **and** s3
* Memory64 runner flags are the formal path
* No concurrent selfhost compile / flat-src writers

Probe same day: s2 / s2-runtime validate OK; **s3 missing**; another
`fixpoint --build` was running; `arukellt-s2-clock.wasm` fails validate
(`func 4697`: expected i64, found i32). Status: **BLOCKED** on start gate + Work 1.

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

- [x] #730 includes KEEP_CLOCK validate + real `--time` as completion criteria (see #730 L83+)
- [x] Clock-capable s2 validates; stage-3 `--time` receipt at
      `.build/selfhost/selfhost-latency-receipt.json` (host sha256, target, wall, RSS)
- [x] Dominant phase: **`emit.code.locals`** (13.6 s / 30.7% on baseline receipt)
- [x] Hotspot change landed: CSR producer write index + prior def-site / has_ref caches
      (`code_ref_locals_fn_cache.ark`, `code_ref_locals_block_scan.ark`, `ctx_record.ark`)
- [x] **Dominant phase halved**: emit.code.locals **13.6 s → 2.8 s (−79.5%, target ≤6.8 s)**
- [x] Cold stage-3 wall **<5 min** (≈33 s on 2026-07-24 receipt machine; also &lt;2 min)
- [x] Follow-up plan for further cold improvement: next dominant is `lower.reachability` ≈13 s
- [x] Docs: research memo + plan updated; #813 start gate cleared

## 2026-07-24 close review notes

Start gate green (#813 done). Baseline + after receipts via
`scripts/debug/latency_rss_phase_probe.py`. Producer-index after:
`emit.code.locals=2782ms`, `total=31343ms`, `wall=32953ms`.
`selfhost fixpoint --build` green (`sha256=06b61c60…`). `verify lane` green.
KEEP_CLOCK smoke green. #824 not required (decl_emit not dominant).

## Non-goals

- Treating lean-bootstrap / page size as the primary latency fix
- Implementing #824 before a decl_emit-majority receipt
- Claiming 5–10 s cold full selfhost in this issue’s acceptance

## References

- `#823` A/B: BFS 124 s vs legacy 134 s; prune 8748→7991; phase ms still 0
- `#730` Memory64 / fixpoint
- Live profile 2026-07-20: stage-3 ≈ 23.5 min, check ≈ 9.1 s, hello ≈ 0.03 s
