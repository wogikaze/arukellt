---
Status: open
Created: 2026-10-28
Updated: 2026-10-28
ID: 953
Track: compiler
Depends on: none
Orchestration class: implementation-ready
Orchestration upstream: none
Blocks v{N}: none
Priority: 1
Source: "ABC481 arukellt port + WAT emission session (2026-10-28, HEAD 0a64bdb62)"
Related: "#933, #936, #940, #955, ADR-035"
---

# 953 — `Ord::cmp` traps at runtime on both targets; `std::collections::sort::sort` is unusable

## Summary

Calling `Ord::cmp` on a primitive traps with `wasm trap: wasm 'unreachable' instruction
executed`. The same happens for `i32`, `i64` and `f64` (`std/core/cmp.ark` impls), on both
`wasm32` and `wasm32-gc`. As a consequence `std::collections::sort::sort` (which uses
`is_less` -> `a.cmp(b)`) traps for every input, including `n = 3`.

## Repro (cmp)

```ark
use std::host::stdio

fn main() {
    let o: Ordering = 1i32.cmp(3i32)
    match o {
        Ordering::Less => stdio::println("less"),
        Ordering::Equal => stdio::println("equal"),
        Ordering::Greater => stdio::println("greater"),
    }
}
```

Actual:

```
$ scripts/run/arukellt-selfhost.sh compile --target wasm32 --emit wasm -o .build/cmp.wasm tmp/cmp.ark
$ wasmtime run --dir=. .build/cmp.wasm
wasm trap: wasm `unreachable` instruction executed
  at main
```

`wasm-tools dump .build/cmp.wasm` shows `00 unreachable` immediately after the `cmp`
result is dropped, i.e. the generated exhaustiveness check rejects the returned
`Ordering` value. `wasm32-gc` behaves the same.

## Repro (sort)

```ark
use std::host::stdio
use std::collections::sort
use std::prelude::to_string

fn main() {
    let v: Vec<i32> = Vec::new<i32>()
    push(v, 3)
    push(v, 1)
    push(v, 2)
    sort::sort(v)
    stdio::println(to_string(get_unchecked(v, 0)))
}
```

Actual: `wasm trap: wasm 'unreachable'` inside `sort__sort__i32`.

## Notes

- `1 < 3` works; only the trait method result path is broken.
- `Vec::sort_by`/`sort_by_key` rely on the comparator and are equally affected.
- Suspected root: `Ordering` enum ABI/discriminant mismatch between the impl in
  `std/core/cmp.ark` and match lowering, in the #933 family (typechecker results are
  reused by a name-keyed MIR second inference: #936 name dispatch, #940 bare type-name
  identity).

## Acceptance

- [ ] `cmp` returns `Less`/`Equal`/`Greater` correctly for i32/i64/f64 on both targets.
- [ ] Fixture: `sort::sort` sorts n=3 and n=40 `Vec<i32>` and `Vec<i64>` on both targets.
- [ ] Regression fixture covers the `unreachable` trap.

## Update (2026-10-28, later session)

The original repro omitted `use std::core::cmp`. With that import, `1i32.cmp(3i32)`
works on both targets (prints `less`). Without it, the method resolves to a stub and
traps with `unreachable`, so the first half of this issue is a name-resolution stub
problem rather than a broken `Ord` impl.

The free `std::collections::sort::sort` still traps on `Vec<i32>`:

```ark
use std::collections::sort
// v = [3, 1, 2]
sort::sort(v)   // wasm trap: unreachable
```

So the issue should be rescoped to the free sort path; `Vec::sort_by` with a
comparator that returns `Ordering` directly works (see the updated implementation and
`tests/fixtures/stdlib_vec/sort_by_ordering.ark`).
