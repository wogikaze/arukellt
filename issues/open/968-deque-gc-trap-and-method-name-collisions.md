---
Status: open
Created: 2026-10-28
Updated: 2026-10-30
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

## Re-verification (2026-10-30, wave/deque-frees)

The two reported failures no longer reproduce on the current compiler (s2
built from `494727819` + this wave, wasm32-gc pin `56ff0808…`):

- **Repro A** (`Deque::new`, push_back/push_front, `pop_front`,
  `is_empty`, `len`): prints `0 / false / 1 / 2` on `wasm32-gc`; the same
  operations run on `wasm32` with the pinned compiler. The receiver methods
  now resolve from the mangled `Deque` owner, so `Vec::len` / `Vec::is_empty`
  are not selected.
- **Repro B** (0-1 BFS): the ABC481-D shape was scaled to a 450x450 grid
  (202,500 nodes, ~810k directed edges, 0-1 weights). An assert-only copy
  passes on both targets with the current s2: `wasm32` core Wasm exit 0 and
  `wasm32-gc` P2 component exit 0 (`visits = 202500`, `total = 45410625`,
  `max_dist = 449`). The printable P2 component also passes.
- **Regression check**: `tests/fixtures/stdlib_deque/deque_bfs.ark` (run +
  t3-run) pins the same BFS shape, and the existing `stdlib_deque/*` fixtures
  cover the remaining method surface.

One unrelated limitation blocks printable `wasm32` runs: current s2 emits
invalid Wasm for `Stdio::println(x.to_string())` on `wasm32`
(`type mismatch: expected i32 but nothing on stack`, func validation). The
pinned compiler predates it and the same program compiles on `wasm32-gc`, so
the BFS acceptance evidence uses assert-only code on `wasm32`.

## Workaround

The shipped D solution keeps the manual array-based deque. Numeric limits were moved
to `std::core::limits` functions (issue #969), which work on both targets.

## Acceptance

- [ ] `Deque` push/pop/is_empty/len are usable on both targets in a BFS fixture.
- [ ] No name collision with `Vec::is_empty` / `Vec::len`.
- [ ] Fixture: 0-1 BFS over `Deque<i32>` at N=M=200k on both targets.
