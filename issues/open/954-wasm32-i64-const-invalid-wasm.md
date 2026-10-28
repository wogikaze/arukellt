---
Status: open
Created: 2026-10-28
Updated: 2026-10-28
ID: 954
Track: compiler
Depends on: none
Orchestration class: implementation-ready
Orchestration upstream: none
Blocks v{N}: none
Priority: 1
Source: "ABC481 arukellt port + WAT emission session (2026-10-28, HEAD 0a64bdb62)"
Related: "#933, #935, #936, ADR-053"
---

# 954 — wasm32: referencing a top-level `i64` const produces invalid Wasm

## Summary

A top-level `const NAME: i64 = <literal>` that is referenced from code makes the
`wasm32` backend emit an invalid module:

```
error — W0004 generated Wasm failed validation:
  error: func 10 failed to validate
      0: type mismatch: expected i32, found i64
```

`wasm32-gc` compiles the same program fine, so the fault is in the linear-memory
lowering of constant references.

## Repro

`tmp/i64const.ark`:

```ark
const LIMIT: i64 = 4096i64

fn main() {
    let x: i64 = 100i64 * LIMIT
    if x > 0i64 {
        let y: i32 = 1
        let z: i32 = y + 1
        if z > 1 {
            return
        }
    }
}
```

```
$ export PATH=<wasmtime>:$PATH
$ scripts/run/arukellt-selfhost.sh compile --target wasm32 --emit wasm -o .build/i64const.wasm tmp/i64const.ark
arukellt-selfhost: error — W0004 generated Wasm failed validation: .build/i64const.wasm
  error: func 10 failed to validate
      0: type mismatch: expected i32, found i64
```

`--target wasm32-gc --emit wasm` succeeds.

## Workaround

Inline the literal at every use site (this is what ABC481 E and G do). That removes
the const function call from the lowered module.

## Notes

- `i32` consts and local `let` with i64 work; only the top-level const reference fails.
- Same family as #936 (prelude/core symbols dispatched by name with wrong types) and
  #933 (MIR re-infers types by name).

## Acceptance

- [ ] Repro compiles and validates on `wasm32`.
- [ ] Fixture covers i64/i32/f64 consts referenced from expressions and calls on both targets.
