# LSP: rename を semantic-aware に置き換える

**Status**: open
**Created**: 2026-03-31
**Updated**: 2026-03-31
**ID**: 339
**Depends on**: 338
**Track**: lsp-semantic
**Blocks v1 exit**: no
**Priority**: 6

## Summary

`rename()` を token text 一括置換から semantic rename に置き換える。現在は同一ファイル中の同名 identifier token を全て置換するため、shadowing や別 scope の同名変数まで巻き込む。`prepare_rename()` も identifier token の上かどうかしか見ておらず、rename 対象として妥当な symbol かを検証していない。

## Current state

- `crates/ark-lsp/src/server.rs:2448-2494`: `rename()` が同一ファイル内の同名 token を全置換
- `prepare_rename()` は identifier token の位置チェックのみ — keyword や builtin への rename を拒否しない
- cross-file rename なし
- false positive: 同名の別変数、struct field 名、enum variant 名が巻き込まれる

## Acceptance

- [ ] rename が semantic symbol ID に基づき、定義と全参照のみを置換する
- [ ] shadowing された同名変数が巻き込まれない
- [ ] `prepare_rename()` が keyword / builtin / non-renamable への rename を拒否する
- [ ] cross-file rename が動作する (#333 前提)

## References

- `crates/ark-lsp/src/server.rs:2448-2494` — `rename()` text-based 実装
- `crates/ark-lsp/src/server.rs:2496-2520` — `prepare_rename()` identifier check のみ
