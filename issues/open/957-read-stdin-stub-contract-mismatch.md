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

## Update (2026-10-28, Wave A) — measured, then doc and body aligned

Measured with the current pinned bootstrap
(`bootstrap/arukellt-selfhost.wasm`, sha256 `8df2698c…`) and with
`.build/selfhost/arukellt-s2-runtime.wasm`:

| stdin | target | len | first | last |
|-------|--------|-----|-------|------|
| `abc` (3 B, no newline) | `wasm32` | 3 | 97 (`a`) | 99 (`c`) |
| `abc` | `wasm32-gc` component | 3 | 97 | 99 |
| 420 B (`ABCDEFG…`, no newline) | both | 420 | 65 (`A`) | 71 (`G`, index 419) |

So the runtime reads stdin on both targets and the literal `""` body was not
the executed path.

Changes:

- `std/host/stdio.ark`: the doc comment now says "Reads stdin until EOF and
  returns the bytes read as a `String`; returns an empty string when stdin is
  already at EOF", matching the manifest entry
  (`intrinsic = "__intrinsic_stdin_read_to_string"`). The body now calls
  `__intrinsic_stdin_read_to_string()`, with an inline comment explaining why
  the body is not the executed path: call sites of `stdio::read_stdin` lower to
  the `runtime.read_stdin` CoreOp through
  `corehir/core_op_binding_generated.ark`, so the declaration no longer
  silently returns `""` on a path that resolves it directly. The stale #807
  reference is gone.
- Added `tests/fixtures/stdin/read_stdin_eof_has_zero_length.ark`, which
  asserts `byte_len(stdio::read_stdin()) == 0` at EOF. It is **not** registered
  in `tests/fixtures/manifest.txt` (Lead owns manifest edits). Note it overlaps
  `tests/fixtures/stdlib_io/read_stdin_empty.ark`; the new fixture pins the
  length contract, which is the only stdin state the harness can feed
  (`fixture_parity.py` runs with `stdin=subprocess.DEVNULL` and has no
  per-fixture stdin sidecar).
- Verified: `python3 scripts/manager.py fmt <3 files>` PASS,
  `python3 scripts/manager.py docs check` 6/6 PASS,
  `python3 scripts/check/check-comment-policy.py --changed` PASS. The EOF
  fixture exits 0 on both `wasm32` and `wasm32-gc`.
