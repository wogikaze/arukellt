# Selfhost resolver に visibility enforcement を実装する

**Status**: open
**Created**: 2026-03-31
**Updated**: 2026-03-31
**ID**: 310
**Depends on**: 309
**Track**: selfhost-frontend
**Blocks v1 exit**: no
**Priority**: 9

## Summary

pub/private の区別を selfhost resolver と typechecker に追加する。現在 `pub` キーワードは lexer/parser で認識されるが、resolver は全 symbol を可視扱いにしており、private symbol への外部 module からのアクセスが通ってしまう。

## Current state

- `src/compiler/lexer.ark`: `pub` を `TK_PUB` として tokenize する
- `src/compiler/parser.ark`: `pub` を fn/struct の修飾として parse する
- `src/compiler/resolver.ark`: symbol 登録時に visibility 情報を保持しない — 全 symbol が暗黙 public
- `pub(crate)` は parse されない
- field-level visibility は未実装

## Acceptance

- [ ] private symbol への外部 module からのアクセスが compile error になる
- [ ] `pub(crate)` を parse して crate 内可視として扱える
- [ ] struct field の pub/private が個別に効く
- [ ] error メッセージに visibility violation の理由が含まれる

## References

- `src/compiler/resolver.ark:22-30` — SymbolKind 定義 (visibility 情報なし)
- `crates/ark-resolve/src/bind.rs` — Rust resolver の visibility check
- `crates/ark-parser/src/ast.rs` — Visibility AST node
