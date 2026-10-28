---
Status: open
Created: 2026-10-28
Updated: 2026-10-28
ID: 960
Track: compiler
Depends on: none
Orchestration class: implementation-ready
Orchestration upstream: none
Blocks v{N}: none
Priority: 1
Source: "ABC481 arukellt port ergonomics review (2026-10-28, HEAD 0a64bdb62)"
Related: "#953, #955, #936, ADR-015"
---

# 960 — `String::split` is broken on both targets: runtime OOB on wasm32, invalid Wasm on wasm32-gc

## Summary

`String::split(delim)` (`std/collections/string.ark` L953) does not work on either
target. This forces every program that reads whitespace-separated input to ship a
hand-written byte tokenizer, because the natural one-liner crashes or fails to compile.

- `wasm32`: compiles, then traps at runtime with a negative address
  (`memory fault at wasm address 0xfffffffe`).
- `wasm32-gc`: the compiler itself fails with `compiler produced invalid core Wasm`.

## Repro (wasm32)

```ark
use std::host::stdio

fn main() {
    let s: String = String_from("4 9")
    let parts: Vec<String> = s.split(" ")
    stdio::eprintln("parts=".concat(parts.len().to_string()))
}
```

```
$ scripts/run/arukellt-selfhost.sh compile --target wasm32 --emit wasm -o .build/split32.wasm tmp/split.ark
$ wasmtime run --dir=. .build/split32.wasm
wasm trap: out of bounds memory access
  memory fault at wasm address 0xfffffffe in linear memory of size 0x20050000
```

## Repro (wasm32-gc)

```
$ scripts/run/arukellt-selfhost.sh compile --target wasm32-gc --emit component -o .build/split.component.wasm tmp/split.ark
arukellt-selfhost: error — compiler produced invalid core Wasm
```

## Notes

- `String::trim()` works on wasm32 (returns the trimmed length correctly), so the
  failure is in `split` itself, not in all String methods.
- `split` uses `Vec<String>` plus slicing; the negative address suggests a pointer /
  length field is being treated as a signed value somewhere in the lowering.
- Likely the same family as #953 (Ordering/match miscompile) and #955 (Vec<i64> heap
  corruption): #936 name-based dispatch and #933's MIR re-inference decide the callee
  and layout by name.

## Workaround

Manual tokenization over `Vec<i32>` bytes (see ABC481 `read_input`/`next_int` helpers):
copy `byte_at` bytes into `Vec<i32>` and parse integers by hand.

## Acceptance

- [ ] Repro splits `"4 9"` into 2 elements and runs on both targets.
- [ ] Fixture covers multi-space, newline, and trailing-delimiter inputs.
- [ ] `String::lines()` (same area) audited in the same fixture.
