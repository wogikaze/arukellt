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
