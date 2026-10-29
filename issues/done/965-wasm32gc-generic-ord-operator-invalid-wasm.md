---
Status: done
Created: 2026-10-28
Updated: 2026-10-29
ID: 965
Track: compiler
Depends on: none
Orchestration class: implementation-ready
Orchestration upstream: none
Blocks v{N}: none
Priority: 1
Source: "Vec/stdlib surface expansion session (2026-10-28, HEAD 0a64bdb62)"
Related: "#933, #953, #955, #962, #966, #975, ADR-036"
---

# 965 — wasm32-gc: `Vec<T: Ord>::sort` / `sort_unstable` / `is_sorted` / `select_nth` emit invalid core Wasm for `i64` and `String`

## Summary

After the sort-panic lane (`df8040652`) moved the no-argument algorithms to
`impl<T: Ord> Vec<T>`, `i32` worked on both targets but `i64` and `String`
still produced an invalid core module on `wasm32-gc`:

```
$ scripts/run/arukellt-selfhost.sh compile --target wasm32-gc --emit component -o out.wasm probe.ark
arukellt-selfhost: error — compiler produced invalid core Wasm
```

The original "generic `<` on `T: Ord`" repro was already fixed by the time
this lane started; the concrete failures were the bounded `Vec<T>` methods.

## Root cause

The invalid module had two independent type mismatches, both visible in the
emitted WAT (`--emit wat`, then `wasm-tools validate -v`):

1. **Method signature registration did not project impl type binders.**
   `sig_reg_params_from_method_info` registered a monomorphized method's
   parameters from the declared spelling (`self: Vec<T>` → `vec:T`) instead of
   substituting the instance's type argument. The signature fallback maps an
   unresolved `vec:T` to the default `vec:i32` layout, so
   `Vec__sort__i64` was declared as `(func (param (ref null 10)))` where type
   10 is `Vec<i32>`, while its MIR body used type 22 (`Vec<i64>`):

   ```
   (func (type 69) ... )              ;; type 69 = (param (ref null 10)) — Vec<i32>
     local.get 0
     ref.cast (ref null 22)           ;; body receiver is Vec<i64>
   ```

   `wasm-tools validate -v` reported
   `type mismatch: expected (ref null $type), found (ref null $type) (at offset 0x74c)`.

2. **Generic constructor calls inside generic bodies kept the AST type name.**
   `lower_direct_call_callee_name` builds `Vec__with_capacity__<annotation>`
   from the source text, so inside `sort<T>` the scratch constructor became
   `Vec__with_capacity__T`. The mono span rewrite never saw it (the name was
   already mangled), and the emitter lowered it as the generic i32 vec. The
   body then stored a `Vec<i32>` result into the `Vec<i64>` scratch local:

   ```
   local.get 2
   call 17                          ;; Vec<i32> with_capacity
   local.set 6                      ;; local 6 is (ref null 22) — Vec<i64>
   ```

`String` failed the same way (`Vec<String>` → `Vec<i32>` signature, then an
invalid `local.set`). On the older pinned bootstrap the same probe instead
lowered the call to `unreachable`, which traps at runtime.

## Fix

- `src/compiler/mir/lower/signature_registry_entry.ark`:
  `sig_reg_params_from_method_info` now takes the declared method type
  parameters (impl binders first), substitutes each parameter type through the
  mono suffix / recorded `MonoInstance`, and derives the parameter value type
  from the substituted name — the same projection the top-level function path
  already used.
- `src/compiler/mir/lower/call_mono.ark`:
  `concretize_vec_ctor_projection` now also rewrites a bare type-variable
  element (`Vec::with_capacity<T>` / `Vec::new<T>`) through the enclosing
  instance's mono map, so the call names `Vec__with_capacity__i64` and lowers
  through the existing wasm name fallback with the concrete element array.

With the fix the same WAT declares
`(func (param (ref null 22)))` for `Vec<i64>::sort`,
`(func (param (ref null 19)))` for `Vec<String>::sort`, and the scratch
constructor returns the concrete element vec.

## Acceptance

- [x] Repro compiles and runs on `wasm32-gc`.
      `tmp` probe `less<T: Ord>` / `same<T: Ord>` prints `true` / `true` on
      `wasm32-gc`.
- [x] Generic `Vec<T: Ord>::sort` fixture passes on both targets.
      `tests/fixtures/stdlib_vec/vec_sort_ord_i64.ark` and
      `vec_sort_ord_string.ark` (+ `_wasm32` variants with
      `--target wasm32 --wasi-version wasi-p1 --memory-width memory32`)
      compile, `wasm-tools validate`, execute, and match `.expected` on both
      targets. `check-cross-backend-parity.py --fixture ...` reports
      `parity=2`.
- [x] `T: Eq` bound resolves the existing `impl Eq for i32`.
      `std::core::cmp` is module-gated; with `use std::core::cmp`,
      `fn equal<T: Eq>(a: T, b: T) -> bool { a == b }` compiles and prints
      `true` on `wasm32-gc`. Without the import it still fails `E0200`, which
      is the documented module-gated impl contract, not a missing impl.

## Verification

- `python3 scripts/manager.py verify fixtures` — 1899 compile+validate PASS,
  0 failed; 110 runtime executed; 36 `test:` fixtures PASS.
- `python3 scripts/check/check-cross-backend-parity.py --fixture
  stdlib_vec/vec_sort_ord_i64.ark --fixture stdlib_vec/vec_sort_ord_string.ark`
  → `parity=2`.
- `python3 scripts/manager.py verify lane` PASS.
- `python3 scripts/manager.py docs check` 6/6.
- `python3 scripts/manager.py selfhost fixpoint` PASS —
  sha256(s2) == sha256(s3) =
  `569c3e16940569b173ea9a320f674078612f2535997b23546155771b60c11f3a`.

