---
Status: done
Created: 2026-03-27
Updated: 2026-03-30
ID: 19
Track: gc-native
Depends on: 17
Orchestration class: implementation-ready
Blocks v1 exit: no
String literal: 
Operations: 
---

# GC-native strings: array i8 + array.new_data
;; Result: "(ref $string) with 5 elements"
;; Print helper: copy GC string → linear mem → fd_write
;; Loop: "for i in 0..len { i32.store8 (12+i) (array.get_u $string s i) }"
- [x] All `t3-compile: hello/*` fixtures compile.
- [x] All `run: hello/*` fixtures pass execution.
- `(mut i8)` is required: `array.copy` and `array.new_data` target arrays
- Print buffer size: linear memory scratch area at offset 12 can hold up to
# GC-native strings: array i8 + array.new_data

## Summary

Implement GC-native string representation. A string is `(ref $string)` where
`$string = (array (mut i8))`. String literals use `array.new_data` to
initialize directly from data segments. All string builtins (len, char_at,
concat, substring, comparison, contains, starts_with, etc.) are rewritten
to operate on GC arrays. Print helpers copy GC array bytes to linear memory
for WASI fd_write.

## Design

```wat
(type $string (array (mut i8)))
```

**String literal:**

```
;; "hello" stored in data segment $d0 at bytes [0..5]
(array.new_data $string $d0 (i32.const 0) (i32.const 5))
;; Result: (ref $string) with 5 elements
```

**Operations:**

```wat
;; string_len(s)
(array.len (local.get $s))                     ;; → i32

;; char_at(s, i)
(array.get_u $string (local.get $s) (local.get $i))  ;; → i32 (byte value)

;; concat(a, b)
(local.set $la (array.len (local.get $a)))
(local.set $lb (array.len (local.get $b)))
(local.set $result
  (array.new_default $string (i32.add (local.get $la) (local.get $lb))))
(array.copy $string $string
  (local.get $result) (i32.const 0)            ;; dest, dest_offset
  (local.get $a) (i32.const 0)                 ;; src, src_offset
  (local.get $la))                             ;; length
(array.copy $string $string
  (local.get $result) (local.get $la)
  (local.get $b) (i32.const 0)
  (local.get $lb))
(local.get $result)

;; Print helper: copy GC string → linear mem → fd_write
;; Loop: for i in 0..len { i32.store8 (12+i) (array.get_u $string s i) }
;; Then set IOV and call fd_write
```

## Acceptance Criteria

- [x] `$string = (array (mut i8))` registered in GcTypeRegistry.
- [x] String literals emit `array.new_data $string $data_idx (offset) (len)`.
- [x] Data segments hold raw UTF-8 bytes (passive segments with dataidx).
- [x] `string_len` → `array.len`.
- [x] `char_at` → `array.get_u $string`.
- [x] String concatenation (`+` operator) → allocate + `array.copy` × 2.
- [x] `substring` → allocate + `array.copy`.
- [x] String equality comparison → element-wise `array.get_u` loop.
- [x] `contains`, `starts_with`, `ends_with` → appropriate loop patterns.
- [x] `to_uppercase`, `to_lowercase` → element-wise transform if supported.
- [x] Print helper (`__print_str_ln`) copies GC array to linear memory[12..],
      sets up IOV at [0..8], calls fd_write.
- [x] Empty string `""` works correctly (zero-length array).
- [x] All `t3-compile:stdlib_string/*` fixtures compile.
- [x] All `run:stdlib_string/*` fixtures pass execution.
- [x] All `t3-compile:hello/*` fixtures compile.
- [x] All `run:hello/*` fixtures pass execution.

## Key Files

- `crates/ark-wasm/src/emit/t3_wasm_gc.rs` — string emission, builtins, print helpers

## Notes

- `(mut i8)` is required: `array.copy` and `array.new_data` target arrays
  must have mutable element types. String immutability is source-language
  enforced, not Wasm type enforced.
- `array.new_data` takes `(i32 offset_into_data_seg, i32 num_elements)`.
  Each data segment holds one or more string literals; track per-literal
  offset and length.
- Print buffer size: linear memory scratch area at offset 12 can hold up to
  ~65500 bytes (one 64KB page minus IOV header). For strings longer than
  this, use chunked writes.
- `to_string` for integers/floats (producing GC string from numeric values)
  is deferred to issue 025 (builtins).