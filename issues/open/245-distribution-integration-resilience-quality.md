# 配布物・統合面・失敗時の回復性の出荷品質基準を策定する

**Status**: open
**Created**: 2026-03-30
**Updated**: 2026-03-30
**ID**: 245
**Depends on**: 241, 242, 243
**Track**: main
**Blocks v1 exit**: yes

## Summary

「fixture が緑」は「出荷品質」ではない。
配布物（バイナリ・VSIX）・統合面（インストール直後の動作）・失敗時の回復性（エラーからの復旧）まで
保証して初めて「出荷可能」と言える。この issue でその基準を策定・実装する。

## Acceptance

- [ ] 配布バイナリが「インストール → hello world」まで動くことが CI で確認されている
- [ ] VSIX が「インストール → 拡張機能有効化 → LSP 接続」まで動くことが CI で確認されている
- [ ] 失敗時（バイナリ破損・設定不整合・LSP クラッシュ）からの回復手順が文書化されている
- [ ] 出荷品質チェックリストが存在し、リリース前に実行される

## Scope

### 配布物の品質検証

- バイナリの最小スモークテスト（`arukellt --version`・`arukellt run hello.ark`）の CI 化
- VSIX パッケージの自動生成と基本動作確認
- tarball / installer の整合性チェック（checksums）

### 統合面のテスト

- 「クリーンな環境」でのインストールから動作確認まで自動化
- 各 OS（Linux / macOS / Windows）での最低限の動作確認

### 失敗時の回復性

- LSP クラッシュ後の自動再起動の確認
- 設定不整合からのリセット手順の文書化
- バイナリ破損検出と再インストール案内

### 出荷チェックリスト

- `docs/release-checklist.md` の作成
- 各項目に担当（CI 自動 / 手動確認）を明記

## References

- `issues/open/241-define-primary-target-and-tier-others.md`
- `issues/open/242-ci-layer-structure.md`
- `issues/open/225-document-release-criteria-based-on-guarantees.md`
- `extensions/arukellt-all-in-one/`
