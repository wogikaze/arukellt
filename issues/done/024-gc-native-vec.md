---
Status: done
Created: 2026-03-27
Updated: 2026-03-30
ID: 20
Track: gc-native
Depends on: 19
Orchestration class: implementation-ready
Blocks v1 exit: no
Monomorphize per element type: vec_i32, vec_i64, vec_f64, vec_string.
---

# GC-native Vec<T>: struct + array, monomorphized
;; Vec: ":new() with initial capacity 16"
;; 1. Check: "len == array.len(data) → grow (2× cap, array.copy, replace)"
- [x] Vec literal / `Vec: ":new()` → `array.new_default` + `struct.new $vec_T`."
- [x] Vec grow: "allocate new `array.new_default` (2× cap), `array.copy`"
- [x] HOF: `map`, `filter`, `fold` use `call_ref` for the function argument.
- [x] All `t3-compile: stdlib_hof_i64_f64/*` fixtures compile.
- [x] All `run: stdlib_hof_i64_f64/*` fixtures pass execution.
- Current bridge mode encodes Vec as `[data_ptr: "i32, len:i32, cap:i32]` in"
linear memory. The GC version is structurally different: capacity is
# GC-native Vec<T>: struct + array, monomorphized

## Summary

Implement GC-native Vec<T> as `(struct (field (mut (ref $arr_T))) (field (mut i32)))`.
The backing array provides capacity via `array.len`. Vec operations (push,
get, set, len, contains, reverse, remove) and higher-order functions
(map, filter, fold) are rewritten to use GC struct/array instructions.
Monomorphize per element type: vec_i32, vec_i64, vec_f64, vec_string.

## Design

```wat
(type $arr_i32 (array (mut i32)))
(type $vec_i32 (struct
  (field (mut (ref $arr_i32)))   ;; backing array
  (field (mut i32))              ;; current length
))

;; Vec::new() with initial capacity 16
(array.new_default $arr_i32 (i32.const 16))
(i32.const 0)
(struct.new $vec_i32)

;; push(v, x):
;; 1. Check: len == array.len(data) → grow (2× cap, array.copy, replace)
;; 2. array.set $arr_i32 (data) (len) (x)
;; 3. struct.set $vec_i32 1 (v) (len + 1)

;; get(v, i):
(struct.get $vec_i32 0 (local.get $v))   ;; backing array
(local.get $i)
(array.get $arr_i32)                      ;; element

;; len(v):
(struct.get $vec_i32 1 (local.get $v))   ;; → i32
```

## Acceptance Criteria

- [x] GcTypeRegistry creates `$arr_T` + `$vec_T` pairs for i32, i64, f64, String.
- [x] Vec literal / `Vec::new()` → `array.new_default` + `struct.new $vec_T`.
- [x] `push` → bounds check + optional grow + `array.set` + len increment.
- [x] `get` → `struct.get` data + `array.get`.
- [x] `set` → `struct.get` data + `array.set`.
- [x] `len` → `struct.get` length field.
- [x] Vec grow: allocate new `array.new_default` (2× cap), `array.copy`
      old→new, `struct.set` data field to new array.
- [x] `contains_T` → loop with `array.get` + comparison.
- [x] `reverse_T` → in-place swap loop via `array.get`/`array.set`.
- [x] `remove_T` → shift elements + shrink length.
- [x] HOF: `map`, `filter`, `fold` use `call_ref` for the function argument.
- [x] All `t3-compile:stdlib_vec/*` fixtures compile.
- [x] All `run:stdlib_vec/*` fixtures pass execution.
- [x] All `t3-compile:stdlib_vec_ops/*` fixtures compile.
- [x] All `run:stdlib_vec_ops/*` fixtures pass execution.
- [x] All `t3-compile:stdlib_hof_i64_f64/*` fixtures compile.
- [x] All `run:stdlib_hof_i64_f64/*` fixtures pass execution.

## Key Files

- `crates/ark-wasm/src/emit/t3_wasm_gc.rs` — Vec ops emission

## Notes

- Current bridge mode encodes Vec as `[data_ptr:i32, len:i32, cap:i32]` in
  linear memory. The GC version is structurally different: capacity is
  implicit from `array.len` of the backing array.
- `Vec<String>` needs `$arr_string = (array (mut (ref null $string)))` with
  nullable refs for `array.new_default` (default initializes to ref.null).
- HOF (map/filter/fold) require `call_ref` which depends on issue 025
  (closures) but can be implemented together since the function reference
  infrastructure is shared.