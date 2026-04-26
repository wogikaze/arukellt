# Scalar Types Specification

> **Archive**: This document is a historical reference and is not the current behavior contract.
> For current verified behavior, see [../current-state.md](../current-state.md).

## Overview

Arukellt provides a complete set of scalar types for integer, floating-point,
and boolean operations. All scalar types are value types and participate in
the type system's static checking.

## Scalar Type Inventory

| Type  | Width | Signed | Wasm valtype | Range                    |
|-------|-------|--------|-------------|--------------------------|
| `i8`  | 8     | yes    | `i32`       | -128 to 127              |
| `i16` | 16    | yes    | `i32`       | -32768 to 32767          |
| `i32` | 32    | yes    | `i32`       | -2^31 to 2^31-1          |
| `i64` | 64    | yes    | `i64`       | -2^63 to 2^63-1          |
| `u8`  | 8     | no     | `i32`       | 0 to 255                 |
| `u16` | 16    | no     | `i32`       | 0 to 65535               |
| `u32` | 32    | no     | `i32`       | 0 to 4294967295          |
| `u64` | 64    | no     | `i64`       | 0 to 2^64-1              |
| `f32` | 32    | n/a    | `f32`       | IEEE 754 single          |
| `f64` | 64    | n/a    | `f64`       | IEEE 754 double          |
| `bool`| 1     | n/a    | `i32`       | true / false             |
| `char`| 32    | n/a    | `i32`       | Unicode scalar value     |

## Literal Syntax

Suffix literals specify the type explicitly:

```
42i32       // i32 (default for int literals)
42i64       // i64
42u8        // u8
42u16       // u16
42u32       // u32
42u64       // u64
42i8        // i8
42i16       // i16
3.14f64     // f64 (default for float literals)
3.14f32     // f32
```

Hex literals also support suffixes:

```
0xFFu8      // u8, value 255
0xDEADu16   // u16, value 57005
0xCAFEBABEu32 // u32
```

Without a suffix, integer literals default to `i32` and float literals to `f64`.

## Type Annotations

Variables can be explicitly typed via annotation:

```
let x: u8 = 42       // integer literal coerced to u8
let y: u64 = 100     // integer literal coerced to u64
let z: f32 = 3.14    // float literal coerced to f32
```

## Implicit Conversion Policy

**No implicit conversions between scalar types.** All conversions must be
explicit via conversion functions. This prevents silent precision loss and
ensures type safety.

## Arithmetic Operations

All scalar types support basic arithmetic: `+`, `-`, `*`, `/`, `%`.
Comparison operators `<`, `<=`, `>`, `>=`, `==`, `!=` also work on all
scalar types.

### Unsigned Semantics in Wasm

Since `u8`, `u16`, `u32`, `i8`, `i16` are stored as `i32` in Wasm:

- Division: unsigned types use `i32.div_u` / `i64.div_u`
- Remainder: unsigned types use `i32.rem_u` / `i64.rem_u`
- Comparison: unsigned types use `i32.lt_u` / `i64.lt_u` etc.
- **Note**: Current implementation uses signed Wasm operations for all
  i32-width types. Unsigned division/comparison semantics are a future
  refinement.

### Overflow Behavior

Arithmetic overflow follows Wasm's wrapping semantics:
- `u8` and `u16` operations may produce results outside their range at
  the i32 level. Masking to enforce range (e.g., `result & 0xFF` for u8)
  is planned for a future pass.

## Wasm Mapping Details

### GC Packed Types

Wasm GC defines `i8` and `i16` as *packed types* (`packedtype ::= i8 | i16`)
that exist only within `storagetype` for struct/array fields. They are **not**
valid as function parameters, local variables, or return values in Wasm.

Arukellt's `i8` and `i16` types are always compiled as `i32` for locals,
params, and returns. When stored in GC struct/array fields, the compiler
may use packed types for space efficiency (future optimization).

### Type Width at Wasm Level

| Arukellt type | Wasm local type | Notes                         |
|---------------|-----------------|-------------------------------|
| u8/u16/u32    | i32             | Same bit pattern as i32       |
| i8/i16        | i32             | Sign-extended in i32          |
| u64           | i64             | Same bit pattern as i64       |
| f32           | f32             | Native Wasm f32               |

## Conversion Functions (Planned)

The following conversion functions are planned for `std/prelude.ark`:

```
u8_to_i32(v: u8) -> i32
i32_to_u8(v: i32) -> u8
u16_to_i32(v: u16) -> i32
i32_to_u16(v: i32) -> u16
u32_to_i32(v: u32) -> i32
i32_to_u32(v: i32) -> u32
u64_to_i64(v: u64) -> i64
i64_to_u64(v: i64) -> u64
i8_to_i32(v: i8) -> i32
i32_to_i8(v: i32) -> i8
i16_to_i32(v: i16) -> i32
i32_to_i16(v: i32) -> i16
```

**Status**: These require emitter support for functions with new scalar
type parameters. Currently deferred until the emitter's function
compilation handles all scalar types in parameter/return positions.

## Related Issues

- Conversion functions: Requires function param/return type handling for
  new scalars in T1/T3 emitters
- Unsigned arithmetic: `div_u`, `rem_u`, `lt_u` etc. for correctness
- Overflow masking: `& 0xFF` for u8, `& 0xFFFF` for u16 after arithmetic
- GC packed fields: Use `i8`/`i16` packed types in struct/array storage
