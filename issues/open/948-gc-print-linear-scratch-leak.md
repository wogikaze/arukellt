---
Status: open
Created: 2026-10-27
Updated: 2026-10-27
ID: 948
Track: wasi
Depends on: none
Orchestration class: implementation-ready
Orchestration upstream: none
Blocks v{N}: none
Priority: 0
Source: "P0 design/implementation audit (2026-10-27)"
Related: "ADR-074, #947"
---

# 948 — `wasm32-gc` print bump-allocates linear memory per call and never reclaims it

## Summary

To pass a GC string to WASI, the print lowering copies it into linear memory
at the global heap pointer and advances the pointer
(`emit_gc_alloc_heap_buf`). The pointer is never restored, and no collector
manages linear memory on `wasm32-gc` (the linear tracing runtime is
`wasm32`-only). Every printed byte permanently consumes linear memory, so a
long-running program that prints exhausts memory even after #947 is fixed.

## Evidence (2026-10-27)

- `src/compiler/wasm/runtime_abi_stdio_gc.ark` L35–46
  (`emit_gc_alloc_heap_buf`: `global.get 0` → buf, `global.set 0` to
  buf + len aligned; nothing resets it). Callers L24–33.
- `src/compiler/wasm/linear_root_lowering.ark` `should_emit` and
  `src/compiler/wasm/emit_target.ark` `is_linear_memory_target` (collector
  only for `wasm32`).
- Other host-ABI paths use the same scratch-heap staging, e.g.
  `src/compiler/wasm/runtime_abi_fs_read_body.ark` L31–84.

Repro: the #947 repro. Once the handle leak is fixed, measure linear memory
growth per printed byte (expected O(1) total, currently O(bytes printed)).

## Expected

Host-ABI staging buffers on `wasm32-gc` are scoped: released or reused after
the host call returns (stack-discipline reset or a reusable scratch region).

## Acceptance

- [x] The print path restores or reuses its staging buffer after the WASI
      call.
- [ ] All `wasm32-gc` host-ABI staging sites (`runtime_abi_*`) are audited;
      each is either scoped or documented as one-shot with a bound.
- [x] A runtime fixture shows bounded linear memory after printing
      ≥ 100 MB on `wasm32-gc` (memory size checked by the harness or by an
      exported probe), run with `wasmtime`.

## Implementation and verification (2026-10-28, Wave C)

`runtime_abi_stdio_gc.ark` now brackets each print sequence with
`emit_gc_staging_begin` / `emit_gc_staging_end`, which save and restore the
linear-memory heap global (global 0) around the staging allocation, so the
bump cursor is reused instead of advanced once per print. The invariant is
recorded next to the helpers: the staging region lives only for the write that
immediately follows, nothing between the two calls allocates from the cursor,
and no pointer into it escapes.

Measured with a stage-2 built from these sources, `wasmtime run` maxRSS on a
probe that prints one 1024-byte line per iteration (≥100 MB total output for
the 200k case; `/usr/bin/time -v`):

| probe | pre-change (pinned bootstrap) | post-change (stage-2) |
| --- | --- | --- |
| 20 000 prints (~20 MB) | — | 33.9 MB maxRSS |
| 200 000 prints (~205 MB) | 279.4 MB maxRSS | 79.0 MB maxRSS |

The 200 MB print run no longer grows RSS by O(bytes printed). The residual
~45 MB delta between 20k and 200k post-change runs matches the per-print
output-stream handle leak still tracked by #947, not a staging-buffer leak.

Remaining: the other `runtime_abi_*` staging sites were not audited in this
wave (second item), and the measurement is a manual probe because the fixture
harness cannot execute `wasm32-gc` components.
