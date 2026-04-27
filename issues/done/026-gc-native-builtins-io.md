---
Status: done
Created: 2026-03-27
Updated: 2026-03-27
ID: 22
Track: gc-native
Depends on: 023, 024, 025
Orchestration class: implementation-ready
Blocks v1 exit: no
This includes: "`to_string` for all types (produces GC string), `parse_i32/"
---

# GC-native builtins: to_string, parse, math, I/O
- [x] `parse_i32(s: "ref $string)` → `(ref $Result)` with Ok(i32) or Err(String)."
- [x] All `t3-compile: "for_loops/*` fixtures compile (often use println)."
- [x] All `run: for_loops/*` fixtures pass execution.
- The `to_string` pattern for integers: extract digits via repeated
# GC-native builtins: to_string, parse, math, I/O

## Summary

Rewrite all remaining builtin / stdlib functions for GC-native codegen.
This includes: `to_string` for all types (produces GC string), `parse_i32/
i64/f64` (GC string → Result enum), `println` for all types, mathematical
builtins, and `read_line`.

## Context

In bridge mode, builtins operate on linear memory i32 pointers. In GC-native
mode, all string/enum/vec values are GC references. The I/O bridge pattern
(copy GC string → linear memory → fd_write) is established in issue 023.
This issue covers the remaining builtins that depend on strings, enums, and
vectors all being GC-native.

## Acceptance Criteria

### to_string family

- [x] `to_string_i32(n)` → digit extraction loop, build `(ref $string)`.
- [x] `to_string_i64(n)` → same pattern for i64 values.
- [x] `to_string_f64(n)` → float formatting, build `(ref $string)`.
- [x] `to_string_bool(b)` → `"true"` / `"false"` GC string.
- [x] `to_string_char(c)` → single-byte GC string.

### println family

- [x] `println_i32`, `println_i64`, `println_f64`, `println_bool`, `println_char`
      → convert to GC string, copy to linear mem, fd_write.
- [x] `println_string` → copy GC array to linear mem, fd_write.
- [x] Struct/enum println if supported.

### parse family

- [x] `parse_i32(s: ref $string)` → `(ref $Result)` with Ok(i32) or Err(String).
- [x] `parse_i64(s)` → `(ref $Result_i64)` with Ok(i64) or Err(String).
- [x] `parse_f64(s)` → `(ref $Result_f64)` with Ok(f64) or Err(String).

### Math builtins

- [x] `abs`, `min`, `max`, `pow`, `sqrt`, `floor`, `ceil`, `round` — these
      are mostly Wasm-native and should not need major changes.
- [x] `random` if supported.

### I/O

- [x] `read_line` → fd_read from linear mem buffer → build GC string.
- [x] `print` (without newline) if supported.

### Test fixtures

- [x] All `t3-compile:stdlib_io/*` fixtures compile.
- [x] All `run:stdlib_io/*` fixtures pass execution.
- [x] All `t3-compile:stdlib_math/*` fixtures compile.
- [x] All `run:stdlib_math/*` fixtures pass execution.
- [x] All `t3-compile:for_loops/*` fixtures compile (often use println).
- [x] All `run:for_loops/*` fixtures pass execution.

## Key Files

- `crates/ark-wasm/src/emit/t3_wasm_gc.rs` — builtin emission helpers

## Notes

- The `to_string` pattern for integers: extract digits via repeated
  `%10`/`/10`, store into a temporary GC array, then reverse. Can use
  `array.new_default $string (max_digits)` then `array.set` each byte,
  finally `array.copy` to a right-sized result.
- I/O bridge must handle the 1-page memory constraint — chunk output if
  a string exceeds ~65500 bytes.