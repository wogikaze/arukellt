# LSP: 標準ライブラリの定義解決を実装する

**Status**: open
**Created**: 2026-03-31
**Updated**: 2026-03-31
**ID**: 334
**Depends on**: 333
**Track**: lsp-navigation
**Blocks v1 exit**: no
**Priority**: 3

## Summary

`std/manifest.toml` と `std/**/*.ark` を source of truth として、stdlib の関数・型に対して go to definition / hover / signature help が動作するようにする。現在 completion に出る stdlib module 群は `server.rs` 内の固定配列 (43 builtin + 6 module) であり、`std/manifest.toml` や実ソースとは連動していない。

## Current state

- `crates/ark-lsp/src/server.rs:258-343`: 43 個の builtin 関数を hardcoded で completion に出す
- `crates/ark-lsp/src/server.rs:344-413`: 6 module (stdio, fs, env, math, string, collections) を hardcoded
- `crates/ark-stdlib/src/lib.rs`: stdlib descriptor があるが LSP から未参照
- `std/manifest.toml`: 263 関数の signature / module / doc が定義されている
- `std/**/*.ark`: 実装ソースが存在し、definition location として使える
- 現在 stdlib 関数に go to definition しても Location が返らない

## Acceptance

- [ ] `std/manifest.toml` から function signature / module / doc を LSP 起動時に読み込む
- [ ] stdlib 関数への go to definition が `std/**/*.ark` 内の実装位置を返す
- [ ] stdlib 関数への hover が signature + doc を表示する
- [ ] completion の stdlib 候補が manifest 駆動になる (hardcoded 配列を廃止)

## References

- `crates/ark-lsp/src/server.rs:258-413` — hardcoded builtin / module 一覧
- `crates/ark-stdlib/src/lib.rs` — stdlib descriptor
- `std/manifest.toml` — canonical stdlib 定義
- `std/**/*.ark` — stdlib 実装ソース
