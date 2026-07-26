---
Status: done
Created: 2026-07-10
Updated: 2026-07-25
Closed: 2026-07-25
ID: 730
Track: selfhost-infra
Depends on: "726"
Related: "#727, #830, #686, #813, #823, #829, #726, #808, #834"
Orchestration class: done
Blocks v4 exit: False
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
| Pinned / `BOOTSTRAP_EMIT` → `wasm32-gc` | **Moved to #834** — validate root cause fixed (`clone` identity, `06ba2d35`); full self-compile pin blocked by host RSS / grow |
| `verify quick` 0 failures | **Moved to #834** |

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

## Ownership transferred from #726 (2026-07-25)

After #726 narrow-close, this issue owns:

- Any remaining T3 **compile-fail** historically tied to bootstrap 4GB / Memory64
  (e.g. `stdlib_wit/wit_ast_parse` when still failing under pinned bootstrap).
- **`verify quick` 0 failures** as the aggregate close gate (not #726).

## Acceptance Criteria

Scope for this issue: **Memory64 unblock + wasm32 pin + validate root cause for
known wasm32-gc self-emit failure**. Pinning bootstrap to wasm32-gc and
`verify quick` 0 are **#834**.

- [x] `selfhost fixpoint --build` can produce s2/s3 wasm (#813, 2026-07-24)
- [x] ~~`verify quick` passes (0 failures)~~ — **移管 #834**
- [x] Pinned wasm refreshed with current source (**wasm32** stable fixpoint `48ad40ee…`)
- [x] ~~Pinned / bootstrap emit path is native `wasm32-gc` / `wasi-p2` / Memory64~~ — **移管 #834**
- [x] Stage-3 no longer hangs in MIR lower after typecheck (`471661a3`)
- [x] `ARUKELLT_OVERLAY_KEEP_CLOCK=1` produces a compiler wasm that
      `wasm-tools validate` accepts (smoke: `scripts/tests/test_selfhost_keep_clock_time_smoke.py`, 2026-07-21+)
- [x] That clock-capable artifact prints non-zero `--time` / `lower.*` phase ms on
      full selfhost (#829 receipt `.build/selfhost/selfhost-latency-receipt.json`, 2026-07-24)
- [x] Root cause of wasm32-gc self-emit `func 8204` (`doc_parse_manifest` casting
      `DocParseFnState` to String via generic `clone`) fixed — `06ba2d35`;
      fixture `structs/struct_clone_pass_to_fn.ark` validates

## Close note — 2026-07-25

Narrow-close: Memory64 / wasm32 pin / KEEP_CLOCK / clone typing root cause done.
Remaining pin + `verify quick` owned by [#843](../open/843-wasm32-gc-bootstrap-pin.md)
(full selfhost wasm32-gc compile needs >4GiB grow or ~21GiB RSS host).
`$issue-close-review`: **APPROVE** (FD-08 scope split + open owner #834).

## References

- `bootstrap/PROVENANCE.md`
- [#813](../done/813-selfhost-fixpoint-not-reached.md) (done)
- [#829](../done/829-selfhost-latency-phase-reprofile-hotspot.md) (done)
