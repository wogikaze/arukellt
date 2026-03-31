# Extension quality / packaging / marketplace readiness

**Status**: done
**Created**: 2026-03-30
**Updated**: 2026-03-30
**ID**: 214
**Depends on**: 184, 185, 186, 187, 188
**Track**: parallel
**Blocks v1 exit**: no

## Summary

extension integration tests・smoke tests・fixture project 群・packaging check・marketplace metadata・icon / branding・changelog・release 手順・web extension 対応可否・remote / codespaces 互換性確認を整備する。

現状は品質保証の仕組みがなく、marketplace への配布に必要なメタデータ・アイコン・changelog も未整備。

## Acceptance

- [x] smoke tests / integration tests が CI で実行される
- [x] marketplace 配布に必要な metadata (icon, categories, keywords, publisher) が整っている
- [x] changelog と release 手順が文書化されている

## Scope

### Tests

- extension integration tests（VS Code test runner 使用）
- smoke tests（拡張を起動して最低限の動作確認）
- fixture project 群（ark.toml あり / なし、multi-root、stdlib のみ等）
- packaging check（`vsce package` が警告なく通る）

### Marketplace metadata

- icon / banner 画像
- categories: `Programming Languages`, `Linters`, `Debuggers`, `Testing`
- keywords, repository, bugs, homepage 各フィールド
- `engines.vscode` の適切なバージョン指定

### Branding / docs

- README に screenshot / gif、セットアップ手順、対応 target 一覧、既知の制約、troubleshooting
- CHANGELOG.md の初期エントリ
- sample `tasks.json` / `launch.json` / `ark.toml` の同梱

### Compatibility

- web extension 対応可否の調査と判断
- remote / dev container / GitHub Codespaces での動作確認
- release 手順（semver, tag, publish workflow）の文書化

## References

- `issues/open/183-vscode-arukellt-all-in-one-extension-epic.md`
- `issues/open/184-vscode-extension-foundation.md`
- `extensions/arukellt-all-in-one/`
