---
Status: done
Created: 2026-03-30
Updated: 2026-03-30
ID: 212
Track: parallel
Depends on: 190
Orchestration class: implementation-ready
---
# Extension sidebar + project UI tree view
**Blocks v1 exit**: no

## Summary

VS Code の Explorer / Sidebar 向けの専用 view を実装する。
symbols / modules / scripts / tests をまとめた tree view、context menu からの実行導線、editor title / editor context / explorer context への command 追加、setup walkthrough を含む。

現状は output channel と status bar のみで、サイドバー専用 view がなく、context menu からの実行導線もない。

## Acceptance

- [x] サイドバーに専用 view があり、modules / scripts / tests を tree 表示できる
- [x] context menu からの run / check / test 実行導線がある
- [x] editor title / editor context / explorer context に主要 command が追加されている

## Scope

### Sidebar view

- `arukellt` container view の登録
- project overview tree（modules / scripts / tests / targets）
- tree item クリックでファイルを開く導線
- refresh ボタン

### Context menus

- explorer context: `Run File` / `Check File` / `Compile File`
- editor title: `Run` / `Check` / `Compile` ボタン
- editor context: `Run Selection` / `Explain Code`（#205 連携）

### Quick pick execution

- `Arukellt: Run…` — ファイル / スクリプト / テストを quick pick で選択して実行
- `Arukellt: Test…` — テスト quick pick
- `Arukellt: Script…` — script quick pick（#203 連携）

## References

- `issues/open/190-vscode-commands-tasks-and-status-surfaces.md`
- `issues/open/184-vscode-extension-foundation.md`
- `issues/open/203-script-run-and-script-list-cli-surface.md`
- `extensions/arukellt-all-in-one/src/`