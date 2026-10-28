---
Status: open
Created: 2026-10-28
Updated: 2026-10-28
ID: 967
Track: compiler
Depends on: none
Orchestration class: implementation-ready
Orchestration upstream: none
Blocks v{N}: none
Priority: 1
Source: "Vec/stdlib surface expansion session (2026-10-28, HEAD 0a64bdb62)"
Related: "#917, #955, #962, #966, #860"
---

# 967 — Generic Vec methods on `Vec<String>` and generic `iter()` are missing/broken on wasm32

## Summary

Adding the requested Vec surface exposed two gaps around type-generic collections:

1. Generic bodies work for `Vec<i32>` (`first/last/swap/reverse/insert/remove/swap_remove`
   pass on both targets) but trap on `wasm32` for `Vec<String>`. The first trapping
   operation observed was `swap`/`insert`:
   ```
   let v: Vec<String> = ...
   v.swap(0, 2)            // wasm trap: unreachable
   ```
2. `Vec<T>::iter()` exists only as monomorphic impls for `Vec<i32>` and `Vec<String>`
   (`std/core/iter.ark`). `Vec<i64>.iter()` fails typechecking:
   ```
   error[E0200|typecheck]: type mismatch: i32 vs i64
   ```
   so the requested generic `iter()` + `map/filter/collect` chain has no generic
   `Vec<T>` entry point.

## Notes

- The generic impl methods in `std/collections/vec.ark` deliberately call only builtin
  sibling methods (`self.len/get_unchecked/set/push/pop`); calling module-level
  `vec_*` helpers from a specialized impl body lowers to an `unreachable` stub
  (same shape as #917).
- Iterator adapters themselves are generic (`VecIter<T>::map/filter/collect`), but
  `IntoIterator for Vec<T>` is only implemented for i32/String.

## Acceptance

- [ ] `Vec<String>` generic mutation methods run on `wasm32`.
- [ ] Generic `Vec<T>::iter()` (or `IntoIterator for Vec<T>`) typechecks for at least
      i32/i64/String, and `v.iter().map(f).filter(g).collect()` runs on both targets.
- [ ] Fixture covers the iterator chain for i64 and String.