## Residual

- `reg-vt-audit` still reports `Vec__new__i64 old=1 new=5`: the MIR-level
  legacy return vt of generic `Vec::new` differs from the signature registry's
  GC ref. Emitted signatures and both-target execution are correct, so this is
  diagnostic noise in the same subsystem, not a codegen failure here.
- The `String` `<` / `>` operators do not implement string ordering on
  `wasm32` (linear): `"apple" < "pear"` evaluates `false` there while
  `a.cmp(b)` is correct. The new fixtures assert ordering through `Ord::cmp`
  for that reason. This is a separate operator-lowering defect; it is not part
  of the `Vec<T: Ord>` method surface.

## Follow-up (2026-10-29, wave/vec-sort-custom)

The two residuals this lane left behind are fixed and `std::seq::sort_by` is
retired. Minimal repros used the pinned bootstrap before the fix:

- `Vec<i64>` 5-element `.sort()` on `wasm32` printed
  `10,20,30,50,34359738373` (expected `5,10,20,30,50`); the gc component path
  was correct.
- `Vec<cmp::Reverse<i32>>.sort()` trapped on both targets inside
  `Vec__sort__Reverse_i32_`; `v.sort_by(cmp)` worked.

### Root causes

1. **Linear constructor sizing ignored the resolved element type.** WAT for the
   scratch constructor inside generic `sort` allocated 44 bytes
   (`12-byte header + 8 * 4`) while publishing an eight-element capacity, so a
   `Vec<i64>` scratch held only four elements and the fifth write overflowed
   into the next bump allocation. The `call_fallback_intrinsics` vec-new
   fallback passed a hard-coded `heap_bump = 32`, and `emit_raw_array_new`
   derived the stride from the unresolved `vec:T` local.
2. **Parametric `Ord` dispatch lost the element type.** Inside `Vec<T: Ord>`
   methods the receiver is annotated but the accessor sig was not:
   `fn get_unchecked(self, idx)` left the result as an unbound type parameter,
   so `left.cmp(right)` could not record the `T: Ord` impl demand edge. MIR
   lowered the callee as the non-existent `Reverse_i32::cmp` (`fid=-1`) while
   the parametric instance is named `Reverse__cmp__i32`, and the emitter's
   unresolved-call fallback emitted `drop; drop; unreachable` for the merge
   comparator. `min` / `max` / `is_sorted` / `binary_search` shared the defect.

### Fix

- `std/collections/vec.ark`: `fn get_unchecked(self: Vec<T>, idx: i32) -> T`
  now states the receiver type, so the element type reaches the signature and
  the checker records/rebinds the `T: Ord` var-method site for parametric
  impls. No other accessor signature changed.
- `src/compiler/wasm/intrinsic_vec_type_new.ark`: new
  `resolve_element_type_name_for_new_dest` / `linear_element_stride` /
  `linear_vec_new_heap_bump` resolve the constructor element from the local
  annotation, immediate store target, monomorphized constructor spelling, or
  enclosing fn return, then size the linear buffer as eight element strides.
  `call_fallback_intrinsics` and `emit_raw_array_new` use it; the duplicated
  stride mapping in `lm_grow_elem_stride` now shares `linear_element_stride`.
- Retired `std::seq::sort_by`: manifest entry and declaration deleted, E0100
  hint `removed free sort algorithm; use ``v.sort()`` via
  ``use std::collections::vec``` added, the now-dead
  `seq::sort_by` identity marker removed from MIR lowering, call sites migrated
  to `v.sort()` / `v.sort_by(cmp)`. The `sort_i32` E0100 hint now points at
  `v.sort()` as well.

### Evidence

- `tests/fixtures/stdlib_vec/vec_sort_ord_i64{,_wasm32}.ark` and
  `vec_sort_ord_string{,_wasm32}.ark` now sort 5 elements (sort,
  sort_unstable, select_nth); both targets execute and match `.expected`.
- New `tests/fixtures/stdlib_trait/ord_sort_custom_struct{,_wasm32}.ark`
  covers `Vec<cmp::Reverse<i32>>` sort / is_sorted / min / max on both
  targets.
- New gc core/test-path coverage (`arukellt test`):
  `stdlib_sort/sort_i64.ark` (migrated to `v.sort()`),
  `stdlib_sort/sort_string.ark`, `stdlib_sort/sort_custom_ord.ark`
  (`Reverse` and a user `impl<T> Ord for Rev<T>`), all PASS.
- `python3 scripts/check/check-cross-backend-parity.py` on the three sort
  fixtures → `parity=3`.
- `python3 scripts/manager.py verify fixtures` → 1910 compile+validate PASS,
  0 failed; 110 runtime executed; 38/38 `test:` fixtures PASS; 2
  runtime-unsupported (unchanged).
- `python3 scripts/manager.py verify lane` PASS; `docs regenerate` +
  `docs check` 6/6; `check-stdlib-eradication` 193/193 + 2/2.
- `python3 scripts/manager.py selfhost fixpoint` PASS —
  sha256(s2) == sha256(s3) =
  `c00d2e0ccc3c0bad6b044e5bf0fc850a8af2da2348a92388562de670f7deb3bf`
  (pinned base `bootstrap/arukellt-selfhost.wasm`, sha256
  `471855ad9db035c1ada136852fc55e896b477cf35303be066f352d9d563a8895`).

### Residual

- `min` / `max` / keyed variants still trap on an empty vector by contract.
- `f64` is not `Ord`; use `v.sort_by(total_cmp)`.
- `#971`'s capturing-comparator item (E0213) remains open; its aggregate
  `sort_by` repros now run on both targets (see that issue's update).
