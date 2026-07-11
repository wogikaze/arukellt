# Selfhosting Stdlib Checklist (Verified)

Last verified: 2026-04-14
Scope: `src/compiler/*.ark` only (selfhost compiler implementation)

This checklist is usage-based. It tracks only stdlib APIs that are actually
called by the selfhost compiler sources today.

## Verification Method

1. Enumerate std imports and qualified calls in `src/compiler/*.ark`.
2. Remove comment-only references.
3. Cross-check against:
   - `std/host/*.ark` exported functions
   - `std/prelude.ark` exported functions
   - compiler builtins used by selfhost (`len`, `push`, `get_unchecked`,
     `to_string`, `Ok`, `Err`)

## Host Module Dependencies (Explicit `use std::host::*`)

| Module | Function | Status | Source of truth |
|---|---|---|---|
| `std::host::stdio` | `println` | available | `std/host/stdio.ark` |
| `std::host::stdio` | `eprintln` | available | `std/host/stdio.ark` |
| `std::host::fs` | `read_to_string` | available | `std/host/fs.ark` |
| `std::host::fs` | `write_bytes` | available | `std/host/fs.ark` |
| `std::host::process` | `exit` | available | `std/host/process.ark` |
| `std::host::env` | `args` | available | `std/host/env.ark` |

Notes:
- `std::host::stdio::print`, `std::host::fs::write_string`,
  `std::host::process::abort`, `std::host::env::{arg_count,arg_at,has_flag,var}`
  exist but are not currently called by `src/compiler/*.ark`.

## Prelude / Intrinsic Dependencies (Unqualified Calls)

### Exported by `std/prelude.ark`

| Function | Status |
|---|---|
| `String_from` | available |
| `String_new` | available |
| `Vec_new_String` | available |
| `Vec_new_i32` | available |
| `bool_to_string` | available |
| `char_at` | available |
| `clone` | available |
| `concat` | available |
| `eq` | available |
| `f64_bits_hi` | available |
| `f64_bits_lo` | available |
| `f64_to_string` | available |
| `i32_to_string` | available |
| `index_of` | available |
| `parse_f64` | available |
| `substring` | available |

### Compiler builtins used by selfhost

| Function / Variant | Status | Notes |
|---|---|---|
| `len` | available | builtin collection length |
| `push` | available | builtin Vec append |
| `get_unchecked` | available | builtin unchecked Vec access |
| `to_string` | available | polymorphic builtin (documented in prelude comments) |
| `Ok` | available | `Result` variant constructor/pattern |
| `Err` | available | `Result` variant constructor/pattern |

## Gap Analysis

- Missing functions required by current selfhost compiler usage: **none**.
- Stub functions required by current selfhost compiler usage: **none found**.

## Coverage Summary

- Host APIs used: 6/6 available
- Prelude exports used: 16/16 available
- Builtins used: 6/6 available
- Total required by current `src/compiler/*.ark`: 28/28 available

## Maintenance Rule

When `src/compiler/*.ark` adds new std imports or new unqualified std/prelude
calls, re-run this checklist verification and update this file in the same
change.
