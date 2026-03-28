# std::wit

**Stability**: Experimental
**Module**: `std::wit`

## Overview

WIT (WebAssembly Interface Types) primitive type constants and name lookup.
Each WIT primitive type is mapped to an integer constant. Full WIT parser and type mapping are deferred to v4.

> ⚠️ **Experimental**: API may change in minor versions.

## Type Constants

| Function | Value | WIT Type |
|----------|-------|----------|
| `wit_type_bool()` | 1 | `bool` |
| `wit_type_u8()` | 2 | `u8` |
| `wit_type_u16()` | 3 | `u16` |
| `wit_type_u32()` | 4 | `u32` |
| `wit_type_u64()` | 5 | `u64` |
| `wit_type_s8()` | 6 | `s8` |
| `wit_type_s16()` | 7 | `s16` |
| `wit_type_s32()` | 8 | `s32` |
| `wit_type_s64()` | 9 | `s64` |
| `wit_type_f32()` | 10 | `f32` |
| `wit_type_f64()` | 11 | `f64` |
| `wit_type_char()` | 12 | `char` |
| `wit_type_string()` | 13 | `string` |

**Example:**

```ark
use std::wit

let ty = wit::wit_type_i32()
println(i32_to_string(ty)) // "4"
```

## Functions

### `wit_type_name(ty: i32) -> String`

Returns the WIT type name for a given type constant. Returns `"unknown"` for unrecognized values.

**Example:**

```ark
use std::wit

let name = wit::wit_type_name(4)
println(name) // "u32"

let name2 = wit::wit_type_name(13)
println(name2) // "string"

let name3 = wit::wit_type_name(99)
println(name3) // "unknown"
```
