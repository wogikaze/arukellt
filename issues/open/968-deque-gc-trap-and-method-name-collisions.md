---
Status: open
Created: 2026-10-28
Updated: 2026-10-28
ID: 968
Track: stdlib-api
Depends on: none
Orchestration class: implementation-ready
Orchestration upstream: none
Blocks v{N}: none
Priority: 1
Source: "numeric limits / VecDeque session (2026-10-28, HEAD 0a64bdb62)"
Related: "#966, #967, #917, ADR-086"
---

# 968 — `Deque<T>` traps on wasm32-gc in real use; `is_empty`/`len` collide with the Vec methods

## Summary

`std::collections::deque::Deque<T>` exists (the `VecDeque` equivalent) and a minimal
push/pop works on `wasm32`, but:

1. `dq.is_empty()` / `dq.len()` are dispatched to the `Vec` methods of the same name
   (see #966), so a `while !dq.is_empty()` loop traps or reads the wrong layout.
2. A 0-1 BFS using `Deque` (the ABC481 D shape) traps on `wasm32-gc` with
   `out of bounds array access` even on the 7-node sample, whether the loop uses
   `is_empty()` or drains via `match dq.pop_front()`.

## Repro A (method collision)

```ark
use std::collections::deque
use std::host::stdio

fn main() {
    let d = deque::Deque::new<i32>()
    d.push_back(1)
    d.push_front(0)
    stdio::println(d.pop_front().unwrap().to_string())   // wasm32: prints 0
    stdio::println(d.is_empty().to_string())             // gc: trap / wrong layout
}
```

## Repro B (BFS shape, gc)

`tmp/ergo/d_deque.ark` is the ABC481 D solution rewritten with
`deque::Deque::new<i32>()`, `push_front` / `push_back` / `pop_front`. All three
samples pass on `wasm32`; `wasm32-gc` traps at `main` with
`out of bounds array access`.

## Workaround

The shipped D solution keeps the manual array-based deque. Numeric limits were moved
to `std::core::limits` functions (issue #969), which work on both targets.

## Acceptance

- [ ] `Deque` push/pop/is_empty/len are usable on both targets in a BFS fixture.
- [ ] No name collision with `Vec::is_empty` / `Vec::len`.
- [ ] Fixture: 0-1 BFS over `Deque<i32>` at N=M=200k on both targets.
