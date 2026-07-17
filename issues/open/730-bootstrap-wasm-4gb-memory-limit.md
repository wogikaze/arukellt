---
Status: open
Created: 2026-07-10
Updated: 2026-07-17
ID: 730
Track: selfhost-infra
Depends on: "726"
Related: "#727, #686, #823"
Orchestration class: architecture-investigation
Blocks v4 exit: True
---

# 730 — Bootstrap wasm 4GB memory limit blocks pinned wasm refresh

## Summary

The pinned bootstrap wasm (`bootstrap/arukellt-selfhost.wasm`) cannot compile
the current `src/compiler/` source because the compiler's bump allocator exceeds
the wasm32 4GB linear memory limit.

## Status (2026-07-17)

| Item | State |
|------|--------|
| Hard 4GiB ceiling | Unblocked via Memory64 path (`wasm32to64` / convert-only + wasmtime `-W memory64=y`) |
| Stage-3 MIR lower hang | **Fixed** in `471661a3` (prune-before-sync) |
| Full selfhost `compile` past lower | **OK** — `compilation succeeded (phase 6)` ~10–11 min with convert-only 8GiB s2 |
| `selfhost fixpoint --build` | Still open (runtime path / write races / remaining emit issues) |
| `verify quick` 0 failures | Still open (see below) |
| Pinned wasm refresh | Still open |

### Landed fixes

1. **Memory64 bootstrap** (`f3cec6b9` and earlier): patcher `--to-memory64` / `--convert-only`,
   address canon, grow-site `ge_u`+`65536`, runners with `max-memory-size=16GiB`.
2. **Prune-before-sync** (`471661a3`):
   - `lower_to_mir_with_roots` prunes with export-surface roots **before** typed sync.
   - `session_lower_mir` / `_component` use that path.
   - `is_t3_wasm_emit` treats `wasm32-gc` as T3 even without `"-p2"` in the target string.

Hang root cause: `ctx_sync_typed_value_types` on the unpruned flat selfhost MIR
(roughly O(locals²) per function) after ~10GiB emit.

### Runtime guidance (stage-3)

Prefer **convert-only + `--initial-pages=131072` (8GiB)** over `--to-memory64`
(heap-grow + convert) until grow-site OOB is fully settled. Concurrent rebuilds
of `.build/selfhost/flat-src` can cause `file write error` / module-load failures
even after a successful compile.

### verify quick (2026-07-17 snapshot)

163 passed / 8 failed (not the old “stuck in lower” hang):

| Check | Observed failure mode |
|-------|------------------------|
| false-done close-gate | wasm validate `func 10/11` |
| selfhost analysis API (#568) | fail |
| GC array smoke | wasm compile validate `func 10` |
| runtime Wasm debug smoke (#638) | validate `func 11` |
| selfhost LSP lifecycle (#569) | fail |
| T3 fixture WASM validation (#686) | many fixtures: validate `func 10/11` |
| docs consistency | generated docs out of date |

## Progress detail (2026-07-16)

Chosen path: **Memory64** (ADR-007 already lists it as `wasm32-gc` default emit OK).

1. **Bootstrap unblock**: `wasm-heap-grow-patcher --to-memory64` converts pinned/s2
   wasm32 modules; selfhost runners pass `-W memory64=y` and
   `-W max-memory-size=16GiB`.
2. **Native emit**: `wasm32-gc` emits Memory64 memory + i64 heap and widens former
   i32 LM values via emitter helpers (`uses_memory64` = GC target).
3. **Selfhost retarget**: stage-2 still emits `wasm32` from pinned bootstrap;
   stage-3+ uses `--target wasm32-gc --wasi-version wasi-p2`. GC Memory64 modules
   skip `wasm32to64` (preserves GC types).
4. **Converter hardening**:
   - Stage-2 often has **zero** `memory.grow` sites → heap-grow patch is required.
   - Grow injection uses `ge_u` + `65536` (was `gt_u` / `65535`).
   - `wasm32to64` keeps load sign-extend for sentinels, leaves `i32.add`/`sub` as
     full i64 (past 4GiB), and **canonizes** negative addresses before mem ops
     so sign-extended pointers in `[2GiB, 4GiB)` stay valid.

### Stage-3 hang bisect (historical)

| Probe | Result |
|-------|--------|
| `check` (through typecheck) | **OK ~54s**, RSS ~230MB |
| Forced E0200 in `main.ark` | **OK ~15s** |
| `compile --dump-phases mir` (pre-fix) | **TIMEOUT ≥10min**, RSS ~0.7→10GB then plateau; no MIR dump |
| After `471661a3` | Reaches mir-verify and `compilation succeeded (phase 6)` |

## Root Cause (4GiB)

The compiler uses a bump allocator (global 0 = heap pointer, monotonic,
never freed). Every allocation increases the heap pointer. The compiler
allocates AST nodes, MIR instructions, wasm bytes, etc. for all ~1894
source files. With ~106K lines of source, the bump allocator needs more
than 4GB. wasm32 has a hard 4GB linear memory limit.

### Memory.grow check bugs (secondary, pinned-era)

1. **`gt_u` instead of `ge_u`**
2. **`65535` instead of `65536`** in pages-needed calculation

Binary patches help slightly but do not remove the 4GB ceiling.

## Acceptance Criteria

- [ ] `selfhost fixpoint --build` can produce s2/s3 wasm
- [ ] `verify quick` passes (0 failures)
- [ ] Pinned wasm can be refreshed with current source
- [x] Stage-3 no longer hangs in MIR lower after typecheck (`471661a3`)

## Next (remaining for close)

1. Stabilize stage-3 runtime path in `scripts/selfhost/checks.py` (convert-only 8GiB
   vs `--to-memory64`) and avoid flat-src races during fixpoint.
2. Clear remaining `verify quick` failures (validate `func 10/11`, docs regenerate).
3. Refresh pinned bootstrap once fixpoint is green.
