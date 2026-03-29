# IDE surface: rename / code actions / workspace symbols / formatting

**Status**: open
**Created**: 2026-03-29
**Updated**: 2026-03-29
**ID**: 185
**Depends on**: none
**Track**: parallel
**Blocks v1 exit**: no

## Summary

現在の `ark-lsp` は diagnostics / hover / completion / definition / references / document symbols / semantic tokens までは提供しているが、`crates/ark-lsp/src/lib.rs` に明記されている通り rename / code actions / workspace symbols / incremental parsing は未対応である。

`arukellt-all-in-one` を「全部入り」の IDE 体験にするなら、単なる参照ジャンプだけでなく、編集・横断検索・自動修正・整形まで含めた authoring surface が必要になる。本 issue はその不足分を language-server 側で埋める。

## 受け入れ条件

1. `textDocument/rename` が local bindings / top-level functions / structs / enums / traits / imports に対して安全に動作する
2. `workspace/symbol` が project 全体の関数 / 型 / 定数 / trait / module を検索できる
3. `textDocument/codeAction` が少なくとも import 挿入、明白な quick fix、organize imports を提供する
4. `textDocument/formatting` が deterministic に動作し、canonical surface と矛盾しない
5. LSP test / snapshot が rename, code action, formatting, workspace symbol の代表ケースをカバーする
6. 文書サイズが大きい場合でも、必要なら差分再解析や index cache を導入して編集体験が破綻しない

## 実装タスク

1. symbol table / span index を rename と workspace symbol 用に拡張する
2. compile diagnostics と結びつく quick fix surface を定義する
3. formatter の canonical rules と出力安定性を定義する
4. LSP request / response 実装と tests を追加する
5. 必要であれば incremental parse / workspace cache を追加する

## 参照

- `crates/ark-lsp/src/lib.rs`
- `crates/ark-lsp/src/server.rs`
- `docs/compiler/diagnostics.md`
- `docs/language/spec.md`
- `issues/done/171-canonical-to-string-surface.md`
