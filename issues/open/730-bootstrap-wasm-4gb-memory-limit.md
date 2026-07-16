---
Status: open
Created: 2026-07-10
Updated: 2026-07-17
ID: 730
Track: selfhost-infra
Depends on: "726"
Related: "#727, #686"
Orchestration class: architecture-investigation
Blocks v4 exit: True
---

# 730 — Bootstrap wasm 4GB memory limit blocks pinned wasm refresh

## Summary

The pinned bootstrap wasm (`bootstrap/arukellt-selfhost.wasm`) cannot compile
the current `src/compiler/` source because the compiler's bump allocator exceeds
the wasm32 4GB linear memory limit.

## Progress (2026-07-16)

Chosen path: **Memory64** (ADR-007 already lists it as `wasm32-gc` default emit OK).

1. **Bootstrap unblock**: `wasm-heap-grow-patcher --to-memory64` converts pinned/s2
   wasm32 modules; selfhost runners pass `-W memory64=y` and
   `-W max-memory-size=16GiB`.
2. **Native emit**: `wasm32-gc` emits Memory64 memory + i64 heap and widens former
   i32 LM values via emitter helpers (`uses_memory64` = GC target).
3. **Selfhost retarget**: stage-2 still emits `wasm32` from pinned bootstrap;
   stage-3+ uses `--target wasm32-gc --wasi-version wasi-p2`. GC Memory64 modules
   skip `wasm32to64` (preserves GC types).
4. **Converter hardening (in progress)**:
   - Stage-2 often has **zero** `memory.grow` sites → heap-grow patch is required.
   - Grow injection uses `ge_u` + `65536` (was `gt_u` / `65535`).
   - `wasm32to64` keeps load sign-extend for sentinels, leaves `i32.add`/`sub` as
     full i64 (past 4GiB), and **canonizes** negative addresses before mem ops
     so sign-extended pointers in `[2GiB, 4GiB)` stay valid.
   - Observed stage-3 RSS past **10GiB** without the old 4GiB OOB; compile still
     may hang in lower/emit (separate from the hard 4GiB ceiling).

### Stage-3 hang bisect (2026-07-16 evening)

Invocation must match fixpoint (`wasmtime … --dir flat-src --dir root -- compile …`).
Wrong cwd / missing `--` produces a **parser** OOB red herring.

| Probe | Result |
|-------|--------|
| `check` (through typecheck) | **OK ~54s**, RSS ~230MB |
| Forced E0200 in `main.ark` | **OK ~15s** |
| `compile --dump-phases mir` | **TIMEOUT ≥10min**, RSS rises ~0.7→10GB in ~1min then plateaus; **no `=== MIR` dump** |
| `--emit component` (forces `session_lower_mir_component`) | Same hang pattern |
| `--target wasm32-wasi-p2` (alias with `-p2`) | Same hang pattern |

**Conclusion:** hang is **after typecheck, before MIR dump** — inside
`lower_checked_program` → `lower_to_mir*` / opt / verify (almost certainly MIR lower
allocating ~10GiB then CPU-bound). Not the wasm32 4GiB ceiling anymore.

**Note:** `is_t3_wasm_emit` previously keyed off `target` containing `"-p2"`, so
`--target wasm32-gc` took the non-T3 `session_lower_mir` branch. Component emit
still hung, so fixing that gate alone was not sufficient; lower of the flat
selfhost graph was the bottleneck.

### Fix: prune-before-sync (2026-07-17)

Root cause of the stage-3 hang: `lower_entry_input_to_mir` ran
`ctx_sync_typed_value_types` (full-module sync + propagate + re-sync) on the
**unpruned** flat selfhost MIR. Sync is roughly O(locals²) per function, so the
mega-module CPU-spun after ~10GiB emit.

Changes:

1. `lower_to_mir_with_roots` — prune with export-surface roots **before** typed sync.
2. `session_lower_mir` / `session_lower_mir_component` call that path (no late prune).
3. `is_t3_wasm_emit` treats `wasm32-gc` as T3 even when `"-p2"` is not in the target string.

Verification with a rebuilt s2 (fix baked in) + convert-only Memory64
(`--initial-pages=131072`): full selfhost `compile` reaches
`compilation succeeded (phase 6)` in ~10–11 minutes (hang gone). Prefer
convert-only 8GiB over `--to-memory64` for stage-3 runtime until grow-site
OOB is fully settled.

## Root Cause

The compiler uses a bump allocator (global 0 = heap pointer, monotonic,
never freed). Every allocation increases the heap pointer. The compiler
allocates AST nodes, MIR instructions, wasm bytes, etc. for all 1894
source files. With 106K lines of source, the bump allocator needs more
than 4GB.

wasm32 has a hard 4GB linear memory limit. Even with `memory.grow` checks
in the pinned wasm, the memory cannot exceed 4GB.

### Memory.grow check bugs (secondary)

The pinned wasm's memory.grow checks have two bugs:

1. **`gt_u` instead of `ge_u`**: The check `heap_ptr > memory_size` should
   be `heap_ptr >= memory_size`. When `heap_ptr == memory_size`, the check
   doesn't trigger and the next load/store crashes.

2. **`65535` instead of `65536`**: The pages-needed calculation
   `(heap_ptr - memory_size + 65535) >> 16` gives 0 pages when
   `heap_ptr == memory_size`, causing `memory.grow(0)` which is a no-op.

These bugs were identified via binary analysis of the wasm. Binary patches
(`gt_u` → `ge_u`, `65535` → `65536`) allow the memory to grow a few more
pages, but the 4GB limit is still hit.

### Verification

Tested with various initial memory sizes (16MB, 256MB, 512MB, 1GB, 2GB,
3GB, 4GB-128KB) + max=4GB + binary patches. All crash with
"out of bounds memory access" at or slightly above the initial size.

With initial=65534 (4GB-128KB), the wasm runs for ~15 seconds before
crashing, confirming the compiler needs more than 4GB.

## Impact on verify quick (9 failures)

| Check | Cause |
|-------|-------|
| false-done close-gate enforcement | wasm crash (4GB) |
| selfhost LSP lifecycle gate (#569) | wasm crash (4GB) |
| selfhost formatter parity gate (#216) | wasm crash (4GB) |
| runtime Wasm debug smoke gate (#638) | wasm crash (4GB) |
| LSP performance smoke tests (#463) | wasm crash (4GB) |
| GC array smoke gate | s2 build failed (4GB) |
| T3 fixture WASM validation gate (#686) | stale pinned wasm bugs + compile-fail |
| init template gate (#464) | stale pinned wasm generates old API (`i32_to_string`) |
| docs consistency | regenerated (fixed) |

## Possible Solutions

1. **Fix the bump allocator** — Add free/reset capability to the compiler's
   allocator. Major change affecting all allocation sites.

2. **Use wasm64 (memory64)** — wasmtime supports `-W memory64=y`. Would
   require changing all memory access instructions from i32 to i64
   addresses in the compiler emitter.

3. **Module-by-module compilation** — Compile one module at a time,
   resetting the heap between modules. Requires compiler pipeline changes.

4. **Reduce source size** — Split the compiler into smaller compilation
   units or reduce the number of files.

5. **External native compiler** — Build a Rust native compiler that can
   cross-compile to wasm, bypassing the 4GB limit.

## Acceptance Criteria

- [ ] `selfhost fixpoint --build` can produce s2/s3 wasm
- [ ] `verify quick` passes (0 failures)
- [ ] Pinned wasm can be refreshed with current source
