# Prelude Migration Guide (v3)

## Overview

The Arukellt prelude contains 67 functions auto-imported into every module.
In v3, these are classified into **Core** (always available) and **Compat**
(to be moved to explicit module imports in v4).

## Core Prelude (15 symbols — remains auto-imported)

| Symbol | Category |
|--------|----------|
| `println` | I/O |
| `print` | I/O |
| `eprintln` | I/O |
| `String_from` | String construction |
| `String_new` | String construction |
| `eq` | String comparison |
| `concat` | String operations |
| `len` | String / Vec length |
| `slice` | String slicing |
| `split` | String splitting |
| `join` | String joining |
| `panic` | Error handling |
| `assert` | Testing |
| `i32_to_string` | Conversion |
| `bool_to_string` | Conversion |

## Compat Prelude (deprecated — migrate to module APIs in v4)

| Old API | New API (v4 target) |
|---------|---------------------|
| `Vec_new_i32()` | `collections::vec_new_i32()` |
| `Vec_new_String()` | `collections::vec_new_string()` |
| `push(v, x)` | `v.push(x)` (method syntax, v4) |
| `get_unchecked(v, i)` | `v.get(i)` (method syntax, v4) |
| `set(v, i, x)` | `v.set(i, x)` (method syntax, v4) |
| `sort_i32(v)` | `seq::sort_i32(v)` |
| `sort_String(v)` | `seq::sort_string(v)` |
| `reverse_i32(v)` | `seq::reverse_i32(v)` |
| `contains_i32(v, x)` | `seq::contains(v, x)` |
| `remove_i32(v, i)` | `collections::remove(v, i)` |
| `f64_to_string(x)` | `text::format_f64(x)` |
| `i64_to_string(x)` | `text::format_i64(x)` |

## Migration Steps

1. Add explicit `use std::text`, `use std::seq`, etc. imports
2. Replace deprecated function calls with module-qualified calls
3. In v4, `use std::prelude_compat` will restore old names if needed

## Timeline

- **v3**: Both old and new APIs work. No warnings yet.
- **v4**: W0100 deprecation warnings for compat functions.
- **v5**: Compat prelude removed. Module imports required.
