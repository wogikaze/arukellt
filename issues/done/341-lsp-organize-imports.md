# LSP: organize imports を formatter 副作用から独立した semantic 操作にする

**Status**: done
**Created**: 2026-03-31
**Updated**: 2026-03-31
**ID**: 341
**Depends on**: 340
**Track**: lsp-semantic
**Blocks v1 exit**: no
**Priority**: 10

## Summary

`source.organizeImports` を formatter の full rewrite 副作用から分離し、import の追加・削除・並び替えだけを行う独立した semantic code action にする。現在は organize imports が formatter と同じ `format_source()` を呼び、ファイル全体を再整形してしまう。

## Current state

- `crates/ark-lsp/src/server.rs`: `source.organizeImports` code action が `format_source()` を呼ぶ
- formatter (`fmt.rs`) が import を stdlib-first にソートする副作用を持つが、unused import の削除はしない
- 責務が混在: import 操作と formatting が同じ entry point を共有
- unused import の検出・削除がない

## Acceptance

- [x] `source.organizeImports` が import 文のみを操作し、他のコードを変更しない
- [x] unused import が検出され、削除候補として提案される
- [x] import 順の正規化 (stdlib → project → alias) が独立操作として動作する
- [x] formatter とは別の code path で実行される

## References

- `crates/ark-lsp/src/server.rs` — `source.organizeImports` code action
- `crates/ark-parser/src/fmt.rs` — formatter の import sort 副作用
