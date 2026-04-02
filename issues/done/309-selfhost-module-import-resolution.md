# Selfhost resolver に module/import resolution を実装する

**Status**: done
**Created**: 2026-03-31
**Updated**: 2026-03-31
**ID**: 309
**Depends on**: 308
**Track**: selfhost-frontend
**Blocks v1 exit**: no
**Priority**: 4

## Summary

selfhost resolver (`src/compiler/resolver.ark`) の名前解決を flat namespace から module/import 対応に引き上げる。現在は全 symbol が一つの scope chain に乗り、`use` / `import` 文は登録されるだけで実際のモジュールから symbol を引くことができない。stdlib module (`stdio`, `fs`, `process`, `env`) も builtin として手動登録されており、manifest 駆動の module 解決にはなっていない。

## Current state

- `src/compiler/resolver.ark` (517 行): 9 種の symbol kind、scope chain walk、builtin 手動登録
- `use` 文は `NK_USE_DECL` として parse・登録されるが、対象 module のファイル読み込みや symbol import が行われない
- qualified name (`module::item`) の解決がない
- builtin として `stdio`, `fs`, `process`, `env` を手動登録 (manifest.toml 非連動)
- glob import (`use foo::*`) なし
- re-export (`pub use`) なし
- 循環 import 検出なし

## Acceptance

- [x] `use path::to::item` が実 module から symbol を import する
- [x] qualified name (`module::fn()`) が正しく解決される
- [x] `.ark` ファイル単位の module スコープ分離が動作する
- [x] 循環 import が検出され compile error になる
- [x] stdlib module が manifest.toml に基づいて解決される

## References

- `src/compiler/resolver.ark` — selfhost resolver 本体
- `crates/ark-resolve/src/bind.rs` — Rust resolver (50+ 関数)
- `crates/ark-resolve/src/` — Rust resolver 全体 (8 ファイル)
- `std/manifest.toml` — stdlib module 定義
