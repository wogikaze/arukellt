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

- [x] `println` of a 1 MB string on `wasm32-gc` succeeds (looped write) or fails with a documented, actionable diagnostic.
- [ ] Fixture covers >4096-byte stdout on `wasm32-gc`.

## Implementation and verification (2026-10-28, Wave C)

`helpers_io_p2.ark` gained `emit_p2_write_string_chunked`: it reads the payload
length from the staging buffer prefix and loops `blocking-write-and-flush` in
`constants::P2_BWAF_MAX_CHUNK()` (4096) byte steps, leaving
`emit_p2_write_string_local` as the single-call path for short static buffers
(newline, panic/assert text). All four gc print entry points
(`print`/`println`/`eprint`/`eprintln` in `runtime_abi_stdio_gc.ark`) now call the
chunked emitter. Source: `helpers_io_p2.ark`, `runtime_abi_stdio_gc.ark`,
`constants.ark`.

Verified with a stage-2 built from these sources
(`ARUKELLT_BUILD_DIR=wt/gc-wasi/build`, s2 produced 21:09):

- `println(repeat("x", 5001))` on `wasm32-gc` component → exit 0, full 5001 bytes
  plus newline, then `done` (previously trapped with
  `Buffer too large for blocking-write-and-flush`).
- `println(repeat("y", 1048576))` (1 MB) → exit 0, 1 048 581 bytes of stdout.
- `wasm32` linear target, same 5001-byte program → exit 0, 5006 bytes (no
  regression).

Remaining: the fixture harness does not execute `wasm32-gc` components (`run:`
fixtures run on the `wasm32` linear target; `test:` via `arukellt test` also
executes `wasm32`), so the ">4096-byte stdout on `wasm32-gc`" item needs either a
component-exec fixture kind or a documented manual probe. The manual probe above
is the current evidence.
