# Setup doctor / command graph / environment inspection

**Status**: open
**Created**: 2026-03-30
**Updated**: 2026-03-30
**ID**: 191
**Depends on**: 189
**Track**: parallel
**Blocks v1 exit**: no

## Summary

初回セットアップ walkthrough・doctor 自動表示・`arukellt` binary discovery の強化・version mismatch 検知・workspace trust 対応・target ごとの必要ランタイム診断を実装する。

現状は最小の doctor コマンドのみ。binary discovery は設定値の単純読み取りにとどまり、version check・mismatch 案内・workspace trust 考慮がない。

## Acceptance

- [ ] binary discovery と version check が actionable なメッセージを出す
- [ ] 初回セットアップ walkthrough と doctor 自動表示がある
- [ ] workspace trust を考慮した安全な有効化フローがある

## Scope

### Binary discovery

- PATH / absolute path / relative path の安全診断
- 外部依存ツールの検出（wasm-tools, wasmtime 等）
- version mismatch 検知と案内
- one-click で docs を開く導線

### Setup walkthrough

- 初回有効化時の walkthrough 自動表示
- 必要ツールのインストール案内（actionable ボタン付き）
- target ごとの必要ランタイム診断

### Workspace trust

- trust 未付与時の機能制限フロー
- trust 付与後の自動再起動 / 再診断

### Doctor command

- `Arukellt: Setup Doctor` の詳細化
- PATH / version / target / missing tool を一覧表示
- 各診断項目に fix / docs リンクを付与

## References

- `issues/open/189-vscode-extension-package-and-language-client-bootstrap.md`
- `issues/open/184-vscode-extension-foundation.md`
- `extensions/arukellt-all-in-one/src/`
