# `std::wasi::<capability>` namespace 導入と target-gated 診断

**Status**: open
**Created**: 2026-03-29
**Updated**: 2026-03-29
**ID**: 137
**Depends on**: —
**Track**: wasi-feature
**Blocks v1 exit**: no

## Summary

`use std::wasi::<capability>` を compiler / stdlib / generated docs に通し、
target ごとに使える capability を compile-time に判定する。
`std::wasi::http` のような P2 専用 module は T1 で明示的に reject し、
`std::wasi::cli` のような共通 capability は T1/T3 の両方で使えるようにする。

## 受け入れ条件

1. `std::wasi::<capability>` module を `std/manifest.toml` と resolver が理解する
2. unsupported target/module 組み合わせで専用 diagnostics を返す
3. T1 success, T3 success, T1 reject, T3 reject-not-expected を網羅する compile fixture を追加する
4. `cargo test --workspace --exclude ark-llvm --exclude ark-lsp` が通る
5. `bash scripts/verify-harness.sh --quick` が status 0

## 実装タスク

1. stdlib module naming / manifest metadata に target support を追加する
2. resolver / typecheck / wasm backend で capability support matrix を参照する
3. unsupported import に対して actionable な compile-time error を出す
4. generated docs に target support を反映する

## 参照

- `docs/adr/ADR-011-wasi-host-layering.md`
- `issues/open/074-wasi-p2-native-component.md`
