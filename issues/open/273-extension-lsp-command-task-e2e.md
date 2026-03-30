# LSP handshake・command execution・task execution の E2E を実装する

**Status**: open
**Created**: 2026-03-30
**Updated**: 2026-03-30
**ID**: 273
**Depends on**: 272
**Track**: main
**Blocks v1 exit**: yes

## Summary

LSP 接続・コマンド実行・task provider の動作が CI で保護されていない。これらが壊れても手で試すまで分からない状態になっている。

## Acceptance

- [ ] LSP handshake（initialize/initialized の往復）が成功することをテストで確認できる
- [ ] `arukellt.build` 等の登録コマンドが実行できることをテストで確認できる
- [ ] task provider が `arukellt: build` タスクを返すことをテストで確認できる
- [ ] task 実行時に `ark build` が正しい引数で呼ばれることをテストで確認できる

## Scope

- LSP handshake E2E テストの実装（`vscode-languageclient` の初期化完了を待つ）
- コマンド実行テストの実装（`vscode.commands.executeCommand`）
- task provider テストの実装（`vscode.tasks.fetchTasks`）

## References

- `extensions/arukellt-all-in-one/src/extension.js`
- `issues/open/272-extension-install-activate-binary-e2e.md`
- `issues/open/254-vscode-extension-e2e.md`
