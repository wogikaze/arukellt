---
Status: open
Created: 2026-10-28
Updated: 2026-10-28
ID: 957
Track: stdlib-api
Depends on: none
Orchestration class: implementation-ready
Orchestration upstream: none
Blocks v{N}: none
Priority: 3
Source: "ABC481 arukellt port + WAT emission session (2026-10-28, HEAD 0a64bdb62)"
Related: "#807, #936, ADR-015"
---

# 957 — `std::host::stdio::read_stdin` body/docs say 'returns empty', but runtime reads stdin

## Summary

`std/host/stdio.ark` declares:

```ark
/// Returns empty until the GC WASI stdin path validates and returns stable
/// bytes (#807 read_stdin_empty / rpn_repl). Matches `std::io::stdin()` empty
/// Reader contract in the current runtime.
pub fn read_stdin() -> String {
    ""
}
```

but the emitted program actually reads stdin: with 14 bytes on stdin, both
`wasm32-gc` component and `wasm32` return `len == 14`. The body is bypassed because
the emitter dispatches `stdio::read_stdin` by name (the #936 placeholder-dispatch
class), so the documented contract and the behavior disagree.

## Repro

```ark
use std::host::stdio
use std::prelude::{to_string, byte_len}

fn main() {
    let s: String = stdio::read_stdin()
    stdio::eprintln("read_stdin len=".concat(to_string(byte_len(s))))
}
```

```
$ printf 'input 5 bytes\n' | wasmtime run --wasm gc --wasm function-references --dir=. .build/readstdin.component.wasm
read_stdin len=14            # wasm32 core target prints the same
```

## Why it matters

- Users reading the source believe stdin is unavailable and switch to file-path
  arguments (this is exactly what the ABC481 solutions did before we discovered the
  intrinsic worked).
- Any future change that honors the literal body would silently break programs.

## Fix direction

1. Make the declared body match the runtime behavior (or delete the placeholder and
   expose stdin through a real intrinsic-backed body).
2. Update the doc comment; remove the stale #807 reference if the GC stdin path is now
   validated, or link the current tracking issue.
3. Add a fixture asserting non-empty stdin on both targets.

## Acceptance

- [ ] `read_stdin` doc, body and runtime agree.
- [ ] Fixture: stdin `abc` -> `len == 3` on `wasm32` and `wasm32-gc` (or a documented empty contract on both).
