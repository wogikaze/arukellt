# 失敗ログの検証面（output channel・status bar・user message）を確立する

**Status**: done
**Created**: 2026-03-30
**Updated**: 2026-03-30
**ID**: 275
**Depends on**: 272
**Track**: main
**Blocks v1 exit**: no

## Summary

binary missing・LSP クラッシュ・task 失敗などのエラー時に、ユーザーへの通知が output channel・status bar・user message notification の各面で正しく機能しているかが検証されていない。

## Acceptance

- [ ] binary missing 時に output channel にエラーが記録されることをテストで確認できる
- [ ] binary missing 時に status bar にエラー表示が出ることをテストで確認できる
- [ ] LSP 起動失敗時に user message notification が表示されることをテストで確認できる
- [ ] 各面のメッセージ内容が「何が足りないか・どこを直すか」を案内する内容になっていることを確認できる

## Scope

- output channel のログ取得・検証ヘルパーの実装
- status bar アイテムの状態検証ヘルパーの実装
- user message notification の検証ヘルパーの実装
- 各エラー条件に対してヘルパーを使ったテストを追加

## References

- `extensions/arukellt-all-in-one/src/extension.js`
- `issues/open/240-actionable-error-guidance-implementation.md`
- `issues/open/272-extension-install-activate-binary-e2e.md`
- `issues/open/254-vscode-extension-e2e.md`
