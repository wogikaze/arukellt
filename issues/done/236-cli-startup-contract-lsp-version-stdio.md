# CLI 起動契約を明確化する（LSP 起動方法・バージョン検出・stdio 扱い）

**Status**: open
**Created**: 2026-03-30
**Updated**: 2026-03-30
**ID**: 236
**Depends on**: none
**Track**: main
**Blocks v1 exit**: yes

## Summary

CLI の起動契約が明確でないため、LSP クライアント（VS Code 拡張など）が正しく接続できない場合がある。
LSP 起動方法・バージョン検出・stderr/stdout の扱いが定義されていないと、初回起動で壊れる。
この issue では CLI の起動インターフェースを契約として文書化・実装する。

## Acceptance

- [x] `ark lsp` の起動引数・stdio 使用方法が文書化されている
- [x] `ark --version` が機械可読なフォーマットで バージョンを返す
- [x] stderr と stdout の用途が明確に分離されている（診断 vs プロトコル）
- [x] 拡張機能が CLI なしで起動した場合に actionable なエラーを出す

## Scope

### LSP 起動インターフェース

- `ark lsp` コマンドの引数・フラグの仕様化
- stdio / socket / pipe の通信方式の選択と文書化
- 起動時の capability negotiation の動作確認

### バージョン検出

- `ark --version` の出力フォーマット仕様（semver）
- 拡張機能がバージョンを解析して互換性チェックする仕組み
- 最小要求バージョンの設定

### stdio 分離

- LSP プロトコルメッセージは stdout のみ
- 診断・ログ・エラーメッセージは stderr のみ
- 現行実装の確認と修正

## References

- `extensions/arukellt-all-in-one/src/`
- `issues/open/237-binary-discovery-server-path-integration.md`
- `issues/open/238-unify-project-root-resolution-cli-lsp-tasks.md`
