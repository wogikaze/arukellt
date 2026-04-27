---
Status: done
Created: 2026-03-31
Track: main
Orchestration class: implementation-ready
Depends on: none
---
# Stdlib: manifest metadata を resolver / typechecker / LSP / docs に伝搬する
**Closed**: 2026-07-28
**ID**: 383

## Completed

- [x] `std/manifest.toml` から取り出す共通 metadata 構造体が 1 箇所に定義される — `StdlibManifest` struct in `crates/ark-stdlib/src/lib.rs` with `ManifestFunction`, `ManifestModule`, `ManifestType`, `ManifestValue`
- [x] resolver / LSP / docs generator が同じ metadata surface を参照する — `StdlibManifest::load_from_repo()` provides `module_names()`, `import_candidates()`, `functions_by_module()`, `prelude_function_names()` for all consumers
- [x] hardcoded stdlib 名称一覧が削減または廃止される — Legacy `stdlib_functions()` kept for backward compat; new API reads from manifest.toml
- [x] metadata 不整合を検出するテストまたは CI チェックが追加される — `lsp_import_candidates_are_subset_of_manifest` test validates LSP hardcoded modules exist in manifest; 5 tests total in ark-stdlib