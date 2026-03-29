# Debug UX: source-level debugging と DAP surface

**Status**: open
**Created**: 2026-03-29
**Updated**: 2026-03-29
**ID**: 187
**Depends on**: none
**Track**: parallel
**Blocks v1 exit**: no

## Summary

`arukellt-all-in-one` に debugger を載せるには、拡張側の UI だけでは足りない。コンパイラ / 実行系 / CLI 側に、breakpoint 解決、stack frame、locals、evaluate、step 実行を支える debug surface が必要である。

現状の repo には `ARUKELLT_DUMP_PHASES` などの dump / debug aid はあるが、source-level debugging を支える public surface や DAP adapter は存在しない。本 issue では Wasm 実行を含む Arukellt のデバッグ contract を定義し、VS Code から扱える最小実装を作る。

## 受け入れ条件

1. emitted artifact か sidecar metadata に、source file / line / column と実行位置を対応付ける情報が含まれる
2. `arukellt debug-adapter` もしくは同等の stdio DAP endpoint が存在する
3. launch / attach の起動契約が定義され、少なくとも 1 つの標準実行経路（例: `wasm32-wasi-p2` + wasmtime）で動作する
4. breakpoint、continue、step in / over / out、call stack、locals、evaluate が最小限動作する
5. 例外 / panic / trap / assertion failure 時に、ユーザーが source location まで戻れる
6. docs / smoke tests / sample `launch.json` がある

## 実装タスク

1. debug metadata の形式（DWARF / custom map / sidecar）を決める
2. 実行系との接続方法を定義し、DAP adapter を実装する
3. variables / scopes / stack frame 取得の surface を定義する
4. panic / trap / assertion failure と source mapping を接続する
5. VS Code launch config 例と troubleshooting docs を整備する

## 参照

- `docs/current-state.md`
- `docs/compiler/bootstrap.md`
- `docs/compiler/diagnostics.md`
- `docs/compiler/pipeline.md`
- `crates/arukellt/src/main.rs`
