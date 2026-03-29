# v5 Driver: file loading and pipeline orchestration

**Status**: open
**Created**: 2026-03-29
**ID**: 176
**Depends on**: 162, 175
**Track**: main
**Blocks v1 exit**: no

## Summary

入力ファイルの読み込み、lexer→parser 呼び出し、結果出力を selfhost driver に接続する。Phase 2 以降は resolver / typechecker / backend の接続点にもなるため、CLI surface とは別 issue にする。

## Acceptance

- [ ] ファイル読み込みから parser 実行までの orchestration が追跡できる
- [ ] pipeline entry / exit での error propagation が整理されている
- [ ] 後続の resolver / typechecker / backend が接続される位置が明確

## References

- `issues/open/175-v5-driver-cli-surface.md`
- `crates/ark-driver/src/session.rs`
