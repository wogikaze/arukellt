# Selfhost resolver に module/import resolution を実装する

**Status**: done
**Created**: 2026-03-31
**Updated**: 2026-04-14
**Closed**: 2026-04-14
**ID**: 309
**Depends on**: 308
**Track**: selfhost-frontend
**Blocks v1 exit**: no
**Priority**: 4

## Summary

selfhost resolver (`src/compiler/resolver.ark`) の名前解決を flat namespace から module/import 対応に引き上げる。現在は全 symbol が一つの scope chain に乗り、`use` / `import` 文は登録されるだけで実際のモジュールから symbol を引くことができない。stdlib module (`stdio`, `fs`, `process`, `env`) も builtin として手動登録されており、manifest 駆動の module 解決にはなっていない。

## Implementation

- `src/compiler/resolver.ark`: `resolve_program()`, `ModuleDecls` struct, `register_module_export()`, NK_PATH() visibility-aware qualified resolution
- `src/compiler/driver.ark`: `load_imported_modules()`, `load_single_module()`, `LoadState` with circular import detection, `is_stdlib_path()`, `parent_dir()`
- Fixtures: `tests/fixtures/selfhost/resolver_import_basic/`, `resolver_circular_import/`, `resolver_stdlib_module/`
- All three fixture sets pass; manifest.txt updated

## Acceptance

- [x] `use path::to::item` が実 module から symbol を import する
- [x] qualified name (`module::fn()`) が正しく解決される
- [x] `.ark` ファイル単位の module スコープ分離が動作する
- [x] 循環 import が検出され compile error になる
- [x] stdlib module が manifest.toml に基づいて解決される

## Close evidence

- `src/compiler/resolver.ark` — `resolve_program()` with module decl registration, NK_PATH() visibility check
- `src/compiler/driver.ark` — `load_single_module()` reads .ark files from disk, parses them; circular detection via loading_stack
- `tests/fixtures/selfhost/resolver_import_basic/main.ark` passes (output: 42\n42)
- `tests/fixtures/selfhost/resolver_circular_import/main.ark` passes (diag: circular import detected)
- `tests/fixtures/selfhost/resolver_stdlib_module/main.ark` passes (output: stdlib module resolved)
- `bash scripts/run/verify-harness.sh --quick` → 19/19 PASS

## References

- `src/compiler/resolver.ark` — selfhost resolver 本体
- `crates/ark-resolve/src/bind.rs` — Rust resolver (50+ 関数)
- `crates/ark-resolve/src/` — Rust resolver 全体 (8 ファイル)
- `std/manifest.toml` — stdlib module 定義
