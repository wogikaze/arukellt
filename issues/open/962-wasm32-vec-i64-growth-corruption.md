---
Status: open
Created: 2026-10-28
Updated: 2026-10-28
ID: 962
Track: compiler
Depends on: none
Orchestration class: architecture-implementation
Orchestration upstream: none
Blocks v{N}: none
Priority: 1
Source: "ABC481 arukellt refactor session (2026-10-28, HEAD 0a64bdb62)"
Related: "#955, #874, #922, ADR-040, ADR-053"
---

# 962 — wasm32: a single growing `Vec<i64>` loses elements past index 4

## Summary

On `wasm32`, appending more than four `i64` values to a `Vec<i64>` corrupts the
elements from index 4 on. The first four reads are correct; later reads return heap
garbage. Both `v.push(x)` (method) and `push(v, x)` (intrinsic) reproduce, and
`Vec::with_capacity<i64>(64)` does not help. `Vec<i32>` growth is fine, and
`wasm32-gc` is fine.

This is separate from #955 (two simultaneously live `Vec<i64>`); a single vector is
enough.

## Repro

`tmp/vec64_grow.ark`:

```ark
use std::host::stdio
use std::prelude::{i32_to_i64, i64_to_i32}

fn main() {
    let v: Vec<i64> = Vec::new<i64>()
    let mut i: i32 = 0
    while i < 8 {
        push(v, i32_to_i64(i) * 7i64 + 100i64)
        i = i + 1
    }
    stdio::println(i64_to_i32(v.get_unchecked(4) % 1000000007i64).to_string())
}
```

Expected `128`; actual a large garbage value (observed `158428767`). Reading all eight
elements shows `100 107 114 121 <garbage> ...`.

- 32 elements: observed correct on the same build, so the loss depends on the growth
  state/GC layout rather than a fixed index.
- 200k pushes in one `while` loop can also appear correct while early elements are
  wrong, so checksum-only tests can miss it.

## Workarounds used in ABC481

- Read input into `Vec<i32>` (all values fit i32) instead of `Vec<i64>`.
- For 64-bit keys/coordinates, store 24-bit limbs in `Vec<i32>` and rebuild the i64
  scalar at the point of use (`ark/c.ark`, `ark/e.ark`).

## Notes

- Suspected: managed-array growth for 8-byte elements leaves the handle pointing at the
  pre-growth buffer on the linear backend (#874 lowering, #922 arena family), or the
  name-keyed type identity (#933/#940) confuses the i32/i64 element stride.
- #955 covers two-live-vector corruption; this issue should be fixed by the same
  backend work.

## Acceptance

- [ ] Repro prints `128` and a full 8-element readback matches on `wasm32`.
- [ ] Fixture: 200k `Vec<i64>` pushes with reads at indices 0, 4, 12345, n-1.
- [ ] `wasm32-gc` stays green.
