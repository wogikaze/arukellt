---
Status: open
Created: 2026-07-10
Updated: 2026-07-10
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
the current `src/compiler/` source (106K lines, 1894 files) because the
compiler's bump allocator exceeds the wasm32 4GB linear memory limit.

This blocks:
- `selfhost fixpoint --build` (can't produce s2/s3 wasm)
- `verify quick` (9 checks fail due to wasm crashes or stale pinned wasm)
- Pinned wasm refresh (can't rebuild with current source)

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
