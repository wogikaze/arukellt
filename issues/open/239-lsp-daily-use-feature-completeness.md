# 日常利用に必要な LSP 基本機能を完成させる

**Status**: open
**Created**: 2026-03-30
**Updated**: 2026-03-30
**ID**: 239
**Depends on**: 219
**Track**: main
**Blocks v1 exit**: no

## Summary

LSP が「ある」ことと「日常開発に耐える」ことは別である。
# 219 の機能群のうち、v1 リリース前に確実に動かすべき最低限のサブセット（診断・ホバー・補完・定義ジャンプ・参照検索）を先行して完成させる。
スケーラビリティや高度な機能（inlay hints・folding・linked editing 等）は #219 で継続する。

## Acceptance

- [ ] リアルタイム診断（構文エラー・型エラー）が動作する
- [ ] ホバーで型情報・ドキュメントが表示される
- [ ] 補完が contextual に動作する（キーワード・ローカル変数・stdlib API）
- [ ] 定義ジャンプ（Go to Definition）が動作する
- [ ] 参照検索（Find References）が動作する
- [ ] 大きめのプロジェクト（100 ファイル以上）でもレスポンスが 500ms 以内

## Scope

### 診断

- 構文エラーのリアルタイム表示
- 型エラー・未解決 import の診断
- 診断の範囲と精度の確認（false positive の低減）

### ホバー・補完

- 型情報・関数シグネチャのホバー表示
- stdlib API の補完（doc コメント付き）
- 補完の応答速度改善

### ナビゲーション

- Go to Definition（同一ファイル・クロスファイル・stdlib）
- Find References
- Document Symbols / Workspace Symbols

### パフォーマンス

- インクリメンタル解析の確認
- 大規模プロジェクトでのメモリ使用量の確認

## References

- `issues/open/219-lsp-standard-feature-completeness.md`
- `issues/open/218-navigation-completeness.md`
- `issues/open/238-unify-project-root-resolution-cli-lsp-tasks.md`
