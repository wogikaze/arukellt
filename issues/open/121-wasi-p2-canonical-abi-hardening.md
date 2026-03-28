# WASI P2: Canonical ABI ハンドリングの堅牢化

**Status**: open
**Created**: 2026-03-28
**Updated**: 2026-03-28
**ID**: 121
**Depends on**: 074
**Track**: wasi-feature
**Blocks v4 exit**: no

## Summary

WASI P2 の Component Model では、Canonical ABI (Lift/Lower 規則) が
全てのインターフェース呼び出しの型変換を定義する。
現在の `ark-wasm/src/component/canonical_abi.rs` の Lift/Lower 実装を
`docs/spec/spec-WASI-0.2.10/OVERVIEW.md` の WIT 型規則に照合して完全性を検証・修正する。

## 受け入れ条件

1. WIT の全型 (`bool`, `u8`〜`u64`, `s8`〜`s64`, `f32`, `f64`, `char`, `string`,
   `list<T>`, `record`, `variant`, `enum`, `option<T>`, `result<T,E>`, `tuple`, `resource`) の
   Lift/Lower が `canonical_abi.rs` に実装されていることを確認
2. 各型についてラウンドトリップテスト (Lower → Lift で元の値に戻ること)
3. 未実装型のパニックを適切なエラーに変換

## 参照

- `docs/spec/spec-WASI-0.2.10/OVERVIEW.md` §WIT形式の読み方
- `crates/ark-wasm/src/component/canonical_abi.rs`
