---
Status: done
Created: 2026-03-30
Updated: 2026-03-30
ID: 272
Track: main
Depends on: 271
Orchestration class: implementation-ready
Blocks v1 exit: yes
---
# install/activate/binary discovery の E2E を実装する

## Summary

拡張機能の基本ライフサイクル（インストール・アクティベーション）と binary discovery（`server.path` 設定・binary missing 時のエラー）を E2E テストで保護する。

## Acceptance

- [x] `activate` が成功することをテストで確認できる
- [x] binary が見つからない場合に適切なエラーメッセージが表示されることをテストで確認できる
- [x] `arukellt.server.path` にカスタムパスを設定した場合に、そのパスが LSP 起動に使われることをテストで確認できる
- [x] 上記テストが 271 で配線した test runner で実行される

## Scope

- `extensions/arukellt-all-in-one/src/test/` に E2E テストファイルを作成
- `activate` / `deactivate` / binary missing / custom `server.path` の各テストケースを実装
- モック binary または stub を用いた binary discovery テスト

## References

- `extensions/arukellt-all-in-one/src/extension.js`
- `issues/open/271-vscode-test-runner-wiring.md`
- `issues/open/254-vscode-extension-e2e.md`