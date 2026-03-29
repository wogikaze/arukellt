# Prelude Migration Guide (v3)

> **Historical migration note**: this document records the v3 prelude migration strategy and compatibility story.
> For the current stdlib surface, prefer [`reference.md`](reference.md), [`README.md`](README.md), and [`../current-state.md`](../current-state.md).

## Overview

Arukellt reduced the old large prelude in v3 and moved more APIs behind explicit `std::*` module imports.
This page explains that migration model and keeps the older compatibility guidance in one place.

## What this page is for

Use this page when you need to understand:

- why old monomorphic prelude names were deprecated
- what module-oriented replacements were intended
- how the v3 compatibility window was described

Do **not** treat the counts and timeline here as the current source of truth for the live stdlib surface.

## Core Prelude (historical v3 view)

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

## Compat Prelude (historical migration mapping)

| Old API | New API target |
|---------|----------------|
| `Vec_new_i32()` | `collections::vec_new_i32()` |
| `Vec_new_String()` | `collections::vec_new_string()` |
| `push(v, x)` | `v.push(x)` (planned method-style target) |
| `get_unchecked(v, i)` | `v.get(i)` (planned method-style target) |
| `set(v, i, x)` | `v.set(i, x)` (planned method-style target) |
| `sort_i32(v)` | `seq::sort_i32(v)` |
| `sort_String(v)` | `seq::sort_string(v)` |
| `reverse_i32(v)` | `seq::reverse_i32(v)` |
| `contains_i32(v, x)` | `seq::contains(v, x)` |
| `remove_i32(v, i)` | `collections::remove(v, i)` |
| `f64_to_string(x)` | `text::format_f64(x)` |
| `i64_to_string(x)` | `text::format_i64(x)` |

## Migration Steps

1. Add explicit `use std::text`, `use std::seq`, `use std::collections`, etc. imports.
2. Replace older prelude-style calls with module-qualified calls where applicable.
3. Check `docs/stdlib/reference.md` for the currently documented public API rather than relying on this historical map alone.

## Timeline (historical)

- **v3**: both old and new APIs worked during the migration window
- **v4**: deprecation-warning phase was planned
- **v5**: compat prelude removal was planned

Those labels are preserved here as migration history, not as a statement about the current active roadmap.
