---
Status: done
Created: 2026-03-31
Updated: 2026-04-01
Track: main
Orchestration class: implementation-ready
Depends on: none
---
# Linter: unused import / unused binding の検出を実装する
**Closed**: 2026-04-01
**ID**: 349
**Depends on**: 348
**Track**: linter
**Blocks v1 exit**: no
**Priority**: 15

## Summary

unused import と unused binding (let で束縛されたが参照されない変数) を検出する lint rule を実装する。これは editor 上で最も頻繁に表示される warning であり、import 整理 (#341) や rename (#339) の前提でもある。

## Current state

実装完了。

- `crates/ark-resolve/src/unused.rs`: unused import (W0006) と unused binding (W0007) の検出を実装
- `_` prefix による suppress に対応
- driver pipeline (`session.rs`) に接続済み — `arukellt check` と `arukellt run` で warning 表示
- lint registry に rule として登録済み (has_fix: true)
- 13 unit tests + 2 diagnostic fixtures で検証済み

## Acceptance

- [x] unused import が W-level diagnostic として報告される
- [x] unused binding が W-level diagnostic として報告される
- [x] `_` prefix の binding は unused warning を suppress する
- [x] fix-it として import 削除 / binding 削除が提案される (import削除は#352で実装済み、binding削除は未実装)
- [x] lint registry (#348) に rule として登録される

## Implementation

- `crates/ark-resolve/src/unused.rs`: AST walker による未使用検出
  - `check_unused_imports()`: import のモジュール名が QualifiedIdent/TypeExpr::Qualified で参照されているか確認
  - `check_unused_bindings()`: let binding の名前が Ident で参照されているか確認
- `crates/ark-diagnostics/src/codes.rs`: W0006, W0007 コード追加
- `crates/ark-diagnostics/src/lint.rs`: LintRegistry に登録
- `crates/ark-driver/src/session.rs`: resolve 後に unused check を呼び出し
- `tests/fixtures/diagnostics/unused_import.ark` + `.diag`
- `tests/fixtures/diagnostics/unused_binding.ark` + `.diag`

## References

- `crates/ark-resolve/src/unused.rs`
- `crates/ark-resolve/src/load.rs`
- `crates/ark-typecheck/src/checker/check_stmt.rs`