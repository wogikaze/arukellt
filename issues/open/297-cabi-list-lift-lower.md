# list 型の canonical ABI lift-lower を実装する

**Status**: open
**Created**: 2026-03-31
**Updated**: 2026-03-31
**ID**: 297
**Depends on**: 296
**Track**: component-model
**Blocks v1 exit**: no
**Priority**: 17

## Summary

Component Model export で list 型を使う関数のアダプタが未実装。string 実装で確立した linear memory ↔ GC 変換を list に拡張する。

## Current state

- `crates/ark-wasm/src/emit/t3/cabi_adapters.rs`: list の ParamAdaptation / ReturnAdaptation がない
- `crates/ark-wasm/src/component/wit.rs:29`: `WitType::List(Box<WitType>)` で WIT 生成は可能

## Acceptance

- [ ] `list<s32>` 等のスカラー list を受け取る export が canonical ABI 経由で動作する
- [ ] `list<string>` 等のネスト list を受け取る export が動作する
- [ ] list を返す export が動作する
- [ ] wasmtime からの round-trip テストが pass する

## References

- `crates/ark-wasm/src/emit/t3/cabi_adapters.rs`
- `crates/ark-wasm/src/component/wit.rs`
