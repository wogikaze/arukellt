---
Status: open
Created: 2026-06-10
Updated: 2026-06-10
ID: 487
Track: compiler
Severity: medium
---

# len() returns incorrect results for `Vec<Struct>`

## Description

The selfhost compiler's `len()` built-in function produces incorrect results when called on a `Vec<Struct>` (i.e., a vector whose element type is a structured type with multiple fields). It works correctly for primitive element types such as `Vec<i32>`, `Vec<u8>`, etc.

## Observed Behavior

When iterating over a `Vec<SomeStruct>`, calling `.len()` returns an unexpected/invalid value, making the result unusable for loop bounds or bounds checking. The workaround employed in the codebase is to iterate until an EOF sentinel token is reached instead of using the vector's length.

## Workaround Location

**File:** `src/compiler/lexer.ark`  
**Line:** 1040

```ark
// Iterate until EOF token (len() has a bug with Vec<Struct>)
```

This workaround avoids calling `.len()` on the token vector and instead checks each element against an EOF/terminator sentinel to determine when to stop iterating.

## Expected Behavior

`len()` should return the correct element count for *all* `Vec` types, regardless of element type. Specifically:

- `Vec<Struct>` → correct count of elements
- `Vec<i32>` → correct count of elements (already works)
- `Vec<Vec<T>>` → correct count of elements

## Root Cause

Likely a code-generation issue in how the compiler lowers `len()` for vectors whose element layout involves composite (non-primitive) types. The byte offset calculation or length field access may assume a fixed element size and produce the wrong value when elements are structs.

## Impact

- **Severity:** Medium
- The bug only affects `Vec<Struct>` — primitive-typed vectors work correctly.
- A syntactic workaround exists (iterate by sentinel), so the compiler can still function.
- The workaround adds a maintenance burden and a subtle constraint: developers must remember not to use `.len()` on struct-typed vectors.
- If not fixed before wider use, this could silently produce incorrect behavior in generated code.

## Tags

`codegen`, `vec`, `len`, `struct`, `selfhost-compiler`
