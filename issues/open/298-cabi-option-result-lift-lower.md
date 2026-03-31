# option / result 型の canonical ABI lift-lower を実装する

**Status**: open
**Created**: 2026-03-31
**Updated**: 2026-03-31
**ID**: 298
**Depends on**: 296
**Track**: component-model
**Blocks v1 exit**: no
**Priority**: 19

## Summary

`option<T>` と `result<T, E>` の canonical ABI アダプタが未実装。WIT 生成は可能だが lift-lower コードがない。

## Current state

- `crates/ark-wasm/src/component/wit.rs:30-34`: `WitType::Option` / `WitType::Result` が定義済み
- `crates/ark-wasm/src/emit/t3/cabi_adapters.rs`: option / result のアダプタなし
- WIT canonical ABI 上、option は discriminant + payload、result は ok/err discriminant + payload

## Acceptance

- [ ] `option<s32>` を受け取る/返す export が動作する
- [ ] `result<s32, string>` を受け取る/返す export が動作する
- [ ] wasmtime からの round-trip テストが pass する

## References

- `crates/ark-wasm/src/emit/t3/cabi_adapters.rs`
- `crates/ark-wasm/src/component/wit.rs`
