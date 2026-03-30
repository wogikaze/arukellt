# GC-native builtins: to_string, parse, math, I/O

**Status**: done
**Created**: 2026-03-27
**Updated**: 2026-03-27
**ID**: 026
**Depends on**: 023, 024, 025
**Track**: gc-native
**Blocks v1 exit**: no

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

- [ ] `to_string_i32(n)` → digit extraction loop, build `(ref $string)`.
- [ ] `to_string_i64(n)` → same pattern for i64 values.
- [ ] `to_string_f64(n)` → float formatting, build `(ref $string)`.
- [ ] `to_string_bool(b)` → `"true"` / `"false"` GC string.
- [ ] `to_string_char(c)` → single-byte GC string.

### println family

- [ ] `println_i32`, `println_i64`, `println_f64`, `println_bool`, `println_char`
      → convert to GC string, copy to linear mem, fd_write.
- [ ] `println_string` → copy GC array to linear mem, fd_write.
- [ ] Struct/enum println if supported.

### parse family

- [ ] `parse_i32(s: ref $string)` → `(ref $Result)` with Ok(i32) or Err(String).
- [ ] `parse_i64(s)` → `(ref $Result_i64)` with Ok(i64) or Err(String).
- [ ] `parse_f64(s)` → `(ref $Result_f64)` with Ok(f64) or Err(String).

### Math builtins

- [ ] `abs`, `min`, `max`, `pow`, `sqrt`, `floor`, `ceil`, `round` — these
      are mostly Wasm-native and should not need major changes.
- [ ] `random` if supported.

### I/O

- [ ] `read_line` → fd_read from linear mem buffer → build GC string.
- [ ] `print` (without newline) if supported.

### Test fixtures

- [ ] All `t3-compile:stdlib_io/*` fixtures compile.
- [ ] All `run:stdlib_io/*` fixtures pass execution.
- [ ] All `t3-compile:stdlib_math/*` fixtures compile.
- [ ] All `run:stdlib_math/*` fixtures pass execution.
- [ ] All `t3-compile:for_loops/*` fixtures compile (often use println).
- [ ] All `run:for_loops/*` fixtures pass execution.

## Key Files

- `crates/ark-wasm/src/emit/t3_wasm_gc.rs` — builtin emission helpers

## Notes

- The `to_string` pattern for integers: extract digits via repeated
  `%10`/`/10`, store into a temporary GC array, then reverse. Can use
  `array.new_default $string (max_digits)` then `array.set` each byte,
  finally `array.copy` to a right-sized result.
- I/O bridge must handle the 1-page memory constraint — chunk output if
  a string exceeds ~65500 bytes.
