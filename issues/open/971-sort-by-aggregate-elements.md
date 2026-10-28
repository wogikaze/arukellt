---
Status: open
Created: 2026-10-28
Updated: 2026-10-28
ID: 971
Track: compiler
Depends on: none
Orchestration class: implementation-ready
Orchestration upstream: none
Blocks v{N}: none
Priority: 1
Source: "E sort_by investigation (2026-10-28, HEAD 2436c92b0)"
Related: "#953, #966, #967, #970"
---

# 971 — `Vec::sort_by` cannot sort aggregate elements on wasm32-gc, and traps for `String`

## Summary

After making `sort_by` a stable O(n log n) merge sort, it works for `Vec<i32>` on both
targets, but other element types are broken:

| Element type | `wasm32` | `wasm32-gc` |
| --- | --- | --- |
| `i32` | OK | OK |
| `(i32, i32)` tuple | OK | invalid core Wasm |
| struct (`Point`) | OK | invalid core Wasm |
| `String` | runtime trap | runtime trap |

A comparator also cannot capture the point list, so sorting an index vector by a
comparator over external data is rejected:

```
error[E0213|typecheck]: borrowed closure capture cannot escape its enclosing function
```

## Impact

ABC481 E keeps a hand-written merge sort over `Vec<(i32, i32)>` (about 25 lines) and
passes the point vector through the sort instead of using
`sorted.sort_by(angle_cmp)`. Once aggregate elements work on gc, E can drop the manual
sort and use `sort_by`.

## Repro A (tuple element, gc)

```ark
use std::collections::vec
use std::host::stdio

fn cmp(a: (i32, i32), b: (i32, i32)) -> Ordering {
    let (ax, _ay) = a
    let (bx, _by) = b
    if ax < bx { return Ordering::Less }
    if ax > bx { return Ordering::Greater }
    Ordering::Equal
}

fn main() {
    let v: Vec<(i32, i32)> = Vec::new<(i32, i32)>()
    v.push((2, 9))
    v.push((1, 8))
    v.sort_by(cmp)
    let (x, y) = v.get_unchecked(0)
    stdio::println((x + y).to_string())
}
```

- `wasm32`: prints `9`
- `wasm32-gc`: `compiler produced invalid core Wasm`

The same happens with a `struct Point { x, y }` element type.

## Repro B (String element)

`Vec<String>.sort_by(cmp)` traps at runtime on both targets even when the comparator
only compares lengths.

## Repro C (capturing comparator)

```ark
idx.sort_by(|a: i32, b: i32| {
    let (ax, _ay) = pts.get_unchecked(a)
    // ...
})
```

`error[E0213|typecheck]: borrowed closure capture cannot escape its enclosing function`

So a comparator cannot depend on data outside the two elements; sorting indices by an
external key requires either self-contained elements or a capture mechanism.

## Acceptance

- [ ] `Vec<(i32, i32)>.sort_by` and `Vec<Point>.sort_by` compile and run on `wasm32-gc`.
- [ ] `Vec<String>.sort_by` runs without trapping.
- [ ] A comparator may capture an immutable local vector (or an equivalent index-sort API exists).
