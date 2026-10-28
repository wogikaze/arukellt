---
Status: open
Created: 2026-10-28
Updated: 2026-10-28
ID: 956
Track: wasi
Depends on: none
Orchestration class: implementation-ready
Orchestration upstream: none
Blocks v{N}: none
Priority: 2
Source: "ABC481 arukellt port + WAT emission session (2026-10-28, HEAD 0a64bdb62)"
Related: "#947, #948, ADR-085"
---

# 956 — wasm32-gc: `print`/`println` reject strings over 4096 bytes (`Buffer too large for blocking-write-and-flush`)

## Summary

On the primary `wasm32-gc` / WASI P2 component path, writing a string larger than 4096
bytes in one `print`/`println` call traps:

```
Error: Buffer too large for blocking-write-and-flush (expected at most 4096)
  at <wasm function 18>
```

The same program on `wasm32` (linear memory) prints the full 5001 characters.

## Repro

`tmp/bigprint.ark`:

```ark
use std::host::stdio
use std::prelude::String_from

fn main() {
    let mut s: String = String_from("x")
    let mut i: i32 = 0
    while i < 5000 {
        s = s.concat("y")
        i = i + 1
    }
    stdio::println(s)
}
```

```
$ scripts/run/arukellt-selfhost.sh compile --target wasm32-gc --emit component -o .build/bigprint.component.wasm tmp/bigprint.ark
$ wasmtime run --wasm gc --wasm function-references --dir=. .build/bigprint.component.wasm
Error: Buffer too large for blocking-write-and-flush (expected at most 4096)
```

## Workaround

Chunk output to <=4096 bytes per call; ABC481 C/E/F/G use a `print_lines` helper that
prints 200 lines at a time.

## Notes

The 4096 limit is a host-stream property of the P2 path; the runtime layer should either
loop over the stream or the API contract should state the limit. Related stream issues:
#947 (stdout handle leak) and #948 (linear scratch leak).

## Acceptance

- [ ] `println` of a 1 MB string on `wasm32-gc` succeeds (looped write) or fails with a documented, actionable diagnostic.
- [ ] Fixture covers >4096-byte stdout on `wasm32-gc`.
