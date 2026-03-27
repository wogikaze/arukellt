# String on WasmGC for T3

**Status**: open
**Created**: 2026-03-27
**Updated**: 2026-03-27
**ID**: 005
**Depends on**: 004
**Track**: main
**Blocks v1 exit**: yes

## Summary

Move T3 String handling to a consistently GC-native representation and remove reliance on T1-style string assumptions from the T3 compile path.

## Acceptance Criteria

- [ ] T3 String values compile through a stable GC-native representation rather than a T1-oriented pointer model.
- [ ] Concat, interpolation, slice, casing, equality, and conversion paths compile correctly for T3.
- [ ] String-related T3 fixtures pass without relying on hidden T1 semantics.
- [ ] T3 String implementation details are documented accurately in current-first docs.

## Goal

Complete String as a WasmGC-native value in T3.

## Implementation

- Define the canonical T3 String representation in `crates/ark-wasm/src/emit/t3_wasm_gc.rs` and make all String operations target that representation.
- Ensure frontend-generated `StringConcatMany` / concat-like call chains lower cleanly into the T3 backend.
- Implement or fix T3 paths for:
  - `concat`
  - interpolation via `to_string`
  - `slice`
  - `starts_with` / `ends_with`
  - `to_lower` / `to_upper`
  - string equality
  - printing/host bridge output
- Keep any temporary linear-memory IO bridge isolated to host interaction, not as the String value model itself.

## Dependencies

- Issue 004.

## Impact

- `crates/ark-wasm/src/emit/t3_wasm_gc.rs`
- String fixtures and stdlib string tests
- possibly CoreHIR/MIR lowering around concat normalization

## Tests

- String fixture matrix.
- Interpolation fixtures.
- Print bridge tests.
- Concat-heavy sample/benchmark smoke.

## Docs updates

- `docs/language/syntax.md`
- `docs/language/syntax-v1-preview.md`
- `docs/platform/wasm-features.md`

## Compatibility

- T3 backend representation changes only.
- User-visible String semantics stay the same.

## Notes

- String identity/alias behavior must not regress when host bridge code is adjusted.
