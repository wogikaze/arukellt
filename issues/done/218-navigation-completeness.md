# Navigation completeness: go to implementation / call hierarchy / type hierarchy

**Status**: done
**Created**: 2026-03-30
**Updated**: 2026-03-30
**ID**: 218
**Depends on**: 193
**Track**: parallel
**Blocks v1 exit**: no

## Summary

go to implementation、go to type definition、call hierarchy、type hierarchy、module dependency navigation を実装する。
rename・workspace symbols（#193）とは独立した navigation 機能群。

## Acceptance

- [x] `textDocument/implementation` と `textDocument/typeDefinition` が動作する
- [x] call hierarchy（`callHierarchy/incomingCalls` / `outgoingCalls`）が動作する
- [x] module dependency 間のナビゲーション導線がある

## Scope

### 定義ジャンプ系

- `textDocument/implementation` — trait メソッドの実装へジャンプ
- `textDocument/typeDefinition` — 型名の定義へジャンプ
- peek implementations / peek type definitions 体験

### 階層探索

- call hierarchy（`textDocument/prepareCallHierarchy` / incoming / outgoing calls）
- type hierarchy（`textDocument/prepareTypeHierarchy` / supertypes / subtypes）

### Module dependency navigation

- import 元モジュールへのジャンプ
- module dependency graph の可視化（#204 連携）
- 循環 import の警告と視覚化

## References

- `issues/open/185-lsp-ide-workflows-rename-code-actions-formatting.md`
- `issues/open/193-refactor-search-and-workspace-navigation-surface.md`
- `issues/open/204-project-explain-build-explain-and-script-sandbox-surface.md`
- `crates/ark-lsp/src/lib.rs`
