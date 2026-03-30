# project root・target・emit・manifest・scripts の CLI/LSP/Tasks 解決を統一する

**Status**: open
**Created**: 2026-03-30
**Updated**: 2026-03-30
**ID**: 238
**Depends on**: 231, 236
**Track**: main
**Blocks v1 exit**: yes

## Summary

project root・target・emit 設定・manifest・scripts の解決が CLI・LSP・Tasks provider で一致していない。
「CLI では通るが LSP ではエラー」「Tasks が別の target を見ている」という不整合が、
開発体験を根本から壊す。
この issue では全ツールが同じ解決結果を返すことを保証する。

## Acceptance

- [ ] project root の解決が CLI・LSP・Tasks provider で同じ結果を返す
- [ ] `[targets]` の選択が CLI コンパイルと LSP の解析で一致する
- [ ] `[scripts]` が CLI・Tasks provider・サイドバーで同じリストを返す
- [ ] manifest の変更後に LSP が自動再読み込みする

## Scope

### 共通解決ロジックの実装

- project root 探索を共有ライブラリまたは LSP protocol 経由で統一
- `ark.toml` の parse 結果を CLI・LSP が同一の型で保持する設計

### target 設定の統一

- CLI の `--target` フラグ・`ark.toml` の `[targets]` セクション・LSP の解析 target の優先順位
- デフォルト target の一致確認

### manifest の変更検知

- `ark.toml` の変更を FSWatcher で検知して LSP に通知
- 変更後の workspace 再初期化フローの実装

## References

- `issues/open/231-ark-toml-as-project-model-entry-point.md`
- `issues/open/232-single-file-vs-project-mode-distinction.md`
- `issues/open/236-cli-startup-contract-lsp-version-stdio.md`
- `extensions/arukellt-all-in-one/src/`
