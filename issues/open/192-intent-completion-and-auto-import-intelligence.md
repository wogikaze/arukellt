# Intent completion + auto import intelligence

**Status**: open
**Created**: 2026-03-30
**Updated**: 2026-03-30
**ID**: 192
**Depends on**: none
**Track**: parallel
**Blocks v1 exit**: no

## Summary

LSP completion の品質向上と auto import、missing / unused import の quick fix、organize imports を実装する。
現状の completion は宣言済みシンボルの基本補完にとどまり、import 自動挿入・import 整理・import エラーへの quick fix がない。

## Acceptance

- [ ] auto import completion が動作する（補完選択時に import を自動挿入）
- [ ] missing import / unused import の quick fix がある
- [ ] organize imports が動作する

## Scope

### Auto import completion

- 補完候補選択時に `import` 文を自動挿入
- 同名シンボルが複数モジュールに存在する場合の候補表示
- stdlib / 外部モジュール / ローカルモジュール別の候補順

### Import quick fixes

- missing import: `E` code で import quick fix を提示
- unused import: `W` code + quick fix で import を除去
- ambiguous import: 候補リストから選択する quick pick

### Organize imports

- `source.organizeImports` code action
- on save format と連携するオプション
- import 並び替えルール（stdlib → external → local）

### Navigation 連携

- go to definition から未 import シンボルへの対応
- workspace symbol search からの auto import

## References

- `issues/open/185-lsp-ide-workflows-rename-code-actions-formatting.md`
- `issues/open/193-refactor-search-and-workspace-navigation-surface.md`
- `crates/ark-lsp/src/lib.rs`
- `crates/ark-lsp/src/server.rs`
