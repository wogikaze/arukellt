---
Status: open
Created: 2026-07-10
Updated: 2026-07-24
ID: 730
Track: selfhost-infra
Depends on: "726"
Related: "#727, #686, #813, #823, #829"
Orchestration class: architecture-investigation
Blocks v4 exit: True
---

# 730 — Bootstrap wasm 4GB memory limit blocks pinned wasm refresh

## Summary

The pinned bootstrap wasm (`bootstrap/arukellt-selfhost.wasm`) cannot compile
the current `src/compiler/` source because the compiler's bump allocator exceeds
the wasm32 4GB linear memory limit.

## Status (2026-07-24)

| Item | State |
|------|--------|
| Hard 4GiB ceiling | Unblocked via Memory64 path (`wasm32to64` / convert-only + wasmtime `-W memory64=y`) |
| Stage-3 MIR lower hang | **Fixed** in `471661a3` (prune-before-sync) |
| Full selfhost `compile` past lower | **OK** |
| `selfhost fixpoint --build` | **Green** (#813; emit still `wasm32` / `wasi-p1`) |
| KEEP_CLOCK validate + `--time` | **Green** (#829 receipt) |
| Pinned wasm refresh (wasm32) | **Done** — `48ad40ee4edd…` @ `9951fd2b` (two-round stable fixpoint) |
| Pinned / `BOOTSTRAP_EMIT` → `wasm32-gc` | **Blocked** — s2→wasm32-gc emit fails validate (`func 8204`) |
| `verify quick` 0 failures | Still open (T3 / CLI parity etc.) |

### Landed fixes

1. **Memory64 bootstrap** (`f3cec6b9` and earlier): patcher `--to-memory64` / `--convert-only`,
   address canon, grow-site `ge_u`+`65536`, runners with `max-memory-size=16GiB`.
2. **Prune-before-sync** (`471661a3`):
   - `lower_to_mir_with_roots` prunes with export-surface roots **before** typed sync.
   - `session_lower_mir` / `_component` use that path.
   - `is_t3_wasm_emit` treats `wasm32-gc` as T3 even without `"-p2"` in the target string.
3. **Fixpoint path (#813)**: stage-2/3 share `BOOTSTRAP_EMIT_TARGET=wasm32` via pinned bootstrap host.
4. **Pinned refresh (2026-07-24)**: two-round wasm32 fixpoint pin
   (`08dfbfcb…` → `06b61c60…` → stable `48ad40ee…`);
   `sha256(pinned)==sha256(s2)==sha256(s3)`; `fixture-parity` green; no intentional
   fixture drift observed.

Hang root cause: `ctx_sync_typed_value_types` on the unpruned flat selfhost MIR
(roughly O(locals²) per function) after ~10GiB emit.

### Runtime guidance (stage-3)

Prefer **convert-only + `--initial-pages=131072` (8GiB)** over `--to-memory64`
(heap-grow + convert) until grow-site OOB is fully settled. Concurrent rebuilds
of `.build/selfhost/flat-src` can cause `file write error` / module-load failures
even after a successful compile.

### wasm32-gc pinned experiment (2026-07-24) — Strategy A failed

| Step | Result |
|------|--------|
| s2-runtime compile `--target wasm32-gc --wasi-version wasi-p2` | rc=0, ~3.4 MiB, Memory64 |
| `wasm-tools validate` on that artifact | **FAIL** `func 8204`: expected `(ref null $type)`, found `(ref null $type)` |
| Old pinned bootstrap with `--target wasm32-gc` | Emits bytes **identical** to wasm32 s2 (no true gc emit from that host) |
| Conclusion | Do **not** pin wasm32-gc until self-emit validates. Keep `BOOTSTRAP_EMIT_*=wasm32`. |

Follow-up (before close): fix wasm32-gc selfhost emit validate, then refresh pinned to
Memory64 `wasm32-gc` / `wasi-p2` and restore `_fixpoint_stage3_compiler` → s2-runtime.

## Acceptance Criteria

- [x] `selfhost fixpoint --build` can produce s2/s3 wasm (#813, 2026-07-24)
- [ ] `verify quick` passes (0 failures)
- [x] Pinned wasm refreshed with current source (**wasm32** stable fixpoint `48ad40ee…`)
- [ ] Pinned / bootstrap emit path is native **`wasm32-gc` / `wasi-p2` / Memory64**
- [x] Stage-3 no longer hangs in MIR lower after typecheck (`471661a3`)
- [x] `ARUKELLT_OVERLAY_KEEP_CLOCK=1` produces a compiler wasm that
      `wasm-tools validate` accepts (smoke: `scripts/tests/test_selfhost_keep_clock_time_smoke.py`, 2026-07-21+)
- [x] That clock-capable artifact prints non-zero `--time` / `lower.*` phase ms on
      full selfhost (#829 receipt `.build/selfhost/selfhost-latency-receipt.json`, 2026-07-24)

KEEP_CLOCK is part of Memory64 completion, not a optional latency nicety: without
real clocks, post-#823 hotspot selection is guesswork.

## Next (remaining for close)

1. Fix `wasm32-gc` self-compile validate (`func 8204` ref mismatch) on stage-2 host.
2. Refresh pinned to validating Memory64 `wasm32-gc` artifact; set
   `BOOTSTRAP_EMIT_TARGET=wasm32-gc` / `BOOTSTRAP_EMIT_WASI_VERSION=wasi-p2`.
3. Restore fixpoint stage-3 host to s2-runtime (drop #813 bootstrap-only workaround).
4. Clear remaining `verify quick` failures (T3 validate, CLI component parity / #811).

## References

- `bootstrap/PROVENANCE.md`
- [#813](../done/813-selfhost-fixpoint-not-reached.md) (done)
- [#829](../done/829-selfhost-latency-phase-reprofile-hotspot.md) (done)
