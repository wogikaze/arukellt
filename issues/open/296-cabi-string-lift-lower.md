# string 型の canonical ABI lift-lower を実装する

**Status**: open
**Created**: 2026-03-31
**Updated**: 2026-03-31
**ID**: 296
**Depends on**: 299
**Track**: component-model
**Blocks v1 exit**: no
**Priority**: 17

## Summary

Component Model export で string 型を使う関数のアダプタが未実装。GC array ref ↔ linear memory の変換が必要。

## Current state

- `crates/ark-wasm/src/emit/t3/cabi_adapters.rs`: string を `Scalar(I32)` にフォールバック
- `crates/ark-wasm/src/component/canonical_abi.rs:75`: `CanonicalAbiClass::String` として認識はされている
- `tests/fixtures/component/export_string.ark`: "not yet implemented" コメント

## Acceptance

- [ ] string を受け取る export 関数が canonical ABI 経由で呼び出し可能
- [ ] string を返す export 関数が canonical ABI 経由で呼び出し可能
- [ ] GC array ref → linear memory (realloc 経由) の変換が実装される
- [ ] linear memory → GC array ref の逆変換が実装される
- [ ] wasmtime からの round-trip テストが pass する

## References

- `crates/ark-wasm/src/emit/t3/cabi_adapters.rs`
- `crates/ark-wasm/src/component/canonical_abi.rs`
- Component Model canonical ABI spec (string encoding)
