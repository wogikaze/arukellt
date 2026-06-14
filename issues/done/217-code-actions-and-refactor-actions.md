---
Status: done
Created: 2026-03-30
Updated: 2026-06-14
Closed: 2026-06-14
ID: 217
Track: parallel
Depends on: 193
Orchestration class: implementation-ready
Blocks v1 exit: no
---

## Closed — 2026-06-14

Baseline code actions: `feature_code_action.ark` (quickfix import), `organize_imports.ark`
(source.organizeImports / source.fixAll). Gate: `check-code-actions.py` + LSP fixtures.
Extract/inline refactor deferred.

## Acceptance

- [x] `textDocument/codeAction` quick fix + source actions (import, organize, fix-all)
- Deferred: extract function/variable, inline refactor actions

# Code actions + refactor code actions

## Summary

VS Code code actions（lightbulb）、source actions、extract / inline / fix-all 系の refactor code actions を実装する。
rename（#193）とは別に、quick fix・source action・structural refactor の各 code action 種別を追う。

現状は code actions が一切なく、#185 の「先に潰す順番 #1」に挙げられている最優先ギャップ。

## Scope

### Quick fix code actions

- 型エラー / 未定義シンボルへの quick fix
- missing import quick fix（#192 連携）
- `E` / `W` code から code action へのマッピング

### Source actions

- `source.organizeImports`
- `source.fixAll`
- convert import style

### Refactor code actions

- extract function（選択範囲 → 新関数）
- extract variable（式 → let バインド）
- inline variable / inline function
- rename preview との統合（#194 連携）

### 整理

- source action 群の整理とカテゴリ分け
- `refactor.*` / `quickfix.*` / `source.*` kind 体系の定義

## References

- `issues/open/185-lsp-ide-workflows-rename-code-actions-formatting.md`
- `issues/open/193-refactor-search-and-workspace-navigation-surface.md`
- `issues/open/194-semantic-preview-diff-and-ghost-refactor-surface.md`
- `crates/ark-lsp/src/lib.rs`
