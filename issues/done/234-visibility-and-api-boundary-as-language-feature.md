---
Status: done
Created: 2026-03-30
Updated: 2026-03-30
ID: 234
Track: main
Depends on: 233
Orchestration class: implementation-ready
---
# visibility・public API・internal API を言語機能として確立する
**Blocks v1 exit**: yes

## Summary

`pub` / `priv` や module 境界の visibility ルールが未凍結であり、
大規模コードを書くと「どこからでもアクセスできる」か「何もアクセスできない」の二択になりやすい。
言語として visibility を機能として成立させ、意図的な API 設計ができるようにする。

## Acceptance

- [x] `pub` / `priv` / `pub(crate)` 相当の visibility 修飾子が仕様として定義されている
- [x] module 境界での public API と internal API の区別が言語機能として動作する
- [x] visibility 違反がコンパイルエラーになる
- [x] visibility ルールが仕様書・コンパイラ実装・LSP 診断で一致している

## Scope

### visibility 仕様の設計

- `pub`（パッケージ外公開）/ デフォルト（モジュール内のみ）/ `pub(crate)` 相当（パッケージ内のみ）の定義
- item 種別ごとの visibility 適用範囲（関数・型・定数・モジュール）
- visibility 修飾子の構文

### コンパイラ実装

- visibility チェックの実装（型チェックパスへの統合）
- visibility 違反のエラーメッセージの品質確保

### LSP 連携

- visibility 境界を考慮した補完（private item を外部から補完に出さない）
- visibility 違反の診断をリアルタイムで表示

## References

- `docs/language/spec.md`
- `issues/open/233-module-package-workspace-resolution-spec.md`
- `issues/open/227-document-language-contract.md`