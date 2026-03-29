# `std::host::*` namespace 導入と migration / target-gated 診断

**Status**: open
**Created**: 2026-03-29
**Updated**: 2026-03-29
**ID**: 137
**Depends on**: —
**Track**: wasi-feature
**Blocks v1 exit**: no

## Summary

`use std::host::*` を compiler / stdlib / generated docs に通し、
旧 `std::io/fs/env/process/cli` import と旧 prelude host API を compile-time に移行案内付きで reject する。
同時に `std::host::http` のような P2 専用 module は T1 で明示的に reject する。

## 受け入れ条件

1. `std::host::*` module を `std/manifest.toml` と resolver が理解する
2. 旧 `std::io/fs/env/process/cli` import と旧 prelude host API で専用 migration diagnostics を返す
3. unsupported target/module 組み合わせで専用 diagnostics を返す
4. T1 success, T3 success, old-import reject, old-prelude reject, T1 reject, T3 reject-not-expected を網羅する compile fixture を追加する
5. `cargo test --workspace --exclude ark-llvm --exclude ark-lsp` が通る
6. `bash scripts/verify-harness.sh --quick` が status 0

## 実装タスク

1. stdlib module naming / manifest metadata に host layering と target support を追加する
2. resolver / typecheck / wasm backend で migration diagnostics と capability support matrix を参照する
3. unsupported import / removed prelude API に対して actionable な compile-time error を出す
4. generated docs に host layering と target support を反映する

## 参照

- `docs/adr/ADR-011-wasi-host-layering.md`
- `issues/open/074-wasi-p2-native-component.md`
