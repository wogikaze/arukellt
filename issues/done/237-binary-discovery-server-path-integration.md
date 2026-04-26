# バイナリ探索・server.path・起動引数の統合と自動設定を実装する

**Status**: done
**Created**: 2026-03-30
**Updated**: 2026-03-30
**ID**: 237
**Depends on**: 236
**Track**: main
**Blocks v1 exit**: yes

## Summary

拡張機能のバイナリ探索（`server.path` 設定）・起動引数・workspace 解決が揃っていないと、
初回起動で LSP が接続できず、利用者が詰まる。
この issue では、バイナリ探索から LSP 起動までを自動化し、手動設定なしで動く状態を実現する。

## Acceptance

- [x] `arukellt` バイナリが PATH・既定インストール先・`server.path` 設定の順で自動探索される
- [x] バイナリが見つからない場合に「インストール方法」を案内するメッセージが出る
- [x] `server.path` が未設定でも標準的な環境なら自動的に接続できる
- [x] 拡張機能の出力チャンネルにバイナリ探索の経緯が記録される

## Scope

### バイナリ探索ロジック

- 探索順序の実装：`server.path` 設定 → PATH → 既定インストール先（`~/.ark/bin` など）
- 各探索ステップの結果をログに記録
- 見つかったバイナリのバージョン確認

### 自動設定

- 初回起動時のバイナリ自動発見と設定への反映
- バイナリが複数見つかった場合の優先順位と警告
- `arukellt: Select Binary` コマンドによる手動選択 UI

### エラー案内

- バイナリなし時の「インストール方法」案内（README / ダウンロードリンク）
- バージョン不一致時の「アップグレード方法」案内
- Doctor ページへの導線

## References

- `extensions/arukellt-all-in-one/src/`
- `issues/open/236-cli-startup-contract-lsp-version-stdio.md`
- `issues/done/191-vscode-setup-doctor-and-environment-inspection.md` (参考)

## Completion Note

Closed 2026-04-09. Implemented discoverBinary() with ordered search (server.path > PATH > ~/.ark/bin > ~/.cargo/bin > /usr/local/bin), output channel logging at each probe step, and actionable error message with Open Output / Open Settings buttons when binary not found. showSetupDoctor and verifyBootstrap also updated.
