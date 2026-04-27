---
Status: done
Created: 2026-03-30
Updated: 2026-04-01
ID: 242
Track: main
Depends on: 241
Orchestration class: implementation-ready
---
# CI を unit/fixture/integration/packaging/editor smoke/determinism の各層で構成する
**Blocks v1 exit**: yes

## Summary

現行の CI は unit テスト・fixture ハーネスが中心であり、
packaging（配布物の品質）・editor smoke（拡張機能の基本動作）・determinism（再現性）の層が存在しない。
「fixture が緑」は「出荷品質」ではない。この issue では CI を層構造として再設計する。

## Acceptance

- [x] unit / fixture / integration / packaging / editor smoke / determinism の各層が CI として定義されている
- [x] 各層の目的・対象・合否基準が文書化されている
- [x] 本線 target の全層が CI で実行されている
- [x] 各層の失敗が個別に識別できる（どの層で落ちたかが分かる）

## Scope

### unit 層

- コンパイラ・stdlib・CLI の単体テスト
- 現行の `cargo test` ベースを整理

### fixture 層

- 現行のハーネス（`cargo test -p arukellt --test harness`）の維持
- fixture 失敗の分類（regression / known-fail / new-fail）

### integration 層

- CLI エンドツーエンドテスト（インストール→コンパイル→実行）
- cross-platform 動作確認

### packaging 層

- 配布物（バイナリ・VSIX・tarball）の生成確認
- バージョン表示・ヘルプ・最低限の CLI コマンドのスモークテスト

### editor smoke 層

- 拡張機能の起動・LSP 接続・基本診断の自動確認
- headless VS Code テスト（`@vscode/test-electron`）

### determinism 層

- 同じ入力から同じ出力が生成されることの確認
- バイナリ・Wasm の deterministic build チェック

## References

- `scripts/run/verify-harness.sh`
- `issues/open/241-define-primary-target-and-tier-others.md`
- `extensions/arukellt-all-in-one/`