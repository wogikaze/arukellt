---
Status: open
Created: 2026-10-28
Updated: 2026-10-28
ID: 972
Track: compiler
Depends on: none
Orchestration class: implementation-ready
Orchestration upstream: none
Blocks v{N}: none
Priority: 2
Source: "G refactor session (2026-10-28, HEAD 2436c92b0)"
Related: "#954, #962, #970, #971"
---

# 972 — `(a..b).rev()` and tuple-of-struct values emit invalid Wasm

## Summary

Two codegen gaps surfaced while shortening the ABC481 G solution (549 → 420 lines).
Both transformations were reverted; the shipped G keeps forward loops and split let
bindings.

## Repro A — `Range::rev()`

```ark
use std::host::stdio

fn main() {
    let mut s: i32 = 0
    for i in (0..5).rev() {
        s = s + i
    }
    stdio::println(s.to_string())
}
```

- `wasm32`: `W0004 generated Wasm failed validation: func 10 ...`
- `wasm32-gc`: `compiler produced invalid core Wasm`

Expected: prints `10`.

## Repro B — tuple of struct values on gc

```ark
struct Pair {
    x: i32,
    y: i32,
}

fn make(x: i32, y: i32) -> Pair {
    Pair { x: x, y: y }
}

fn main() {
    let (a, b) = (make(1, 2), make(3, 4))
    stdio::println((a.x + b.y).to_string())
}
```

- `wasm32`: prints `5`
- `wasm32-gc`: `compiler produced invalid core Wasm`

## Related failures seen in the same refactor

- `let (left, right) = (seg_build(...), seg_build(...))` with struct return values:
  invalid core Wasm on gc (same aggregate-tuple lowering as Repro B).
- `Vec<i64>::resize` / `Vec<i64>::extend`: compiles but traps at runtime on gc with
  `unreachable` (same family as #962 growth corruption; `Vec<i64>` push loops work).

## Acceptance

- [ ] `(a..b).rev()` works on both targets.
- [ ] tuple expressions containing struct values compile and run on `wasm32-gc`.
- [ ] `Vec<i64>::resize` / `extend` run without trapping on `wasm32-gc` (#962).
