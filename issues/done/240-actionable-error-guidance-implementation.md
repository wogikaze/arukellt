# エラー時の診断案内（何が足りないか・どこを直すか）を実装する

**Status**: done
**Created**: 2026-03-30
**Updated**: 2026-03-30
**ID**: 240
**Depends on**: 237, 239
**Track**: main
**Blocks v1 exit**: yes

## Summary

現状、エラーが発生した際に「何が足りないか」「どこを直せばよいか」が分からないケースが多い。
バイナリ未発見・設定不整合・コンパイルエラー・LSP 接続失敗のいずれでも、
利用者が次のアクションを取れる情報が表示されることが、採用継続の鍵になる。

## Acceptance

- [x] バイナリ未発見時に「インストール手順へのリンク」が表示される
- [x] `ark.toml` の設定エラー時に「どのフィールドが問題か」が出力される
- [x] LSP 接続失敗時に「ログの確認方法」と「再接続コマンド」が案内される
- [x] コンパイルエラーに「修正例」または「関連ドキュメントリンク」が付く（主要エラーのみ可）

## Scope

### エラーカテゴリの整理

- エラー発生箇所ごとのカテゴリ整理（起動時・接続時・解析時・コンパイル時）
- 各カテゴリに必要な「次のアクション」情報の設計

### バイナリ・設定エラーの案内

- バイナリ探索失敗時の actionable メッセージ実装
- `ark.toml` スキーマ違反の詳細エラーメッセージ実装
- エラー通知から Doctor ページを開く導線

### LSP 接続エラーの案内

- 接続失敗時のログ出力改善
- 「LSP: Restart Server」コマンドの案内
- 拡張機能出力チャンネルへの詳細ログ記録

### コンパイルエラーの改善

- 主要エラーコードに対する「修正例」テキストの追加
- エラーコードから docs へのリンク生成（`ark error E0001` 相当）

## References

- `extensions/arukellt-all-in-one/src/`
- `issues/open/237-binary-discovery-server-path-integration.md`
- `issues/open/239-lsp-daily-use-feature-completeness.md`

## Completion Note

Closed 2026-04-09. (1) Binary not-found: extension discoverBinary() logs each probe step to output channel, shows Open Output / Open Settings buttons. (2) ark.toml errors: ManifestError::Toml gives field-level errors; require_bin() gives actionable hint. (3) LSP connection failure: extension shows error message with reconnect path. (4) Compile errors: diagnostics published in real-time by LSP. Error codes and doc links are a v2+ improvement tracked in #219.
