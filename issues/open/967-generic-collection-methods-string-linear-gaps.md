---
Status: open
Created: 2026-10-28
Updated: 2026-10-30
ID: 967
Track: compiler
Depends on: "#966"
Orchestration class: blocked-by-upstream
Orchestration upstream: "#966"
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

## Implementation (2026-10-30)

`std/core/iter.ark`: the monomorphic `impl Vec<i32> { fn iter }` is replaced by a
single item-generic `impl<T> Vec<T> { fn iter(self: Vec<T>) -> VecIter<T> }`.
A second monomorphic `impl Vec<i64> { fn iter }` cannot work here: every
per-instantiation inherent impl on `Vec` registers the bare `Vec::iter` owner
(`resolver/impl_target.ark` and `corehir/decl_impl_target.ark`
`impl_target_type_name` special-case only `Vec_String`), so the first registered
`Vec::iter` (`Vec<i32>`) shadows the `Vec<i64>` one and the i64 receiver is
rejected with `E0200: type mismatch: i32 vs i64`. The generic impl keeps one
symbol and binds `T` from the receiver, so `Vec<i64>::iter()` now typechecks.
`Vec<String>::iter` stays as-is (`Vec_String` owner) and still wins for String.
i32 is unchanged functionally: sampled fixtures (`iterator/custom_iterator`,
`stdlib_vec/vec_filter`, `vec_any_all_match`, `vec_algorithms`) and an i32
`for x in v.iter()` + `filter().collect()` probe pass on both targets.

### Remaining blocker: i64 item codegen

Compiling is only half the gap. Consuming an i64 iterator emits invalid Wasm, so
no run fixture can be added yet:

```
$ scripts/run/arukellt-selfhost.sh compile --target wasm32 --emit wasm \
    -o wt/lane30/t6.wasm wt/lane30/t6.ark   # v.iter().map(double).filter(is_big).collect()
arukellt-selfhost: error — W0004 generated Wasm failed validation
  error: func 16 failed to validate
  0: type mismatch: expected i32, found i64 (at offset 0xccb)
  # reg-vt-audit: mismatch callee=collect_iterator__MapFilterIter_i64_i64_ old=1 new=5
  #                mismatch callee=MapFilterIter__collect__i64__i64
```

The same i32/i64 slot mismatch appears for `for x in v.iter()` (func 10 invalid),
for an explicit `while ... it.next()` loop (func 12 invalid), and on
`wasm32-gc` (`compiler produced invalid core Wasm`). Creating the iterator and
passing it around does work (`let it = v.iter()`, `it.is_empty()`) and runs on
both targets. So the typechecker half is fixed; the MIR/mono lowering of an i64
`Iterator::Item` through the generic adapters (`VecIter`/`MapIter`/`FilterIter`/
`collect_iterator`) is still i32-shaped and belongs to the compiler lane
(same family as #966 method-dispatch work), not to `std/`.

### Acceptance 1 measurement (not fixed here)

`Vec<String>::swap(0, 2)` on `wasm32` still traps:

```
$ wasmtime run wt/lane30/t9.wasm
2: wasm trap: wasm `unreachable` instruction executed
```

This is the generic-mutation gap tracked under #966; left as unresolved here.

### Fixture (blocked)

The intended fixture (`tests/fixtures/stdlib_vec/vec_iter_i64.ark`: build
`Vec<i64>`, `v.iter().map(double).filter(is_big).collect()`, assert len/values
with `Test::assert_eq_i64`) fails with the W0004 above, so it is not added.
Add it together with the compiler fix and register:

```
run:stdlib_vec/vec_iter_i64.ark
t3-compile:stdlib_vec/vec_iter_i64.ark
t3-run:stdlib_vec/vec_iter_i64.ark
```

## Lead integration decision (2026-10-30) — blocked on #966, std half kept

`std/core/iter.ark` now has one `impl<T> Vec<T> { fn iter }` instead of the
`impl Vec<i32>` special case (the file already uses generic impls for
`VecIter<T>` / `IntoIterator`). This is kept: it removes the i32-only shape and
is the correct form for the eventual compiler fix.

The issue is **not** implementation-ready: both remaining acceptance points
need the same root, which is #966's method/impl identity work:

1. `Vec<String>` generic mutation still traps on `wasm32` (`swap` → `unreachable`);
2. an i64 `Iterator::Item` is lowered `i32`-wide through
   `VecIter`/`MapIter`/`FilterIter`/`collect_iterator`, so every consuming form
   (`for`, `next()` loop, `collect()`) emits invalid Wasm.

Frontmatter now records `Depends on: #966` /
`Orchestration class: blocked-by-upstream`. Re-test with the fixture named
above once #966 lands.

