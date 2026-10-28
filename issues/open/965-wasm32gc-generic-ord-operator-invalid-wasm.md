---
Status: open
Created: 2026-10-28
Updated: 2026-10-28
ID: 965
Track: compiler
Depends on: none
Orchestration class: implementation-ready
Orchestration upstream: none
Blocks v{N}: none
Priority: 1
Source: "Vec/stdlib surface expansion session (2026-10-28, HEAD 0a64bdb62)"
Related: "#933, #953, #955, #962, ADR-036"
---

# 965 — wasm32-gc: generic `<` on `T: Ord` compiles to invalid Wasm; generic `Vec<T: Ord>` algorithms are blocked

## Summary

A generic function that compares `T: Ord` values with `<` compiles on `wasm32` and runs
correctly, but on `wasm32-gc` the compiler emits invalid core Wasm:

```
$ scripts/run/arukellt-selfhost.sh compile --target wasm32-gc --emit component -o .build/less.component.wasm tmp/less.ark
compilation succeeded (phase 6)
arukellt-selfhost: error — compiler produced invalid core Wasm
```

`wasm32` prints `true/false/true/false/true` for the same program.

## Repro

```ark
use std::host::stdio

fn less<T: Ord>(a: T, b: T) -> bool {
    a < b
}

fn same<T: Ord>(a: T, b: T) -> bool {
    let ab: bool = a < b
    if ab {
        return false
    }
    let ba: bool = b < a
    if ba {
        return false
    }
    true
}

fn main() {
    stdio::println(less(1, 3).to_string())
    stdio::println(same(2, 2).to_string())
}
```

`T: Eq` + `==` cannot be used at all yet (`E0200: i32 does not implement Eq` even though
`std/core/cmp.ark` has `impl Eq for i32`), so equality has to be encoded as
`!(a<b) && !(b<a)`.

## Impact

- Blocks the requested generic `Vec<T: Ord>::sort()`, `dedup()`, `lower_bound()`,
  `upper_bound()`, and generic `contains()`.
- The stdlib therefore ships **i32-specialized** comparison methods for now
  (`std/collections/vec.ark`), which work on both targets.
- Related: #953 (`Ord::cmp` traps at runtime), #955/#962 (`Vec<i64>` corruption).

## Acceptance

- [ ] Repro compiles and runs on `wasm32-gc`.
- [ ] Generic `Vec<T: Ord>::sort` fixture passes on both targets.
- [ ] `T: Eq` bound resolves the existing `impl Eq for i32`.
