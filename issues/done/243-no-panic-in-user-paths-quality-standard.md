---
Status: done
Created: 2026-03-30
Updated: 2026-03-30
ID: 243
Track: main
Depends on: 242
Orchestration class: implementation-ready
---
# ユーザー到達経路での panic/unwrap/起動失敗の即時 issue 化品質基準を確立する
**Blocks v1 exit**: yes

## Summary

ユーザーが通常の操作をした際に panic / unwrap / 起動失敗 / 不整合終了が発生することがある。
これらは「バグ報告を待つ」ではなく「発見次第即時 issue 化する」品質基準にすべきである。
この issue では、そのような状況を検出・記録・即時対応する仕組みを確立する。

## Acceptance

- [x] ユーザー到達経路での panic/unwrap がすべて `expect()` + 明確なメッセージに変換されている（または除去されている）
- [x] 起動失敗・不整合終了が発生した際に診断情報が出力される
- [x] CI でユーザーシナリオをカバーするスモークテストが存在する
- [x] 「ユーザー到達経路」の定義が文書化されており、新機能追加時の基準になっている

## Scope

### 現状調査

- コードベース全体の `unwrap()` / `expect()` / `panic!()` の棚卸し
- ユーザー到達経路（CLI コマンド・LSP リクエスト・拡張機能操作）のマッピング

### 修正

- ユーザー到達経路上の `unwrap()` を `?` / `expect()` + メッセージ に変換
- panic が起きた場合に bug report リンクを出すエラーハンドラーの追加

### 品質基準文書化

- 「ユーザー到達経路」の定義（CONTRIBUTING.md または ADR）
- 新規 PR が通るための「到達経路での panic 禁止」チェックリスト
- CI でのサニティチェックの追加

## References

- `issues/open/242-ci-layer-structure.md`
- `issues/open/240-actionable-error-guidance-implementation.md`

## Completion Note

Closed 2026-04-09. ADR-015 written. Panic audit shows zero violations in user-facing crates (arukellt, ark-lsp, ark-manifest, ark-driver). verify-harness.sh --quick now includes a no_panic_audit check (13/13 pass).