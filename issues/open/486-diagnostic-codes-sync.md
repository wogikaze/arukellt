# Diagnostic codes sync: implement missing codes

**Status:** open  
**Track:** verification-hygiene  
**Updated:** 2026-06-10

## Summary

The diagnostic codes check (`scripts/check/check-diagnostic-codes.sh`) reports 39 codes documented in `docs/compiler/error-codes.md` that are missing from the implementation.

## Missing Codes

### Resolve (E01xx)

| Code | Description |
|------|-------------|
| E0101 | duplicate definition |
| E0102 | access to private symbol |
| E0103 | circular import |
| E0104 | module not found |

### Resolve / Module (E012x)

| Code | Description |
|------|-------------|
| E0120 | (module resolution) |
| E0121 | (module resolution) |
| E0122 | (module resolution) |
| E0123 | (module resolution) |
| E0124 | (module resolution) |

### Typecheck (E02xx)

| Code | Description |
|------|-------------|
| E0201 | missing type annotation |
| E0202 | wrong number of arguments |
| E0203 | invalid generic usage |
| E0204 | non-exhaustive match |
| E0205 | mismatched match arm types |
| E0206 | invalid pattern |
| E0207 | cannot mutate immutable variable |
| E0208 | missing return value |
| E0209 | unreachable pattern |
| E0210 | incompatible error type for `?` operator |
| E0211 | module contains only unimplemented host stubs |

### Language version / Target (E03xx)

| Code | Description |
|------|-------------|
| E0300 | traits are not available in v0 |
| E0301 | method call syntax is not available in v0 |
| E0302 | nested generics are not allowed in v0 |
| E0303 | `for` loop is not available in v0 |
| E0304 | operator overloading is not available in v0 |
| E0305 | unsupported target |
| E0306 | invalid emit kind for target |
| E0307 | feature not available for target |

### Misc (E05xx)

| Code | Description |
|------|-------------|
| E0501 | symbol not found in module |

### Warnings (Wxxxx)

| Code | Description |
|------|-------------|
| W0001 | possible unintended sharing of reference type |
| W0002 | deprecated target alias |
| W0003 | ambiguous import |
| W0004 | generated Wasm module failed validation |
| W0005 | non-exportable function skipped |
| W0006 | unused import |
| W0007 | unused binding |
| W0008 | deprecated API |
| W0009 | WASI Preview 2 native Wasm imports are not fully implemented |
| W0101 | deprecated `import <name>` syntax |

## Reference Files

- **Documentation:** `docs/compiler/error-codes.md` — canonical list of all diagnostic codes
- **Implementation:** `src/compiler/diagnostics.ark` — where code constants should be defined

## What Needs to Be Done

1. Define a constant function in `src/compiler/diagnostics.ark` for each missing code, following the existing pattern (e.g. `pub fn DIAG_PARSE_UNEXPECTED() -> String { String_from("E0001") }`)
2. The check script is strict: it searches the `src/compiler/` tree for each documented code string, so the constant must be defined or the code referenced in compiler source.

## Notes

- Codes **E0401** and **E0090** are already in use but hardcoded as string literals in `src/compiler/driver.ark` rather than defined as named constants in `diagnostics.ark`. They are not flagged by the check because the script finds them referenced in source, but they would benefit from being converted to proper constants for consistency.
- Codes **E0100**, **E0200**, **E0400**, **E0402**, and **E0500** are similarly hardcoded in `src/compiler/main.ark` and `src/compiler/driver.ark` — present in implementation but not as named constants.
## Verification

After adding the missing codes, run:
```bash
bash scripts/check/check-diagnostic-codes.sh
```
This should exit with status 0.
