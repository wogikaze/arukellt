# Component Model を「相互運用できる」状態へ移す

**Status**: open
**Created**: 2026-03-31
**ID**: 284
**Depends on**: —
**Track**: main
**Blocks v1 exit**: no
**Priority**: 4

## Summary

`--emit component` と `--emit wit` は載っているが、canonical ABI coverage は未完了で、async もなく、jco 側は upstream block のまま。つまり今は「コンポーネント生成機能」はあるが、「安定した接続先のある製品機能」にはなっていない。対応範囲を先に固定し、その範囲だけ ABI テストを厚くする。

## Current state

- ✅ `--emit component` で `.component.wasm` 生成可能
- ✅ `--emit wit` で WIT 生成可能
- ✅ enum / record の canonical ABI adapter 実装済み（28 テスト pass）
- 🔴 string / list / complex 型の canonical ABI lift-lower は未完了
- 🔴 async Component Model 未対応
- 🔴 jco browser-facing flow は upstream block（`issues/blocked/037`）
- 🔴 multi-export world（複数関数 export）の interop テストが不十分
- 🔴 他言語ホスト（Python / Rust / Go）との相互運用テストがない

## Acceptance

- [ ] 公式サポートする型の範囲が明文化される（i32, i64, f32, f64, bool, string, enum, record, option, result, list）
- [ ] string 型の canonical ABI lift-lower が実装され、テストがある
- [ ] list 型の canonical ABI lift-lower が実装され、テストがある
- [ ] option / result 型の canonical ABI lift-lower が実装され、テストがある
- [ ] multi-export world のテストが追加される
- [ ] wasmtime からの呼び出しテストが CI で pass する
- [ ] jco の blocked 状態が `issues/blocked/037` に正確に記録されている
- [ ] `docs/current-state.md` §V2 の carry-over limitations が更新される
- [ ] サポート対象外の型を export しようとした場合に明確な compile error が出る

## Approach

1. 対応型の tier を定義: Tier 1 (scalar + enum + record), Tier 2 (string + list + option + result), Tier 3 (resource + stream + future)
2. Tier 2 の canonical ABI adapter を `cabi_adapters.rs` に実装
3. 各型に対する round-trip テストを `tests/component-interop/` に追加
4. wasmtime CLI での呼び出しテストを CI に追加
5. 未対応型の export に対する compile error を追加
6. jco の upstream status を確認し `issues/blocked/037` を更新
7. docs 更新

## References

- `crates/ark-wasm/src/emit/t3/cabi_adapters.rs`
- `crates/ark-wasm/src/emit/t3/mod.rs`
- `tests/component-interop/`
- `issues/blocked/037-jco-gc-support.md`
- `docs/current-state.md` §V2
