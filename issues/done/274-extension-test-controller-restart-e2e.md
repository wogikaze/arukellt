---
Status: done
Created: 2026-03-30
Updated: 2026-03-30
ID: 274
Track: main
Depends on: 273
Orchestration class: implementation-ready
Blocks v1 exit: no
---
# test controller discovery と restart の E2E を実装する

## Summary

test controller discovery（`.ark` ファイルのテストが VS Code テストエクスプローラに表示されること）と拡張機能の restart コマンドが CI で保護されていない。

## Acceptance

- [x] test controller が `.ark` ソース内の test 関数を発見することをテストで確認できる
- [x] `arukellt.restartServer` コマンドが LSP を再起動することをテストで確認できる
- [x] restart 後に LSP handshake が再度成功することをテストで確認できる

## Scope

- test controller discovery テストの実装
- restart コマンド E2E テストの実装（再起動前後の状態変化を確認）
- テスト用の `.ark` サンプルファイルを `extensions/arukellt-all-in-one/src/test/fixtures/` に配置

## References

- `extensions/arukellt-all-in-one/src/extension.js`
- `issues/open/273-extension-lsp-command-task-e2e.md`
- `issues/open/254-vscode-extension-e2e.md`