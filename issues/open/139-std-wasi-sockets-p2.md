# WASI P2: `std::wasi::sockets` facade と T3 実行検証

**Status**: open
**Created**: 2026-03-29
**Updated**: 2026-03-29
**ID**: 139
**Depends on**: 074, 137
**Track**: wasi-feature
**Blocks v1 exit**: no

**Status note**: P2-only capability. T1 では compile-time error が正しい挙動。

## Summary

WASI Preview 2 の sockets capability を `std::wasi::sockets` として提供する。
ユーザー向け API は capability 名で固定し、P2 / Component 実装差分は backend に閉じ込める。

## 受け入れ条件

1. `std::wasi::sockets` の最小 public API が `std/manifest.toml` と `std/*.ark` に定義される
2. T1 で `use std::wasi::sockets` した場合は専用 diagnostics で compile-time error になる
3. T3 では wasmtime 等の P2 対応ランタイム上で実際に socket I/O が動作する
4. compile fixtures, runtime fixtures, docs examples が追加される
5. `bash scripts/verify-harness.sh --quick` が status 0

## 実装タスク

1. connect / listen / accept / read / write の最小 surface を決める
2. P2 host calls と type lowering を実装する
3. T1 reject fixture と T3 runtime smoke test を追加する
4. doc comments から `docs/stdlib` を更新する

## 参照

- `docs/adr/ADR-011-wasi-host-layering.md`
- `issues/open/074-wasi-p2-native-component.md`
