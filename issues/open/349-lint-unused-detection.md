# Linter: unused import / unused binding の検出を実装する

**Status**: open
**Created**: 2026-03-31
**Updated**: 2026-03-31
**ID**: 349
**Depends on**: 348
**Track**: linter
**Blocks v1 exit**: no
**Priority**: 15

## Summary

unused import と unused binding (let で束縛されたが参照されない変数) を検出する lint rule を実装する。これは editor 上で最も頻繁に表示される warning であり、import 整理 (#341) や rename (#339) の前提でもある。

## Current state

- `crates/ark-resolve/src/load.rs`: import を解決するが、使用回数のトラッキングなし
- unused import の warning なし — `arukellt check` で報告されない
- unused binding (`let x = 1;` で `x` 未使用) の warning なし
- `_` prefix による suppress 規約もない

## Acceptance

- [ ] unused import が W-level diagnostic として報告される
- [ ] unused binding が W-level diagnostic として報告される
- [ ] `_` prefix の binding は unused warning を suppress する
- [ ] fix-it として import 削除 / binding 削除が提案される
- [ ] lint registry (#348) に rule として登録される

## References

- `crates/ark-resolve/src/load.rs` — import 解決
- `crates/ark-typecheck/src/checker/check_stmt.rs` — let binding 処理
