---
Status: open
Created: 2026-10-28
Updated: 2026-10-28
ID: 955
Track: compiler
Depends on: none
Orchestration class: architecture-implementation
Orchestration upstream: none
Blocks v{N}: none
Priority: 1
Source: "ABC481 arukellt port + WAT emission session (2026-10-28, HEAD 0a64bdb62)"
Related: "#933, #874, #922, #875, ADR-040, ADR-053"
---

# 955 — wasm32: two live `Vec<i64>` corrupt the managed heap (out-of-bounds trap)

## Summary

On `wasm32` (linear-memory backend), a program that keeps two `Vec<i64>` alive and
reads from one while pushing into the other corrupts the heap and traps:

```
wasm trap: out of bounds memory access
  at main
  memory fault at wasm address 0x2a0624e8 in linear memory of size 0x20050000
```

A single `Vec<i64>` works (200k pushes and reads verified). `wasm32-gc` runs the same
program fine.

## Repro

`tmp/twovec.ark` (uses `i32_to_i64`, shortened here):

```ark
fn main() {
    let n: i32 = 1000
    let a: Vec<i64> = Vec::new<i64>()
    let mut i: i32 = 0
    while i < n {
        push(a, i32_to_i64(i) * 1000000007i64 + 12345i64)
        i = i + 1
    }
    let b: Vec<i64> = Vec::new<i64>()
    i = 0
    while i < n {
        let x: i64 = get_unchecked(a, i)
        push(b, x)
        i = i + 1
    }
}
```

```
$ scripts/run/arukellt-selfhost.sh compile --target wasm32 --emit wasm -o .build/twovec.wasm tmp/twovec.ark
$ wasmtime run --dir=. .build/twovec.wasm
wasm trap: out of bounds memory access
```

`--target wasm32-gc --emit component` run prints `1000` / `12345` / `1000` as expected.

## Impact

Any algorithm that needs 64-bit key/coordinate arrays (sorting keys, coordinates,
modular arithmetic tables) is blocked on the `wasm32` target. This is why the ABC481
`wasm32` solutions had to encode 64-bit values as two 24-bit `i32` limbs:

- C (`ark/c.ark`): name keys as 2x i32 limbs in an open-addressing table.
- E (`ark/e.ark`): coordinates as 2x i32 limbs, reconstructed per comparison.

## Notes

- Interleaving `Vec<i32>` and one `Vec<i64>` works; two `Vec<i64>` is the trigger.
- Suspected: managed-array growth/GC move for 8-byte element vectors on the linear
  backend (#874 fallback managed value lowering, #922 arena/OOM family), or the
  #933/#940 name-keyed type identity mixing i32/i64 layouts.

## Acceptance

- [ ] Repro runs on `wasm32` and prints the same values as `wasm32-gc`.
- [ ] Fixture: two live `Vec<i64>` with 200k elements, read/push interleaved.
- [ ] No regression in existing `wasm32` fixtures.
