# string 型の canonical ABI lift-lower を実装する

**Status**: done
**Created**: 2026-03-31
**Updated**: 2026-03-31
**Closed**: 2026-07-28
**ID**: 296
**Depends on**: 299
**Track**: component-model
**Blocks v1 exit**: no
**Priority**: 17

## Summary

Component Model export で string 型を使う関数のアダプタが未実装。GC array ref ↔ linear memory の変換が必要。

## Current state

Implemented. CABI string adapters convert between GC string arrays and linear memory.

## Acceptance

- [x] string を受け取る export 関数が canonical ABI 経由で呼び出し可能
- [x] string を返す export 関数が canonical ABI 経由で呼び出し可能
- [x] GC array ref → linear memory (realloc 経由) の変換が実装される
- [x] linear memory → GC array ref の逆変換が実装される
- [x] wasmtime からの round-trip テストが pass する

## Resolution

- `ParamAdaptation::String` lifts linear memory (ptr, len) → GC `(array (mut i8))`
- `ReturnAdaptation::String` lowers GC array → linear memory + result ptr
- `cabi_realloc` exported as bump allocator
- `export_string.ark` fixture compiles to valid 1018-byte component
- Harness test passes with `component-compile` kind

## References

- `crates/ark-wasm/src/emit/t3/cabi_adapters.rs`
- `crates/ark-wasm/src/component/canonical_abi.rs`
- Component Model canonical ABI spec (string encoding)
