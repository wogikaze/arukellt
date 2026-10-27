---
Status: open
Created: 2026-10-27
Updated: 2026-10-27
ID: 947
Track: wasi
Depends on: none
Orchestration class: implementation-ready
Orchestration upstream: none
Blocks v{N}: none
Priority: 0
Source: "P0 design/implementation audit (2026-10-27)"
Related: "ADR-054, #948"
---

# 947 — `wasm32-gc` print leaks a WASI output-stream handle per call; programs die after 500,000 lines

## Summary

On the primary target, each `println` / `print` imports and calls
`wasi:cli/stdout@0.2 get-stdout`, which returns a new `output-stream`
resource, and the handle is never dropped. The guest component does not
import a `resource-drop` for the stream. Each print grows the host resource
table, and once the table is full the program traps.

## Evidence (2026-10-27)

- `src/compiler/wasm/runtime_abi_stdio_gc.ark` L24–33 (`emit_gc_println` /
  `emit_gc_print` → `helpers_io_p2::emit_p2_write_string_local(…,
  P2_IMPORT_GET_STDOUT())` per call, no drop).
- `wasm-tools print` of the output lists `get-stdout` but no stream
  `resource-drop`.

Repro (`print_leak.ark`):

```ark
use std::host::stdio

fn main() {
    let line = String_from("0123456789012345678901234567890123456789012345678901234567890123456789012345678901234567890123456789")
    let mut i = 0
    while i < 3000000 {
        stdio::println(line)
        i = i + 1
    }
    stdio::println("done")
}
```

Observed with `.build/selfhost/arukellt-s2-runtime.wasm`
(sha256 `1e11e8e7f2c5…`):

- `wasm32`: runs to completion and prints `done`.
- `wasm32-gc --emit component`: `wasmtime run` writes exactly 500,000 lines,
  then `resource table has no free keys` (trap in `main`).

## Expected

Unbounded printing works on `wasm32-gc`; resource usage per print is O(1).

## Acceptance

- [ ] stdout / stderr streams are acquired once and reused, or dropped after
      each write via the canonical `resource-drop`; no handle is leaked per
      call.
- [ ] Every other P2 import that returns an owned resource (stderr, stdin,
      streams, fs descriptors, …) is audited for the same pattern.
- [ ] A runtime fixture prints ≥ 1,000,000 lines on `wasm32-gc` and exits 0
      (it may be in a slow lane, but must be gated).
- [ ] A static check asserts the emitted component drops or reuses the
      stream handle.
